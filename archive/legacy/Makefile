.PHONY: phase0 phase1 phase2 phase3 phase4 phase5 full \
        clean clean-manifests clean-build \
        fmt lint audit test-unit \
        help

PYTHON   := python3
MANIFEST_DIR := build/manifests

# ─── Phase targets ────────────────────────────────────────────────────────────

phase0:
	@echo "==> Phase 0: Foundation (PKI + Contract + Infra)"
	$(PYTHON) orchestrator.py --phase 0

phase1: phase0
	@echo "==> Phase 1: Firmware + Coordinator + SDK"
	$(PYTHON) orchestrator.py --phase 1

phase2: phase1
	@echo "==> Phase 2: Integration assembly"
	$(PYTHON) orchestrator.py --phase 2

phase3: phase2
	@echo "==> Phase 3: Schema compatibility verification"
	$(PYTHON) orchestrator.py --phase 3

phase4: phase3
	@echo "==> Phase 4: End-to-end tests"
	$(PYTHON) orchestrator.py --phase 4

phase5: phase4
	@echo "==> Phase 5: Release artifacts"
	$(PYTHON) orchestrator.py --phase 5

full:
	$(PYTHON) orchestrator.py --phase all

# ─── Development helpers ──────────────────────────────────────────────────────

fmt:
	cargo fmt --all
	cd firmware && idf.py --preview clang-check 2>/dev/null || true

lint:
	cargo clippy --workspace --all-targets -- -D warnings
	cargo clippy --manifest-path programs/dice/Cargo.toml -- -D warnings

audit:
	cargo audit

test-unit:
	cargo test --workspace
	cd programs/dice && anchor test --skip-deploy || true

# ─── Utility ──────────────────────────────────────────────────────────────────

clean-manifests:
	rm -f $(MANIFEST_DIR)/*.json

clean-build:
	rm -rf build/reports/*
	cargo clean

clean: clean-manifests clean-build

# ─── Individual agent entry points ────────────────────────────────────────────

run-coordinator:
	cargo run --release --package dice-coordinator

build-firmware:
	cd firmware && idf.py build

build-contract:
	anchor build

build-sdk:
	cargo build --package dice-vrf

build-mock-node:
	cargo build --release --package mock-firmware-node

# ─── Help ─────────────────────────────────────────────────────────────────────

help:
	@echo ""
	@echo "  DICE build system"
	@echo ""
	@echo "  make phase0          Foundation: PKI + Contract + Infra"
	@echo "  make phase1          Firmware + Coordinator + SDK"
	@echo "  make phase2          Integration assembly"
	@echo "  make phase3          Schema compatibility checks"
	@echo "  make phase4          End-to-end tests"
	@echo "  make phase5          Release artifacts"
	@echo "  make full            Run all phases"
	@echo ""
	@echo "  make fmt             Format all Rust + C code"
	@echo "  make lint            Run clippy"
	@echo "  make audit           Run cargo audit"
	@echo "  make test-unit       Run all unit tests"
	@echo "  make clean           Clean build artifacts and manifests"
	@echo ""
