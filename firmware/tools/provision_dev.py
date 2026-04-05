#!/usr/bin/env python3
"""
DICE Development Provisioning Tool

Generates a secp256k1 ECDSA keypair and writes it to the ESP32-S3's NVS
partition so the firmware can boot past crypto init.

For DEVELOPMENT only — production provisioning should use a proper PKI
with CA-signed device certificates.

Usage:
    python provision_dev.py [--port COM4]

What it does:
    1. Generates a secp256k1 ECDSA private key (DER format)
    2. Generates a self-signed device certificate (PEM)
    3. Generates a self-signed CA certificate (PEM)
    4. Creates an NVS partition binary with:
       - priv_key_der: device private key (DER blob)
       - client_cert_pem: device certificate (PEM string)
       - client_key_pem: device private key (PEM string)
       - ca_cert_pem: CA certificate (PEM string)
       - coordinator_uri: default dev coordinator URI
    5. Flashes the NVS partition to the device
"""

import argparse
import os
import sys
import struct
import tempfile
import subprocess
import hashlib

# Try to use cryptography library, fall back to openssl CLI
try:
    from cryptography.hazmat.primitives.asymmetric import ec
    from cryptography.hazmat.primitives import serialization, hashes
    from cryptography.hazmat.backends import default_backend
    from cryptography import x509
    from cryptography.x509.oid import NameOID
    import datetime
    HAS_CRYPTO_LIB = True
except ImportError:
    HAS_CRYPTO_LIB = False


def generate_keys_cryptography():
    """Generate secp256k1 keypair using cryptography library."""
    # Generate secp256k1 private key
    private_key = ec.generate_private_key(ec.SECP256K1(), default_backend())

    # DER format (for NVS priv_key_der blob)
    priv_der = private_key.private_bytes(
        encoding=serialization.Encoding.DER,
        format=serialization.PrivateFormat.PKCS8,
        encryption_algorithm=serialization.NoEncryption(),
    )

    # PEM format (for NVS client_key_pem)
    priv_pem = private_key.private_bytes(
        encoding=serialization.Encoding.PEM,
        format=serialization.PrivateFormat.PKCS8,
        encryption_algorithm=serialization.NoEncryption(),
    ).decode("utf-8")

    # Generate self-signed device certificate
    subject = x509.Name([
        x509.NameAttribute(NameOID.COMMON_NAME, u"DICE-DEV-NODE"),
        x509.NameAttribute(NameOID.ORGANIZATION_NAME, u"DICE Network"),
    ])

    cert = (
        x509.CertificateBuilder()
        .subject_name(subject)
        .issuer_name(subject)  # self-signed
        .public_key(private_key.public_key())
        .serial_number(x509.random_serial_number())
        .not_valid_before(datetime.datetime.utcnow())
        .not_valid_after(datetime.datetime.utcnow() + datetime.timedelta(days=3650))
        .sign(private_key, hashes.SHA256(), default_backend())
    )

    cert_pem = cert.public_bytes(serialization.Encoding.PEM).decode("utf-8")

    # Generate self-signed CA certificate (for dev, CA = device)
    ca_key = ec.generate_private_key(ec.SECP256K1(), default_backend())
    ca_subject = x509.Name([
        x509.NameAttribute(NameOID.COMMON_NAME, u"DICE-DEV-CA"),
        x509.NameAttribute(NameOID.ORGANIZATION_NAME, u"DICE Network"),
    ])
    ca_cert = (
        x509.CertificateBuilder()
        .subject_name(ca_subject)
        .issuer_name(ca_subject)
        .public_key(ca_key.public_key())
        .serial_number(x509.random_serial_number())
        .not_valid_before(datetime.datetime.utcnow())
        .not_valid_after(datetime.datetime.utcnow() + datetime.timedelta(days=3650))
        .add_extension(x509.BasicConstraints(ca=True, path_length=0), critical=True)
        .sign(ca_key, hashes.SHA256(), default_backend())
    )
    ca_pem = ca_cert.public_bytes(serialization.Encoding.PEM).decode("utf-8")

    # Get compressed public key (33 bytes)
    pub_numbers = private_key.public_key().public_numbers()
    x_bytes = pub_numbers.x.to_bytes(32, byteorder="big")
    prefix = b"\x02" if pub_numbers.y % 2 == 0 else b"\x03"
    compressed_pubkey = prefix + x_bytes

    return priv_der, priv_pem, cert_pem, ca_pem, compressed_pubkey


# ── NVS Partition Binary Generator ──────────────────────────────────
# Minimal NVS partition binary generator (avoids dependency on ESP-IDF's
# nvs_partition_gen.py which needs the full IDF Python environment).

NVS_PAGE_SIZE = 4096
NVS_ENTRY_SIZE = 32
NVS_ENTRIES_PER_PAGE = 126  # (4096 - 32) / 32

NS_INDEX = 0  # namespace index counter


def nvs_page_header(page_num, status=0xFFFFFFFE):
    """Generate NVS page header (32 bytes)."""
    # Page header: state(4) + seq_no(4) + version(1) + unused(19) + crc32(4)
    state = status  # 0xFFFFFFFE = ACTIVE
    seq_no = page_num
    version = 0xFE  # NVS version 2
    unused = b"\xFF" * 19
    header_data = struct.pack("<II", state, seq_no) + bytes([version]) + unused
    crc = nvs_crc32(header_data)
    return header_data + struct.pack("<I", crc)


def nvs_crc32(data):
    """CRC32 used by NVS (standard zlib CRC32)."""
    import binascii
    return binascii.crc32(data) & 0xFFFFFFFF


def nvs_entry_str(ns_idx, key, value, entry_bitmap):
    """Create NVS entry for a string value. Returns list of 32-byte entries."""
    key_bytes = key.encode("utf-8")[:15].ljust(15, b"\x00") + b"\x00"

    # String data (null-terminated)
    str_data = value.encode("utf-8") + b"\x00"
    str_len = len(str_data)

    # Number of additional entries needed for the data
    span = 1 + (str_len + 31) // 32  # 1 header + ceil(data/32) data entries

    # Entry header
    entry_type = 0x21  # SZ (string)
    chunk_idx = 0xFF
    data_crc = nvs_crc32(str_data)

    # 8 bytes of data field in header entry: size(2) + rsv(2) + crc32(4)
    data_field = struct.pack("<HHI", str_len, 0xFFFF, data_crc)

    header = bytes([ns_idx, entry_type, span, chunk_idx]) + key_bytes + data_field
    header_crc = nvs_crc32(header)

    entries = [header + struct.pack("<I", header_crc)]  # 32 + 4 = ... wait

    # Actually NVS entry is 32 bytes total including the CRC
    # Let me redo: entry = ns(1) + type(1) + span(1) + chunk_idx(1) + key(16) + data(8) + crc(4) = 32
    # But that's only 32. The CRC is part of the 32.

    # Correct layout: [ns:1][type:1][span:1][chunk:1][key:16][data:8] = 28 bytes, then [crc:4] = 32 total
    entry_no_crc = bytes([ns_idx, entry_type, span, chunk_idx]) + key_bytes + data_field
    crc = nvs_crc32(entry_no_crc)
    header_entry = entry_no_crc + struct.pack("<I", crc)
    entries = [header_entry]

    # Data entries (32 bytes each, padded with 0xFF)
    for i in range(0, str_len, 32):
        chunk = str_data[i : i + 32]
        chunk = chunk.ljust(32, b"\xFF")
        entries.append(chunk)

    return entries


def nvs_entry_blob(ns_idx, key, data, entry_bitmap):
    """Create NVS entry for a blob value. Returns list of 32-byte entries."""
    key_bytes = key.encode("utf-8")[:15].ljust(15, b"\x00") + b"\x00"

    blob_len = len(data)
    span = 1 + (blob_len + 31) // 32

    entry_type = 0x41  # BLOB (legacy single-page blob)
    chunk_idx = 0xFF
    data_crc = nvs_crc32(data)

    data_field = struct.pack("<HHI", blob_len, 0xFFFF, data_crc)

    entry_no_crc = bytes([ns_idx, entry_type, span, chunk_idx]) + key_bytes + data_field
    crc = nvs_crc32(entry_no_crc)
    header_entry = entry_no_crc + struct.pack("<I", crc)
    entries = [header_entry]

    for i in range(0, blob_len, 32):
        chunk = data[i : i + 32]
        chunk = chunk.ljust(32, b"\xFF")
        entries.append(chunk)

    return entries


def nvs_entry_namespace(key):
    """Create NVS namespace entry. Returns (ns_index, entry)."""
    global NS_INDEX
    NS_INDEX += 1
    ns_idx = NS_INDEX

    key_bytes = key.encode("utf-8")[:15].ljust(15, b"\x00") + b"\x00"
    entry_type = 0x01  # NS
    span = 1
    chunk_idx = 0xFF

    # Data field for namespace: ns_index as u8 in first byte
    data_field = struct.pack("<BBBBBBBB", ns_idx, 0, 0, 0, 0, 0, 0, 0)

    entry_no_crc = bytes([0, entry_type, span, chunk_idx]) + key_bytes + data_field
    crc = nvs_crc32(entry_no_crc)
    entry = entry_no_crc + struct.pack("<I", crc)

    return ns_idx, entry


def build_nvs_partition(entries_data, partition_size=0x6000):
    """Build a complete NVS partition binary."""
    # Build bitmap and entries
    all_entries = []

    # Namespace entry
    ns_idx, ns_entry = nvs_entry_namespace("dice")
    all_entries.append(ns_entry)

    # Data entries
    for key, value, vtype in entries_data:
        if vtype == "string":
            ents = nvs_entry_str(ns_idx, key, value, None)
        elif vtype == "blob":
            ents = nvs_entry_blob(ns_idx, key, value, None)
        else:
            raise ValueError(f"Unknown type: {vtype}")
        all_entries.extend(ents)

    # Build page
    num_entries = len(all_entries)
    if num_entries > NVS_ENTRIES_PER_PAGE:
        print(f"WARNING: {num_entries} entries exceeds single page limit of {NVS_ENTRIES_PER_PAGE}")

    # Page header (32 bytes)
    page_header = nvs_page_header(0)

    # Entry state bitmap (32 bytes) — 2 bits per entry
    # 11 = empty, 10 = written, 00 = erased
    bitmap = bytearray(32)
    for i in range(32):
        bitmap[i] = 0xFF  # All empty initially

    # Mark used entries as "written" (10 in binary)
    for i in range(num_entries):
        byte_idx = i // 4
        bit_idx = (i % 4) * 2
        bitmap[byte_idx] &= ~(0x03 << bit_idx)  # Clear both bits
        bitmap[byte_idx] |= 0x02 << bit_idx  # Set to 10 (written)

    # Build page data
    page_data = page_header + bytes(bitmap)

    # Add entries
    for entry in all_entries:
        page_data += entry

    # Pad to page size
    page_data = page_data.ljust(NVS_PAGE_SIZE, b"\xFF")

    # Build full partition (remaining pages are empty/0xFF)
    partition = page_data
    remaining_pages = (partition_size // NVS_PAGE_SIZE) - 1
    partition += b"\xFF" * (remaining_pages * NVS_PAGE_SIZE)

    return partition[:partition_size]


def main():
    parser = argparse.ArgumentParser(description="DICE Development Provisioning Tool")
    parser.add_argument("--port", default="COM4", help="Serial port (default: COM4)")
    parser.add_argument("--coordinator-uri", default="ws://192.168.1.100:9001",
                        help="Coordinator WebSocket URI (default: ws://192.168.1.100:9001)")
    parser.add_argument("--no-flash", action="store_true",
                        help="Generate files only, don't flash to device")
    parser.add_argument("--output-dir", default=None,
                        help="Output directory for generated files")
    args = parser.parse_args()

    if not HAS_CRYPTO_LIB:
        print("ERROR: 'cryptography' Python package required.")
        print("Install with: pip install cryptography")
        sys.exit(1)

    print("=" * 60)
    print("DICE Development Provisioning Tool")
    print("=" * 60)

    # 1. Generate keys
    print("\n[1/4] Generating secp256k1 keypair...")
    priv_der, priv_pem, cert_pem, ca_pem, compressed_pubkey = generate_keys_cryptography()
    print(f"  Private key: {len(priv_der)} bytes (DER)")
    print(f"  Public key:  {compressed_pubkey.hex()}")
    print(f"  Device cert: {len(cert_pem)} bytes (PEM)")

    # 2. Build NVS partition
    print("\n[2/4] Building NVS partition...")
    entries = [
        ("priv_key_der", priv_der, "blob"),
        ("client_cert_pem", cert_pem, "string"),
        ("client_key_pem", priv_pem, "string"),
        ("ca_cert_pem", ca_pem, "string"),
        ("coordinator_uri", args.coordinator_uri, "string"),
    ]

    nvs_bin = build_nvs_partition(entries, partition_size=0x6000)
    print(f"  NVS partition: {len(nvs_bin)} bytes")

    # 3. Save files
    out_dir = args.output_dir or tempfile.mkdtemp(prefix="dice_prov_")
    nvs_path = os.path.join(out_dir, "nvs_provisioned.bin")
    key_path = os.path.join(out_dir, "device_key.der")
    pubkey_path = os.path.join(out_dir, "device_pubkey.hex")

    with open(nvs_path, "wb") as f:
        f.write(nvs_bin)
    with open(key_path, "wb") as f:
        f.write(priv_der)
    with open(pubkey_path, "w") as f:
        f.write(compressed_pubkey.hex() + "\n")

    print(f"\n[3/4] Files saved to: {out_dir}")
    print(f"  NVS binary:   {nvs_path}")
    print(f"  Private key:  {key_path}")
    print(f"  Public key:   {pubkey_path}")

    # 4. Flash NVS partition
    if args.no_flash:
        print("\n[4/4] Skipping flash (--no-flash)")
    else:
        print(f"\n[4/4] Flashing NVS partition to {args.port} at offset 0x9000...")
        esptool_cmd = [
            sys.executable, "-m", "esptool",
            "--chip", "esp32s3",
            "-p", args.port,
            "-b", "460800",
            "write_flash",
            "0x9000", nvs_path,
        ]
        try:
            result = subprocess.run(esptool_cmd, capture_output=True, text=True, timeout=30)
            if result.returncode == 0:
                print("  Flash successful!")
                print("\n" + "=" * 60)
                print("PROVISIONING COMPLETE")
                print("=" * 60)
                print(f"\nDevice public key: {compressed_pubkey.hex()}")
                print(f"Coordinator URI:   {args.coordinator_uri}")
                print("\nPress RESET on the device to reboot with the new identity.")
                print("The LED should go from RED to YELLOW (connecting) to GREEN (online).")
            else:
                print(f"  Flash FAILED: {result.stderr}")
        except FileNotFoundError:
            print("  ERROR: esptool not found. Flash manually:")
            print(f"  esptool.py --chip esp32s3 -p {args.port} write_flash 0x9000 {nvs_path}")
        except subprocess.TimeoutExpired:
            print("  ERROR: Flash timed out. Is the device connected?")

    print()


if __name__ == "__main__":
    main()
