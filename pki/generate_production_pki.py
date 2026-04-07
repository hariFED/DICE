#!/usr/bin/env python3
"""DICE Production CA Ceremony — 2-Tier PKI Generator."""

from cryptography import x509
from cryptography.x509.oid import NameOID, ExtendedKeyUsageOID
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import ec
from cryptography.hazmat.backends import default_backend
import datetime
import os
import ipaddress
import sys

out = os.path.join(os.path.dirname(__file__), "production")
os.makedirs(out, exist_ok=True)

print("=" * 60)
print("  DICE PRODUCTION CA CEREMONY")
print("  2-Tier PKI: Root CA -> Intermediate CA -> Certs")
print("=" * 60)
print()

# ── TIER 1: ROOT CA (20 year) ──────────────────────────────

print("[1/5] Generating Root CA...")
root_key = ec.generate_private_key(ec.SECP256R1(), default_backend())
root_subject = x509.Name([
    x509.NameAttribute(NameOID.ORGANIZATION_NAME, "DICE Network"),
    x509.NameAttribute(NameOID.COMMON_NAME, "DICE Root CA"),
])
root_cert = (
    x509.CertificateBuilder()
    .subject_name(root_subject)
    .issuer_name(root_subject)
    .public_key(root_key.public_key())
    .serial_number(x509.random_serial_number())
    .not_valid_before(datetime.datetime.utcnow())
    .not_valid_after(datetime.datetime.utcnow() + datetime.timedelta(days=7300))
    .add_extension(x509.BasicConstraints(ca=True, path_length=1), critical=True)
    .add_extension(
        x509.KeyUsage(
            digital_signature=True, key_cert_sign=True, crl_sign=True,
            content_commitment=False, key_encipherment=False,
            data_encipherment=False, key_agreement=False,
            encipher_only=False, decipher_only=False,
        ),
        critical=True,
    )
    .sign(root_key, hashes.SHA256(), default_backend())
)

with open(os.path.join(out, "root_ca.crt"), "wb") as f:
    f.write(root_cert.public_bytes(serialization.Encoding.PEM))
with open(os.path.join(out, "root_ca.key"), "wb") as f:
    f.write(root_key.private_bytes(
        serialization.Encoding.PEM,
        serialization.PrivateFormat.PKCS8,
        serialization.NoEncryption(),
    ))
print(f"  Root CA: O=DICE Network, CN=DICE Root CA")
print(f"  Valid: 20 years")

# ── TIER 2: INTERMEDIATE CA (5 year) ───────────────────────

print()
print("[2/5] Generating Intermediate CA (signed by Root)...")
inter_key = ec.generate_private_key(ec.SECP256R1(), default_backend())
inter_subject = x509.Name([
    x509.NameAttribute(NameOID.ORGANIZATION_NAME, "DICE Network"),
    x509.NameAttribute(NameOID.COMMON_NAME, "DICE Intermediate CA"),
])
inter_cert = (
    x509.CertificateBuilder()
    .subject_name(inter_subject)
    .issuer_name(root_subject)
    .public_key(inter_key.public_key())
    .serial_number(x509.random_serial_number())
    .not_valid_before(datetime.datetime.utcnow())
    .not_valid_after(datetime.datetime.utcnow() + datetime.timedelta(days=1825))
    .add_extension(x509.BasicConstraints(ca=True, path_length=0), critical=True)
    .add_extension(
        x509.KeyUsage(
            digital_signature=True, key_cert_sign=True, crl_sign=True,
            content_commitment=False, key_encipherment=False,
            data_encipherment=False, key_agreement=False,
            encipher_only=False, decipher_only=False,
        ),
        critical=True,
    )
    .sign(root_key, hashes.SHA256(), default_backend())
)

with open(os.path.join(out, "intermediate_ca.crt"), "wb") as f:
    f.write(inter_cert.public_bytes(serialization.Encoding.PEM))
with open(os.path.join(out, "intermediate_ca.key"), "wb") as f:
    f.write(inter_key.private_bytes(
        serialization.Encoding.PEM,
        serialization.PrivateFormat.PKCS8,
        serialization.NoEncryption(),
    ))
print(f"  Intermediate CA: O=DICE Network, CN=DICE Intermediate CA")
print(f"  Signed by: DICE Root CA")
print(f"  Valid: 5 years")

# ── COORDINATOR CERT (2 year, serverAuth) ──────────────────

print()
print("[3/5] Generating Coordinator server cert (signed by Intermediate)...")
coord_key = ec.generate_private_key(ec.SECP256R1(), default_backend())
coord_subject = x509.Name([
    x509.NameAttribute(NameOID.ORGANIZATION_NAME, "DICE Network"),
    x509.NameAttribute(NameOID.COMMON_NAME, "DICE Coordinator"),
])
coord_cert = (
    x509.CertificateBuilder()
    .subject_name(coord_subject)
    .issuer_name(inter_subject)
    .public_key(coord_key.public_key())
    .serial_number(x509.random_serial_number())
    .not_valid_before(datetime.datetime.utcnow())
    .not_valid_after(datetime.datetime.utcnow() + datetime.timedelta(days=730))
    .add_extension(x509.BasicConstraints(ca=False, path_length=None), critical=True)
    .add_extension(
        x509.ExtendedKeyUsage([ExtendedKeyUsageOID.SERVER_AUTH]),
        critical=False,
    )
    .add_extension(
        x509.SubjectAlternativeName([
            x509.DNSName("localhost"),
            x509.DNSName("coord.dice.network"),
            x509.IPAddress(ipaddress.IPv4Address("127.0.0.1")),
            x509.IPAddress(ipaddress.IPv4Address("0.0.0.0")),
        ]),
        critical=False,
    )
    .sign(inter_key, hashes.SHA256(), default_backend())
)

with open(os.path.join(out, "coordinator.crt"), "wb") as f:
    f.write(coord_cert.public_bytes(serialization.Encoding.PEM))
with open(os.path.join(out, "coordinator.key"), "wb") as f:
    f.write(coord_key.private_bytes(
        serialization.Encoding.PEM,
        serialization.PrivateFormat.PKCS8,
        serialization.NoEncryption(),
    ))
print(f"  SAN: localhost, coord.dice.network, 127.0.0.1")
print(f"  Valid: 2 years")

# ── DEVICE CLIENT CERT (10 year, clientAuth) ───────────────

print()
print("[4/5] Generating Device client cert (signed by Intermediate)...")
dev_key = ec.generate_private_key(ec.SECP256R1(), default_backend())
dev_subject = x509.Name([
    x509.NameAttribute(NameOID.ORGANIZATION_NAME, "DICE Network"),
    x509.NameAttribute(NameOID.COMMON_NAME, "DICE-Node-001"),
])
dev_cert = (
    x509.CertificateBuilder()
    .subject_name(dev_subject)
    .issuer_name(inter_subject)
    .public_key(dev_key.public_key())
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

with open(os.path.join(out, "device.crt"), "wb") as f:
    f.write(dev_cert.public_bytes(serialization.Encoding.PEM))
with open(os.path.join(out, "device.key"), "wb") as f:
    f.write(dev_key.private_bytes(
        serialization.Encoding.PEM,
        serialization.PrivateFormat.PKCS8,
        serialization.NoEncryption(),
    ))
print(f"  CN: DICE-Node-001")
print(f"  Valid: 10 years")

# ── CA BUNDLE ──────────────────────────────────────────────

print()
print("[5/5] Creating CA bundle (intermediate + root)...")
bundle = (
    inter_cert.public_bytes(serialization.Encoding.PEM)
    + root_cert.public_bytes(serialization.Encoding.PEM)
)
with open(os.path.join(out, "ca_bundle.crt"), "wb") as f:
    f.write(bundle)
print(f"  ca_bundle.crt: {len(bundle)} bytes")

# ── SUMMARY ────────────────────────────────────────────────

print()
print("=" * 60)
print("  CEREMONY COMPLETE")
print("=" * 60)
print()
print("Trust chain:")
print("  Root CA (20yr)")
print("    -> Intermediate CA (5yr)")
print("         -> Coordinator cert (2yr, serverAuth)")
print("         -> Device cert (10yr, clientAuth)")
print()
print("Files:")
for f in sorted(os.listdir(out)):
    size = os.path.getsize(os.path.join(out, f))
    print(f"  pki/production/{f:30s} {size:6d} bytes")
print()
print("IMPORTANT: Store root_ca.key securely. It can sign new")
print("intermediate CAs. If compromised, entire PKI is broken.")
