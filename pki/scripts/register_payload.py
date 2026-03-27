"""
Generate on-chain registration payloads for provisioned devices.

Reads from device_registry.json and produces one JSON file per device
in pki/build/registration_payloads/ with the data needed to call
the Solana `register_device` instruction.

Output per device:
{
  "device_id": "dice-dev-001",
  "device_pubkey_hex": "02...",    // 33 bytes compressed secp256k1
  "device_pubkey_b58": "...",      // base58 encoded
  "instruction": "register_device",
  "program_id": "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS"
}
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Base58 encoder (no external dependency; matches Bitcoin/Solana alphabet)
# ---------------------------------------------------------------------------
_BASE58_ALPHABET = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"


def _b58encode(data: bytes) -> str:
    """Encode bytes to a Base58 string using the Bitcoin/Solana alphabet."""
    # Count leading zero bytes
    leading_zeros = 0
    for byte in data:
        if byte == 0:
            leading_zeros += 1
        else:
            break

    # Convert bytes to integer
    num = int.from_bytes(data, "big")
    result = []
    while num > 0:
        num, remainder = divmod(num, 58)
        result.append(_BASE58_ALPHABET[remainder])

    result.extend([_BASE58_ALPHABET[0]] * leading_zeros)
    result.reverse()
    return bytes(result).decode("ascii")


# ---------------------------------------------------------------------------
# Default constants
# ---------------------------------------------------------------------------
DICE_PROGRAM_ID = "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS"
REGISTER_INSTRUCTION = "register_device"

# Default paths (relative to this script's parent-parent = pki/)
_PKI_DIR = Path(__file__).resolve().parent.parent
_DEFAULT_REGISTRY = _PKI_DIR / "device_registry.json"
_DEFAULT_OUTPUT_DIR = _PKI_DIR / "build" / "registration_payloads"


def build_payload(device_record: dict, program_id: str = DICE_PROGRAM_ID) -> dict:
    """Build an on-chain registration payload from a device registry record.

    Args:
        device_record: A single record from device_registry.json
        program_id:    Solana program ID for the register_device instruction

    Returns:
        Dict ready to be serialised to JSON.
    """
    pubkey_hex: str = device_record["pubkey_hex"]
    pubkey_bytes = bytes.fromhex(pubkey_hex)
    pubkey_b58 = _b58encode(pubkey_bytes)

    return {
        "device_id": device_record["device_id"],
        "device_pubkey_hex": pubkey_hex,
        "device_pubkey_b58": pubkey_b58,
        "cert_serial": device_record.get("cert_serial", ""),
        "cert_fingerprint": device_record.get("cert_fingerprint", ""),
        "firmware_hash": device_record.get("firmware_hash", ""),
        "provisioned_at": device_record.get("provisioned_at", ""),
        "mode": device_record.get("mode", "devnet"),
        "instruction": REGISTER_INSTRUCTION,
        "program_id": program_id,
    }


def generate_payloads(
    registry_path: Path,
    output_dir: Path,
    program_id: str = DICE_PROGRAM_ID,
    device_ids: list[str] | None = None,
) -> list[Path]:
    """Read device_registry.json and write one payload JSON per device.

    Args:
        registry_path: Path to device_registry.json
        output_dir:    Directory where payload JSON files are written
        program_id:    Solana program ID
        device_ids:    If provided, only generate payloads for these device IDs

    Returns:
        List of written file paths.
    """
    registry_path = Path(registry_path)
    output_dir = Path(output_dir)

    if not registry_path.exists():
        print(f"[ERROR] Registry not found: {registry_path}", file=sys.stderr)
        sys.exit(1)

    with registry_path.open(encoding="utf-8") as fh:
        records: list[dict] = json.load(fh)

    if not isinstance(records, list):
        print("[ERROR] device_registry.json must be a JSON array.", file=sys.stderr)
        sys.exit(1)

    if device_ids:
        records = [r for r in records if r.get("device_id") in device_ids]

    output_dir.mkdir(parents=True, exist_ok=True)
    written: list[Path] = []

    for record in records:
        device_id = record.get("device_id")
        if not device_id:
            print(f"[WARN] Skipping record missing device_id: {record}", file=sys.stderr)
            continue

        payload = build_payload(record, program_id=program_id)
        out_path = output_dir / f"{device_id}_registration.json"

        with out_path.open("w", encoding="utf-8") as fh:
            json.dump(payload, fh, indent=2)
            fh.write("\n")

        print(f"[INFO] Written: {out_path}")
        written.append(out_path)

    print(f"\n[OK] Generated {len(written)} registration payload(s) in {output_dir}")
    return written


def _parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate Solana on-chain registration payloads from device_registry.json."
    )
    parser.add_argument(
        "--registry",
        type=Path,
        default=_DEFAULT_REGISTRY,
        help=f"Path to device_registry.json (default: {_DEFAULT_REGISTRY}).",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=_DEFAULT_OUTPUT_DIR,
        help=f"Output directory for payload JSON files (default: {_DEFAULT_OUTPUT_DIR}).",
    )
    parser.add_argument(
        "--program-id",
        default=DICE_PROGRAM_ID,
        help=f"Solana program ID (default: {DICE_PROGRAM_ID}).",
    )
    parser.add_argument(
        "--device-ids",
        nargs="+",
        default=None,
        metavar="DEVICE_ID",
        help="Filter to specific device IDs (default: all devices in registry).",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> None:
    args = _parse_args(argv)
    generate_payloads(
        registry_path=args.registry,
        output_dir=args.output_dir,
        program_id=args.program_id,
        device_ids=args.device_ids,
    )


if __name__ == "__main__":
    main(sys.argv[1:])
