"""
Generate a device ECDSA secp256k1 keypair.
Keypair is generated on the provisioning station (not on the ESP32) for
security — at provisioning time the device is unverified.

Output:
    - private key: DER-encoded (for flashing to device NVS)
    - public key: 33-byte compressed point, hex-encoded (for registration)
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from cryptography.hazmat.primitives.asymmetric import ec
from cryptography.hazmat.primitives import serialization


def generate_keypair() -> tuple[bytes, bytes]:
    """Generate an ECDSA secp256k1 keypair.

    Returns:
        (private_key_der, compressed_pubkey_33bytes) where:
            - private_key_der: DER-encoded private key (PKCS#8)
            - compressed_pubkey_33bytes: 33-byte SEC1 compressed public key
    """
    private_key = ec.generate_private_key(ec.SECP256K1())

    private_key_der = private_key.private_bytes(
        encoding=serialization.Encoding.DER,
        format=serialization.PrivateFormat.PKCS8,
        encryption_algorithm=serialization.NoEncryption(),
    )

    public_key = private_key.public_key()
    compressed_pubkey = public_key.public_bytes(
        encoding=serialization.Encoding.X962,
        format=serialization.PublicFormat.CompressedPoint,
    )

    assert len(compressed_pubkey) == 33, (
        f"Compressed public key must be 33 bytes, got {len(compressed_pubkey)}"
    )

    return private_key_der, compressed_pubkey


def save_keypair(
    private_der: bytes,
    pubkey_hex: str,
    output_dir: Path,
    device_id: str | None = None,
) -> None:
    """Save keypair files to output_dir.

    Files written:
        <output_dir>/device_key.der       — raw DER private key (for NVS flash)
        <output_dir>/device_pubkey.hex    — hex-encoded 33-byte compressed pubkey
        <output_dir>/device_key.pem       — PEM private key (for signing/debug)

    Args:
        private_der:  DER-encoded PKCS#8 private key bytes
        pubkey_hex:   Hex string of the 33-byte compressed public key
        output_dir:   Directory to write into (created if it doesn't exist)
        device_id:    Optional device identifier stored alongside the key files
    """
    output_dir = Path(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    # Write raw DER private key — this is what gets flashed to NVS
    der_path = output_dir / "device_key.der"
    der_path.write_bytes(private_der)
    der_path.chmod(0o600)

    # Write hex pubkey
    hex_path = output_dir / "device_pubkey.hex"
    hex_path.write_text(pubkey_hex + "\n", encoding="utf-8")

    # Reconstruct key object to produce PEM for human inspection / signing use
    private_key = serialization.load_der_private_key(private_der, password=None)
    pem_bytes = private_key.private_bytes(
        encoding=serialization.Encoding.PEM,
        format=serialization.PrivateFormat.PKCS8,
        encryption_algorithm=serialization.NoEncryption(),
    )
    pem_path = output_dir / "device_key.pem"
    pem_path.write_bytes(pem_bytes)
    pem_path.chmod(0o600)

    if device_id is not None:
        id_path = output_dir / "device_id.txt"
        id_path.write_text(device_id + "\n", encoding="utf-8")

    print(f"Keypair saved to {output_dir}")
    print(f"  DER private key : {der_path}")
    print(f"  Compressed pubkey (hex): {pubkey_hex}")


def _parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate an ECDSA secp256k1 keypair for a DICE device."
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        required=True,
        help="Directory where key files will be written.",
    )
    parser.add_argument(
        "--device-id",
        type=str,
        default=None,
        help="Optional device identifier string (stored in device_id.txt).",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> None:
    args = _parse_args(argv)

    private_der, compressed_pubkey = generate_keypair()
    pubkey_hex = compressed_pubkey.hex()

    save_keypair(
        private_der=private_der,
        pubkey_hex=pubkey_hex,
        output_dir=args.output_dir,
        device_id=args.device_id,
    )


if __name__ == "__main__":
    main(sys.argv[1:])
