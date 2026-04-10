#!/usr/bin/env python3
"""
DICE Node Provisioner — Flash firmware + unique identity to an ESP32-S3

Usage:
    python provision_node.py --port COM4 --node-id 1
    python provision_node.py --port COM5 --node-id 2
    python provision_node.py --port COM6 --node-id 3
    python provision_node.py --port COM7 --node-id 4

Each node gets:
    - Same firmware binary
    - Unique secp256k1 ECDSA keypair (for VRF signing)
    - Unique TLS client certificate (signed by DICE CA, for mTLS)
    - Shared CA cert + coordinator URI
"""

import argparse
import os
import sys
import subprocess
import hashlib
import datetime

def main():
    parser = argparse.ArgumentParser(description="DICE Node Provisioner")
    parser.add_argument("--port", required=True, help="Serial port (e.g., COM4)")
    parser.add_argument("--node-id", type=int, required=True, help="Node number (1-20)")
    parser.add_argument("--coordinator-uri", default="wss://192.168.31.162:9001",
                        help="Coordinator WebSocket URI")
    parser.add_argument("--no-firmware", action="store_true",
                        help="Skip firmware flash (only provision NVS)")
    args = parser.parse_args()

    PROJECT_DIR = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    PROV_DIR = os.path.join(PROJECT_DIR, "firmware", "tools", "prov_output")
    CERTS_DIR = os.path.join(PROJECT_DIR, "pki", "production")
    FIRMWARE_DIR = os.path.join(PROJECT_DIR, "firmware", "build")
    NODE_DIR = os.path.join(PROV_DIR, f"node_{args.node_id:03d}")

    os.makedirs(NODE_DIR, exist_ok=True)

    print("=" * 60)
    print(f"  DICE Node Provisioner — Node #{args.node_id}")
    print(f"  Port: {args.port}")
    print(f"  Coordinator: {args.coordinator_uri}")
    print("=" * 60)
    print()

    # ── Step 1: Generate secp256k1 keypair ────────────────────────────
    print("[1/5] Generating secp256k1 keypair...")

    from cryptography.hazmat.primitives.asymmetric import ec
    from cryptography.hazmat.primitives import serialization, hashes
    from cryptography.hazmat.backends import default_backend
    from cryptography import x509
    from cryptography.x509.oid import NameOID, ExtendedKeyUsageOID

    # VRF signing key (secp256k1)
    vrf_key = ec.generate_private_key(ec.SECP256K1(), default_backend())
    vrf_der = vrf_key.private_bytes(
        serialization.Encoding.DER,
        serialization.PrivateFormat.PKCS8,
        serialization.NoEncryption(),
    )

    # Compressed public key
    pub_nums = vrf_key.public_key().public_numbers()
    x_bytes = pub_nums.x.to_bytes(32, "big")
    prefix = b"\x02" if pub_nums.y % 2 == 0 else b"\x03"
    compressed_pubkey = (prefix + x_bytes).hex()

    der_path = os.path.join(NODE_DIR, "priv_key.der")
    with open(der_path, "wb") as f:
        f.write(vrf_der)

    print(f"  VRF pubkey: {compressed_pubkey}")
    print(f"  DER key: {len(vrf_der)} bytes -> {der_path}")

    # ── Step 2: Generate TLS client certificate ───────────────────────
    print()
    print("[2/5] Generating TLS client certificate (signed by CA)...")

    # Load intermediate CA
    inter_cert_path = os.path.join(CERTS_DIR, "intermediate_ca.crt")
    inter_key_path = os.path.join(CERTS_DIR, "intermediate_ca.key")

    if not os.path.exists(inter_cert_path) or not os.path.exists(inter_key_path):
        print(f"  ERROR: Intermediate CA not found at {CERTS_DIR}")
        print(f"  Run pki/generate_production_pki.py first")
        sys.exit(1)

    with open(inter_cert_path, "rb") as f:
        inter_cert = x509.load_pem_x509_certificate(f.read(), default_backend())
    with open(inter_key_path, "rb") as f:
        inter_key = serialization.load_pem_private_key(f.read(), password=None, backend=default_backend())

    # TLS key (secp256r1 — TLS requires P-256, not secp256k1)
    tls_key = ec.generate_private_key(ec.SECP256R1(), default_backend())

    dev_subject = x509.Name([
        x509.NameAttribute(NameOID.ORGANIZATION_NAME, "DICE Network"),
        x509.NameAttribute(NameOID.COMMON_NAME, f"DICE-Node-{args.node_id:03d}"),
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

    cert_path = os.path.join(NODE_DIR, "device.crt")
    key_path = os.path.join(NODE_DIR, "device.key")
    with open(cert_path, "wb") as f:
        f.write(dev_cert.public_bytes(serialization.Encoding.PEM))
    with open(key_path, "wb") as f:
        f.write(tls_key.private_bytes(
            serialization.Encoding.PEM,
            serialization.PrivateFormat.PKCS8,
            serialization.NoEncryption(),
        ))

    print(f"  CN: DICE-Node-{args.node_id:03d}")
    print(f"  Cert: {cert_path}")

    # ── Step 3: Generate NVS partition ────────────────────────────────
    print()
    print("[3/5] Generating NVS partition...")

    ca_bundle_path = os.path.join(CERTS_DIR, "ca_bundle.crt")
    if not os.path.exists(ca_bundle_path):
        # Fall back to single CA cert
        ca_bundle_path = os.path.join(CERTS_DIR, "root_ca.crt")

    csv_path = os.path.join(NODE_DIR, "nvs.csv")
    with open(csv_path, "w") as f:
        f.write("key,type,encoding,value\n")
        f.write("dice,namespace,,\n")
        f.write(f"coordinator_uri,data,string,{args.coordinator_uri}\n")
        f.write(f"priv_key_der,file,binary,{der_path}\n")
        f.write(f"client_cert_pem,file,string,{cert_path}\n")
        f.write(f"client_key_pem,file,string,{key_path}\n")
        f.write(f"ca_cert_pem,file,string,{ca_bundle_path}\n")

    # Find nvs_partition_gen.py
    idf_path = os.environ.get("IDF_PATH", "C:/Espressif/frameworks/esp-idf-v5.2.6")
    nvs_gen = os.path.join(idf_path, "components", "nvs_flash",
                           "nvs_partition_generator", "nvs_partition_gen.py")

    nvs_bin_path = os.path.join(NODE_DIR, "nvs.bin")

    # Find Python
    python = sys.executable
    if "espressif" not in python.lower():
        # Try ESP-IDF Python
        esp_python = "C:/Espressif/python_env/idf5.2_py3.11_env/Scripts/python.exe"
        if os.path.exists(esp_python):
            python = esp_python

    result = subprocess.run(
        [python, nvs_gen, "generate", csv_path, nvs_bin_path, "0x6000"],
        capture_output=True, text=True, timeout=10,
    )
    if result.returncode != 0:
        print(f"  ERROR: NVS generation failed: {result.stderr}")
        sys.exit(1)

    print(f"  NVS binary: {nvs_bin_path}")

    # ── Step 4: Flash firmware ────────────────────────────────────────
    if not args.no_firmware:
        print()
        print("[4/5] Flashing firmware + NVS...")

        bootloader = os.path.join(FIRMWARE_DIR, "bootloader", "bootloader.bin")
        partition_table = os.path.join(FIRMWARE_DIR, "partition_table", "partition-table.bin")
        firmware = os.path.join(FIRMWARE_DIR, "dice_firmware.bin")

        if not os.path.exists(firmware):
            print(f"  ERROR: Firmware not found at {firmware}")
            print(f"  Build with: idf.py build")
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
        result = subprocess.run(flash_cmd, capture_output=True, text=True, timeout=60)
        if result.returncode != 0:
            print(f"  ERROR: Flash failed: {result.stderr[-200:]}")
            sys.exit(1)

        print("  Flash complete!")
    else:
        print()
        print("[4/5] Skipping firmware (--no-firmware)")
        print("  Flashing NVS only...")
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
            print(f"  ERROR: NVS flash failed: {result.stderr[-200:]}")
            sys.exit(1)
        print("  NVS flash complete!")

    # ── Step 5: Save device info ──────────────────────────────────────
    print()
    print("[5/5] Saving device identity...")

    info_path = os.path.join(NODE_DIR, "device_info.txt")
    device_id = hashlib.sha256(bytes.fromhex(compressed_pubkey)).hexdigest()
    with open(info_path, "w") as f:
        f.write(f"Node ID:         {args.node_id}\n")
        f.write(f"VRF Pubkey:      {compressed_pubkey}\n")
        f.write(f"Device ID:       {device_id}\n")
        f.write(f"TLS CN:          DICE-Node-{args.node_id:03d}\n")
        f.write(f"Coordinator:     {args.coordinator_uri}\n")
        f.write(f"Port:            {args.port}\n")
        f.write(f"Provisioned:     {datetime.datetime.now().isoformat()}\n")

    print(f"  Saved to: {info_path}")

    # ── Done ──────────────────────────────────────────────────────────
    print()
    print("=" * 60)
    print(f"  NODE #{args.node_id} PROVISIONED")
    print("=" * 60)
    print()
    print(f"  VRF Pubkey:  {compressed_pubkey}")
    print(f"  Device ID:   {device_id[:16]}...")
    print(f"  TLS CN:      DICE-Node-{args.node_id:03d}")
    print(f"  Coordinator: {args.coordinator_uri}")
    print()
    print("  Next steps:")
    print("  1. Device boots into captive portal (blue LED)")
    print("  2. Connect to DICE-XXXX WiFi, enter WiFi creds")
    print("  3. Device connects to coordinator (green LED)")
    print("  4. Register device on-chain:")
    print(f"     npx ts-node tests/devnet_setup.ts  (with pubkey {compressed_pubkey[:16]}...)")
    print()


if __name__ == "__main__":
    main()
