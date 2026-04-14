"""Re-issue coordinator.crt with additional IP SANs while keeping the same key.

Keeps intermediate CA and all device certs intact. Only regenerates
coordinator.crt (signed by the existing intermediate CA) so that TLS
hostname verification succeeds when devices dial the coordinator by LAN IP.

Usage:
    python pki/reissue_coordinator_cert.py 192.168.1.5
"""

import datetime
import ipaddress
import os
import sys

from cryptography import x509
from cryptography.hazmat.backends import default_backend
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.x509.oid import ExtendedKeyUsageOID, NameOID

if len(sys.argv) < 2:
    print("usage: reissue_coordinator_cert.py <lan_ip> [<lan_ip>...]")
    sys.exit(1)

extra_ips = [ipaddress.IPv4Address(a) for a in sys.argv[1:]]

out = os.path.join(os.path.dirname(os.path.abspath(__file__)), "production")
inter_cert_path = os.path.join(out, "intermediate_ca.crt")
inter_key_path = os.path.join(out, "intermediate_ca.key")
coord_key_path = os.path.join(out, "coordinator.key")
coord_cert_path = os.path.join(out, "coordinator.crt")

with open(inter_cert_path, "rb") as f:
    inter_cert = x509.load_pem_x509_certificate(f.read(), default_backend())
with open(inter_key_path, "rb") as f:
    inter_key = serialization.load_pem_private_key(f.read(), password=None, backend=default_backend())
with open(coord_key_path, "rb") as f:
    coord_key = serialization.load_pem_private_key(f.read(), password=None, backend=default_backend())

san_entries = [
    x509.DNSName("localhost"),
    x509.DNSName("coord.dice.network"),
    x509.IPAddress(ipaddress.IPv4Address("127.0.0.1")),
    x509.IPAddress(ipaddress.IPv4Address("0.0.0.0")),
] + [x509.IPAddress(ip) for ip in extra_ips]

coord_subject = x509.Name([
    x509.NameAttribute(NameOID.ORGANIZATION_NAME, "DICE Network"),
    x509.NameAttribute(NameOID.COMMON_NAME, "DICE Coordinator"),
])

coord_cert = (
    x509.CertificateBuilder()
    .subject_name(coord_subject)
    .issuer_name(inter_cert.subject)
    .public_key(coord_key.public_key())
    .serial_number(x509.random_serial_number())
    .not_valid_before(datetime.datetime.utcnow())
    .not_valid_after(datetime.datetime.utcnow() + datetime.timedelta(days=730))
    .add_extension(x509.BasicConstraints(ca=False, path_length=None), critical=True)
    .add_extension(x509.ExtendedKeyUsage([ExtendedKeyUsageOID.SERVER_AUTH]), critical=False)
    .add_extension(x509.SubjectAlternativeName(san_entries), critical=False)
    .sign(inter_key, hashes.SHA256(), default_backend())
)

with open(coord_cert_path, "wb") as f:
    f.write(coord_cert.public_bytes(serialization.Encoding.PEM))

print(f"Re-issued {coord_cert_path}")
print("SAN:")
for e in san_entries:
    print(f"  {e}")
