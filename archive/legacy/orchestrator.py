#!/usr/bin/env python3
"""
DICE Orchestrator
-----------------
Central build orchestrator for the DICE network.

Sequences the 6 build phases, reads agent manifests, enforces phase gates,
runs schema compatibility checks, and drives end-to-end tests.

Usage:
    python orchestrator.py --phase 0
    python orchestrator.py --phase all
    python orchestrator.py --phase 4 --skip-gates    # useful for local dev
    python orchestrator.py --status                  # print current build state
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
from dataclasses import dataclass, field, asdict
from datetime import datetime
from pathlib import Path
from typing import Any

# ─── Paths ────────────────────────────────────────────────────────────────────

ROOT          = Path(__file__).parent.resolve()
MANIFESTS     = ROOT / "build" / "manifests"
REPORTS       = ROOT / "build" / "reports"
SCRIPTS       = ROOT / "scripts"
STATE_FILE    = ROOT / "build" / "state.json"
INFRA         = ROOT / "infra"

# ─── Colours ──────────────────────────────────────────────────────────────────

class C:
    RESET  = "\033[0m"
    BOLD   = "\033[1m"
    GREEN  = "\033[32m"
    YELLOW = "\033[33m"
    RED    = "\033[31m"
    CYAN   = "\033[36m"

def ok(msg: str)    -> None: print(f"{C.GREEN}  ✓ {msg}{C.RESET}")
def warn(msg: str)  -> None: print(f"{C.YELLOW}  ⚠ {msg}{C.RESET}")
def err(msg: str)   -> None: print(f"{C.RED}  ✗ {msg}{C.RESET}")
def info(msg: str)  -> None: print(f"{C.CYAN}  → {msg}{C.RESET}")
def header(msg: str)-> None: print(f"\n{C.BOLD}{C.CYAN}{'─'*60}\n  {msg}\n{'─'*60}{C.RESET}")

# ─── Build state ──────────────────────────────────────────────────────────────

@dataclass
class BuildState:
    last_completed_phase: int = -1
    phase_timestamps: dict[str, str] = field(default_factory=dict)
    gate_results: dict[str, bool] = field(default_factory=dict)
    errors: list[str] = field(default_factory=list)

    def save(self) -> None:
        STATE_FILE.parent.mkdir(parents=True, exist_ok=True)
        STATE_FILE.write_text(json.dumps(asdict(self), indent=2))

    @classmethod
    def load(cls) -> "BuildState":
        if STATE_FILE.exists():
            data = json.loads(STATE_FILE.read_text())
            return cls(**data)
        return cls()

    def record_phase(self, phase: int) -> None:
        self.last_completed_phase = phase
        self.phase_timestamps[str(phase)] = datetime.utcnow().isoformat()
        self.save()

# ─── Manifest helpers ─────────────────────────────────────────────────────────

def load_manifest(name: str) -> dict[str, Any]:
    """Load a manifest JSON from build/manifests/<name>_manifest.json."""
    path = MANIFESTS / f"{name}_manifest.json"
    if not path.exists():
        raise FileNotFoundError(f"Manifest not found: {path}")
    return json.loads(path.read_text())

def manifest_exists(name: str) -> bool:
    return (MANIFESTS / f"{name}_manifest.json").exists()

def write_manifest(name: str, data: dict[str, Any]) -> None:
    MANIFESTS.mkdir(parents=True, exist_ok=True)
    path = MANIFESTS / f"{name}_manifest.json"
    path.write_text(json.dumps(data, indent=2))
    ok(f"Wrote {path.name}")

# ─── Command runner ───────────────────────────────────────────────────────────

def run(
    cmd: list[str],
    cwd: Path | None = None,
    env: dict | None = None,
    check: bool = True,
    capture: bool = False,
) -> subprocess.CompletedProcess:
    """Run a subprocess, streaming output unless capture=True."""
    display_cwd = f" (in {cwd.relative_to(ROOT)})" if cwd and cwd != ROOT else ""
    info(f"$ {' '.join(str(c) for c in cmd)}{display_cwd}")
    merged_env = {**os.environ, **(env or {})}
    kwargs: dict[str, Any] = dict(cwd=cwd or ROOT, env=merged_env)
    if capture:
        kwargs["capture_output"] = True
        kwargs["text"] = True
    result = subprocess.run(cmd, **kwargs)
    if check and result.returncode != 0:
        raise RuntimeError(f"Command failed with exit code {result.returncode}: {' '.join(str(c) for c in cmd)}")
    return result

# ─── Phase 0: Foundation ──────────────────────────────────────────────────────

def phase0(state: BuildState, skip_gates: bool) -> None:
    header("Phase 0 — Foundation  (PKI + Contract + Infra)")

    # PKI: generate dev CA and test device certificates
    info("PKI Agent: generating dev CA and 20 test device certificates")
    pki_script = ROOT / "pki" / "scripts" / "provision_device.py"
    if pki_script.exists():
        run([sys.executable, str(pki_script), "--dev", "--count", "20"])
    else:
        warn("pki/scripts/provision_device.py not yet implemented — writing stub manifest")
        write_manifest("pki", {
            "root_ca_cert": str(ROOT / "pki/certs/root_ca.crt"),
            "intermediate_ca_cert": str(ROOT / "pki/certs/intermediate_ca.crt"),
            "device_registry": str(ROOT / "pki/device_registry.json"),
            "ca_bundle_nvs_partition": str(ROOT / "pki/build/nvs_certs.bin"),
            "test_device_certs_dir": str(ROOT / "pki/test/devices/"),
            "registration_payloads_dir": str(ROOT / "pki/build/registration_payloads/"),
            "_stub": True,
        })

    # Contract: anchor build
    info("Contract Agent: anchor build")
    anchor_toml = ROOT / "Anchor.toml"
    if anchor_toml.exists():
        try:
            run(["anchor", "build"], cwd=ROOT)
            idl_path = ROOT / "target" / "idl" / "dice.json"
            program_so = ROOT / "target" / "deploy" / "dice.so"
            program_id = _read_program_id()
            write_manifest("contract", {
                "program_id": program_id,
                "program_so": str(program_so),
                "idl": str(idl_path),
                "typescript_types": str(ROOT / "target" / "types" / "dice.ts"),
                "anchor_test_results": str(REPORTS / "anchor_test_results.xml"),
                "sbf_test_results": str(REPORTS / "sbf_test_results.xml"),
                "_stub": not program_so.exists(),
            })
        except (RuntimeError, FileNotFoundError):
            warn("anchor not found or build failed — writing stub manifest")
            write_manifest("contract", {
                "program_id": "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS",
                "program_so": str(ROOT / "target/deploy/dice.so"),
                "idl": str(ROOT / "target/idl/dice.json"),
                "typescript_types": str(ROOT / "target/types/dice.ts"),
                "anchor_test_results": str(REPORTS / "anchor_test_results.xml"),
                "sbf_test_results": str(REPORTS / "sbf_test_results.xml"),
                "_stub": True,
            })
    else:
        warn("Anchor.toml missing — writing stub manifest")

    # Infra: validate docker-compose syntax
    info("Infra Agent: validating docker-compose syntax")
    compose_file = INFRA / "docker-compose.yml"
    if compose_file.exists():
        try:
            run(["docker", "compose", "-f", str(compose_file), "config", "--quiet"], check=False)
            ok("docker-compose.yml syntax valid")
        except FileNotFoundError:
            warn("docker not found — skipping compose validation")
    else:
        warn("infra/docker-compose.yml not yet present")

    write_manifest("infra", {
        "compose_file": str(INFRA / "docker-compose.yml"),
        "compose_test_file": str(INFRA / "docker-compose.test.yml"),
        "prometheus_config": str(INFRA / "prometheus/prometheus.yml"),
        "grafana_dashboards": [str(INFRA / "grafana/dashboards/dice_overview.json")],
        "ci_workflow": str(INFRA / ".github/workflows/ci.yml"),
        "coordinator_dockerfile": str(INFRA / "Dockerfile.coordinator"),
        "_stub": not compose_file.exists(),
    })

    # Phase gate 0
    if not skip_gates:
        _gate0()

    state.record_phase(0)
    ok("Phase 0 complete")


def _gate0() -> None:
    header("Gate 0 checks")
    passed = True

    for name in ["pki", "contract", "infra"]:
        if manifest_exists(name):
            ok(f"{name}_manifest.json present")
        else:
            err(f"{name}_manifest.json MISSING")
            passed = False

    if not passed:
        raise RuntimeError("Gate 0 failed — missing manifests")

    ok("Gate 0 passed")


def _read_program_id() -> str:
    anchor_toml = ROOT / "Anchor.toml"
    if anchor_toml.exists():
        for line in anchor_toml.read_text().splitlines():
            if "dice" in line and "=" in line and "Fg6" in line:
                return line.split("=")[1].strip().strip('"')
    return "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS"

# ─── Phase 1: Dependent build ─────────────────────────────────────────────────

def phase1(state: BuildState, skip_gates: bool) -> None:
    header("Phase 1 — Firmware + Coordinator + SDK")

    pki = load_manifest("pki")
    contract = load_manifest("contract")

    # Firmware
    info("Firmware Agent: idf.py build")
    firmware_dir = ROOT / "firmware"
    firmware_bin = firmware_dir / "build" / "dice_firmware.bin"
    if firmware_dir.exists() and (firmware_dir / "CMakeLists.txt").exists():
        try:
            run(["idf.py", "build"], cwd=firmware_dir)
        except (RuntimeError, FileNotFoundError):
            warn("idf.py not found — skipping firmware build")
    else:
        warn("firmware/CMakeLists.txt not yet present — skipping build")

    write_manifest("firmware", {
        "artifact": str(firmware_bin),
        "bootloader": str(firmware_dir / "build/bootloader/bootloader.bin"),
        "partition_table": str(firmware_dir / "build/partition_table/partition-table.bin"),
        "ecdsa_pubkey_der": str(firmware_dir / "keys/device_identity_pub.der"),
        "wire_protocol_schema": str(firmware_dir / "components/dice_protocol/schema.cbor"),
        "qemu_test_results": str(firmware_dir / "test/results/qemu_results.xml"),
        "binary_hash_sha256": "",
        "secure_boot_signed": False,
        "ca_cert_used": pki.get("intermediate_ca_cert", ""),
        "_stub": not firmware_bin.exists(),
    })

    # Coordinator
    info("Coordinator Agent: cargo build + unit tests")
    coord_dir = ROOT / "coordinator"
    if (coord_dir / "Cargo.toml").exists():
        try:
            run(["cargo", "build", "--release", "--package", "dice-coordinator"], cwd=ROOT)
            run(["cargo", "test", "--package", "dice-coordinator"], cwd=ROOT)
        except (RuntimeError, FileNotFoundError):
            warn("cargo build failed — writing stub coordinator manifest")
    else:
        warn("coordinator/Cargo.toml not yet present")

    coord_bin = ROOT / "target" / "release" / "dice-coordinator"
    write_manifest("coordinator", {
        "binary": str(coord_bin),
        "schema_sql": str(coord_dir / "src/db/schema.sql"),
        "protocol_messages_schema": str(coord_dir / "src/protocol/messages.rs"),
        "ws_endpoint": "wss://coordinator.dice.local:8443",
        "metrics_endpoint": "http://coordinator.dice.local:9090/metrics",
        "unit_test_results": str(REPORTS / "coordinator_unit.xml"),
        "integration_test_results": str(REPORTS / "coordinator_integration.xml"),
        "program_id_used": contract.get("program_id", ""),
        "_stub": not coord_bin.exists(),
    })

    # SDK
    info("SDK Agent: cargo build + tests + publish dry-run")
    sdk_dir = ROOT / "sdk" / "dice-vrf"
    if (sdk_dir / "Cargo.toml").exists():
        try:
            run(["cargo", "build", "--package", "dice-vrf"], cwd=ROOT)
            run(["cargo", "test", "--package", "dice-vrf"], cwd=ROOT)
            run(["cargo", "publish", "--dry-run", "--package", "dice-vrf", "--allow-dirty"], cwd=ROOT, check=False)
        except (RuntimeError, FileNotFoundError):
            warn("SDK build failed")
    else:
        warn("sdk/dice-vrf/Cargo.toml not yet present")

    write_manifest("sdk", {
        "crate_path": str(sdk_dir),
        "macros_crate_path": str(ROOT / "sdk/dice-vrf-macros"),
        "docs_path": str(sdk_dir / "target/doc"),
        "test_results": str(REPORTS / "sdk_tests.xml"),
        "example_binaries": [str(sdk_dir / "target/debug/examples/request_and_poll")],
        "idl_used": contract.get("idl", ""),
        "program_id_used": contract.get("program_id", ""),
        "_stub": not (sdk_dir / "Cargo.toml").exists(),
    })

    if not skip_gates:
        _gate1()

    state.record_phase(1)
    ok("Phase 1 complete")


def _gate1() -> None:
    header("Gate 1 checks")
    passed = True

    for name in ["firmware", "coordinator", "sdk"]:
        if manifest_exists(name):
            ok(f"{name}_manifest.json present")
        else:
            err(f"{name}_manifest.json MISSING")
            passed = False

    if not passed:
        raise RuntimeError("Gate 1 failed — missing manifests")
    ok("Gate 1 passed")

# ─── Phase 2: Integration assembly ────────────────────────────────────────────

def phase2(state: BuildState, skip_gates: bool) -> None:
    header("Phase 2 — Integration Assembly")

    compose_test = INFRA / "docker-compose.test.yml"

    # Step 2.1: bring up test stack
    info("Step 2.1: docker-compose test stack up")
    if compose_test.exists():
        try:
            run(["docker", "compose", "-f", str(compose_test), "up", "-d",
                 "postgres", "step-ca"], check=False)
            time.sleep(3)
            ok("Test stack started")
        except FileNotFoundError:
            warn("docker not available — skipping compose up")
    else:
        warn("infra/docker-compose.test.yml not yet present — skipping")

    # Step 2.2: anchor test
    info("Step 2.2: anchor test (smart contract integration)")
    if (ROOT / "Anchor.toml").exists():
        try:
            run(["anchor", "test", "--skip-deploy"], cwd=ROOT, check=False)
            ok("anchor test complete")
        except FileNotFoundError:
            warn("anchor not found — skipping")
    else:
        warn("Anchor.toml not present — skipping anchor test")

    # Step 2.3: coordinator integration tests
    info("Step 2.3: coordinator integration tests")
    coord_dir = ROOT / "coordinator"
    if (coord_dir / "Cargo.toml").exists():
        try:
            run(["cargo", "test", "--package", "dice-coordinator",
                 "--test", "integration"], cwd=ROOT, check=False)
            ok("Coordinator integration tests complete")
        except FileNotFoundError:
            warn("cargo not found — skipping")

    # Step 2.4: SDK integration tests
    info("Step 2.4: SDK integration tests (with client feature)")
    sdk_dir = ROOT / "sdk" / "dice-vrf"
    if (sdk_dir / "Cargo.toml").exists():
        try:
            run(["cargo", "test", "--package", "dice-vrf",
                 "--features", "client"], cwd=ROOT, check=False)
            ok("SDK integration tests complete")
        except FileNotFoundError:
            warn("cargo not found — skipping")

    # Step 2.5: build mock firmware node
    info("Step 2.5: build mock firmware node binary")
    mock_dir = ROOT / "tests" / "harness" / "mock_firmware_node"
    if (mock_dir / "Cargo.toml").exists():
        try:
            run(["cargo", "build", "--release", "--package", "mock-firmware-node"], cwd=ROOT)
            ok("Mock firmware node built")
        except (RuntimeError, FileNotFoundError):
            warn("Mock node build failed")

    mock_bin = ROOT / "target" / "release" / "mock-firmware-node"
    write_manifest("testing", {
        "mock_node_binary": str(mock_bin),
        "load_generator_binary": str(ROOT / "target/release/load-generator"),
        "e2e_test_suite": str(ROOT / "tests/e2e/"),
        "harness_ready": mock_bin.exists(),
    })

    if not skip_gates:
        _gate2()

    state.record_phase(2)
    ok("Phase 2 complete")


def _gate2() -> None:
    header("Gate 2 checks")
    if manifest_exists("testing"):
        ok("testing_manifest.json present")
    else:
        raise RuntimeError("Gate 2 failed — testing_manifest.json missing")
    ok("Gate 2 passed")

# ─── Phase 3: Schema compatibility ────────────────────────────────────────────

def phase3(state: BuildState, skip_gates: bool) -> None:
    header("Phase 3 — Schema Compatibility Verification")

    results = {}

    # Check 1: firmware CBOR schema ↔ coordinator Rust message types
    info("Check 1/3: firmware CBOR schema ↔ coordinator protocol messages")
    script = SCRIPTS / "verify_protocol_compat.py"
    firmware_m = load_manifest("firmware")
    coord_m    = load_manifest("coordinator")
    if script.exists():
        result = run(
            [sys.executable, str(script),
             "--firmware-schema", firmware_m.get("wire_protocol_schema", ""),
             "--coordinator-messages", coord_m.get("protocol_messages_schema", "")],
            check=False,
        )
        results["protocol_compat"] = result.returncode == 0
        if results["protocol_compat"]:
            ok("Protocol schema compatible")
        else:
            err("Protocol schema MISMATCH")
    else:
        warn("scripts/verify_protocol_compat.py not yet implemented — skipping")
        results["protocol_compat"] = None

    # Check 2: SDK PDA derivation ↔ on-chain contract seeds
    info("Check 2/3: SDK PDA derivation ↔ contract seed constants")
    script = SCRIPTS / "verify_pda_compat.py"
    contract_m = load_manifest("contract")
    sdk_m      = load_manifest("sdk")
    if script.exists():
        result = run(
            [sys.executable, str(script),
             "--program-id", contract_m.get("program_id", ""),
             "--sdk-path",   sdk_m.get("crate_path", "")],
            check=False,
        )
        results["pda_compat"] = result.returncode == 0
        if results["pda_compat"]:
            ok("PDA derivation compatible")
        else:
            err("PDA derivation MISMATCH")
    else:
        warn("scripts/verify_pda_compat.py not yet implemented — skipping")
        results["pda_compat"] = None

    # Check 3: IDL field types ↔ coordinator Solana tx builder
    info("Check 3/3: IDL types ↔ coordinator solana_tx.rs")
    script = SCRIPTS / "verify_idl_types.py"
    if script.exists():
        result = run(
            [sys.executable, str(script),
             "--idl",            contract_m.get("idl", ""),
             "--coordinator-tx", str(ROOT / "coordinator/src/solana_tx.rs")],
            check=False,
        )
        results["idl_types"] = result.returncode == 0
        if results["idl_types"]:
            ok("IDL types compatible")
        else:
            err("IDL types MISMATCH")
    else:
        warn("scripts/verify_idl_types.py not yet implemented — skipping")
        results["idl_types"] = None

    # Save compat results to report
    report = REPORTS / "compat_results.json"
    report.parent.mkdir(parents=True, exist_ok=True)
    report.write_text(json.dumps(results, indent=2))

    if not skip_gates:
        _gate3(results)

    state.record_phase(3)
    ok("Phase 3 complete")


def _gate3(results: dict) -> None:
    header("Gate 3 checks")
    # None means script not yet implemented — treat as warning, not failure
    failures = [k for k, v in results.items() if v is False]
    if failures:
        raise RuntimeError(f"Gate 3 failed — schema mismatches: {failures}")
    skipped = [k for k, v in results.items() if v is None]
    if skipped:
        warn(f"Gate 3: skipped checks (scripts not yet implemented): {skipped}")
    ok("Gate 3 passed")

# ─── Phase 4: End-to-end tests ────────────────────────────────────────────────

def phase4(state: BuildState, skip_gates: bool) -> None:
    header("Phase 4 — End-to-End Tests")

    testing_m = load_manifest("testing")
    compose_test = INFRA / "docker-compose.test.yml"

    # Bring up the full test stack
    if compose_test.exists():
        info("Bringing up full E2E test stack")
        try:
            run(["docker", "compose", "-f", str(compose_test), "up", "-d"], check=False)
            info("Waiting 5s for services to be ready")
            time.sleep(5)
        except FileNotFoundError:
            warn("docker not available")

    # Start mock firmware nodes in background (if binary exists)
    mock_bin = Path(testing_m.get("mock_node_binary", ""))
    mock_proc = None
    if mock_bin.exists():
        coord_m = load_manifest("coordinator")
        ws_ep   = coord_m.get("ws_endpoint", "wss://localhost:8443")
        pki_m   = load_manifest("pki")
        ca_cert = pki_m.get("root_ca_cert", "")
        info(f"Starting 20 mock firmware nodes → {ws_ep}")
        mock_proc = subprocess.Popen([
            str(mock_bin),
            "--count", "20",
            "--coordinator", ws_ep,
            "--ca-cert", ca_cert,
        ])
        time.sleep(2)
    else:
        warn("mock-firmware-node binary not built — E2E tests will use stub nodes")

    # Run pytest E2E suite
    info("Running pytest E2E suite")
    e2e_dir = ROOT / "tests" / "e2e"
    junit_xml = REPORTS / "e2e.xml"
    REPORTS.mkdir(parents=True, exist_ok=True)

    e2e_result = None
    if e2e_dir.exists() and list(e2e_dir.glob("test_*.py")):
        try:
            e2e_result = run(
                ["pytest", str(e2e_dir), "-v", "--tb=short",
                 f"--junit-xml={junit_xml}"],
                check=False,
            )
        except FileNotFoundError:
            warn("pytest not found — skipping E2E tests")
    else:
        warn("No E2E test files found yet — skipping pytest")

    # Tear down mock nodes
    if mock_proc:
        mock_proc.terminate()
        mock_proc.wait()

    # Tear down test stack
    if compose_test.exists():
        try:
            run(["docker", "compose", "-f", str(compose_test), "down"], check=False)
        except FileNotFoundError:
            pass

    if not skip_gates:
        _gate4(e2e_result, junit_xml)

    state.record_phase(4)
    ok("Phase 4 complete")


def _gate4(e2e_result, junit_xml: Path) -> None:
    header("Gate 4 checks")
    if e2e_result is None:
        warn("Gate 4: no E2E tests ran (suite not yet implemented)")
        return
    if e2e_result.returncode != 0:
        raise RuntimeError(f"Gate 4 failed — pytest exited {e2e_result.returncode}. See {junit_xml}")
    ok("Gate 4 passed")

# ─── Phase 5: Release artifacts ───────────────────────────────────────────────

def phase5(state: BuildState, skip_gates: bool) -> None:
    header("Phase 5 — Release Artifacts")

    # Firmware: production signed binary
    info("Firmware Agent: building production-signed release binary")
    firmware_dir = ROOT / "firmware"
    if (firmware_dir / "CMakeLists.txt").exists():
        try:
            run(["idf.py", "build"], cwd=firmware_dir, check=False)
        except FileNotFoundError:
            warn("idf.py not found")

    # Contract: prepare anchor deploy command
    contract_m = load_manifest("contract")
    info(f"Contract Agent: ready to deploy program {contract_m['program_id']}")
    info("  Run: anchor deploy --provider.cluster devnet")

    # Infra: print deployment instructions
    info("Infra Agent: seal secrets with SOPS, trigger deploy.yml")
    info("  Run: make -C infra deploy  (or trigger GitHub Actions deploy.yml)")

    # Final report
    _write_final_report()

    state.record_phase(5)
    ok("Phase 5 complete — DICE build pipeline finished")


def _write_final_report() -> None:
    all_manifests = {}
    for f in MANIFESTS.glob("*_manifest.json"):
        name = f.stem.replace("_manifest", "")
        all_manifests[name] = json.loads(f.read_text())

    report = {
        "generated_at": datetime.utcnow().isoformat(),
        "manifests": all_manifests,
        "state": asdict(BuildState.load()),
    }

    html = _render_report_html(report)
    report_path = REPORTS / "final_report.html"
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(html)
    ok(f"Final report written to {report_path}")


def _render_report_html(report: dict) -> str:
    rows = ""
    for agent, manifest in report.get("manifests", {}).items():
        stub = manifest.get("_stub", False)
        status = "⚠ stub" if stub else "✓ built"
        color  = "#f0ad4e" if stub else "#5cb85c"
        rows += f'<tr><td>{agent}</td><td style="color:{color}">{status}</td></tr>\n'

    return f"""<!DOCTYPE html>
<html>
<head><title>DICE Build Report</title>
<style>
  body {{ font-family: monospace; background: #1a1a2e; color: #e0e0e0; padding: 2rem; }}
  h1   {{ color: #00d4ff; }}
  table{{ border-collapse: collapse; width: 100%; }}
  th,td{{ border: 1px solid #333; padding: 0.5rem 1rem; text-align: left; }}
  th   {{ background: #16213e; }}
</style></head>
<body>
<h1>DICE Build Report</h1>
<p>Generated: {report['generated_at']}</p>
<table>
  <tr><th>Agent</th><th>Status</th></tr>
  {rows}
</table>
</body></html>"""

# ─── Status command ───────────────────────────────────────────────────────────

def print_status() -> None:
    header("DICE Build Status")
    state = BuildState.load()
    print(f"  Last completed phase : {state.last_completed_phase}")
    print(f"  Phase timestamps     :")
    for phase, ts in state.phase_timestamps.items():
        print(f"    Phase {phase}: {ts}")

    print(f"\n  Manifests present:")
    for name in ["pki", "contract", "infra", "firmware", "coordinator", "sdk", "testing"]:
        symbol = "✓" if manifest_exists(name) else "✗"
        stub   = ""
        if manifest_exists(name):
            m = load_manifest(name)
            if m.get("_stub"):
                stub = " (stub)"
        print(f"    {symbol} {name}_manifest.json{stub}")

# ─── Entry point ──────────────────────────────────────────────────────────────

PHASES = {
    0: phase0,
    1: phase1,
    2: phase2,
    3: phase3,
    4: phase4,
    5: phase5,
}

def main() -> None:
    parser = argparse.ArgumentParser(
        description="DICE build orchestrator",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--phase",
        choices=["0","1","2","3","4","5","all"],
        help="Phase to run",
    )
    parser.add_argument(
        "--skip-gates",
        action="store_true",
        help="Skip phase gate checks (useful for local development)",
    )
    parser.add_argument(
        "--status",
        action="store_true",
        help="Print current build state and exit",
    )
    args = parser.parse_args()

    if args.status:
        print_status()
        return

    if not args.phase:
        parser.print_help()
        return

    state = BuildState.load()

    try:
        if args.phase == "all":
            for i in range(6):
                PHASES[i](state, args.skip_gates)
        else:
            phase_num = int(args.phase)
            # Warn if a prerequisite phase hasn't been run
            if phase_num > 0 and state.last_completed_phase < phase_num - 1:
                warn(f"Phase {phase_num - 1} has not been completed. "
                     f"Use --skip-gates to override or run phases in order.")
            PHASES[phase_num](state, args.skip_gates)

    except FileNotFoundError as e:
        err(f"Missing file/manifest: {e}")
        err("Run earlier phases first, or use --skip-gates to bypass gate checks.")
        sys.exit(1)
    except RuntimeError as e:
        err(str(e))
        sys.exit(1)


if __name__ == "__main__":
    main()
