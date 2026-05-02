"""Re-issue certs/coordinator.crt with the SAN entries the production
deployment actually needs: coordination.dicelabs.net (the device-facing
hostname), the VPS public IP, plus the legacy localhost / loopback entries.

Same key (certs/coordinator.key) so we don't have to redeploy keys —
just swap in the new cert and restart the coord container.

Run from repo root:
    python pki/scripts/reissue_coord_cert_dicelabs.py
"""
from __future__ import annotations

import datetime as dt
import ipaddress
from pathlib import Path

from cryptography import x509
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.x509.oid import ExtendedKeyUsageOID, NameOID

REPO = Path(__file__).resolve().parent.parent.parent
CA_CERT = REPO / "certs" / "ca.crt"
CA_KEY = REPO / "certs" / "ca.key"
COORD_KEY = REPO / "certs" / "coordinator.key"
COORD_CERT_OUT = REPO / "certs" / "coordinator.crt"

VPS_IP = "69.62.78.76"

SAN_ENTRIES = [
    x509.DNSName("coordination.dicelabs.net"),
    x509.DNSName("api.dicelabs.net"),
    x509.DNSName("dicelabs.net"),
    x509.DNSName("localhost"),
    x509.DNSName("coordinator.dice.local"),
    x509.IPAddress(ipaddress.IPv4Address(VPS_IP)),
    x509.IPAddress(ipaddress.IPv4Address("127.0.0.1")),
    x509.IPAddress(ipaddress.IPv4Address("0.0.0.0")),
]


def main() -> None:
    ca_cert = x509.load_pem_x509_certificate(CA_CERT.read_bytes())
    ca_key = serialization.load_pem_private_key(CA_KEY.read_bytes(), password=None)
    coord_key = serialization.load_pem_private_key(COORD_KEY.read_bytes(), password=None)

    subject = x509.Name(
        [
            x509.NameAttribute(NameOID.ORGANIZATION_NAME, "DICE Network"),
            x509.NameAttribute(NameOID.COMMON_NAME, "DICE Coordinator"),
        ]
    )
    now = dt.datetime.now(dt.timezone.utc)

    cert = (
        x509.CertificateBuilder()
        .subject_name(subject)
        .issuer_name(ca_cert.subject)
        .public_key(coord_key.public_key())
        .serial_number(x509.random_serial_number())
        .not_valid_before(now)
        .not_valid_after(now + dt.timedelta(days=365 * 5))
        .add_extension(x509.BasicConstraints(ca=False, path_length=None), critical=True)
        .add_extension(
            x509.KeyUsage(
                digital_signature=True,
                content_commitment=False,
                key_encipherment=True,
                data_encipherment=False,
                key_agreement=False,
                key_cert_sign=False,
                crl_sign=False,
                encipher_only=False,
                decipher_only=False,
            ),
            critical=True,
        )
        .add_extension(x509.ExtendedKeyUsage([ExtendedKeyUsageOID.SERVER_AUTH]), critical=False)
        .add_extension(x509.SubjectAlternativeName(SAN_ENTRIES), critical=False)
        .sign(ca_key, hashes.SHA256())
    )

    COORD_CERT_OUT.write_bytes(cert.public_bytes(serialization.Encoding.PEM))
    print(f"[ok] re-issued {COORD_CERT_OUT}")
    print(f"[issuer]  {ca_cert.subject.rfc4514_string()}")
    print(f"[subject] {subject.rfc4514_string()}")
    print(f"[validity] {now} -> {now + dt.timedelta(days=365 * 5)}")
    print("[san]")
    for e in SAN_ENTRIES:
        print(f"  {e}")


if __name__ == "__main__":
    main()
