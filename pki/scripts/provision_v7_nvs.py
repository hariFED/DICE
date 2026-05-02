"""
Provision NVS partition for v7 firmware end-to-end testing.

Unlike the existing flash_nvs.py (which writes a legacy schema with keys
`priv_key_der`, `coord_ca_cert`, `device_id`, `mode`), this writer emits
the schema the current v7 firmware actually reads:

    priv_key_der      — secp256k1 device key (DER)
    client_cert_pem   — mTLS client cert (PEM, matches device key)
    client_key_pem    — mTLS client key (PEM, same secp256k1 key)
    ca_cert_pem       — CA cert for verifying coordinator's cert (PEM)
    coordinator_uri   — wss://<ip>:<port>/ws

The DICE firmware's crypto subsystem explicitly rejects non-secp256k1 keys,
so we generate a fresh secp256k1 keypair and sign a client cert over it
using the existing CA in certs/ca.key (which can be any curve — P-256 is
fine for the signer, the subject key can still be secp256k1).

Usage (from repo root, Python with `cryptography` installed):
    python pki/scripts/provision_v7_nvs.py \\
        --ca-cert certs/ca.crt \\
        --ca-key  certs/ca.key \\
        --coordinator-uri wss://192.168.31.162:8443/ws \\
        --output-dir build/v7_nvs \\
        --device-id dice-dev-v7-001

Produces:
    build/v7_nvs/device_key.der
    build/v7_nvs/device_cert.pem
    build/v7_nvs/device_key.pem
    build/v7_nvs/nvs.csv
    build/v7_nvs/nvs.bin         (if nvs_partition_gen is importable)

The .bin file is ready to flash at partition offset 0x9000 via:
    python -m esptool --chip esp32s3 -p COM7 write_flash 0x9000 build/v7_nvs/nvs.bin
"""
from __future__ import annotations

import argparse
import csv
import datetime as dt
import subprocess
import sys
from pathlib import Path

from cryptography import x509
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import ec
from cryptography.x509.oid import ExtendedKeyUsageOID, NameOID


NVS_PARTITION_OFFSET = "0x9000"
NVS_PARTITION_SIZE = 0x6000  # 24 KiB — matches firmware/partitions.csv


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--ca-cert", required=True, type=Path)
    p.add_argument("--ca-key", required=True, type=Path)
    p.add_argument(
        "--mtls-client-cert",
        required=True,
        type=Path,
        help="Pre-existing mTLS client cert (secp256r1 — rustls doesn't accept secp256k1)",
    )
    p.add_argument(
        "--mtls-client-key",
        required=True,
        type=Path,
        help="Pre-existing mTLS client key matching --mtls-client-cert",
    )
    p.add_argument("--coordinator-uri", required=True, type=str)
    p.add_argument(
        "--output-dir",
        required=True,
        type=Path,
        help="Directory to write all provisioning artifacts into",
    )
    p.add_argument(
        "--device-id",
        default="dice-dev-v7-001",
        help="CN for the DICE identity (informational — the DICE key is a fresh secp256k1)",
    )
    p.add_argument(
        "--validity-days",
        default=365 * 5,
        type=int,
        help="Device cert validity period (days)",
    )
    return p.parse_args()


def generate_secp256k1_keypair() -> ec.EllipticCurvePrivateKey:
    """Fresh secp256k1 key — DICE firmware only accepts this curve."""
    return ec.generate_private_key(ec.SECP256K1())


def sign_device_cert(
    device_key: ec.EllipticCurvePrivateKey,
    ca_cert: x509.Certificate,
    ca_key: ec.EllipticCurvePrivateKey,
    device_id: str,
    validity_days: int,
) -> x509.Certificate:
    """Sign a device client cert using the dev CA."""
    subject = x509.Name([
        x509.NameAttribute(NameOID.ORGANIZATION_NAME, "DICE Network"),
        x509.NameAttribute(NameOID.COMMON_NAME, device_id),
    ])
    now = dt.datetime.now(dt.timezone.utc)
    cert = (
        x509.CertificateBuilder()
        .subject_name(subject)
        .issuer_name(ca_cert.subject)
        .public_key(device_key.public_key())
        .serial_number(x509.random_serial_number())
        .not_valid_before(now)
        .not_valid_after(now + dt.timedelta(days=validity_days))
        .add_extension(x509.BasicConstraints(ca=False, path_length=None), critical=True)
        .add_extension(
            x509.KeyUsage(
                digital_signature=True,
                content_commitment=False,
                key_encipherment=False,
                data_encipherment=False,
                key_agreement=False,
                key_cert_sign=False,
                crl_sign=False,
                encipher_only=False,
                decipher_only=False,
            ),
            critical=True,
        )
        .add_extension(
            x509.ExtendedKeyUsage([ExtendedKeyUsageOID.CLIENT_AUTH]),
            critical=False,
        )
        .sign(ca_key, hashes.SHA256())
    )
    return cert


def compressed_pubkey(key: ec.EllipticCurvePrivateKey) -> bytes:
    """Return the 33-byte compressed secp256k1 public key."""
    return key.public_key().public_bytes(
        encoding=serialization.Encoding.X962,
        format=serialization.PublicFormat.CompressedPoint,
    )


def write_nvs_csv(csv_path: Path, files: dict[str, Path], strings: dict[str, str]) -> None:
    """Write an NVS CSV consumable by nvs_partition_gen."""
    rows: list[list[str]] = [
        ["key", "type", "encoding", "value"],
        ["dice", "namespace", "", ""],
    ]
    for key, file_path in files.items():
        # `file` type + `string` encoding reads the file verbatim into a string blob.
        # For binary blobs (priv_key_der) use `binary`.
        encoding = "binary" if key == "priv_key_der" else "string"
        rows.append([key, "file", encoding, str(file_path)])
    for key, value in strings.items():
        rows.append([key, "data", "string", value])

    with csv_path.open("w", newline="", encoding="utf-8") as fh:
        csv.writer(fh).writerows(rows)


def build_nvs_image(csv_path: Path, output_bin: Path) -> bool:
    """Invoke nvs_partition_gen.py to produce a binary NVS image.

    Returns True on success.
    """
    # Try direct script path (ESP-IDF v5.2.6 layout).
    idf_script = Path(
        "C:/Espressif/frameworks/esp-idf-v5.2.6/components/nvs_flash/"
        "nvs_partition_generator/nvs_partition_gen.py"
    )
    if not idf_script.exists():
        # Fallback: try IDF_PATH
        import os
        idf_path = os.environ.get("IDF_PATH")
        if idf_path:
            alt = Path(idf_path) / "components/nvs_flash/nvs_partition_generator/nvs_partition_gen.py"
            if alt.exists():
                idf_script = alt

    if not idf_script.exists():
        print(f"[nvs_gen] could not find nvs_partition_gen.py", file=sys.stderr)
        return False

    cmd = [
        sys.executable,
        str(idf_script),
        "generate",
        str(csv_path),
        str(output_bin),
        hex(NVS_PARTITION_SIZE),
    ]
    print(f"[nvs_gen] {' '.join(cmd)}")
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=60)
    except FileNotFoundError:
        print("[nvs_gen] python not on PATH? unreachable", file=sys.stderr)
        return False

    if result.stdout:
        print(result.stdout, end="")
    if result.stderr:
        print(result.stderr, end="", file=sys.stderr)

    return result.returncode == 0


def main() -> None:
    args = parse_args()
    out = args.output_dir
    out.mkdir(parents=True, exist_ok=True)

    # ── 1. Load the CA (just to print info, not strictly needed now) ────
    ca_cert = x509.load_pem_x509_certificate(args.ca_cert.read_bytes())
    print(f"[ca] loaded {args.ca_cert}")
    print(f"[ca] subject = {ca_cert.subject.rfc4514_string()}")

    # ── 2. Generate fresh secp256k1 key for DICE identity ONLY ──────────
    #
    # The DICE protocol (VRF commit-reveal, payout binding) requires
    # secp256k1. rustls (used by the coordinator's mTLS) does NOT accept
    # secp256k1 client certs, so mTLS must use a separate secp256r1 key.
    device_key = generate_secp256k1_keypair()
    pubkey_compressed = compressed_pubkey(device_key)
    print(f"[dice-id] generated secp256k1 key (for priv_key_der)")
    print(f"[dice-id] compressed pubkey = {pubkey_compressed.hex()}")

    # ── 3. Load the existing mTLS client cert/key (secp256r1) ───────────
    mtls_cert_bytes = args.mtls_client_cert.read_bytes()
    mtls_key_bytes = args.mtls_client_key.read_bytes()
    mtls_cert = x509.load_pem_x509_certificate(mtls_cert_bytes)
    mtls_key = serialization.load_pem_private_key(mtls_key_bytes, password=None)
    curve_name = mtls_key.curve.name if hasattr(mtls_key, "curve") else "n/a"
    print(f"[mtls] loaded {args.mtls_client_cert}")
    print(f"[mtls] subject = {mtls_cert.subject.rfc4514_string()}")
    print(f"[mtls] curve = {curve_name}")
    if curve_name == "secp256k1":
        print(
            "[mtls] WARNING: mTLS key is secp256k1 — rustls will likely reject it. "
            "Use a P-256 / secp256r1 cert instead."
        )

    # ── 4. Serialize to files ───────────────────────────────────────────
    device_key_der_path = out / "dice_identity.der"
    client_key_pem_path = out / "mtls_client_key.pem"
    client_cert_pem_path = out / "mtls_client_cert.pem"
    ca_cert_pem_path = out / "ca_cert.pem"

    # DER for priv_key_der (firmware reads via mbedtls_pk_parse_key for DICE ops)
    device_key_der_path.write_bytes(
        device_key.private_bytes(
            encoding=serialization.Encoding.DER,
            format=serialization.PrivateFormat.PKCS8,
            encryption_algorithm=serialization.NoEncryption(),
        )
    )
    # PEM for client_key_pem — from the mTLS key, NOT the DICE key
    client_key_pem_path.write_bytes(mtls_key_bytes)
    # PEM for client_cert_pem — from the mTLS cert
    client_cert_pem_path.write_bytes(mtls_cert_bytes)
    # Copy CA cert PEM for ca_cert_pem
    ca_cert_pem_path.write_bytes(args.ca_cert.read_bytes())

    print(f"[artifacts] {device_key_der_path}    ({device_key_der_path.stat().st_size} bytes) <- priv_key_der")
    print(f"[artifacts] {client_key_pem_path}    ({client_key_pem_path.stat().st_size} bytes) <- client_key_pem")
    print(f"[artifacts] {client_cert_pem_path}   ({client_cert_pem_path.stat().st_size} bytes) <- client_cert_pem")
    print(f"[artifacts] {ca_cert_pem_path}       ({ca_cert_pem_path.stat().st_size} bytes) <- ca_cert_pem")

    # ── 5. Build NVS CSV ────────────────────────────────────────────────
    csv_path = out / "nvs.csv"
    write_nvs_csv(
        csv_path,
        files={
            "priv_key_der": device_key_der_path,
            "client_cert_pem": client_cert_pem_path,
            "client_key_pem": client_key_pem_path,
            "ca_cert_pem": ca_cert_pem_path,
        },
        strings={
            "coordinator_uri": args.coordinator_uri,
        },
    )
    print(f"[nvs] CSV written: {csv_path}")

    # ── 6. Build NVS binary ─────────────────────────────────────────────
    nvs_bin_path = out / "nvs.bin"
    if build_nvs_image(csv_path, nvs_bin_path):
        print(f"[nvs] binary image: {nvs_bin_path} ({nvs_bin_path.stat().st_size} bytes)")
    else:
        print("[nvs] image build FAILED — you'll need to run nvs_partition_gen manually")
        print("     from the CSV.")
        sys.exit(1)

    # ── 7. Print next step ──────────────────────────────────────────────
    print()
    print("=" * 64)
    print("Next step: flash NVS partition to device (app partition untouched)")
    print("=" * 64)
    print(f"  python -m esptool --chip esp32s3 -p COM7 \\")
    print(f"      write_flash {NVS_PARTITION_OFFSET} {nvs_bin_path}")
    print()
    print("Device pubkey (hex, for your records):")
    print(f"  {pubkey_compressed.hex()}")


if __name__ == "__main__":
    main()
