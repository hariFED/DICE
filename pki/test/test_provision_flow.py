"""
Tests for PKI provisioning scripts.
Run: pytest pki/test/test_provision_flow.py -v
"""

from __future__ import annotations

import json
import sys
import tempfile
from pathlib import Path

import pytest

# ---------------------------------------------------------------------------
# Ensure pki/scripts is importable regardless of where pytest is invoked from
# ---------------------------------------------------------------------------
_PKI_DIR = Path(__file__).resolve().parent.parent
_SCRIPTS_DIR = _PKI_DIR / "scripts"
if str(_SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS_DIR))

from gen_dev_keypair import generate_keypair, save_keypair
from cryptography.hazmat.primitives.asymmetric import ec
from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ec import SECP256K1


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

@pytest.fixture()
def tmp_registry(tmp_path: Path) -> Path:
    """Return a path to a temporary (non-existent) registry JSON file."""
    return tmp_path / "device_registry.json"


@pytest.fixture()
def provision_env(tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    """Monkeypatch provision_device paths to use a temp directory."""
    import provision_device as pd

    registry_path = tmp_path / "device_registry.json"
    manifests_dir = tmp_path / "build" / "manifests"
    payloads_dir = tmp_path / "build" / "registration_payloads"
    certs_dir = tmp_path / "certs"

    monkeypatch.setattr(pd, "_REGISTRY_PATH", registry_path)
    monkeypatch.setattr(pd, "_MANIFESTS_DIR", manifests_dir)
    monkeypatch.setattr(pd, "_PAYLOADS_DIR", payloads_dir)
    monkeypatch.setattr(pd, "_CERTS_DIR", certs_dir)

    return {
        "tmp_path": tmp_path,
        "registry_path": registry_path,
        "manifests_dir": manifests_dir,
        "payloads_dir": payloads_dir,
    }


# ---------------------------------------------------------------------------
# Keypair generation tests
# ---------------------------------------------------------------------------

class TestGenerateKeypair:
    def test_generate_keypair_produces_valid_secp256k1_key(self):
        """generate_keypair() must return a valid ECDSA secp256k1 private key."""
        private_der, compressed_pubkey = generate_keypair()

        # Load DER private key
        private_key = serialization.load_der_private_key(private_der, password=None)

        # Verify it is EC and on the secp256k1 curve
        assert isinstance(private_key, ec.EllipticCurvePrivateKey)
        assert isinstance(private_key.curve, SECP256K1)

    def test_compressed_pubkey_is_33_bytes(self):
        """Compressed secp256k1 public key must be exactly 33 bytes."""
        _private_der, compressed_pubkey = generate_keypair()
        assert len(compressed_pubkey) == 33

    def test_compressed_pubkey_prefix_is_02_or_03(self):
        """Compressed point prefix must be 0x02 (even y) or 0x03 (odd y)."""
        _private_der, compressed_pubkey = generate_keypair()
        assert compressed_pubkey[0] in (0x02, 0x03)

    def test_keypairs_are_unique(self):
        """Successive calls must produce different keypairs."""
        _, pk1 = generate_keypair()
        _, pk2 = generate_keypair()
        assert pk1 != pk2

    def test_save_keypair_writes_expected_files(self, tmp_path: Path):
        """save_keypair() must write device_key.der, device_pubkey.hex, device_key.pem."""
        private_der, compressed_pubkey = generate_keypair()
        pubkey_hex = compressed_pubkey.hex()

        save_keypair(private_der, pubkey_hex, output_dir=tmp_path, device_id="test-001")

        assert (tmp_path / "device_key.der").exists()
        assert (tmp_path / "device_pubkey.hex").exists()
        assert (tmp_path / "device_key.pem").exists()
        assert (tmp_path / "device_id.txt").read_text().strip() == "test-001"

    def test_save_keypair_der_roundtrips(self, tmp_path: Path):
        """DER key written by save_keypair() must load back to same curve."""
        private_der, compressed_pubkey = generate_keypair()
        pubkey_hex = compressed_pubkey.hex()

        save_keypair(private_der, pubkey_hex, output_dir=tmp_path)
        reloaded_der = (tmp_path / "device_key.der").read_bytes()
        reloaded_key = serialization.load_der_private_key(reloaded_der, password=None)

        assert isinstance(reloaded_key.curve, SECP256K1)


# ---------------------------------------------------------------------------
# Provisioning flow tests
# ---------------------------------------------------------------------------

class TestProvisionDevMode:
    def test_provision_dev_mode_creates_device_records(
        self, provision_env: dict
    ):
        """provision_device.py --dev --count 3 must add 3 records to device_registry.json."""
        import provision_device as pd

        pd.main(["--dev", "--count", "3"])

        registry_path = provision_env["registry_path"]
        assert registry_path.exists(), "device_registry.json was not created"
        records = json.loads(registry_path.read_text())
        assert len(records) == 3

    def test_device_registry_has_required_fields(self, provision_env: dict):
        """Each registry record must contain all required fields."""
        import provision_device as pd

        pd.main(["--dev", "--count", "2"])

        registry_path = provision_env["registry_path"]
        records = json.loads(registry_path.read_text())

        required_fields = {
            "device_id",
            "pubkey_hex",
            "cert_serial",
            "cert_fingerprint",
            "firmware_hash",
            "provisioned_at",
            "provisioned_by",
            "mode",
        }

        for record in records:
            missing = required_fields - set(record.keys())
            assert not missing, f"Record {record.get('device_id')} missing fields: {missing}"

    def test_provision_dev_mode_writes_manifest(self, provision_env: dict):
        """provision_device.py must write build/manifests/pki_manifest.json."""
        import provision_device as pd

        pd.main(["--dev", "--count", "1"])

        manifest_path = provision_env["manifests_dir"] / "pki_manifest.json"
        assert manifest_path.exists(), "pki_manifest.json was not created"

        manifest = json.loads(manifest_path.read_text())
        assert manifest["device_count"] == 1
        assert "devices" in manifest

    def test_provision_dev_mode_pubkey_hex_is_66_chars(self, provision_env: dict):
        """pubkey_hex in registry must be 66 hex characters (33 bytes compressed)."""
        import provision_device as pd

        pd.main(["--dev", "--count", "1"])

        registry_path = provision_env["registry_path"]
        records = json.loads(registry_path.read_text())
        for record in records:
            assert len(record["pubkey_hex"]) == 66, (
                f"pubkey_hex for {record['device_id']} is not 66 chars: {record['pubkey_hex']}"
            )

    def test_provision_dev_mode_device_ids_are_unique(self, provision_env: dict):
        """All provisioned device IDs must be unique."""
        import provision_device as pd

        pd.main(["--dev", "--count", "5"])

        registry_path = provision_env["registry_path"]
        records = json.loads(registry_path.read_text())
        ids = [r["device_id"] for r in records]
        assert len(ids) == len(set(ids)), "Duplicate device IDs found in registry"

    def test_provision_dev_mode_mode_field_is_devnet(self, provision_env: dict):
        """mode field in registry records must be 'devnet' when using --dev."""
        import provision_device as pd

        pd.main(["--dev", "--count", "2"])

        registry_path = provision_env["registry_path"]
        records = json.loads(registry_path.read_text())
        for record in records:
            assert record["mode"] == "devnet", (
                f"Expected mode=devnet, got {record['mode']} for {record['device_id']}"
            )


# ---------------------------------------------------------------------------
# Registration payload tests
# ---------------------------------------------------------------------------

class TestRegisterPayloadRoundtrip:
    def test_register_payload_roundtrip(self, provision_env: dict):
        """Provision devices, then generate registration payloads; verify JSON is valid."""
        import provision_device as pd
        from register_payload import generate_payloads

        pd.main(["--dev", "--count", "3"])

        registry_path = provision_env["registry_path"]
        payloads_dir = provision_env["payloads_dir"]

        written = generate_payloads(
            registry_path=registry_path,
            output_dir=payloads_dir,
        )

        assert len(written) == 3, f"Expected 3 payload files, got {len(written)}"

        for payload_path in written:
            assert payload_path.exists()
            payload = json.loads(payload_path.read_text())
            assert "device_id" in payload
            assert "device_pubkey_hex" in payload
            assert "device_pubkey_b58" in payload
            assert "instruction" in payload
            assert payload["instruction"] == "register_device"
            assert "program_id" in payload

    def test_register_payload_pubkey_b58_is_nonempty(self, provision_env: dict):
        """Base58-encoded pubkey in payload must be a non-empty string."""
        import provision_device as pd
        from register_payload import generate_payloads

        pd.main(["--dev", "--count", "1"])

        registry_path = provision_env["registry_path"]
        payloads_dir = provision_env["payloads_dir"]

        written = generate_payloads(
            registry_path=registry_path,
            output_dir=payloads_dir,
        )

        assert written, "No payload files written"
        payload = json.loads(written[0].read_text())
        b58 = payload["device_pubkey_b58"]
        assert isinstance(b58, str) and len(b58) > 0

    def test_register_payload_hex_and_b58_are_consistent(self, provision_env: dict):
        """Hex and base58 in payload must encode the same bytes."""
        import provision_device as pd
        from register_payload import generate_payloads, _b58encode

        pd.main(["--dev", "--count", "1"])

        registry_path = provision_env["registry_path"]
        payloads_dir = provision_env["payloads_dir"]

        written = generate_payloads(
            registry_path=registry_path,
            output_dir=payloads_dir,
        )

        payload = json.loads(written[0].read_text())
        hex_bytes = bytes.fromhex(payload["device_pubkey_hex"])
        expected_b58 = _b58encode(hex_bytes)
        assert payload["device_pubkey_b58"] == expected_b58
