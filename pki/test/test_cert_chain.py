"""
Tests for PKI certificate chain integrity.
Run: pytest pki/test/test_cert_chain.py -v
"""

from __future__ import annotations

import datetime
import sys
import tempfile
from pathlib import Path

import pytest

# ---------------------------------------------------------------------------
# Ensure pki/scripts is importable
# ---------------------------------------------------------------------------
_PKI_DIR = Path(__file__).resolve().parent.parent
_SCRIPTS_DIR = _PKI_DIR / "scripts"
if str(_SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS_DIR))

from cryptography import x509
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import ec
from cryptography.x509.oid import ExtendedKeyUsageOID, NameOID


# ---------------------------------------------------------------------------
# Helpers to build minimal CA / device certificates for testing
# ---------------------------------------------------------------------------

def _make_ca_cert(
    common_name: str,
    signing_key: ec.EllipticCurvePrivateKey | None = None,
    issuer_cert: x509.Certificate | None = None,
    issuer_key: ec.EllipticCurvePrivateKey | None = None,
    path_length: int | None = 0,
) -> tuple[x509.Certificate, ec.EllipticCurvePrivateKey]:
    """Create a minimal CA certificate (self-signed or intermediate)."""
    key = signing_key or ec.generate_private_key(ec.SECP256R1())

    subject = x509.Name([
        x509.NameAttribute(NameOID.COMMON_NAME, common_name),
        x509.NameAttribute(NameOID.ORGANIZATION_NAME, "DICE Test PKI"),
    ])

    now = datetime.datetime.now(datetime.timezone.utc)

    if issuer_cert is None:
        # Self-signed root
        issuer_name = subject
        sign_key = key
    else:
        issuer_name = issuer_cert.subject
        sign_key = issuer_key

    cert = (
        x509.CertificateBuilder()
        .subject_name(subject)
        .issuer_name(issuer_name)
        .public_key(key.public_key())
        .serial_number(x509.random_serial_number())
        .not_valid_before(now)
        .not_valid_after(now + datetime.timedelta(days=365 * 5))
        .add_extension(
            x509.BasicConstraints(ca=True, path_length=path_length),
            critical=True,
        )
        .add_extension(
            x509.KeyUsage(
                digital_signature=False,
                content_commitment=False,
                key_encipherment=False,
                data_encipherment=False,
                key_agreement=False,
                key_cert_sign=True,
                crl_sign=True,
                encipher_only=False,
                decipher_only=False,
            ),
            critical=True,
        )
        .sign(sign_key, hashes.SHA256())
    )
    return cert, key


def _make_device_cert(
    device_id: str,
    issuer_cert: x509.Certificate,
    issuer_key: ec.EllipticCurvePrivateKey,
    is_ca: bool = False,
    include_client_auth: bool = True,
) -> x509.Certificate:
    """Create a device leaf certificate signed by the given issuer."""
    device_key = ec.generate_private_key(ec.SECP256K1())

    subject = x509.Name([
        x509.NameAttribute(NameOID.COMMON_NAME, f"dice-device-{device_id}"),
    ])

    now = datetime.datetime.now(datetime.timezone.utc)
    builder = (
        x509.CertificateBuilder()
        .subject_name(subject)
        .issuer_name(issuer_cert.subject)
        .public_key(device_key.public_key())
        .serial_number(x509.random_serial_number())
        .not_valid_before(now)
        .not_valid_after(now + datetime.timedelta(days=365 * 10))
        .add_extension(
            x509.BasicConstraints(ca=is_ca, path_length=None),
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
    )

    if include_client_auth:
        builder = builder.add_extension(
            x509.ExtendedKeyUsage([ExtendedKeyUsageOID.CLIENT_AUTH]),
            critical=False,
        )

    return builder.sign(issuer_key, hashes.SHA256())


def _cert_to_pem(cert: x509.Certificate) -> bytes:
    return cert.public_bytes(serialization.Encoding.PEM)


def _build_ca_bundle(
    root_cert: x509.Certificate,
    intermediate_cert: x509.Certificate,
) -> bytes:
    return _cert_to_pem(root_cert) + _cert_to_pem(intermediate_cert)


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

@pytest.fixture(scope="module")
def pki_hierarchy():
    """Build a minimal Root CA → Intermediate CA → Device cert hierarchy for testing."""
    root_cert, root_key = _make_ca_cert("DICE Test Root CA", path_length=1)
    int_cert, int_key = _make_ca_cert(
        "DICE Test Intermediate CA",
        issuer_cert=root_cert,
        issuer_key=root_key,
        path_length=0,
    )
    device_cert = _make_device_cert("test-001", issuer_cert=int_cert, issuer_key=int_key)

    return {
        "root_cert": root_cert,
        "root_key": root_key,
        "int_cert": int_cert,
        "int_key": int_key,
        "device_cert": device_cert,
    }


@pytest.fixture()
def ca_bundle_pem(pki_hierarchy: dict, tmp_path: Path) -> Path:
    """Write CA bundle PEM to a temp file and return its path."""
    bundle_bytes = _build_ca_bundle(
        pki_hierarchy["root_cert"],
        pki_hierarchy["int_cert"],
    )
    bundle_path = tmp_path / "ca_bundle.pem"
    bundle_path.write_bytes(bundle_bytes)
    return bundle_path


# ---------------------------------------------------------------------------
# CA bundle tests
# ---------------------------------------------------------------------------

class TestCaBundleStructure:
    def test_ca_bundle_structure(self, ca_bundle_pem: Path):
        """CA bundle PEM must contain exactly two valid PEM certificates."""
        bundle_data = ca_bundle_pem.read_bytes()

        # Count PEM headers
        pem_headers = bundle_data.count(b"-----BEGIN CERTIFICATE-----")
        assert pem_headers == 2, (
            f"CA bundle should contain 2 certificates, found {pem_headers}"
        )

    def test_ca_bundle_first_cert_is_root(self, pki_hierarchy: dict, ca_bundle_pem: Path):
        """First certificate in the CA bundle must be the Root CA."""
        bundle_text = ca_bundle_pem.read_text()
        pem_blocks = [
            b.strip() + "\n-----END CERTIFICATE-----"
            for b in bundle_text.split("-----END CERTIFICATE-----")
            if "-----BEGIN CERTIFICATE-----" in b
        ]

        first_cert = x509.load_pem_x509_certificate(pem_blocks[0].encode())
        root_cn = pki_hierarchy["root_cert"].subject.get_attributes_for_oid(NameOID.COMMON_NAME)
        first_cn = first_cert.subject.get_attributes_for_oid(NameOID.COMMON_NAME)
        assert root_cn[0].value == first_cn[0].value

    def test_ca_bundle_certs_are_cas(self, ca_bundle_pem: Path):
        """Both certs in the bundle must have basicConstraints isCA=true."""
        bundle_data = ca_bundle_pem.read_bytes()

        # Split into individual PEM blocks
        pem_certs = []
        lines = bundle_data.decode().split("\n")
        current: list[str] = []
        for line in lines:
            current.append(line)
            if "-----END CERTIFICATE-----" in line:
                pem_certs.append("\n".join(current).encode())
                current = []

        assert len(pem_certs) == 2

        for pem_cert in pem_certs:
            cert = x509.load_pem_x509_certificate(pem_cert)
            bc = cert.extensions.get_extension_for_class(x509.BasicConstraints)
            assert bc.value.ca is True, (
                f"CA cert '{cert.subject}' must have isCA=True"
            )

    def test_ca_bundle_intermediate_signed_by_root(self, pki_hierarchy: dict):
        """Intermediate CA certificate must be signed by the Root CA."""
        root_cert = pki_hierarchy["root_cert"]
        int_cert = pki_hierarchy["int_cert"]

        # issuer of intermediate must match subject of root
        assert int_cert.issuer == root_cert.subject

        # Verify signature using root public key
        root_cert.public_key().verify(
            int_cert.signature,
            int_cert.tbs_certificate_bytes,
            ec.ECDSA(hashes.SHA256()),
        )


# ---------------------------------------------------------------------------
# Device certificate tests
# ---------------------------------------------------------------------------

class TestDeviceCertNotCA:
    def test_device_cert_not_ca(self, pki_hierarchy: dict):
        """Device certificate must have basicConstraints isCA=false."""
        device_cert = pki_hierarchy["device_cert"]
        bc = device_cert.extensions.get_extension_for_class(x509.BasicConstraints)
        assert bc.value.ca is False, (
            "Device certificate must NOT be a CA (isCA must be False)"
        )

    def test_device_cert_has_client_auth_eku(self, pki_hierarchy: dict):
        """Device certificate must have clientAuth in Extended Key Usage."""
        device_cert = pki_hierarchy["device_cert"]
        eku = device_cert.extensions.get_extension_for_class(x509.ExtendedKeyUsage)
        eku_oids = list(eku.value)
        assert ExtendedKeyUsageOID.CLIENT_AUTH in eku_oids, (
            f"Device cert EKU must include clientAuth. Found: {eku_oids}"
        )

    def test_device_cert_has_digital_signature_ku(self, pki_hierarchy: dict):
        """Device certificate must have digitalSignature in Key Usage."""
        device_cert = pki_hierarchy["device_cert"]
        ku = device_cert.extensions.get_extension_for_class(x509.KeyUsage)
        assert ku.value.digital_signature is True, (
            "Device cert must have digitalSignature Key Usage"
        )

    def test_device_cert_signed_by_intermediate(self, pki_hierarchy: dict):
        """Device certificate must be signed by the Intermediate CA."""
        int_cert = pki_hierarchy["int_cert"]
        device_cert = pki_hierarchy["device_cert"]

        assert device_cert.issuer == int_cert.subject

        int_cert.public_key().verify(
            device_cert.signature,
            device_cert.tbs_certificate_bytes,
            ec.ECDSA(hashes.SHA256()),
        )

    def test_device_cert_validity_is_10_years(self, pki_hierarchy: dict):
        """Device certificate validity must be approximately 10 years (87600h)."""
        device_cert = pki_hierarchy["device_cert"]
        validity_days = (
            device_cert.not_valid_after_utc - device_cert.not_valid_before_utc
        ).days
        # Allow ±5 days tolerance
        expected_days = 365 * 10
        assert abs(validity_days - expected_days) <= 5, (
            f"Device cert validity {validity_days} days, expected ~{expected_days}"
        )

    def test_device_cert_cn_prefixed_with_dice_device(self, pki_hierarchy: dict):
        """Device cert CN must be prefixed with 'dice-device-'."""
        device_cert = pki_hierarchy["device_cert"]
        cn_attrs = device_cert.subject.get_attributes_for_oid(NameOID.COMMON_NAME)
        assert cn_attrs, "Device cert must have a Common Name"
        cn = cn_attrs[0].value
        assert cn.startswith("dice-device-"), (
            f"Device cert CN must start with 'dice-device-', got: {cn}"
        )


# ---------------------------------------------------------------------------
# CA cert with wrong properties (negative tests)
# ---------------------------------------------------------------------------

class TestNegativeCases:
    def test_cert_with_is_ca_true_fails_device_check(self, pki_hierarchy: dict):
        """A cert built with isCA=True must NOT pass the device-cert check."""
        int_cert = pki_hierarchy["int_cert"]
        int_key = pki_hierarchy["int_key"]

        # Build a leaf cert that accidentally has isCA=True
        bad_device_cert = _make_device_cert(
            "bad-001",
            issuer_cert=int_cert,
            issuer_key=int_key,
            is_ca=True,
        )

        bc = bad_device_cert.extensions.get_extension_for_class(x509.BasicConstraints)
        # Confirm our test helper produced isCA=True
        assert bc.value.ca is True
        # Confirm the device check would catch this
        assert bc.value.ca is not False

    def test_cert_without_client_auth_fails_eku_check(self, pki_hierarchy: dict):
        """A cert without clientAuth EKU must not be treated as a valid device cert."""
        int_cert = pki_hierarchy["int_cert"]
        int_key = pki_hierarchy["int_key"]

        no_eku_cert = _make_device_cert(
            "no-eku-001",
            issuer_cert=int_cert,
            issuer_key=int_key,
            include_client_auth=False,
        )

        # Should have no EKU extension at all, or at least not clientAuth
        try:
            eku = no_eku_cert.extensions.get_extension_for_class(x509.ExtendedKeyUsage)
            has_client_auth = ExtendedKeyUsageOID.CLIENT_AUTH in list(eku.value)
        except x509.ExtensionNotFound:
            has_client_auth = False

        assert not has_client_auth, (
            "Cert built without clientAuth should not have clientAuth EKU"
        )
