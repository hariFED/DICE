"""
E2E test fixtures and docker-compose lifecycle.
Agent: Testing Agent
"""

import json
import subprocess
import time
from pathlib import Path

import pytest

ROOT       = Path(__file__).parent.parent.parent
INFRA      = ROOT / "infra"
COMPOSE    = INFRA / "docker-compose.test.yml"
MANIFESTS  = ROOT / "build" / "manifests"


def _load_manifest(name: str) -> dict:
    path = MANIFESTS / f"{name}_manifest.json"
    if path.exists():
        return json.loads(path.read_text())
    return {}


@pytest.fixture(scope="session")
def coordinator_manifest():
    return _load_manifest("coordinator")


@pytest.fixture(scope="session")
def contract_manifest():
    return _load_manifest("contract")


@pytest.fixture(scope="session")
def pki_manifest():
    return _load_manifest("pki")


@pytest.fixture(scope="session", autouse=True)
def test_stack():
    """Bring up docker-compose test stack for the entire test session."""
    if not COMPOSE.exists():
        pytest.skip("docker-compose.test.yml not yet present")

    subprocess.run(
        ["docker", "compose", "-f", str(COMPOSE), "up", "-d",
         "postgres", "step-ca"],
        check=False,
    )
    time.sleep(5)   # wait for services to be ready

    yield

    subprocess.run(
        ["docker", "compose", "-f", str(COMPOSE), "down", "--volumes"],
        check=False,
    )
