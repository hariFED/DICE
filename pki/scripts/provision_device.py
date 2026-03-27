"""
DICE Device Provisioning Script
---------------------------------
Provisions one or more ESP32-S3 devices for the DICE network.

For each device:
1. Generate unique ECDSA secp256k1 keypair
2. Sign a device certificate using the Intermediate CA (step-ca)
3. Flash firmware binary to device (esptool.py)
4. Write device private key + CA bundle to NVS partition (esptool.py)
5. Enable Secure Boot v2 (espefuse.py) — IRREVERSIBLE
6. Enable Flash Encryption (espefuse.py) — IRREVERSIBLE
7. Record device in provisioning database (device_registry.json)
8. Produce on-chain registration payload

Usage:
    python provision_device.py --dev --count 20     # dev mode, no real hardware
    python provision_device.py --port /dev/ttyUSB0  # single real device
    python provision_device.py --batch devices.csv  # batch from CSV
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import os
import subprocess
import sys
import tempfile
import uuid
from datetime import datetime, timezone
from pathlib import Path

# ---------------------------------------------------------------------------
# Internal imports from the same pki/scripts package
# ---------------------------------------------------------------------------
_SCRIPTS_DIR = Path(__file__).resolve().parent
_PKI_DIR = _SCRIPTS_DIR.parent

sys.path.insert(0, str(_SCRIPTS_DIR))

from gen_dev_keypair import generate_keypair  # noqa: E402

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------
_REGISTRY_PATH = _PKI_DIR / "device_registry.json"
_MANIFESTS_DIR = _PKI_DIR.parent / "build" / "manifests"  # repo root: build/manifests/
_PAYLOADS_DIR = _PKI_DIR / "build" / "registration_payloads"
_CERTS_DIR = _PKI_DIR / "certs"
_STEP_CA_URL = "https://localhost:9000"
_STEP_CA_FINGERPRINT_FILE = _PKI_DIR / "step-ca" / "ca_fingerprint.txt"

# Stub firmware hash used in dev mode (no real binary)
_DEV_FIRMWARE_HASH = "0" * 64


# ---------------------------------------------------------------------------
# Certificate helpers
# ---------------------------------------------------------------------------

def _sign_cert_step_ca(
    device_id: str,
    pubkey_pem_path: Path,
    cert_output_path: Path,
    ca_url: str = _STEP_CA_URL,
    dry_run: bool = False,
) -> tuple[str, str]:
    """Request a device certificate from step-ca.

    Returns:
        (cert_serial, cert_fingerprint) — both as hex strings.
        In dry-run/dev mode, returns stub values.
    """
    if dry_run:
        stub_serial = uuid.uuid4().hex.upper()
        stub_fp = hashlib.sha256(device_id.encode()).hexdigest().upper()
        # Write a stub self-signed cert PEM so downstream code can parse it
        _write_stub_cert_pem(cert_output_path, device_id)
        return stub_serial, stub_fp

    # Read CA fingerprint if available
    ca_fingerprint_arg: list[str] = []
    if _STEP_CA_FINGERPRINT_FILE.exists():
        fp = _STEP_CA_FINGERPRINT_FILE.read_text().strip()
        ca_fingerprint_arg = ["--ca-url", ca_url, "--root", fp]

    token_cmd = [
        "step", "ca", "token",
        f"dice-device-{device_id}",
        "--provisioner", "dice-provisioner@dice.network",
    ] + ca_fingerprint_arg

    try:
        token_result = subprocess.run(
            token_cmd, capture_output=True, text=True, timeout=30
        )
        if token_result.returncode != 0:
            raise RuntimeError(f"step ca token failed: {token_result.stderr}")
        token = token_result.stdout.strip()
    except FileNotFoundError:
        raise RuntimeError(
            "step CLI not found. Install from https://smallstep.com/cli/"
        )

    sign_cmd = [
        "step", "ca", "sign",
        "--token", token,
        str(pubkey_pem_path),
        str(cert_output_path),
    ] + ca_fingerprint_arg

    sign_result = subprocess.run(
        sign_cmd, capture_output=True, text=True, timeout=30
    )
    if sign_result.returncode != 0:
        raise RuntimeError(f"step ca sign failed: {sign_result.stderr}")

    # Parse cert to get serial + fingerprint
    return _parse_cert_metadata(cert_output_path)


def _parse_cert_metadata(cert_path: Path) -> tuple[str, str]:
    """Return (serial_hex, fingerprint_sha256_hex) for a PEM certificate."""
    from cryptography import x509
    from cryptography.hazmat.primitives import hashes, serialization

    pem_data = cert_path.read_bytes()
    # Handle both PEM and DER
    if pem_data.lstrip().startswith(b"-----"):
        cert = x509.load_pem_x509_certificate(pem_data)
    else:
        cert = x509.load_der_x509_certificate(pem_data)

    serial_hex = format(cert.serial_number, "x").upper()
    der_bytes = cert.public_bytes(serialization.Encoding.DER)
    fingerprint = hashlib.sha256(der_bytes).hexdigest().upper()
    return serial_hex, fingerprint


def _write_stub_cert_pem(cert_path: Path, device_id: str) -> None:
    """Write a stub self-signed certificate for dev mode testing."""
    from cryptography import x509
    from cryptography.hazmat.primitives import hashes, serialization
    from cryptography.hazmat.primitives.asymmetric import ec
    from cryptography.x509.oid import NameOID, ExtendedKeyUsageOID
    import datetime as dt

    key = ec.generate_private_key(ec.SECP256K1())
    subject = issuer = x509.Name([
        x509.NameAttribute(NameOID.COMMON_NAME, f"dice-device-{device_id}"),
        x509.NameAttribute(NameOID.ORGANIZATION_NAME, "DICE Network"),
    ])

    now = dt.datetime.now(dt.timezone.utc)
    cert = (
        x509.CertificateBuilder()
        .subject_name(subject)
        .issuer_name(issuer)
        .public_key(key.public_key())
        .serial_number(x509.random_serial_number())
        .not_valid_before(now)
        .not_valid_after(now + dt.timedelta(days=365 * 10))
        .add_extension(
            x509.BasicConstraints(ca=False, path_length=None),
            critical=True,
        )
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
        .sign(key, hashes.SHA256())
    )

    cert_path.parent.mkdir(parents=True, exist_ok=True)
    cert_path.write_bytes(cert.public_bytes(serialization.Encoding.PEM))


# ---------------------------------------------------------------------------
# Flash helpers (no-op in dev mode)
# ---------------------------------------------------------------------------

def _flash_firmware(port: str, firmware_path: Path | None, dry_run: bool) -> str:
    """Flash firmware to device, return SHA-256 hash of the binary."""
    if dry_run or firmware_path is None:
        print("[DEV] Skipping firmware flash (dev mode).")
        return _DEV_FIRMWARE_HASH

    firmware_bytes = firmware_path.read_bytes()
    fw_hash = hashlib.sha256(firmware_bytes).hexdigest()

    cmd = [
        "esptool.py",
        "--port", port,
        "--baud", "921600",
        "write_flash",
        "0x0",
        str(firmware_path),
    ]
    print(f"[FLASH] {' '.join(cmd)}")
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=120)
    if result.returncode != 0:
        raise RuntimeError(f"Firmware flash failed: {result.stderr}")
    return fw_hash


def _burn_security_fuses(port: str, dry_run: bool) -> None:
    """Burn Secure Boot + Flash Encryption + JTAG disable eFuses."""
    if dry_run:
        print("[DEV] Skipping eFuse burning (dev mode).")
        return

    from burn_efuse import burn_secure_boot, burn_flash_encryption, burn_jtag_disable

    ok = (
        burn_secure_boot(port, dry_run=False)
        and burn_flash_encryption(port, dry_run=False)
        and burn_jtag_disable(port, dry_run=False)
    )
    if not ok:
        raise RuntimeError("eFuse burning failed — check device connection.")


# ---------------------------------------------------------------------------
# Registry helpers
# ---------------------------------------------------------------------------

def _load_registry() -> list[dict]:
    """Load existing device_registry.json or return empty list."""
    if _REGISTRY_PATH.exists():
        with _REGISTRY_PATH.open(encoding="utf-8") as fh:
            return json.load(fh)
    return []


def _save_registry(records: list[dict]) -> None:
    """Persist the device registry to disk."""
    _REGISTRY_PATH.parent.mkdir(parents=True, exist_ok=True)
    with _REGISTRY_PATH.open("w", encoding="utf-8") as fh:
        json.dump(records, fh, indent=2)
        fh.write("\n")


def _make_device_record(
    device_id: str,
    pubkey_hex: str,
    cert_serial: str,
    cert_fingerprint: str,
    firmware_hash: str,
    mode: str,
) -> dict:
    return {
        "device_id": device_id,
        "pubkey_hex": pubkey_hex,
        "cert_serial": cert_serial,
        "cert_fingerprint": cert_fingerprint,
        "firmware_hash": firmware_hash,
        "provisioned_at": datetime.now(timezone.utc).isoformat(),
        "provisioned_by": os.environ.get("USER") or os.environ.get("USERNAME") or "unknown",
        "mode": mode,
    }


# ---------------------------------------------------------------------------
# Manifest
# ---------------------------------------------------------------------------

def _write_manifest(records: list[dict], mode: str) -> Path:
    """Write pki_manifest.json to build/manifests/."""
    _MANIFESTS_DIR.mkdir(parents=True, exist_ok=True)
    manifest_path = _MANIFESTS_DIR / "pki_manifest.json"

    manifest = {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "generated_by": os.environ.get("USER") or os.environ.get("USERNAME") or "unknown",
        "mode": mode,
        "device_count": len(records),
        "registry_path": str(_REGISTRY_PATH),
        "devices": [
            {
                "device_id": r["device_id"],
                "pubkey_hex": r["pubkey_hex"],
                "cert_serial": r["cert_serial"],
                "firmware_hash": r["firmware_hash"],
                "provisioned_at": r["provisioned_at"],
            }
            for r in records
        ],
    }

    with manifest_path.open("w", encoding="utf-8") as fh:
        json.dump(manifest, fh, indent=2)
        fh.write("\n")

    print(f"[INFO] PKI manifest written to {manifest_path}")
    return manifest_path


# ---------------------------------------------------------------------------
# Core provisioning logic
# ---------------------------------------------------------------------------

def provision_one_device(
    device_id: str,
    port: str | None,
    firmware_path: Path | None,
    mode: str,
    dry_run: bool,
) -> dict:
    """Provision a single device and return its registry record.

    Steps:
        1. Generate ECDSA secp256k1 keypair
        2. Sign device certificate (stub in dev mode)
        3. Flash firmware (skipped in dev mode)
        4. Flash NVS partition (skipped in dev mode)
        5. Burn eFuses (skipped in dev mode)
    """
    print(f"\n{'='*60}")
    print(f"Provisioning device: {device_id}  (mode={mode}, dry_run={dry_run})")
    print(f"{'='*60}")

    # 1. Generate keypair
    private_der, compressed_pubkey = generate_keypair()
    pubkey_hex = compressed_pubkey.hex()
    print(f"[OK] Generated secp256k1 keypair for {device_id}")
    print(f"     pubkey: {pubkey_hex}")

    # Save keys to a temp dir for this device
    with tempfile.TemporaryDirectory(prefix=f"dice_prov_{device_id}_") as tmp:
        tmp_path = Path(tmp)

        key_der_path = tmp_path / "device_key.der"
        key_der_path.write_bytes(private_der)
        key_der_path.chmod(0o600)

        # Reconstruct private key and save as PEM for step-ca signing
        from cryptography.hazmat.primitives import serialization as _ser
        private_key = _ser.load_der_private_key(private_der, password=None)
        pubkey_pem_path = tmp_path / "device_pubkey.pem"
        pubkey_pem_path.write_bytes(
            private_key.public_key().public_bytes(
                encoding=_ser.Encoding.PEM,
                format=_ser.PublicFormat.SubjectPublicKeyInfo,
            )
        )

        # 2. Sign device certificate
        cert_path = tmp_path / "device_cert.pem"
        cert_serial, cert_fingerprint = _sign_cert_step_ca(
            device_id=device_id,
            pubkey_pem_path=pubkey_pem_path,
            cert_output_path=cert_path,
            dry_run=dry_run,
        )
        print(f"[OK] Device certificate: serial={cert_serial}")

        # Determine CA cert to embed in NVS (use stub if not present)
        ca_cert_path = _CERTS_DIR / "intermediate_ca.crt"
        if not ca_cert_path.exists():
            ca_cert_path = _CERTS_DIR / "root_ca.crt"
        if not ca_cert_path.exists():
            # Write a stub CA cert for dev mode
            ca_cert_path = tmp_path / "stub_ca.pem"
            _write_stub_cert_pem(ca_cert_path, "stub-ca")

        # 3. Flash firmware
        firmware_hash = _flash_firmware(
            port=port or "/dev/null",
            firmware_path=firmware_path,
            dry_run=dry_run,
        )
        print(f"[OK] Firmware hash: {firmware_hash}")

        # 4. Flash NVS partition
        if dry_run:
            print("[DEV] Skipping NVS partition flash (dev mode).")
        else:
            from flash_nvs import provision_nvs
            ok = provision_nvs(
                port=port,
                private_key_path=key_der_path,
                ca_cert_path=ca_cert_path,
                device_id=device_id,
                mode=mode,
                work_dir=tmp_path / "nvs_build",
            )
            if not ok:
                raise RuntimeError(f"NVS flash failed for {device_id}")
            print(f"[OK] NVS partition flashed for {device_id}")

        # 5. Burn eFuses
        _burn_security_fuses(port=port or "", dry_run=dry_run)
        if dry_run:
            print("[OK] eFuse burn skipped (dev mode).")
        else:
            print(f"[OK] Security eFuses burned for {device_id}")

    # Build registry record
    record = _make_device_record(
        device_id=device_id,
        pubkey_hex=pubkey_hex,
        cert_serial=cert_serial,
        cert_fingerprint=cert_fingerprint,
        firmware_hash=firmware_hash,
        mode=mode,
    )
    return record


def provision_batch(
    device_specs: list[dict],
    firmware_path: Path | None,
    mode: str,
    dry_run: bool,
) -> list[dict]:
    """Provision multiple devices.

    Args:
        device_specs: List of dicts with keys: device_id, port (optional)
        firmware_path: Path to firmware binary (None in dev mode)
        mode: "devnet" or "mainnet"
        dry_run: Skip hardware operations

    Returns:
        List of new device registry records.
    """
    new_records: list[dict] = []
    failed: list[str] = []

    for spec in device_specs:
        device_id = spec["device_id"]
        port = spec.get("port")
        try:
            record = provision_one_device(
                device_id=device_id,
                port=port,
                firmware_path=firmware_path,
                mode=mode,
                dry_run=dry_run,
            )
            new_records.append(record)
            print(f"[OK] Provisioned {device_id}")
        except Exception as exc:
            print(f"[ERROR] Failed to provision {device_id}: {exc}", file=sys.stderr)
            failed.append(device_id)

    if failed:
        print(f"\n[WARN] {len(failed)} device(s) failed: {failed}", file=sys.stderr)

    return new_records


# ---------------------------------------------------------------------------
# Argument parsing
# ---------------------------------------------------------------------------

def _parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Provision DICE ESP32-S3 nodes with keys, certificates, and firmware.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Dev mode: generate 20 device identities without hardware
  python provision_device.py --dev --count 20

  # Single real device on /dev/ttyUSB0
  python provision_device.py --port /dev/ttyUSB0 --firmware build/dice_firmware.bin

  # Batch provisioning from CSV (columns: device_id,port)
  python provision_device.py --batch devices.csv --firmware build/dice_firmware.bin
""",
    )

    source_group = parser.add_mutually_exclusive_group()
    source_group.add_argument(
        "--dev",
        action="store_true",
        help="Dev mode: skip actual hardware, generate dev device identities.",
    )
    source_group.add_argument(
        "--port",
        type=str,
        default=None,
        help="Serial port for a single device (e.g. /dev/ttyUSB0).",
    )
    source_group.add_argument(
        "--batch",
        type=Path,
        default=None,
        help="CSV file with columns: device_id,port",
    )

    parser.add_argument(
        "--count",
        type=int,
        default=1,
        help="Number of device identities to generate in --dev mode (default: 1).",
    )
    parser.add_argument(
        "--firmware",
        type=Path,
        default=None,
        help="Path to compiled firmware binary (.bin). Required for real hardware.",
    )
    parser.add_argument(
        "--mode",
        choices=["devnet", "mainnet"],
        default="devnet",
        help="Network mode to embed in device NVS (default: devnet).",
    )
    parser.add_argument(
        "--device-id-prefix",
        type=str,
        default="dice",
        help="Prefix for auto-generated device IDs in --dev mode (default: dice).",
    )
    parser.add_argument(
        "--no-register-payloads",
        action="store_true",
        help="Skip generating on-chain registration payloads at the end.",
    )

    return parser.parse_args(argv)


# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def main(argv: list[str] | None = None) -> None:
    args = _parse_args(argv)

    # Determine device specs
    device_specs: list[dict] = []

    if args.dev:
        # Auto-generate N device IDs
        for i in range(1, args.count + 1):
            dev_id = f"{args.device_id_prefix}-dev-{i:03d}"
            device_specs.append({"device_id": dev_id, "port": None})
        dry_run = True
        print(f"[DEV MODE] Generating {args.count} device identity/ies (no hardware).")

    elif args.port:
        # Single real device
        device_specs.append({"device_id": f"{args.device_id_prefix}-node-001", "port": args.port})
        dry_run = False

    elif args.batch:
        # Batch from CSV
        batch_path = Path(args.batch)
        if not batch_path.exists():
            print(f"[ERROR] Batch CSV not found: {batch_path}", file=sys.stderr)
            sys.exit(1)
        with batch_path.open(encoding="utf-8") as fh:
            reader = csv.DictReader(fh)
            for row in reader:
                device_specs.append({
                    "device_id": row["device_id"].strip(),
                    "port": row.get("port", "").strip() or None,
                })
        dry_run = False

    else:
        # Default: single device in dev mode
        device_specs.append({"device_id": f"{args.device_id_prefix}-dev-001", "port": None})
        dry_run = True
        print("[INFO] No source specified; running in dev mode for 1 device.")

    # Validate firmware for real hardware provisioning
    if not dry_run and args.firmware is None:
        # Allow proceeding without firmware (some workflows flash separately)
        print(
            "[WARN] No --firmware specified. Firmware flash will be skipped.",
            file=sys.stderr,
        )
    firmware_path = args.firmware if (args.firmware and args.firmware.exists()) else None

    # Provision devices
    new_records = provision_batch(
        device_specs=device_specs,
        firmware_path=firmware_path,
        mode=args.mode,
        dry_run=dry_run,
    )

    if not new_records:
        print("[ERROR] No devices were provisioned successfully.", file=sys.stderr)
        sys.exit(1)

    # Append to registry
    existing_records = _load_registry()
    existing_ids = {r["device_id"] for r in existing_records}

    for rec in new_records:
        if rec["device_id"] in existing_ids:
            print(f"[WARN] Overwriting existing registry entry for {rec['device_id']}")
            existing_records = [r for r in existing_records if r["device_id"] != rec["device_id"]]
        existing_records.append(rec)

    _save_registry(existing_records)
    print(f"\n[OK] Device registry updated: {_REGISTRY_PATH} ({len(existing_records)} total records)")

    # Generate registration payloads
    if not args.no_register_payloads:
        from register_payload import generate_payloads
        new_ids = [r["device_id"] for r in new_records]
        generate_payloads(
            registry_path=_REGISTRY_PATH,
            output_dir=_PAYLOADS_DIR,
            device_ids=new_ids,
        )

    # Write PKI manifest
    _write_manifest(new_records, mode=args.mode)

    print(
        f"\n{'='*60}\n"
        f"[DONE] Provisioned {len(new_records)} device(s).\n"
        f"       Registry: {_REGISTRY_PATH}\n"
        f"       Manifest: {_MANIFESTS_DIR / 'pki_manifest.json'}\n"
        f"{'='*60}"
    )


if __name__ == "__main__":
    main(sys.argv[1:])
