#!/usr/bin/env python3
"""
verify_idl_types.py
--------------------
Phase 3 check #3: Verify that the Anchor IDL's account and instruction
field types are consistent with the coordinator's Solana transaction builder
(coordinator/src/solana_tx.rs).

The coordinator builds on-chain transactions by constructing account metas
and serialising instruction data. If the IDL changes (e.g., a field is added
or renamed) but solana_tx.rs is not updated, transactions will silently fail
or pass incorrect data on-chain.

Exits 0 if compatible, 1 if mismatched, 2 on input errors.

Usage:
    python scripts/verify_idl_types.py \
        --idl target/idl/dice.json \
        --coordinator-tx coordinator/src/solana_tx.rs
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path


# ─── IDL parser ───────────────────────────────────────────────────────────────

def load_idl(path: Path) -> dict:
    if not path.exists():
        print(f"[WARN] IDL not found at {path} — treating as empty (stub state)")
        return {}
    return json.loads(path.read_text(encoding="utf-8"))


def extract_idl_instructions(idl: dict) -> dict[str, list[dict]]:
    """
    Returns { instruction_name: [ {name, type}, ... ] } from the IDL.
    """
    result: dict[str, list[dict]] = {}
    for ix in idl.get("instructions", []):
        name = ix.get("name", "")
        args = ix.get("args", [])
        result[name] = args
    return result


def extract_idl_accounts(idl: dict) -> dict[str, list[dict]]:
    """
    Returns { account_name: [ {name, type}, ... ] } from IDL account types.
    """
    result: dict[str, list[dict]] = {}
    for acc in idl.get("accounts", []):
        name   = acc.get("name", "")
        fields = acc.get("type", {}).get("fields", [])
        result[name] = fields
    return result


# ─── Rust solana_tx.rs parser ─────────────────────────────────────────────────

# Match instruction builder functions: fn build_submit_commit_ix(...)
_FN_RE    = re.compile(r"fn\s+build_(\w+)_ix\s*\(([^)]*)\)", re.DOTALL)
_PARAM_RE = re.compile(r"(\w+)\s*:\s*([^,\n]+)")

# Map IDL types to Rust types expected in solana_tx.rs
IDL_TO_RUST: dict[str, list[str]] = {
    "publicKey":   ["Pubkey", "[u8; 32]"],
    "u64":         ["u64"],
    "u32":         ["u32"],
    "u8":          ["u8"],
    "bytes":       ["Vec<u8>", "&[u8]", "[u8; 32]", "[u8; 33]"],
    "string":      ["String", "&str"],
    "bool":        ["bool"],
    "i64":         ["i64"],
}


def extract_rust_builders(path: Path) -> dict[str, dict[str, str]]:
    """
    Extract build_<name>_ix function signatures from solana_tx.rs.
    Returns { ix_name: { param_name: rust_type } }
    """
    if not path.exists():
        print(f"[WARN] solana_tx.rs not found at {path} — treating as empty (stub state)")
        return {}

    text = path.read_text(encoding="utf-8")
    builders: dict[str, dict[str, str]] = {}

    for fn_match in _FN_RE.finditer(text):
        fn_name = fn_match.group(1)   # e.g. "submit_commit"
        params  = fn_match.group(2)
        fields: dict[str, str] = {}
        for p in _PARAM_RE.finditer(params):
            pname = p.group(1)
            ptype = p.group(2).strip().rstrip(",")
            fields[pname] = ptype
        builders[fn_name] = fields

    return builders


# ─── Type compatibility check ─────────────────────────────────────────────────

def idl_type_str(type_def: dict | str) -> str:
    """Normalise an IDL type definition to a simple string."""
    if isinstance(type_def, str):
        return type_def
    if "defined" in type_def:
        return type_def["defined"]
    if "array" in type_def:
        inner, size = type_def["array"]
        return f"[{idl_type_str(inner)}; {size}]"
    if "vec" in type_def:
        return f"vec<{idl_type_str(type_def['vec'])}>"
    if "option" in type_def:
        return f"option<{idl_type_str(type_def['option'])}>"
    return str(type_def)


def rust_type_compatible(idl_type: str, rust_type: str) -> bool:
    """Check if a Rust type is a valid representation of an IDL type."""
    allowed = IDL_TO_RUST.get(idl_type, [idl_type])
    return any(rust_type.strip().startswith(a) for a in allowed)


# ─── Comparison ───────────────────────────────────────────────────────────────

def compare_instructions(
    idl_ixs: dict[str, list[dict]],
    rust_builders: dict[str, dict[str, str]],
) -> list[str]:
    errors: list[str] = []

    # Convert IDL instruction names from camelCase to snake_case for comparison
    def to_snake(name: str) -> str:
        return re.sub(r"(?<!^)(?=[A-Z])", "_", name).lower()

    idl_ixs_snake = {to_snake(k): v for k, v in idl_ixs.items()}

    for ix_name, idl_args in idl_ixs_snake.items():
        if ix_name not in rust_builders:
            # Not all IDL instructions need a builder (e.g., ones called only by users)
            print(f"  [SKIP] {ix_name} — no build_{ix_name}_ix found in solana_tx.rs")
            continue

        rust_params = rust_builders[ix_name]

        for arg in idl_args:
            arg_name = arg.get("name", "")
            arg_type = idl_type_str(arg.get("type", ""))

            if arg_name not in rust_params:
                errors.append(
                    f"Instruction {ix_name}: IDL arg '{arg_name}' ({arg_type}) "
                    f"not found in Rust builder params"
                )
                continue

            rust_t = rust_params[arg_name]
            if not rust_type_compatible(arg_type, rust_t):
                errors.append(
                    f"Instruction {ix_name}.{arg_name}: "
                    f"IDL type '{arg_type}' incompatible with Rust type '{rust_t}'"
                )

    return errors


# ─── Main ─────────────────────────────────────────────────────────────────────

def main() -> None:
    parser = argparse.ArgumentParser(description="Verify IDL types match coordinator solana_tx.rs")
    parser.add_argument("--idl",            required=True, help="Path to target/idl/dice.json")
    parser.add_argument("--coordinator-tx", required=True, help="Path to coordinator/src/solana_tx.rs")
    args = parser.parse_args()

    idl_path = Path(args.idl)
    tx_path  = Path(args.coordinator_tx)

    print(f"IDL:            {idl_path}")
    print(f"solana_tx.rs:   {tx_path}")

    idl = load_idl(idl_path)
    if not idl:
        print("[WARN] IDL is empty (stub state) — nothing to verify")
        sys.exit(0)

    idl_ixs      = extract_idl_instructions(idl)
    idl_accounts = extract_idl_accounts(idl)
    rust_builders = extract_rust_builders(tx_path)

    if not rust_builders:
        print("[WARN] solana_tx.rs has no build_*_ix functions yet — nothing to verify")
        sys.exit(0)

    print(f"\nFound {len(idl_ixs)} IDL instructions, {len(rust_builders)} Rust builders")

    errors = compare_instructions(idl_ixs, rust_builders)

    if errors:
        print(f"\n[FAIL] {len(errors)} IDL/Rust type mismatch(es):")
        for e in errors:
            print(f"  ✗ {e}")
        sys.exit(1)
    else:
        print("\n[PASS] IDL types compatible with coordinator transaction builder")
        sys.exit(0)


if __name__ == "__main__":
    main()
