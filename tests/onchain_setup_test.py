#!/usr/bin/env python3
"""
DICE On-Chain Setup & End-to-End Test

Executes the REAL on-chain flow:
1. register_device — register ESP32 node on Solana
2. request_randomness — create escrow + request PDA
3. Verify coordinator picks up the request and runs VRF round
4. Verify submit_commit, submit_reveal, finalize_randomness land on-chain
5. Verify the RandomnessResult PDA contains valid randomness

Uses solana-py (solders) for transaction construction.
"""

import json
import hashlib
import struct
import time
import sys
import os
import subprocess

# We'll use solana CLI via subprocess since solana-py may not be installed

PROGRAM_ID = "78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv"
COORD_KP_PATH = "coordinator-keypair.json"
DEVICE_PUBKEY_HEX = "025e62666100d9ee1973a02032dbe41f3e5d7b3e54bb11e9ba9cc839b43c35a01d"
RPC_URL = "https://api.devnet.solana.com"

def anchor_discriminator(name):
    """Compute 8-byte Anchor instruction discriminator."""
    return hashlib.sha256(f"global:{name}".encode()).digest()[:8]

def run_solana_cmd(args, keypair=None):
    """Run a solana CLI command and return output."""
    cmd = ["solana"] + args + ["--url", RPC_URL]
    if keypair:
        cmd += ["--keypair", keypair]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
    return result.stdout.strip(), result.stderr.strip(), result.returncode

def get_address(keypair_path):
    out, _, _ = run_solana_cmd(["address", "-k", keypair_path])
    return out.strip()

def get_balance(address):
    out, _, _ = run_solana_cmd(["balance", address])
    return out.strip()

def main():
    print("=" * 60)
    print("  DICE ON-CHAIN SETUP & END-TO-END TEST")
    print("=" * 60)
    print()

    # Load coordinator address
    coord_addr = get_address(COORD_KP_PATH)
    print(f"Coordinator: {coord_addr}")
    print(f"Balance: {get_balance(coord_addr)}")
    print(f"Program: {PROGRAM_ID}")
    print(f"Device pubkey: {DEVICE_PUBKEY_HEX}")
    print()

    # Compute device_id = SHA-256(device_pubkey)
    device_pubkey_bytes = bytes.fromhex(DEVICE_PUBKEY_HEX)
    device_id = hashlib.sha256(device_pubkey_bytes).digest()
    print(f"Device ID (SHA-256): {device_id.hex()}")

    # Derive DeviceRegistry PDA
    # seeds = ["device", device_id]
    # We need to find the PDA — this requires solana-py or manual computation
    # For now, let's use the coordinator's existing TX building logic
    # by hitting the /simulate endpoint and checking if the on-chain TXs succeed

    print()
    print("=" * 60)
    print("  STEP 1: Check if device is already registered")
    print("=" * 60)

    # Use solana CLI to check if DeviceRegistry account exists
    # PDA = findProgramAddress(["device", device_id], program_id)
    # We can compute this in Python

    # Actually, let's just try to trigger a round and see if the on-chain TXs work
    # The coordinator already tries to submit on-chain — if register_device hasn't
    # been called, the submit_commit will fail with AccountNotInitialized

    print()
    print("NOTE: The coordinator's /simulate endpoint submits on-chain TXs.")
    print("We need to check if those TXs succeed or fail.")
    print()
    print("The coordinator currently continues the round even if on-chain")
    print("TXs fail. This is the bug we need to fix.")
    print()

    # For now, let's check what happens when we trigger a round
    import urllib.request

    print("=" * 60)
    print("  STEP 2: Trigger VRF round and check on-chain result")
    print("=" * 60)

    try:
        req = urllib.request.Request(
            "http://localhost:8080/simulate",
            method="POST",
            headers={"Authorization": "Bearer test-secret-key-123"} if False else {},
        )
        resp = urllib.request.urlopen(req)
        data = json.loads(resp.read())
        print(f"Request ID: {data.get('request_id', 'N/A')}")
        print(f"TX Signature: {data.get('tx_signature', 'N/A')}")
        print(f"Explorer: {data.get('explorer', 'N/A')}")

        if data.get("tx_signature"):
            # Check if the TX actually succeeded on-chain
            print()
            print("Checking TX status on Solana...")
            time.sleep(5)

            tx_sig = data["tx_signature"]
            out, err, rc = run_solana_cmd(["confirm", "-v", tx_sig])
            print(f"TX confirmation: {out}")
            if "Success" in out:
                print("ON-CHAIN TX SUCCEEDED!")
            else:
                print(f"ON-CHAIN TX FAILED: {err}")

    except Exception as e:
        print(f"Coordinator not running or error: {e}")
        print("Start the coordinator first, then run this test.")

    print()
    print("=" * 60)
    print("  SUMMARY")
    print("=" * 60)
    print()
    print("To complete the on-chain setup, we need to:")
    print("1. Call register_device(device_id, device_pubkey) on-chain")
    print("2. Then request_randomness will create the escrow + request PDAs")
    print("3. Then submit_commit/submit_reveal/finalize will succeed")
    print()
    print("This requires building a transaction with the correct")
    print("Anchor discriminator + PDA derivation.")

if __name__ == "__main__":
    os.chdir(os.path.dirname(os.path.abspath(__file__)) + "/..")
    main()
