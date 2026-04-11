#!/usr/bin/env python3
"""
DICE Node Provisioner — factory flash for an ESP32-S3 witness node.

Writes the **factory** portion of the device's NVS:
    - coordinator_uri   (same for every device in one deployment)
    - priv_key_der      (unique per device — VRF signing key)
    - client_cert_pem   (unique per device — mTLS cert signed by intermediate CA)
    - client_key_pem    (unique per device — mTLS key)
    - ca_cert_pem       (same for every device — CA bundle for server-cert verification)
    - node_name         (unique per device — friendly name like "Neo")

What is NOT written here (end-user sets these via captive portal on first boot):
    - wifi_ssid
    - wifi_pass
    - sol_wallet

Usage:
    python provision_node.py --port COM4 --node-name Neo
    python provision_node.py --port COM5 --node-name Trinity
    python provision_node.py --port COM6 --node-name Morpheus
    python provision_node.py --port COM7 --node-name Tank

Override coordinator URI if your LAN IP changes:
    python provision_node.py --port COM4 --node-name Neo \\
        --coordinator-uri wss://192.168.1.42:9001
"""

import argparse
import datetime
import hashlib
import os
import subprocess
import sys

# The four valid node names for this deployment. Extend this set if you add
# more devices — it's a whitelist to catch typos at flash time, not a
# technical limit.
VALID_NODE_NAMES = {"Neo", "Trinity", "Morpheus", "Tank"}

# Default coordinator URI. This is the LAN IP of the machine running the
# DICE coordinator on port 9001 over mTLS. If your coordinator has moved,
# pass --coordinator-uri explicitly.
DEFAULT_COORDINATOR_URI = "wss://192.168.1.6:9001"


def canonical_name(raw: str) -> str:
    """Normalize user-supplied node name to the canonical capitalized form."""
    if not raw:
        raise ValueError("node name cannot be empty")
    lowered = raw.strip().lower()
    for canonical in VALID_NODE_NAMES:
        if canonical.lower() == lowered:
            return canonical
    raise ValueError(
        f"invalid node name '{raw}'. Valid names: {sorted(VALID_NODE_NAMES)}. "
        f"Edit VALID_NODE_NAMES in provision_node.py to add more."
    )


def main() -> None:
    parser = argparse.ArgumentParser(
        description="DICE Node Provisioner — factory flash for ESP32-S3"
    )
    parser.add_argument(
        "--port",
        required=True,
        help="Serial port of the device (e.g. COM4, /dev/ttyUSB0)",
    )
    parser.add_argument(
        "--node-name",
        required=True,
        help=f"Friendly node name. Must be one of: {sorted(VALID_NODE_NAMES)}",
    )
    parser.add_argument(
        "--coordinator-uri",
        default=DEFAULT_COORDINATOR_URI,
        help=f"Coordinator WebSocket URI (default: {DEFAULT_COORDINATOR_URI})",
    )
    parser.add_argument(
        "--no-firmware",
        action="store_true",
        help="Skip firmware flash, only re-write NVS",
    )
    args = parser.parse_args()

    try:
        node_name = canonical_name(args.node_name)
    except ValueError as e:
        print(f"ERROR: {e}")
        sys.exit(2)

    project_dir = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    prov_dir = os.path.join(project_dir, "firmware", "tools", "prov_output")
    certs_dir = os.path.join(project_dir, "pki", "production")
    firmware_dir = os.path.join(project_dir, "firmware", "build")
    node_dir = os.path.join(prov_dir, node_name.lower())

    os.makedirs(node_dir, exist_ok=True)

    # ─── Banner + confirmation ───────────────────────────────────────────
    print("=" * 68)
    print(f"  DICE Node Provisioner — {node_name}")
    print("=" * 68)
    print(f"  Port:            {args.port}")
    print(f"  Coordinator URI: {args.coordinator_uri}")
    print(f"  Output dir:      {node_dir}")
    print("=" * 68)
    print()
    print("  ⚠  Double-check the Coordinator URI above.")
    print("     If the IP or port is wrong, the device will not connect")
    print("     and you'll have to re-flash. Press Enter to continue,")
    print("     Ctrl-C to abort.")
    try:
        input()
    except EOFError:
        pass

    # ─── Step 1: Generate secp256k1 keypair ───────────────────────────────
    print("[1/5] Generating secp256k1 keypair...")

    from cryptography.hazmat.primitives.asymmetric import ec
    from cryptography.hazmat.primitives import serialization, hashes
    from cryptography.hazmat.backends import default_backend
    from cryptography import x509
    from cryptography.x509.oid import NameOID, ExtendedKeyUsageOID

    vrf_key = ec.generate_private_key(ec.SECP256K1(), default_backend())
    vrf_der = vrf_key.private_bytes(
        serialization.Encoding.DER,
        serialization.PrivateFormat.PKCS8,
        serialization.NoEncryption(),
    )

    pub_nums = vrf_key.public_key().public_numbers()
    x_bytes = pub_nums.x.to_bytes(32, "big")
    prefix = b"\x02" if pub_nums.y % 2 == 0 else b"\x03"
    compressed_pubkey = (prefix + x_bytes).hex()

    der_path = os.path.join(node_dir, "priv_key.der")
    with open(der_path, "wb") as f:
        f.write(vrf_der)

    print(f"  VRF pubkey: {compressed_pubkey}")
    print(f"  DER key:    {len(vrf_der)} bytes → {der_path}")

    # ─── Step 2: Generate TLS client certificate ──────────────────────────
    print()
    print("[2/5] Generating TLS client certificate (signed by intermediate CA)...")

    inter_cert_path = os.path.join(certs_dir, "intermediate_ca.crt")
    inter_key_path = os.path.join(certs_dir, "intermediate_ca.key")

    if not os.path.exists(inter_cert_path) or not os.path.exists(inter_key_path):
        print(f"  ERROR: Intermediate CA not found at {certs_dir}")
        print(f"         Run pki/generate_production_pki.py first")
        sys.exit(1)

    with open(inter_cert_path, "rb") as f:
        inter_cert = x509.load_pem_x509_certificate(f.read(), default_backend())
    with open(inter_key_path, "rb") as f:
        inter_key = serialization.load_pem_private_key(
            f.read(), password=None, backend=default_backend()
        )

    # TLS key (secp256r1 — TLS requires P-256, not secp256k1)
    tls_key = ec.generate_private_key(ec.SECP256R1(), default_backend())

    dev_subject = x509.Name([
        x509.NameAttribute(NameOID.ORGANIZATION_NAME, "DICE Network"),
        x509.NameAttribute(NameOID.COMMON_NAME, f"DICE-{node_name}"),
    ])

    dev_cert = (
        x509.CertificateBuilder()
        .subject_name(dev_subject)
        .issuer_name(inter_cert.subject)
        .public_key(tls_key.public_key())
        .serial_number(x509.random_serial_number())
        .not_valid_before(datetime.datetime.utcnow())
        .not_valid_after(datetime.datetime.utcnow() + datetime.timedelta(days=3650))
        .add_extension(x509.BasicConstraints(ca=False, path_length=None), critical=True)
        .add_extension(
            x509.ExtendedKeyUsage([ExtendedKeyUsageOID.CLIENT_AUTH]),
            critical=False,
        )
        .sign(inter_key, hashes.SHA256(), default_backend())
    )

    cert_path = os.path.join(node_dir, "device.crt")
    key_path = os.path.join(node_dir, "device.key")
    with open(cert_path, "wb") as f:
        f.write(dev_cert.public_bytes(serialization.Encoding.PEM))
    with open(key_path, "wb") as f:
        f.write(tls_key.private_bytes(
            serialization.Encoding.PEM,
            serialization.PrivateFormat.PKCS8,
            serialization.NoEncryption(),
        ))

    print(f"  CN:   DICE-{node_name}")
    print(f"  Cert: {cert_path}")

    # ─── Step 3: Generate NVS partition ───────────────────────────────────
    print()
    print("[3/5] Generating NVS partition...")

    ca_bundle_path = os.path.join(certs_dir, "ca_bundle.crt")
    if not os.path.exists(ca_bundle_path):
        # Fall back to single root CA cert if no bundle exists.
        ca_bundle_path = os.path.join(certs_dir, "root_ca.crt")

    if not os.path.exists(ca_bundle_path):
        print(f"  ERROR: Neither ca_bundle.crt nor root_ca.crt exists in {certs_dir}")
        sys.exit(1)

    csv_path = os.path.join(node_dir, "nvs.csv")
    with open(csv_path, "w") as f:
        f.write("key,type,encoding,value\n")
        f.write("dice,namespace,,\n")
        f.write(f"coordinator_uri,data,string,{args.coordinator_uri}\n")
        f.write(f"node_name,data,string,{node_name}\n")
        f.write(f"priv_key_der,file,binary,{der_path}\n")
        f.write(f"client_cert_pem,file,string,{cert_path}\n")
        f.write(f"client_key_pem,file,string,{key_path}\n")
        f.write(f"ca_cert_pem,file,string,{ca_bundle_path}\n")

    # Find nvs_partition_gen.py in the ESP-IDF tree.
    idf_path = os.environ.get("IDF_PATH", "C:/Espressif/frameworks/esp-idf-v5.2.6")
    nvs_gen = os.path.join(
        idf_path, "components", "nvs_flash", "nvs_partition_generator", "nvs_partition_gen.py"
    )

    nvs_bin_path = os.path.join(node_dir, "nvs.bin")

    # Prefer ESP-IDF's bundled Python if we can find it.
    python = sys.executable
    if "espressif" not in python.lower():
        esp_python = "C:/Espressif/python_env/idf5.2_py3.11_env/Scripts/python.exe"
        if os.path.exists(esp_python):
            python = esp_python

    result = subprocess.run(
        [python, nvs_gen, "generate", csv_path, nvs_bin_path, "0x6000"],
        capture_output=True, text=True, timeout=10,
    )
    if result.returncode != 0:
        print("  ERROR: NVS generation failed")
        print(f"         stderr: {result.stderr}")
        sys.exit(1)

    print(f"  NVS binary: {nvs_bin_path}")

    # ─── Step 4: Flash firmware + NVS ─────────────────────────────────────
    if not args.no_firmware:
        print()
        print("[4/5] Flashing firmware + NVS...")

        bootloader = os.path.join(firmware_dir, "bootloader", "bootloader.bin")
        partition_table = os.path.join(firmware_dir, "partition_table", "partition-table.bin")
        firmware = os.path.join(firmware_dir, "dice_firmware.bin")

        missing = [p for p in (bootloader, partition_table, firmware) if not os.path.exists(p)]
        if missing:
            print(f"  ERROR: missing build artifact(s):")
            for p in missing:
                print(f"    {p}")
            print(f"         Run `idf.py build` in firmware/ first.")
            sys.exit(1)

        flash_cmd = [
            python, "-m", "esptool",
            "--chip", "esp32s3",
            "-p", args.port,
            "-b", "460800",
            "--before", "default_reset",
            "--after", "hard_reset",
            "write_flash",
            "0x0", bootloader,
            "0x8000", partition_table,
            "0x9000", nvs_bin_path,
            "0x10000", firmware,
        ]

        print(f"  Flashing {args.port}...")
        result = subprocess.run(flash_cmd, capture_output=True, text=True, timeout=120)
        if result.returncode != 0:
            print(f"  ERROR: flash failed")
            print(f"         stderr tail: {result.stderr[-400:]}")
            sys.exit(1)

        print("  Flash complete.")
    else:
        print()
        print("[4/5] Skipping firmware (--no-firmware), flashing NVS only...")
        flash_cmd = [
            python, "-m", "esptool",
            "--chip", "esp32s3",
            "-p", args.port,
            "-b", "460800",
            "write_flash",
            "0x9000", nvs_bin_path,
        ]
        result = subprocess.run(flash_cmd, capture_output=True, text=True, timeout=30)
        if result.returncode != 0:
            print(f"  ERROR: NVS flash failed")
            print(f"         stderr tail: {result.stderr[-400:]}")
            sys.exit(1)
        print("  NVS flash complete.")

    # ─── Step 5: Save device identity record ──────────────────────────────
    print()
    print("[5/5] Saving device identity record...")

    info_path = os.path.join(node_dir, "device_info.txt")
    device_id = hashlib.sha256(bytes.fromhex(compressed_pubkey)).hexdigest()
    with open(info_path, "w") as f:
        f.write(f"Node Name:    {node_name}\n")
        f.write(f"VRF Pubkey:   {compressed_pubkey}\n")
        f.write(f"Device ID:    {device_id}\n")
        f.write(f"TLS CN:       DICE-{node_name}\n")
        f.write(f"Coordinator:  {args.coordinator_uri}\n")
        f.write(f"Port:         {args.port}\n")
        f.write(f"Provisioned:  {datetime.datetime.now().isoformat()}\n")

    print(f"  Saved: {info_path}")

    # ─── Done ─────────────────────────────────────────────────────────────
    print()
    print("=" * 68)
    print(f"  {node_name.upper()} PROVISIONED — ready for end-user setup")
    print("=" * 68)
    print()
    print(f"  VRF Pubkey:   {compressed_pubkey}")
    print(f"  Device ID:    {device_id[:16]}...")
    print(f"  TLS CN:       DICE-{node_name}")
    print(f"  Coordinator:  {args.coordinator_uri}")
    print()
    print("  Next steps (end-user):")
    print("  1. Device boots into captive portal (blue LED breathing)")
    print(f"  2. Connect phone/laptop to WiFi network 'DICE-{node_name}'")
    print("  3. Browser opens to 192.168.4.1 automatically")
    print("  4. Enter WiFi SSID + password + Solana wallet")
    print("  5. Device tests credentials before saving — on failure you")
    print("     stay on the page with your inputs and a clear error")
    print("  6. On success, device reboots and connects to coordinator")
    print("     (green LED solid)")
    print()
    print("  To change WiFi or wallet later: hold BOOT button 5 seconds")
    print("  — device erases WiFi/wallet and returns to captive portal")
    print()


if __name__ == "__main__":
    main()
