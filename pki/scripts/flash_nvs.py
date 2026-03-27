"""
Flash NVS partition to ESP32 device.

NVS contents written per device:
- namespace "dice":
  - key "priv_key_der"  → device ECDSA private key (DER, 121 bytes)
  - key "coord_ca_cert" → coordinator CA certificate (DER)
  - key "device_id"     → device identifier string
  - key "mode"          → "devnet" or "mainnet"

Uses esptool.py nvs_partition_gen.py for building the partition image.

Usage:
    python flash_nvs.py --port /dev/ttyUSB0 \
        --private-key device_key.der \
        --ca-cert intermediate_ca.crt \
        --device-id dice-dev-001 \
        --mode devnet
"""

from __future__ import annotations

import argparse
import csv
import subprocess
import sys
import tempfile
from pathlib import Path


# Default NVS partition address for ESP32-S3 (matches partitions.csv)
NVS_PARTITION_OFFSET = "0x9000"
# NVS partition size: 24 KiB (0x6000) is a common default
NVS_PARTITION_SIZE = 0x6000


def build_nvs_csv(
    private_key_der: bytes,
    ca_cert_der: bytes,
    device_id: str,
    mode: str,
    csv_path: Path,
) -> None:
    """Write an NVS partition CSV file for nvs_partition_gen.py.

    The CSV format expected by nvs_partition_gen.py:
        key,type,encoding,value
        namespace_name,namespace,,
        key_name,data,encoding,value_or_filepath

    Binary blobs must be written as hex strings or file paths.
    We write them as temporary binary files and reference those.

    Args:
        private_key_der: Raw DER bytes of device private key
        ca_cert_der:     Raw DER bytes of CA certificate
        device_id:       Device identifier string
        mode:            "devnet" or "mainnet"
        csv_path:        Output path for the generated CSV file
    """
    # nvs_partition_gen expects binary data as file references for "binary" type
    key_blob_path = csv_path.parent / "priv_key.bin"
    cert_blob_path = csv_path.parent / "coord_ca.bin"

    key_blob_path.write_bytes(private_key_der)
    cert_blob_path.write_bytes(ca_cert_der)

    rows = [
        ["key", "type", "encoding", "value"],
        ["dice", "namespace", "", ""],
        ["priv_key_der", "file", "binary", str(key_blob_path)],
        ["coord_ca_cert", "file", "binary", str(cert_blob_path)],
        ["device_id", "data", "string", device_id],
        ["mode", "data", "string", mode],
    ]

    with csv_path.open("w", newline="", encoding="utf-8") as fh:
        writer = csv.writer(fh)
        writer.writerows(rows)

    print(f"[INFO] NVS CSV written to {csv_path}")


def generate_nvs_image(csv_path: Path, image_path: Path, size: int = NVS_PARTITION_SIZE) -> bool:
    """Invoke nvs_partition_gen.py to build the binary NVS image.

    Args:
        csv_path:   Path to the NVS CSV input file
        image_path: Output path for the binary NVS partition image
        size:       Partition size in bytes

    Returns:
        True on success.
    """
    cmd = [
        "python",
        "-m",
        "esp_idf_nvs_partition_gen.nvs_partition_gen",
        "generate",
        str(csv_path),
        str(image_path),
        str(size),
    ]

    # Fallback to direct script invocation if the module form is unavailable
    fallback_cmd = [
        "nvs_partition_gen.py",
        "generate",
        str(csv_path),
        str(image_path),
        str(size),
    ]

    for attempt_cmd in (cmd, fallback_cmd):
        print(f"[INFO] Building NVS image: {' '.join(attempt_cmd)}")
        try:
            result = subprocess.run(
                attempt_cmd,
                capture_output=True,
                text=True,
                timeout=30,
            )
            if result.returncode == 0:
                print(f"[INFO] NVS image written to {image_path}")
                return True
            print(f"[WARN] Command failed (rc={result.returncode}): {result.stderr.strip()}")
        except FileNotFoundError:
            continue

    print(
        "[ERROR] nvs_partition_gen.py not found. "
        "Install via: pip install esp-idf-nvs-partition-gen",
        file=sys.stderr,
    )
    return False


def flash_nvs_image(port: str, image_path: Path, offset: str = NVS_PARTITION_OFFSET) -> bool:
    """Flash the NVS binary image to the device using esptool.py.

    Args:
        port:       Serial port (e.g. /dev/ttyUSB0)
        image_path: Path to the binary NVS partition image
        offset:     Flash offset (hex string, default 0x9000)

    Returns:
        True on success.
    """
    cmd = [
        "esptool.py",
        "--port", port,
        "--baud", "460800",
        "write_flash",
        offset,
        str(image_path),
    ]
    print(f"[INFO] Flashing NVS to {port} @ {offset}: {' '.join(cmd)}")
    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=120,
        )
        if result.returncode != 0:
            print(f"[ERROR] esptool.py failed (rc={result.returncode}):", file=sys.stderr)
            print(result.stderr, file=sys.stderr)
            return False
        print(result.stdout)
        return True
    except FileNotFoundError:
        print(
            "[ERROR] esptool.py not found. Install via: pip install esptool",
            file=sys.stderr,
        )
        return False
    except subprocess.TimeoutExpired:
        print("[ERROR] esptool.py timed out after 120 seconds.", file=sys.stderr)
        return False


def provision_nvs(
    port: str,
    private_key_path: Path,
    ca_cert_path: Path,
    device_id: str,
    mode: str,
    work_dir: Path | None = None,
    dry_run: bool = False,
) -> bool:
    """End-to-end NVS provisioning: build image then flash.

    Args:
        port:             Serial port for flashing
        private_key_path: Path to DER private key file
        ca_cert_path:     Path to CA certificate (PEM or DER)
        device_id:        Device identifier string
        mode:             "devnet" or "mainnet"
        work_dir:         Temp directory for intermediate files (auto-created if None)
        dry_run:          If True, build image but do not flash

    Returns:
        True on success.
    """
    private_key_path = Path(private_key_path)
    ca_cert_path = Path(ca_cert_path)

    if not private_key_path.exists():
        print(f"[ERROR] Private key not found: {private_key_path}", file=sys.stderr)
        return False
    if not ca_cert_path.exists():
        print(f"[ERROR] CA cert not found: {ca_cert_path}", file=sys.stderr)
        return False

    private_key_der = private_key_path.read_bytes()

    # Accept both PEM and DER CA cert; convert PEM to DER if needed
    ca_raw = ca_cert_path.read_bytes()
    if ca_raw.lstrip().startswith(b"-----"):
        from cryptography import x509
        from cryptography.hazmat.primitives import serialization as _ser
        cert = x509.load_pem_x509_certificate(ca_raw)
        ca_cert_der = cert.public_bytes(_ser.Encoding.DER)
    else:
        ca_cert_der = ca_raw

    _cleanup = work_dir is None
    if work_dir is None:
        _tmp = tempfile.mkdtemp(prefix=f"dice_nvs_{device_id}_")
        work_dir = Path(_tmp)
    else:
        work_dir = Path(work_dir)
        work_dir.mkdir(parents=True, exist_ok=True)

    csv_path = work_dir / "nvs_data.csv"
    image_path = work_dir / "nvs.bin"

    build_nvs_csv(
        private_key_der=private_key_der,
        ca_cert_der=ca_cert_der,
        device_id=device_id,
        mode=mode,
        csv_path=csv_path,
    )

    ok = generate_nvs_image(csv_path, image_path)
    if not ok:
        return False

    if dry_run:
        print(f"[DRY-RUN] Would flash NVS image {image_path} to {port}")
        return True

    return flash_nvs_image(port, image_path)


def _parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Build and flash NVS partition for a DICE ESP32-S3 device."
    )
    parser.add_argument("--port", required=True, help="Serial port (e.g. /dev/ttyUSB0).")
    parser.add_argument(
        "--private-key",
        type=Path,
        required=True,
        help="Path to DER-encoded device private key.",
    )
    parser.add_argument(
        "--ca-cert",
        type=Path,
        required=True,
        help="Path to coordinator CA certificate (PEM or DER).",
    )
    parser.add_argument(
        "--device-id",
        required=True,
        help="Device identifier string (e.g. dice-dev-001).",
    )
    parser.add_argument(
        "--mode",
        choices=["devnet", "mainnet"],
        default="devnet",
        help="Network mode to embed in NVS (default: devnet).",
    )
    parser.add_argument(
        "--work-dir",
        type=Path,
        default=None,
        help="Directory for intermediate build files (default: auto temp dir).",
    )
    parser.add_argument(
        "--offset",
        default=NVS_PARTITION_OFFSET,
        help=f"Flash offset for NVS partition (default: {NVS_PARTITION_OFFSET}).",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Build NVS image but do not flash to device.",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> None:
    args = _parse_args(argv)
    ok = provision_nvs(
        port=args.port,
        private_key_path=args.private_key,
        ca_cert_path=args.ca_cert,
        device_id=args.device_id,
        mode=args.mode,
        work_dir=args.work_dir,
        dry_run=args.dry_run,
    )
    if not ok:
        sys.exit(1)
    print("[OK] NVS provisioning complete.")


if __name__ == "__main__":
    main(sys.argv[1:])
