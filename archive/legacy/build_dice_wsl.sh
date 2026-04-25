#!/usr/bin/env bash
set -e
export PATH="$HOME/.local/share/solana/install/active_release/bin:$HOME/.cargo/bin:/usr/local/bin:/usr/bin:/bin"
cd /mnt/c/Users/Abcom/DICE
echo "solana: $(solana --version)"
echo "cargo:  $(cargo --version 2>&1 || echo MISSING)"
echo "rustc:  $(rustc --version 2>&1 || echo MISSING)"
echo "------- start cargo build-sbf --------"
cargo build-sbf --manifest-path programs/dice/Cargo.toml 2>&1 | tail -50
echo "------- ls dice.so --------"
ls -la target/deploy/dice.so
