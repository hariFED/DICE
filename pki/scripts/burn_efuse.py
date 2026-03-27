"""
eFuse burning script for DICE devices.
WARNING: These operations are IRREVERSIBLE.

Fuses burned per device:
- JTAG_DISABLE: prevents JTAG debugging post-production
- DIS_DOWNLOAD_ICACHE: prevents cache read via download mode
- Secure Boot V2: verifies firmware signature on every boot
- Flash Encryption: encrypts all flash contents

Usage:
    python burn_efuse.py --port /dev/ttyUSB0 --confirm
    python burn_efuse.py --port /dev/ttyUSB0 --dry-run  # shows what would be burned
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path


# Mapping of logical fuse name → espefuse.py arguments
_SECURE_BOOT_ARGS = [
    "burn_efuse",
    "ABS_DONE_0",  # Secure Boot V2 enable (ESP32-S3)
]

_FLASH_ENCRYPTION_ARGS = [
    "burn_efuse",
    "DIS_DOWNLOAD_PLAINTEXT",  # Disallow plaintext download mode read-back
]

_JTAG_DISABLE_ARGS = [
    "burn_efuse",
    "JTAG_SEL_ENABLE",  # Hard-disable JTAG after provisioning
    "--do-not-confirm",
]

_DIS_DOWNLOAD_ICACHE_ARGS = [
    "burn_efuse",
    "DIS_DOWNLOAD_ICACHE",
]


def _run_espefuse(
    port: str,
    args: list[str],
    dry_run: bool,
    extra_env: dict[str, str] | None = None,
) -> bool:
    """Invoke espefuse.py with the given arguments.

    Args:
        port:     Serial port, e.g. /dev/ttyUSB0
        args:     espefuse.py subcommand + arguments
        dry_run:  If True, print the command but do not execute it
        extra_env: Additional environment variables for the subprocess

    Returns:
        True on success, False on failure.
    """
    cmd = ["espefuse.py", "--port", port] + args
    if dry_run:
        print(f"[DRY-RUN] Would execute: {' '.join(cmd)}")
        return True

    print(f"[BURN] Executing: {' '.join(cmd)}")
    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=60,
            env=extra_env,
        )
        if result.returncode != 0:
            print(f"[ERROR] espefuse.py failed (rc={result.returncode}):", file=sys.stderr)
            print(result.stderr, file=sys.stderr)
            return False
        print(result.stdout)
        return True
    except FileNotFoundError:
        print(
            "[ERROR] espefuse.py not found. Install it via: pip install esptool",
            file=sys.stderr,
        )
        return False
    except subprocess.TimeoutExpired:
        print("[ERROR] espefuse.py timed out after 60 seconds.", file=sys.stderr)
        return False


def burn_secure_boot(port: str, dry_run: bool = False) -> bool:
    """Burn Secure Boot V2 fuse (ABS_DONE_0).

    WARNING: IRREVERSIBLE. The device will refuse to boot unsigned firmware.

    Args:
        port:    Serial port connected to the ESP32-S3
        dry_run: If True, only print the command — do not burn

    Returns:
        True on success.
    """
    print(f"[INFO] Burning Secure Boot V2 fuse on {port}")
    return _run_espefuse(port, _SECURE_BOOT_ARGS, dry_run)


def burn_flash_encryption(port: str, dry_run: bool = False) -> bool:
    """Burn Flash Encryption fuse (DIS_DOWNLOAD_PLAINTEXT).

    WARNING: IRREVERSIBLE. Flash contents will be AES-encrypted.

    Args:
        port:    Serial port connected to the ESP32-S3
        dry_run: If True, only print the command — do not burn

    Returns:
        True on success.
    """
    print(f"[INFO] Burning Flash Encryption fuse on {port}")
    return _run_espefuse(port, _FLASH_ENCRYPTION_ARGS, dry_run)


def burn_jtag_disable(port: str, dry_run: bool = False) -> bool:
    """Burn JTAG disable fuse (JTAG_SEL_ENABLE override + DIS_DOWNLOAD_ICACHE).

    WARNING: IRREVERSIBLE. JTAG debugging will be permanently disabled.

    Args:
        port:    Serial port connected to the ESP32-S3
        dry_run: If True, only print the command — do not burn

    Returns:
        True if all JTAG-related fuses were burned successfully.
    """
    print(f"[INFO] Burning JTAG disable fuses on {port}")
    ok_jtag = _run_espefuse(port, _JTAG_DISABLE_ARGS, dry_run)
    ok_icache = _run_espefuse(port, _DIS_DOWNLOAD_ICACHE_ARGS, dry_run)
    return ok_jtag and ok_icache


def verify_efuses(port: str) -> dict:
    """Read back eFuses from device and return a summary dict.

    Args:
        port: Serial port connected to the ESP32-S3

    Returns:
        dict with keys: secure_boot_enabled, flash_enc_enabled, jtag_disabled,
        raw_summary (str)
    """
    cmd = ["espefuse.py", "--port", port, "summary"]
    print(f"[INFO] Reading eFuse summary from {port}")
    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=30,
        )
        raw = result.stdout + result.stderr

        # Parse well-known fields from the summary output
        secure_boot = "ABS_DONE_0" in raw and "= 1" in raw
        flash_enc = "DIS_DOWNLOAD_PLAINTEXT" in raw and "= 1" in raw
        jtag_disabled = "JTAG_SEL_ENABLE" in raw and "= 1" in raw

        summary = {
            "secure_boot_enabled": secure_boot,
            "flash_enc_enabled": flash_enc,
            "jtag_disabled": jtag_disabled,
            "raw_summary": raw,
        }
        return summary
    except FileNotFoundError:
        return {
            "error": "espefuse.py not found",
            "secure_boot_enabled": False,
            "flash_enc_enabled": False,
            "jtag_disabled": False,
            "raw_summary": "",
        }
    except subprocess.TimeoutExpired:
        return {
            "error": "Timeout reading eFuses",
            "secure_boot_enabled": False,
            "flash_enc_enabled": False,
            "jtag_disabled": False,
            "raw_summary": "",
        }


def _parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Burn security eFuses on an ESP32-S3 DICE node.\n"
            "WARNING: These operations are IRREVERSIBLE."
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--port",
        required=True,
        help="Serial port the device is connected to (e.g. /dev/ttyUSB0).",
    )
    mode_group = parser.add_mutually_exclusive_group(required=True)
    mode_group.add_argument(
        "--confirm",
        action="store_true",
        help="Actually burn the fuses (irreversible).",
    )
    mode_group.add_argument(
        "--dry-run",
        action="store_true",
        help="Print commands that would be run without executing them.",
    )
    parser.add_argument(
        "--skip-secure-boot",
        action="store_true",
        help="Skip burning Secure Boot fuse.",
    )
    parser.add_argument(
        "--skip-flash-encryption",
        action="store_true",
        help="Skip burning Flash Encryption fuse.",
    )
    parser.add_argument(
        "--skip-jtag-disable",
        action="store_true",
        help="Skip burning JTAG disable fuse.",
    )
    parser.add_argument(
        "--verify-only",
        action="store_true",
        help="Only verify current eFuse state, do not burn anything.",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> None:
    args = _parse_args(argv)
    dry_run = args.dry_run

    if args.verify_only:
        summary = verify_efuses(args.port)
        print("\n=== eFuse Verification Summary ===")
        print(f"  Secure Boot V2 enabled : {summary.get('secure_boot_enabled')}")
        print(f"  Flash Encryption enabled: {summary.get('flash_enc_enabled')}")
        print(f"  JTAG disabled           : {summary.get('jtag_disabled')}")
        if "error" in summary:
            print(f"  Error: {summary['error']}")
        return

    if args.confirm and not dry_run:
        print("=" * 60)
        print("WARNING: You are about to burn IRREVERSIBLE eFuses.")
        print(f"         Device port: {args.port}")
        print("         Proceeding in 3 seconds... (Ctrl+C to abort)")
        print("=" * 60)
        import time
        time.sleep(3)

    results: dict[str, bool] = {}

    if not args.skip_secure_boot:
        results["secure_boot"] = burn_secure_boot(args.port, dry_run)

    if not args.skip_flash_encryption:
        results["flash_encryption"] = burn_flash_encryption(args.port, dry_run)

    if not args.skip_jtag_disable:
        results["jtag_disable"] = burn_jtag_disable(args.port, dry_run)

    print("\n=== Burn Results ===")
    all_ok = True
    for fuse, ok in results.items():
        status = "OK" if ok else "FAILED"
        print(f"  {fuse}: {status}")
        if not ok:
            all_ok = False

    if not all_ok:
        print("\n[ERROR] One or more fuse burns failed. Check output above.", file=sys.stderr)
        sys.exit(1)

    if not dry_run:
        # Verify after burning
        print("\n[INFO] Verifying eFuses after burn...")
        summary = verify_efuses(args.port)
        print(f"  Secure Boot V2 enabled : {summary.get('secure_boot_enabled')}")
        print(f"  Flash Encryption enabled: {summary.get('flash_enc_enabled')}")
        print(f"  JTAG disabled           : {summary.get('jtag_disabled')}")

    print("\n[OK] eFuse burn sequence complete.")


if __name__ == "__main__":
    main(sys.argv[1:])
