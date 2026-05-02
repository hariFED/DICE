# Anchor 1.0.0 Migration Reference (from 0.31.x)

> **Scope:** Single-source migration guide for upgrading a Solana program crate from `anchor-lang = "0.31.1"` to `anchor-lang = "1.0.0"`. Covers every public-API breaking change relevant to a `#[program]` crate that does its own CPIs, calls `invoke`, builds `AccountMeta` lists, and uses `solana_program::*` re-exports.
>
> **Anchor 1.0.0 release date:** 2026-04-02 [Source: https://github.com/solana-foundation/anchor/blob/master/CHANGELOG.md].
>
> **Recommended Solana toolchain:** Solana CLI **3.1.10**, on-chain runtime **3.x** [Source: https://www.anchor-lang.com/docs/updates/release-notes/1-0-0].
>
> **Audience:** This was written for the DICE coordinator + on-chain crates (`programs/dice`, `dapp-examples/coin-toss`, `dapp-examples/pulse`, plus the `coordinator` host process that builds and submits instructions).

---

## TL;DR — What actually changed for our codebase

1. **Anchor 1.0 dropped its hidden re-export of `solana-program 2.x`** and now re-exports a *constellation of split crates* (`solana-pubkey` / `solana-address`, `solana-instruction`, `solana-account-info`, `solana-cpi`, `solana-invoke`, `solana-sysvar`, etc.) all pinned to **Solana 3.x** [Source: https://github.com/solana-foundation/anchor/pull/4031, https://raw.githubusercontent.com/solana-foundation/anchor/v1.0.0/lang/src/lib.rs].
2. **`Pubkey` is now `solana_address::Pubkey` (a type alias for `Address`)**, not `solana_program::pubkey::Pubkey`. If your crate has *both* `anchor-lang = "1.0"` *and* a direct `solana-program = "2"` (or any dep that pulls v2), rustc sees two distinct `Pubkey` types and emits diagnostics like `expected __Pubkey, found Pubkey`. The double-underscore name is the rustc disambiguation suffix for a re-imported alias inside macro-generated code [Source: https://github.com/anza-xyz/solana-sdk/issues/204, https://docs.rs/anchor-lang/1.0.0/anchor_lang/prelude/struct.Pubkey.html].
3. **`CpiContext::new` no longer takes the program `AccountInfo`** — it takes the program **`Pubkey`** [Source: https://github.com/solana-foundation/anchor/pull/2762, https://raw.githubusercontent.com/solana-foundation/anchor/v1.0.0/lang/src/context.rs].
4. **`invoke()` moved to `solana-invoke` and its second arg is `&[AccountInfo<'_>]`**. A `&Vec<AccountInfo>` no longer auto-deref-coerces in some macro-expanded sites; pass `&account_infos[..]` or `account_infos.as_slice()` [Source: https://docs.rs/solana-invoke/latest/solana_invoke/fn.invoke.html, https://github.com/solana-foundation/anchor/pull/3900].
5. **`hash`, `keccak`, `secp256k1_recover` are no longer re-exported under `anchor_lang::solana_program::*`.** They still exist in the standalone `solana-program 3.x` crate (as re-exports of `solana-keccak-hasher`, `solana-secp256k1-recover`, etc.), so depending on `solana-program = "2"` or `"3"` directly is the canonical fix [Source: https://raw.githubusercontent.com/solana-foundation/anchor/v1.0.0/lang/src/lib.rs, https://docs.rs/solana-program/latest/solana_program/index.html].
6. **`Context<'info, T<'info>>` is the canonical handler signature in 1.0.** The 0.30 → 0.31 collapse is unchanged in 1.0 [Source: https://www.anchor-lang.com/docs/updates/release-notes/1-0-0].
7. **Duplicate mutable accounts are rejected by default.** Add the `dup` constraint when intentional (PR #3946).
8. **`#[derive(Accounts)]` with bare `AccountInfo<'info>` fields now warns** — switch to `UncheckedAccount<'info>` (PR #3854).

---

## Section 1 — Workspace deps

### 1.1 Crate-level pins (verbatim from the Anchor 1.0 workspace)

From `solana-foundation/anchor` tag `v1.0.0` workspace `Cargo.toml` [Source: https://raw.githubusercontent.com/solana-foundation/anchor/v1.0.0/Cargo.toml]:

```toml
# Solana SDK split crates
solana-program        = "3.0.0"
solana-account        = "3.2.0"
solana-pubkey         = "3.0.0"   # itself re-exports solana-address
solana-instruction    = "3.0.0"
solana-account-info   = "3.1.0"
solana-sysvar         = "3.1.1"
solana-program-error  = "3.0.0"
solana-cpi            = "3.0.0"

# Off-chain / client crates
solana-account-decoder = "3.0.14"
solana-cli-config      = "3.0.14"
solana-client          = "3.0.14"
solana-clock           = "3.0.1"
solana-keypair         = "3.0.1"
solana-signature       = "3.3.0"
solana-signer          = "3.0.0"
solana-transaction     = "3.0.1"
```

### 1.2 `anchor-lang` direct dependencies

From `lang/Cargo.toml` at v1.0.0 [Source: https://raw.githubusercontent.com/solana-foundation/anchor/v1.0.0/lang/Cargo.toml]:

```toml
[package]
edition = "2021"

[dependencies]
borsh                 = "1.5.7"
bytemuck              = { version = "1", features = ["derive"] }
base64                = "0.21"
bincode               = "1"
const-crypto          = "0.3.0"
thiserror             = "1"
# plus workspace pins for: solana-account-info, solana-clock, solana-cpi,
# solana-define-syscall, solana-feature-gate-interface, solana-instruction,
# solana-instructions-sysvar, solana-invoke, solana-loader-v3-interface,
# solana-msg, solana-program-entrypoint, solana-program-error,
# solana-program-memory, solana-program-option, solana-program-pack,
# solana-pubkey, solana-sdk-ids, solana-stake-interface,
# solana-system-interface, solana-sysvar, solana-sysvar-id
```

> **Notable:** `anchor-lang` does **not** depend on the umbrella `solana-program` crate at all. It composes itself from the split crates. The `pub mod solana_program` inside `anchor-lang` is now a **synthetic facade** that re-exports the split crates into the old paths.

### 1.3 `borsh` requirement

`anchor-lang = "1.0.0"` requires **`borsh = "1.5.7"`** (PR #4012). Your current pin of `borsh = "^1.5"` is compatible; bump it to `borsh = "1.5.7"` (or `^1.5.7`) to silence the cargo resolver and to match Anchor's bound exactly [Source: https://github.com/solana-foundation/anchor/pull/4012, https://raw.githubusercontent.com/solana-foundation/anchor/v1.0.0/lang/Cargo.toml].

### 1.4 `anchor-spl` pin

`anchor-spl = "1.0.0"` is paired with `anchor-lang ^1.0.0` and exposes the modules `token`, `token_2022`, `token_2022_extensions`, `token_interface`, `associated_token`, `mint`, `metadata`, `memo`, `governance`, `stake` [Source: https://docs.rs/anchor-spl/1.0.0/anchor_spl/index.html]. Update `anchor-spl` lockstep with `anchor-lang`.

### 1.5 Rust / edition

`anchor-lang` v1.0 stays on **`edition = "2021"`**. No MSRV bump is documented in the changelog; CI uses Rust 1.79+ paths. The 0.32 release added an MSRV note to the Rust template (PR #3873). Your existing `rust-toolchain.toml` is fine if it pins Rust ≥ 1.79.

### 1.6 What Anchor 1.0 **drops** from its public API surface vs 0.31

Removed or renamed re-exports under `anchor_lang::solana_program` (verified by reading `lang/src/lib.rs` at v1.0.0 [Source: https://raw.githubusercontent.com/solana-foundation/anchor/v1.0.0/lang/src/lib.rs]):

| 0.31 path | 1.0 status | Replacement |
|---|---|---|
| `anchor_lang::solana_program::hash` | **Removed** | direct `solana-program = "3"` dep, then `solana_program::hash::hashv` |
| `anchor_lang::solana_program::keccak` | **Removed** | direct `solana-program = "3"` (re-exports `solana-keccak-hasher`) |
| `anchor_lang::solana_program::secp256k1_recover` | **Removed** | direct `solana-program = "3"` (re-exports `solana-secp256k1-recover`) |
| `anchor_lang::solana_program::sysvar::slot_hashes` | **Moved** | `anchor_lang::prelude::SlotHashes` (re-exported from `solana_sysvar::slot_hashes::SlotHashes`) |
| `anchor_lang::solana_program::system_instruction` | Re-exported via `solana_system_interface::instruction` | path still works |
| `anchor_lang::solana_program::pubkey::Pubkey` | Re-exported but **points to `solana_address::Pubkey`** | use `anchor_lang::prelude::Pubkey` |
| `anchor_lang::solana_program::instruction::AccountMeta` | Re-exported via `solana_instruction::*` | use `anchor_lang::prelude::AccountMeta` |
| `anchor_lang::solana_program::program::invoke` | Re-exported via `solana_invoke::invoke` | use `anchor_lang::solana_program::program::invoke` (path preserved) |

**Other removals from the framework:**

- `interface-instructions` feature and the `#[interface]` attribute (PR #4156) [Source: CHANGELOG].
- `EventData`, `EventIndex`, `StateCoder` traits/types (gone since 0.31, still gone) [Source: 0.31 release notes].
- `borsh 0.9` support (gone since 0.31) [Source: PR #3199].
- `solana-account-decoder` re-export from `anchor-client` (PR #4373).
- `[registry]` section in `Anchor.toml` (PR #4299).
- The `login`, `publish`, and Solang CLI commands.
- `program arch` options (PR #4295).
- The legacy on-chain IDL instructions — replaced by **Program Metadata Program (PMP)** (PR #3798).

---

## Section 2 — `Pubkey` and the `__Pubkey` mismatch

### 2.1 Why the error exists

Anchor 1.0 re-exports `Pubkey` from `solana-pubkey 3.x`, which itself re-exports `solana-address::Pubkey`, which is a type alias for `solana_address::Address` [Source: https://docs.rs/anchor-lang/1.0.0/anchor_lang/prelude/struct.Pubkey.html, https://github.com/anza-xyz/solana-sdk/issues/204].

Confirmation from the 1.0 source [Source: https://raw.githubusercontent.com/solana-foundation/anchor/v1.0.0/lang/src/lib.rs]:

```rust
// inside `pub mod solana_program { ... }`
pub use solana_pubkey as pubkey;
// ...
// inside `pub mod prelude { ... }`
pub use crate::solana_program::{
    account_info::{next_account_info, AccountInfo},
    instruction::AccountMeta,
    program_error::ProgramError,
    pubkey::Pubkey,
    *,
};
```

If **any** other crate in the dep graph (your direct `solana-program = "2"`, or a transitive dep on `mpl-token-metadata`, `spl-token`, `solana-program 1.x`, etc.) brings in `solana_program::pubkey::Pubkey` from a *different* major version, rustc treats them as two distinct nominal types. Inside Anchor's `#[derive(Accounts)]` and `#[account]` macro expansion, the macro re-imports `Pubkey` with a private alias for hygiene; when the diagnostic prints, that alias is rendered as `__Pubkey` (rustc's default disambiguation for shadowed/aliased names from generated code).

Therefore the error message **"expected `__Pubkey`, found `Pubkey`"** is shorthand for **"expected `solana_pubkey::Pubkey` (the one Anchor's macro grabbed), found `solana_program::pubkey::Pubkey` (the one your hand-written code passed in)"**. It is **not** a real `__Pubkey` newtype; it is a name-collision diagnostic.

### 2.2 Canonical fix

**Always import `Pubkey` and `AccountMeta` from `anchor_lang::prelude`** when you are inside an Anchor program crate. Do **not** import them from `solana_program::pubkey` or `solana_program::instruction` even if you have a direct `solana-program = "3"` dep — because `anchor_lang::prelude::Pubkey` is the version Anchor's generated `#[derive(Accounts)]` code will expect.

```rust
// 0.31 — happened to work because anchor re-exported solana-program 2.x
use anchor_lang::prelude::*;
use solana_program::instruction::AccountMeta;     // BAD in 1.0 if solana-program = "2"
use solana_program::pubkey::Pubkey;               // BAD in 1.0

let meta = AccountMeta::new(*acc.key, acc.is_signer);
```

```rust
// 1.0 — single source of truth: anchor_lang::prelude
use anchor_lang::prelude::*;
// AccountMeta and Pubkey are already in prelude — no extra imports needed.

let meta = AccountMeta::new(*acc.key, acc.is_signer);
```

If you genuinely need a function from `solana-program` (e.g. `secp256k1_recover`, `hashv`, `keccak::hashv`) that is **not** re-exported by Anchor 1.0, do **two** things:

1. Add `solana-program = "3.0"` (or whatever `anchor-lang 1.0` uses) to your crate's `Cargo.toml` so versions match.
2. **Convert** when crossing the boundary:

```rust
use anchor_lang::prelude::Pubkey as AnchorPubkey;
use solana_program::pubkey::Pubkey as SpPubkey;

// They are the same 32 bytes either way; conversion is bytewise.
let sp_key: SpPubkey = SpPubkey::new_from_array(anchor_pubkey.to_bytes());
let back:  AnchorPubkey = AnchorPubkey::new_from_array(sp_key.to_bytes());
```

If you align both deps to **the same Solana 3.x line** (recommended), they are *literally the same type* and no conversion is needed — but only if the resolver picks one version. Verify with `cargo tree -i solana-pubkey` and `cargo tree -i solana-program`.

### 2.3 `Pubkey::try_from(&[u8])`

Still works. `solana_pubkey::Pubkey` (and its `solana_address::Pubkey` alias) implements `TryFrom<&[u8]>` returning `Result<Pubkey, std::array::TryFromSliceError>` because it is a transparent newtype around `[u8; 32]`. No code change required for the coordinator's slice-to-pubkey conversions [Source: https://docs.rs/solana-address/latest/solana_address/].

### 2.4 Quick before/after

```rust
// BEFORE (0.31)
use anchor_lang::prelude::*;
use solana_program::instruction::AccountMeta;

pub fn build(acc: &AccountInfo) -> AccountMeta {
    AccountMeta::new(*acc.key, acc.is_signer)   // *acc.key is solana_program::pubkey::Pubkey
}
```

```rust
// AFTER (1.0)
use anchor_lang::prelude::*;
// AccountMeta and Pubkey come from prelude (i.e. solana_instruction::AccountMeta
// + solana_address::Pubkey), matching what Anchor's macros generate.

pub fn build(acc: &AccountInfo) -> AccountMeta {
    AccountMeta::new(*acc.key, acc.is_signer)
}
```

---

## Section 3 — `CpiContext::new` signature change

### 3.1 New struct definition

From `lang/src/context.rs` at v1.0.0 [Source: https://raw.githubusercontent.com/solana-foundation/anchor/v1.0.0/lang/src/context.rs]:

```rust
pub struct CpiContext<'a, 'b, 'c, 'info, T>
where
    T: ToAccountMetas + ToAccountInfos<'info>,
{
    pub accounts:           T,
    pub remaining_accounts: Vec<AccountInfo<'info>>,
    pub program_id:         Pubkey,                    // <-- was AccountInfo<'info> in 0.31
    pub signer_seeds:       &'a [&'b [&'c [u8]]],
}

impl<'a, 'b, 'c, 'info, T> CpiContext<'a, 'b, 'c, 'info, T> {
    pub fn new(program_id: Pubkey, accounts: T) -> Self { ... }

    pub fn new_with_signer(
        program_id: Pubkey,
        accounts: T,
        signer_seeds: &'a [&'b [&'c [u8]]],
    ) -> Self { ... }

    pub fn with_signer(mut self, signer_seeds: &'a [&'b [&'c [u8]]]) -> Self { ... }
    pub fn with_remaining_accounts(mut self, ra: Vec<AccountInfo<'info>>) -> Self { ... }
}
```

### 3.2 Migration rule

The first argument is now a **`Pubkey`**, not an `AccountInfo`. The `AccountInfo` of the program is no longer carried inside the context — `solana_invoke::invoke_signed` figures out the program from the instruction itself, and Anchor walks `accounts.to_account_infos()` for the rest [Source: https://github.com/solana-foundation/anchor/pull/2762].

### 3.3 System program transfer — before / after

```rust
// BEFORE — Anchor 0.31
use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};

let cpi_ctx = CpiContext::new(
    ctx.accounts.system_program.to_account_info(),       // AccountInfo
    Transfer {
        from: ctx.accounts.payer.to_account_info(),
        to:   ctx.accounts.vault.to_account_info(),
    },
);
transfer(cpi_ctx, lamports)?;
```

```rust
// AFTER — Anchor 1.0
use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer, System};

let cpi_ctx = CpiContext::new(
    System::id(),                                        // <-- Pubkey, not AccountInfo
    Transfer {
        from: ctx.accounts.payer.to_account_info(),
        to:   ctx.accounts.vault.to_account_info(),
    },
);
transfer(cpi_ctx, lamports)?;
```

> Equivalent shorthand: `ctx.accounts.system_program.key()` returns the same `Pubkey` and is still accepted. Many maintainers prefer the typed `System::id()` because it's a `const` and doesn't require the `system_program` field to be present in the accounts struct.

### 3.4 SPL Token CPI — before / after

```rust
// BEFORE — Anchor 0.31
use anchor_spl::token::{self, Token, Transfer as TokenTransfer};

let cpi_ctx = CpiContext::new(
    ctx.accounts.token_program.to_account_info(),
    TokenTransfer {
        from:      ctx.accounts.from.to_account_info(),
        to:        ctx.accounts.to.to_account_info(),
        authority: ctx.accounts.authority.to_account_info(),
    },
);
token::transfer(cpi_ctx, amount)?;
```

```rust
// AFTER — Anchor 1.0
use anchor_spl::token::{self, Token, Transfer as TokenTransfer};

let cpi_ctx = CpiContext::new(
    Token::id(),                                         // const Pubkey
    TokenTransfer {
        from:      ctx.accounts.from.to_account_info(),
        to:        ctx.accounts.to.to_account_info(),
        authority: ctx.accounts.authority.to_account_info(),
    },
);
token::transfer(cpi_ctx, amount)?;
```

### 3.5 PDA signer variant

```rust
// BEFORE — Anchor 0.31
let signer_seeds: &[&[&[u8]]] = &[&[b"vault", &[bump]]];
let cpi_ctx = CpiContext::new_with_signer(
    ctx.accounts.system_program.to_account_info(),
    Transfer { from, to },
    signer_seeds,
);

// AFTER — Anchor 1.0
let signer_seeds: &[&[&[u8]]] = &[&[b"vault", &[bump]]];
let cpi_ctx = CpiContext::new_with_signer(
    System::id(),
    Transfer { from, to },
    signer_seeds,
);
```

---

## Section 4 — `invoke()` and `AccountInfo` slices

### 4.1 New canonical signature

From `solana-invoke 3.x` [Source: https://docs.rs/solana-invoke/latest/solana_invoke/fn.invoke.html]:

```rust
pub fn invoke(
    instruction:   &Instruction,
    account_infos: &[AccountInfo<'_>],
) -> ProgramResult;

pub fn invoke_signed(
    instruction:    &Instruction,
    account_infos:  &[AccountInfo<'_>],
    signers_seeds:  &[&[&[u8]]],
) -> ProgramResult;
```

The function still lives at the **same path** under Anchor's facade — `anchor_lang::solana_program::program::invoke` — because `lang/src/lib.rs` re-exports it [Source: https://raw.githubusercontent.com/solana-foundation/anchor/v1.0.0/lang/src/lib.rs]:

```rust
pub mod program {
    pub use {
        solana_cpi::*,
        solana_invoke::{invoke, invoke_signed, invoke_signed_unchecked, invoke_unchecked},
    };
}
```

So **no import-path change** is required. Only the **argument type discipline** changed.

### 4.2 Why `&Vec<AccountInfo>` stops compiling

Rust's deref coercion *does* in principle convert `&Vec<T>` to `&[T]` at function call sites. The regression is real but indirect: in 1.0 the `invoke` re-export comes from `solana-invoke` which has a slightly tightened generic bound (no longer takes `impl AsRef<[AccountInfo]>`; it takes a literal slice), so the implicit auto-ref-then-deref chain that 0.31 allowed is broken at some call sites — particularly when the value is built inside a closure or returned from a method.

### 4.3 Fix — explicit slice

```rust
// BEFORE (0.31) — the implicit coercion worked here
let account_infos: Vec<AccountInfo> = vec![a.clone(), b.clone(), c.clone()];
invoke(&ix, &account_infos)?;
```

```rust
// AFTER (1.0) — be explicit
let account_infos: Vec<AccountInfo> = vec![a.clone(), b.clone(), c.clone()];
invoke(&ix, &account_infos[..])?;
// or
invoke(&ix, account_infos.as_slice())?;
```

This is purely a call-site change; the types you store in the `Vec` are unchanged.

### 4.4 If you also need `invoke_signed`

```rust
let signers_seeds: &[&[&[u8]]] = &[&[b"node-vault", node_id.as_ref(), &[bump]]];
invoke_signed(&ix, account_infos.as_slice(), signers_seeds)?;
```

---

## Section 5 — `Context` lifetimes (FYI — already done)

### 5.1 Canonical 1.0 form

```rust
// CANONICAL — same in 0.31 and 1.0
pub fn handler<'info>(ctx: Context<'_, '_, '_, 'info, MyAccounts<'info>>) -> Result<()> { ... }

// SHORTER ALIAS — preferred in the 1.0 release notes
pub fn handler(ctx: Context<MyAccounts>) -> Result<()> { ... }
```

Both forms compile in 1.0. The release notes specifically call out that the explicit four-lifetime form is no longer required for typical handlers [Source: https://www.anchor-lang.com/docs/updates/release-notes/1-0-0]:

```rust
// BEFORE (v0.32)
pub fn my_handler<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, MyAccounts<'info>>,
) -> Result<()> { ... }

// AFTER (v1.0) — works because lifetimes are elided
pub fn my_handler(ctx: Context<MyAccounts>) -> Result<()> { ... }
```

Your current `Context<'info, T<'info>>` (which is the 0.31 sugar form) **continues to work** in 1.0. There is no functional difference.

### 5.2 Remaining accounts

`Context::remaining_accounts` is still `&[AccountInfo<'info>]`. Same call pattern:

```rust
for acc in ctx.remaining_accounts.iter() { ... }
```

If you forward `remaining_accounts` into a `CpiContext`, use the new builder method:

```rust
let cpi_ctx = CpiContext::new(MyProgram::id(), MyAccounts { ... })
    .with_remaining_accounts(ctx.remaining_accounts.to_vec());
```

(This API is unchanged from 0.31; only the first arg of `CpiContext::new` changed.)

---

## Section 6 — Anything else likely to bite

### 6.1 Duplicate mutable accounts now rejected by default (PR #3946)

Anchor 1.0 enforces, at the `try_accounts` validation step, that the **same `Pubkey` does not appear twice as a `mut` account** in a single instruction. This includes `remaining_accounts`. Excluded from the check: `UncheckedAccount`, `Signer`-only, and accounts created with `init`. Including `init_if_needed` accounts in the check was added later (PR #4239).

If you intentionally need a duplicate (e.g. self-pay where `from == to`), opt out per-account with the new `dup` constraint:

```rust
#[derive(Accounts)]
pub struct SelfTransfer<'info> {
    #[account(mut)]
    pub from: Account<'info, MyState>,

    #[account(mut, dup)]                    // <-- explicit opt-in
    pub to: Account<'info, MyState>,
}
```

If you do not opt in and the check trips at runtime, you get an error of type `DuplicateMutableAccountKeys` (re-exported from the prelude in 1.0).

### 6.2 `AccountInfo` field in `#[derive(Accounts)]` is deprecated (PR #3854)

```rust
// 0.31 — fine
#[derive(Accounts)]
pub struct MyCtx<'info> {
    pub raw: AccountInfo<'info>,
}
```

```rust
// 1.0 — compile-time WARNING (will become an error in a future release)
// Use UncheckedAccount and add a /// CHECK: doc comment.
#[derive(Accounts)]
pub struct MyCtx<'info> {
    /// CHECK: validated manually below.
    pub raw: UncheckedAccount<'info>,
}
```

### 6.3 Account validation macros — `init`, `seeds`, `bump`, `has_one`

These are **unchanged in syntax** between 0.31 and 1.0. Some clarifications baked into 1.0:

- `init` constraints validate **owner on reload** now (PR #3837) — if you touched an account and Anchor reloads it after CPI, the owner check is enforced. This may surface latent bugs in code that intentionally mismatched owners post-init.
- `seeds = [...]` parsing was relaxed in 1.0 to accept more PDA seed expressions (PR #3813) — strictly an additive change.
- `init_if_needed` is included in the duplicate-mut check (PR #4239) — see §6.1.
- The `zero` constraint requires a `Discriminator` impl since 0.31 (PR #3118), unchanged in 1.0.

### 6.4 `IdlBuild` trait / `idl-build` feature gating

The `idl-build` feature on `anchor-lang` and `anchor-spl` is still required to emit IDL types. Three new tightenings in 1.0:

- The IDL build now **excludes external accounts** by default (PR #4197).
- The legacy "conflicting account names check" was removed (PR #4294).
- Multiple `#[error_code]` definitions in a single program are **rejected** (PR #4300).
- IDL is built on **stable rustc** (no nightly required) since 0.32 (PR #3842).
- Legacy on-chain IDL instructions are gone — replaced by **Program Metadata Program (PMP)** (PR #3798). Your `anchor idl init` / `idl upgrade` workflows change: see CLI section below.

### 6.5 `Account<'info, T>::try_deserialize`

Signature is unchanged: `pub fn try_deserialize(buf: &mut &[u8]) -> Result<T>`. Internally it now uses the dynamic-length discriminator path added in 0.31 (PR #3098, #3101), so accounts created with custom (`#[account(discriminator = ...)]`) discriminators round-trip correctly.

### 6.6 `secp256k1_recover` availability

- **Removed** from `anchor_lang::solana_program::*`.
- **Still present** in `solana-program 3.x` (re-exported from `solana-secp256k1-recover`) [Source: https://docs.rs/solana-program/latest/solana_program/index.html].
- **Action:** add a direct dep `solana-program = "3"` (matching Anchor's Solana version line) and `use solana_program::secp256k1_recover::secp256k1_recover;`. If your code already adopted `solana-program = "2"` for this same reason, just bump it to `"3"` to match Anchor's own version of `solana-pubkey` and avoid the §2 type collision.

### 6.7 `hashv`, `keccak::hashv`, `slot_hashes`

Same story:

| function / type | 0.31 path inside Anchor | 1.0 fix |
|---|---|---|
| `hash::hashv` | `anchor_lang::solana_program::hash::hashv` | direct dep `solana-program = "3"`, then `use solana_program::hash::hashv;` |
| `keccak::hashv` | `anchor_lang::solana_program::keccak::hashv` | direct dep `solana-program = "3"`, then `use solana_program::keccak::hashv;` |
| `SlotHashes` sysvar | `anchor_lang::solana_program::sysvar::slot_hashes::SlotHashes` | already in `anchor_lang::prelude::SlotHashes` (re-exported from `solana_sysvar::slot_hashes`) — no extra dep needed |

### 6.8 `system_instruction::transfer`

Path moved internally: it's now `solana_system_interface::instruction::transfer`. Anchor re-exports it as `anchor_lang::solana_program::system_instruction::transfer`, so no change needed if you were going through the Anchor facade.

### 6.9 `Migration<'info, From, To>` — new account type (PR #4060)

A net-add: schema-migration helper that lets you upgrade an account from one struct layout to another in a single instruction:

```rust
#[derive(Accounts)]
pub struct UpgradeNode<'info> {
    #[account(mut)]
    pub node: Migration<'info, NodeV1, NodeV2>,
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}
```

Useful for the DICE node-account v2 → v3 migration; can replace hand-written reallocate-and-copy code.

### 6.10 `Lazy` / `LazyAccount`

Added in 0.31 (PR #3194), still feature-gated as `lazy-account` in 1.0. New optimization in 1.0: enums with all unit variants and empty arrays auto-use `Lazy` (PR #4237). No action required unless you opt in.

### 6.11 Generic `Program` type (PR #3878)

`Program<'info>` (no second type parameter) now validates only that the account is executable; useful for wrapper programs whose ID isn't known until runtime. Optional adoption.

### 6.12 Client-side: `anchor-client`

If your `coordinator/` uses `anchor-client`:

- `solana-client` was removed from `anchor-client` (PR #3877). Add `solana-client = "3.0.14"` as a direct dep.
- `solana-account-decoder` is no longer re-exported by `anchor-client` (PR #4373). Add `solana-account-decoder = "3.0.14"` as a direct dep if needed.
- Sending a tx no longer panics on signing failure; it returns `Err` (PR #3865). Adjust error handling in `coordinator/src/solana_tx.rs`.
- `events` closure now accepts `FnMut` (PR #4024) — strictly less restrictive.
- Multiple WebSocket listeners no longer deadlock (PR #4250) — relevant to `coordinator/src/solana_ws.rs`.

### 6.13 CLI changes (relevant for your `Anchor.toml` and CI)

- Drop the `[registry]` section from `Anchor.toml` (PR #4299).
- `anchor test` and `anchor localnet` default to **Surfpool** instead of `solana-test-validator` (PR #4106). Override via `[test]` config or `--validator solana-test-validator`.
- `anchor verify` shells out to `solana-verify` under the hood (PR #3768). Install via `cargo install solana-verify` — AVM auto-installs on first run.
- `anchor deploy` uploads the IDL by default; add `--no-idl` to skip (PR #3863).
- New `[hooks]` section in `Anchor.toml` for `pre-build`, `post-build`, `pre-test`, `post-test`, `pre-deploy`, `post-deploy` shell hooks (PR #3862).
- `idl init` and `idl upgrade` no longer take a `--program-id` argument — derived from `Anchor.toml` / workspace (PR #4130).
- TypeScript packages renamed: `@coral-xyz/anchor` → `@anchor-lang/core` (PR #4141). Update `package.json`, `tsconfig` paths, and all imports in `tests/`, `app/`, `sdk/`.

### 6.14 `Anchor.toml` example for v1.0

```toml
[toolchain]
anchor_version = "1.0.0"
solana_version = "3.1.10"

[features]
resolution      = true
skip-lint       = false

[programs.devnet]
dice = "DiCE..."

[provider]
cluster = "devnet"
wallet  = "~/.config/solana/id.json"

[scripts]
test = "yarn run ts-mocha -p ./tsconfig.json -t 1000000 tests/**/*.ts"

# Optional new in 1.0:
[hooks]
pre-build  = "scripts/check-idl.sh"
post-deploy = "scripts/notify-monitoring.sh"
```

### 6.15 New traits in the prelude (1.0)

`DuplicateMutableAccountKeys`, `Migration`, `Lamports` are now in the prelude. If you have local types named the same, alias your imports.

---

## Section 7 — Concrete migration checklist

Run these in order. Each step is one logical change; commit between steps for easy revert.

### Step 0 — Snapshot

```bash
git switch -c chore/anchor-1.0
cargo tree -i anchor-lang > /tmp/before-anchor.txt
cargo tree -i solana-program > /tmp/before-solana.txt
```

### Step 1 — Bump workspace deps in root `Cargo.toml`

```toml
[workspace.dependencies]
anchor-lang    = "1.0.0"
anchor-spl     = "1.0.0"
anchor-client  = "1.0.0"
solana-program = "3.0"           # MUST match Anchor's solana line
solana-sdk     = "3.0"           # only if used in coordinator
solana-client  = "3.0.14"        # only if coordinator uses it
borsh          = "1.5.7"
```

### Step 2 — Bump per-crate `Cargo.toml`

For every crate under `programs/*/Cargo.toml`, `coordinator/Cargo.toml`, `sdk/*/Cargo.toml`:

```toml
anchor-lang    = { workspace = true, features = ["init-if-needed"] }   # keep your features
anchor-spl     = { workspace = true, features = ["idl-build"] }
solana-program = { workspace = true }
borsh          = { workspace = true }
```

### Step 3 — `Anchor.toml`

- Remove the `[registry]` section if present.
- Bump `anchor_version = "1.0.0"`.
- Bump `solana_version = "3.1.10"`.

### Step 4 — Find/replace passes (run from repo root)

| Find pattern (regex) | Replace with | Files |
|---|---|---|
| `use solana_program::pubkey::Pubkey;` | *delete the line* (use prelude) | `programs/**/*.rs` |
| `use solana_program::instruction::AccountMeta;` | *delete the line* (use prelude) | `programs/**/*.rs` |
| `use anchor_lang::solana_program::hash` | `use solana_program::hash` | `**/*.rs` |
| `use anchor_lang::solana_program::keccak` | `use solana_program::keccak` | `**/*.rs` |
| `use anchor_lang::solana_program::secp256k1_recover` | `use solana_program::secp256k1_recover` | `**/*.rs` |
| `CpiContext::new(\s*ctx\.accounts\.system_program\.to_account_info\(\),` | `CpiContext::new(System::id(),` | `programs/**/*.rs` |
| `CpiContext::new(\s*ctx\.accounts\.token_program\.to_account_info\(\),` | `CpiContext::new(Token::id(),` | `programs/**/*.rs` |
| `CpiContext::new_with_signer(\s*ctx\.accounts\.system_program\.to_account_info\(\),` | `CpiContext::new_with_signer(System::id(),` | `programs/**/*.rs` |
| `invoke\(&ix, &account_infos\)` | `invoke(&ix, &account_infos[..])` | `programs/**/*.rs` |
| `invoke\(&([a-zA-Z_]+), &([a-zA-Z_]+)\)` (where `$2` is `Vec<AccountInfo>`) | `invoke(&$1, $2.as_slice())` | `programs/**/*.rs` |
| `pub [a-z_]+: AccountInfo<'info>,` (in `#[derive(Accounts)]` structs) | `/// CHECK: <reason>\n    pub <name>: UncheckedAccount<'info>,` | `programs/**/*.rs` |

Search-only (no auto-replace; review manually):

```bash
grep -rn "AccountInfo<'info>" programs/ | grep -v UncheckedAccount
grep -rn "CpiContext::new" programs/
grep -rn "invoke(" programs/ coordinator/
grep -rn "use solana_program::" programs/
grep -rn "use anchor_lang::solana_program::" programs/
```

### Step 5 — TypeScript package rename (only if you have a TS sdk/tests)

```bash
# in package.json
sed -i 's|@coral-xyz/anchor|@anchor-lang/core|g' \
    sdk/dice-vrf-ts/package.json tests/**/package.json app/package.json
# in source
grep -rln "@coral-xyz/anchor" sdk/ tests/ app/ | \
    xargs sed -i 's|@coral-xyz/anchor|@anchor-lang/core|g'
# delete sub-path imports
grep -rln "@anchor-lang/core/dist/cjs/idl" sdk/ tests/ app/ | \
    xargs sed -i 's|@anchor-lang/core/dist/cjs/idl|@anchor-lang/core|g'
```

### Step 6 — Build

```bash
cargo clean
cargo update -p anchor-lang -p anchor-spl -p anchor-client -p solana-program -p borsh
anchor build
```

Fix the `expected __Pubkey, found Pubkey` cluster first — those are §2 issues, almost always one stray `use solana_program::pubkey::Pubkey;` you missed.
Then fix the `CpiContext::new` errors — §3.
Then fix the `invoke(&ix, &Vec<...>)` slice errors — §4.
Then fix any duplicate-mut-account validation failures at runtime (`anchor test`) — §6.1.

### Step 7 — Run the full test matrix

```bash
anchor test --skip-build           # uses surfpool by default in 1.0
cargo test -p coordinator
cd sdk/dice-vrf-ts && bun test     # bun is now first-class in anchor-spl 1.0
```

### Step 8 — Re-verify dep graph

```bash
cargo tree -i solana-pubkey        # should show ONE version
cargo tree -i solana-program       # should show 3.x only
cargo tree -i borsh                # should show 1.5.7
```

If `cargo tree` shows two versions of `solana-pubkey` or `solana-program`, hunt the offender with:

```bash
cargo tree -i solana-pubkey:2.0.0   # whichever stray version appears
```

Likely culprits in the DICE workspace: an old `mpl-token-metadata` or a pinned-by-rev fork. Bump to a newer release that targets Solana 3.x, or set a `[patch.crates-io]` override.

### Step 9 — IDL re-publish

The legacy `anchor idl init` flow is gone. Re-publish the IDL via Program Metadata Program (PMP):

```bash
anchor idl upload <program-id> --provider.cluster devnet
```

(`anchor deploy` does this by default in 1.0 unless `--no-idl` is passed.)

### Step 10 — CI

- Update `Dockerfile` / GitHub Actions to use Solana CLI 3.1.10 and AVM 1.0.0.
- The verifiable build image is now `solanafoundation/anchor` (PR #3619, since 0.31.1).

---

## Appendix A — Quick reference: 0.31 path → 1.0 path

| Symbol | 0.31 path | 1.0 path |
|---|---|---|
| `Pubkey` | `anchor_lang::prelude::Pubkey` (= `solana_program::pubkey::Pubkey`) | `anchor_lang::prelude::Pubkey` (= `solana_address::Pubkey`) |
| `AccountMeta` | `anchor_lang::prelude::AccountMeta` (= `solana_program::instruction::AccountMeta`) | `anchor_lang::prelude::AccountMeta` (= `solana_instruction::AccountMeta`) |
| `AccountInfo` | `anchor_lang::prelude::AccountInfo` | same path; underlying = `solana_account_info::AccountInfo` |
| `Instruction` | `anchor_lang::solana_program::instruction::Instruction` | same path; = `solana_instruction::Instruction` |
| `invoke` | `anchor_lang::solana_program::program::invoke` | same path; = `solana_invoke::invoke` |
| `invoke_signed` | `anchor_lang::solana_program::program::invoke_signed` | same path; = `solana_invoke::invoke_signed` |
| `system_program::Transfer` | `anchor_lang::system_program::Transfer` | unchanged |
| `system_program::transfer` | `anchor_lang::system_program::transfer` | unchanged |
| `System::id()` | `anchor_lang::system_program::System::id()` | unchanged; **now the canonical CpiContext arg** |
| `Token::id()` | `anchor_spl::token::Token::id()` | unchanged; **now the canonical CpiContext arg** |
| `Clock`, `Rent`, `EpochSchedule`, `SlotHashes`, `SlotHistory`, `StakeHistory`, `Rewards`, `Instructions` | `anchor_lang::prelude::*` | same — re-exported from `solana_sysvar` and `solana_clock` |
| `hash::hashv` | `anchor_lang::solana_program::hash::hashv` | **direct dep** `solana_program::hash::hashv` |
| `keccak::hashv` | `anchor_lang::solana_program::keccak::hashv` | **direct dep** `solana_program::keccak::hashv` |
| `secp256k1_recover` | `anchor_lang::solana_program::secp256k1_recover::secp256k1_recover` | **direct dep** `solana_program::secp256k1_recover::secp256k1_recover` |

---

## Appendix B — One-shot grep patterns

```bash
# stray solana-program imports inside on-chain programs
grep -rn "^use solana_program::" programs/

# CPI sites that still pass an AccountInfo as program
grep -rn "CpiContext::new" programs/ | grep -v "::id()"

# invoke() call sites
grep -rn "invoke(" programs/ coordinator/ sdk/

# AccountInfo<'info> fields in #[derive(Accounts)] (deprecation candidates)
grep -rn "pub [a-zA-Z_]*: AccountInfo<'info>" programs/

# anchor-lang re-exports of dropped modules
grep -rn "anchor_lang::solana_program::\(hash\|keccak\|secp256k1_recover\)" .

# Anchor.toml registry section
grep -n "\[registry\]" Anchor.toml

# old TS package
grep -rn "@coral-xyz/anchor" sdk/ tests/ app/
```

---

## Appendix C — Sources

Primary references used to compile this document. All fetched between 2026-04-02 (1.0 release) and the date of writing.

- Anchor 1.0 release notes: https://www.anchor-lang.com/docs/updates/release-notes/1-0-0
- Anchor `CHANGELOG.md` at v1.0.0: https://github.com/solana-foundation/anchor/blob/master/CHANGELOG.md
- Anchor 0.31.0 release notes: https://www.anchor-lang.com/docs/updates/release-notes/0-31-0
- Anchor 0.32.0 release notes: https://www.anchor-lang.com/docs/updates/release-notes/0-32-0
- Anchor 1.0.0 workspace `Cargo.toml`: https://raw.githubusercontent.com/solana-foundation/anchor/v1.0.0/Cargo.toml
- Anchor 1.0.0 `lang/Cargo.toml`: https://raw.githubusercontent.com/solana-foundation/anchor/v1.0.0/lang/Cargo.toml
- Anchor 1.0.0 `lang/src/lib.rs` (re-exports): https://raw.githubusercontent.com/solana-foundation/anchor/v1.0.0/lang/src/lib.rs
- Anchor 1.0.0 `lang/src/context.rs` (`CpiContext`): https://raw.githubusercontent.com/solana-foundation/anchor/v1.0.0/lang/src/context.rs
- PR #2762 — Remove program account info from CPI context: https://github.com/solana-foundation/anchor/pull/2762
- PR #4031 — Update to Solana 3.0: https://github.com/solana-foundation/anchor/pull/4031
- PR #3946 — Disallow duplicate mutable accounts: https://github.com/solana-foundation/anchor/pull/3946
- PR #4012 — Borsh upgrade to 1.5.7: https://github.com/solana-foundation/anchor/pull/4012
- PR #3854 — Deprecate `AccountInfo` in `#[derive(Accounts)]`: https://github.com/solana-foundation/anchor/pull/3854
- PR #3837 — Check owner on account reload: https://github.com/solana-foundation/anchor/pull/3837
- PR #3900 — Use `solana-invoke` instead of `solana_cpi::invoke`: https://github.com/solana-foundation/anchor/pull/3900
- PR #3877 — Remove `solana-client` from `anchor-client`: https://github.com/solana-foundation/anchor/pull/3877
- PR #4373 — Remove `solana-account-decoder` re-export: https://github.com/solana-foundation/anchor/pull/4373
- PR #4141 — Rename TypeScript packages: https://github.com/solana-foundation/anchor/pull/4141
- PR #3798 — Replace legacy IDL with PMP: https://github.com/solana-foundation/anchor/pull/3798
- PR #4060 — Add `Migration` account type: https://github.com/solana-foundation/anchor/pull/4060
- PR #3878 — Generic `Program<'info>`: https://github.com/solana-foundation/anchor/pull/3878
- PR #3863 — Upload IDL on deploy by default: https://github.com/solana-foundation/anchor/pull/3863
- PR #4106 — Surfpool default for tests/localnet: https://github.com/solana-foundation/anchor/pull/4106
- PR #3768 — `anchor verify` uses `solana-verify`: https://github.com/solana-foundation/anchor/pull/3768
- `solana-invoke::invoke` signature: https://docs.rs/solana-invoke/latest/solana_invoke/fn.invoke.html
- `solana-program 3.x` module index: https://docs.rs/solana-program/latest/solana_program/index.html
- `solana-pubkey` / `solana-address` rename discussion: https://github.com/anza-xyz/solana-sdk/issues/204
- `anchor-lang::prelude::Pubkey` (1.0.0) on docs.rs: https://docs.rs/anchor-lang/1.0.0/anchor_lang/prelude/struct.Pubkey.html
- `anchor-spl` 1.0.0 on docs.rs: https://docs.rs/anchor-spl/1.0.0/anchor_spl/index.html
- `anchor-lang::system_program::Transfer` (1.0.0): https://docs.rs/anchor-lang/1.0.0/anchor_lang/system_program/struct.Transfer.html
- `anchor-lang::system_program::transfer` (1.0.0): https://docs.rs/anchor-lang/1.0.0/anchor_lang/system_program/fn.transfer.html

---

## Appendix D — Items returned UNKNOWN (no public source could fully confirm)

The following were partially verified via type inference and indirect citation, but a direct authoritative source was not located. Mark these as needing experimental verification on the first build:

1. **Whether `Pubkey` is *literally* a type alias `type Pubkey = Address;` versus a re-export `pub use Address as Pubkey;`** — both produce the same nominal-equivalence behavior, but if the alias form is used you get implicit `Address` ↔ `Pubkey` interchange, while a re-export keeps them as one symbol. The `solana-address` crate page does not show the definition. **UNKNOWN — needs experimental verification** by reading `solana-address/src/lib.rs` once the crate is in your `Cargo.lock`.
2. **Whether the slice-coercion regression in `invoke()` is intentional or a quirk of `solana-invoke 3.x`'s generic bounds.** PR #3900 only describes the swap from `solana_cpi::invoke` to `solana-invoke`; it doesn't justify the tightening. The fix (`.as_slice()` / `&v[..]`) is verified to compile, but the *root cause* is **UNKNOWN — needs experimental verification** if you want to upstream a fix.
3. **Whether `solana-program = "3.0"` and `anchor-lang = "1.0.0"`'s internal `solana-pubkey = "3.0.0"` will resolve to the same `solana-pubkey` version automatically**, or whether you must add an explicit `[patch.crates-io]` for some transitive dep. **UNKNOWN — needs experimental verification** via `cargo tree -i solana-pubkey` after step 6.
4. **MSRV bump.** The 1.0 changelog does not explicitly state a new MSRV. CI uses Rust 1.79+ for older Anchor versions; 1.0 likely needs ≥ 1.79 or ≥ 1.80, but no exact pin is documented. **UNKNOWN — needs experimental verification.**
5. **Exact behavior of `Migration<'info, From, To>` when `From` and `To` have different sizes** — the PR description (#4060) is brief; reallocate-and-rent-top-up semantics are inferred, not documented. **UNKNOWN — needs experimental verification** before relying on it for the DICE node-account migration.

---

*End of document.*
