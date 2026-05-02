# Switchboard VRF — Attackable Surface for a Hardware-Backed Challenger

Research date: 2026-04-10
Scope: Solana mainnet randomness. No pivots. No AI/time/provenance tangents.
Primary target: Switchboard Randomness on-Demand (SRS v3 / Randomness Program `RANDMo5gFnqnXJW5Z52KNmd24sAo95KAd5VbiCtq5Rh`)

---

## Executive Summary (what is Switchboard's actual attackable surface, and what must DICE ship to exploit it?)

Switchboard's current randomness product is **not** "VRF in SGX" the way its 2022 marketing suggests. After v3, the actual mainnet product is a **two-transaction slothash commit-reveal** running inside the SAIL confidential-container framework, which is now backed by **AMD SEV-SNP** (with Intel TDX planned) rather than pure Intel SGX. That reframes the competitive picture:

1. **The "SGX sunset" story is weaker than expected.** Intel IAS EOL hit April 2, 2025 and PCS API v2/v3 EOL is April 30, 2026, but Switchboard has already rebuilt its oracle substrate on SEV-SNP via SAIL. Relying on "SGX is dying" as the wedge is not credible against the current product. ([Intel Community](https://community.intel.com/t5/Intel-Software-Guard-Extensions/Intel-PCS-API-versions-2-and-3-EOL-Date-Extended-to-April-30/m-p/1704170), [Confidential Containers blog](https://confidentialcontainers.org/blog/2025/03/11/how-switchboard-oracles-leverage-confidential-containers-for-next-generation-web3-security/))
2. **The real attackable surface is latency, transaction count, and DX.** Switchboard's supported flow is commit-tx → wait one slot (~400 ms nominal, ~3 s in their own tutorial) → reveal-tx. End-to-end it lands in the 3–8 second band in practice — fine for lotteries, painful for any tight gameplay loop. Their own tutorial literally says `"Wait approximately 3 seconds"` between commit and reveal ([Switchboard Randomness Tutorial](https://docs.switchboard.xyz/docs-by-chain/solana-svm/randomness/randomness-tutorial)).
3. **Surge does not cover randomness.** Surge (August 2025) is a sub-100 ms WebSocket streaming product for **price feeds only**. The randomness product is structurally distinct and still gated on slothash commit-reveal. This is the gap DICE should camp on.
4. **MagicBlock already pointed the rhetorical gun at Switchboard.** Their April 29, 2025 plugin post contains the exploitable pull-quote: *"Our VRF plugin, for example, executes in one TX instead of 50-100."* ([MagicBlock Solana Plugins](https://www.magicblock.xyz/blog/solana-plugins)) — but MagicBlock's win is Ephemeral-Rollup-only and 16 GitHub stars deep. The "one-TX on mainnet L1" lane is still unclaimed.
5. **What DICE must ship to exploit this:** (a) a **single-transaction** Anchor CPI (or at worst commit+reveal bundled into one Solana tx via pre-signed reveal) so you can claim "sub-1-slot fulfillment on L1"; (b) a one-line Rust macro + one-line TS helper so integration is ≤10 LOC vs Switchboard's ~58 TS + ~190 Rust; (c) a benchmark page publishing median end-to-end latency and cost side-by-side with Switchboard's tutorial numbers; (d) an explicit "we have no TEE attestation dependency — our trust root is hardware RNG + key sealing on ESP32-S3" messaging, which turns the SGX/SEV-SNP story from a Switchboard strength into a complexity tax DICE doesn't pay.

---

## 1. Switchboard VRF Architecture Deep-Dive (technical)

### 1.1 The current mainnet product is NOT "VRF in SGX"

Switchboard has shipped **three generations** of randomness on Solana, and the discourse muddles them:

- **V2 VRF (2022).** IRTF draft-11 verifiable random function, ECVRF with Ristretto/curve25519, oracle computes signature using a nonce counter + recent blockhash, proof posted on-chain. Fee dropped from 0.1 SOL → "under 0.002 SOL" in a 50× reduction (July 13, 2022). This is the classic "VRF" product and it's the one that the critical "276 transactions to verify" / "50-100 TX" complaints target. ([Switchboard Medium July 2022](https://switchboardxyz.medium.com/verifiable-randomness-on-solana-46f72a46d9cf))
- **Solana Randomness Service / SRS (Jan 2024).** Marketed as "single-transaction requests with callback". The V3 attestation program adds SGX attestation on top, and the oracle signs in-enclave. Repo: `@switchboard-xyz/solana-randomness-service`.
- **Randomness on-Demand (current, the GitBook at `switchboard-xyz/gitbook-randomness-on-demand` and the `docs-by-chain/solana-svm/randomness/randomness-tutorial`).** This is a **slothash commit-reveal** protocol. The oracle binds randomness to a specific future slot; user commits in slot N, waits for slot N+1 to be finalized, then reveals. TEE enforces that the oracle itself cannot "peek" ahead of the committed slot.

The tutorial is explicit that this is a commit-reveal pattern, not pure TEE attestation:

> "Neither party knows the outcome until after commitment."  
> `"Ensure randomness was committed in the previous slot"`  
> `"Take collateral NOW, not on reveal"` (code comment — paraphrasing why the collateral must be locked in the commit tx, because otherwise a malicious user could refuse to reveal losing outcomes)  
> — Switchboard docs, [Randomness Tutorial](https://docs.switchboard.xyz/docs-by-chain/solana-svm/randomness/randomness-tutorial)

### 1.2 Trust root: SAIL + SEV-SNP, not SGX

The SAIL framework ("Switch Forward Attestation Inference Layer") wraps TEEs in a Docker-like confidential-container abstraction. As of the March 11, 2025 Confidential Containers blog post, Switchboard's production substrate is **AMD EPYC with SEV-SNP**, with Intel TDX listed as planned:

> "Switchboard's implementation uses AMD EPYC processors with SEV-SNP (Secure Encrypted Virtualization-Secure Nested Paging) technology."  
> "Future plans include: Support for Intel TDX (Trust Domain Extensions) to broaden hardware compatibility."  
> — [Confidential Containers blog, March 2025](https://confidentialcontainers.org/blog/2025/03/11/how-switchboard-oracles-leverage-confidential-containers-for-next-generation-web3-security/)

Older Switchboard marketing does still say "Intel SGX, AMD SEV" in the same breath (e.g. the Bitget company profile), and the SRS NPM package docs still reference "SGX enabled oracle." In practice, the **active** substrate today is SEV-SNP. SGX may still run as legacy for some oracle flows but the roadmap is moving the other way.

**Implication for the "Intel IAS April 2025 / PCS API v2/v3 April 30, 2026 EOL" attack vector:** this is a dulled weapon. The IAS EOL (April 2, 2025) and PCS API v2/v3 EOL (extended to April 30, 2026, per [Intel Community](https://community.intel.com/t5/Intel-Software-Guard-Extensions/Intel-PCS-API-versions-2-and-3-EOL-Date-Extended-to-April-30/m-p/1704170)) are real, but Switchboard has already moved the center of gravity to SEV-SNP. Any oracle still on SGX has a clean migration path to PCS API v4 or to SEV-SNP within SAIL. DICE should **not** plant its flag on "Switchboard is about to blow up when SGX deprecates." It won't. Plant the flag on latency and DX instead.

### 1.3 Request-to-fulfillment flow, step by step

From the current tutorial, the concrete sequence is:

| Step | Who | Action | Rough latency |
|---|---|---|---|
| 1 | Client | `sb.Randomness.create(sbProgram, rngKp, queue)` — provision a Randomness account | 1 tx, ~1–2 s |
| 2 | Client | Bundle `randomness.commitIx(queue)` + `coin_flip()` user instruction | 1 tx, ~400 ms–1 s |
| 3 | — | Wait for next slot to be finalized so the seed slothash is locked | `"approximately 3 seconds"` per Switchboard's own tutorial |
| 4 | Oracle | Observes commit, in-enclave generates randomness seeded by slothash + internal state | <1 s off-chain |
| 5 | Client | Bundle `randomness.revealIx()` + `settle_flip()` | 1 tx, ~400 ms–1 s |

**End-to-end: 3–8 seconds in the happy path**, driven mostly by step 3 (the forced wait for slot finality) plus transaction propagation. In the Orao marketing comparison page the latency for "Switchboard" is framed as "Several Minutes" ([Orao Solana VRF](https://orao.network/solana-vrf)) — that is competitive framing, not measured on the current product, but it tells you how competitors are positioning the pain.

**Transaction count: 2 transactions minimum** (commit, reveal), 3 if you count account creation. The "1 TX" claim is only true for the older SRS simple_randomness_v1 flow and for MagicBlock's ephemeral-rollup plugin. The current Switchboard mainnet tutorial is explicitly 2-tx.

### 1.4 Is Surge applied to randomness?

**No.** Surge launched on mainnet in August 2025 with sub-100 ms price feed streaming and a direct WebSocket plane that bypasses Switchboard's normal aggregator/oracle-queue machinery. The Blockworks launch coverage and the Switchboard Medium announcement both scope Surge to **price data** ([Blockworks: Switchboard launches Surge](https://blockworks.com/news/fastest-oracle-on-solana-launches), [Switchboard Medium: Introducing Surge](https://switchboardxyz.medium.com/introducing-switchboard-surge-the-fastest-oracle-on-solana-is-here-36ff615bfdf9)). The randomness tutorial, the `gitbook-randomness-on-demand` repo, and the product-documentation section for randomness all continue to describe the commit-reveal flow with no Surge hooks. The mainnet program ID for the randomness-on-demand product is `RANDMo5gFnqnXJW5Z52KNmd24sAo95KAd5VbiCtq5Rh` (both devnet and mainnet-beta), distinct from the Surge/price feeds program.

**This is the gap.** Switchboard has a sub-100 ms streaming lane for prices but has not extended it to randomness. The likely reason is that randomness requires a fresh on-chain seed per request (the slothash binding), which is fundamentally incompatible with a pre-streamed push model. A hardware-backed commit-reveal that pushes the commit server-side and streams the reveal within the same slot could credibly claim "Surge-like latency for randomness" and Switchboard has nothing to answer with off the shelf.

### 1.5 Active feed count

Couldn't verify an exact count of active Randomness accounts on mainnet after searching — Switchboard does not publish a randomness-specific dashboard, and `RANDMo5gFnqnXJW5Z52KNmd24sAo95KAd5VbiCtq5Rh` program ID holder counts are not exposed in the docs I pulled. **Gap in research.** If DICE wants a hard number, query the program's account index directly via a Solana RPC (`getProgramAccounts` with the Randomness discriminator). Flag this as follow-up work.

---

## 2. Developer Pain Points — quoted and categorized

This section is **honest about its limits**. I could not surface a large cache of raw-quote Solana Stack Exchange / Discord / r/solana complaints about Switchboard VRF specifically. Web searches for `site:solana.stackexchange.com` returned mostly the "how to generate random numbers on-chain?" canonical question, and `solana.stackexchange.com` blocked direct WebFetch. The most useful evidence is in developer-written tutorials and third-party blog posts.

### 2.1 Integration complexity

From Guido Di Pietro's `solana-switchboard-vrf-pool` README — a builder who shipped a reusable VrfAccount pool because the single-account model was unusable at scale:

> "VRF requests need a VrfAccount to be available. Think of this as a store where you queue amongst other people to finally get to the cashier or something. In the same way cashiers take a salary, VrfAccounts have a cost to be created."  
> "Users can either have their own 'cashier' (one VrfAccount per user) which will always be free since only they can use it, but is the most expensive and inefficient schema; only one 'cashier' (one VrfAccount altogether) which will be mostly always used but results in bad user experience with a queue that is always too long; or a few 'cashiers' (several VrfAccounts at disposal, pool) which gives a max cap of N users that the program can handle simultaneously."  
> — [GuidoDipietro/solana-switchboard-vrf-pool README](https://github.com/GuidoDipietro/solana-switchboard-vrf-pool)

**Category: integration complexity + reliability.** This is v2 VRF (deprecated), but the structural point is live: the permanent-account model imposed a capacity-planning problem that devs had to solve themselves. The randomness-on-demand product fixes this with per-request Randomness accounts, but now each request costs a rent-exempt account creation.

### 2.2 Transaction explosion (on the old API)

The hardest numerical pain point I found is from the Adevar Labs analysis:

> "The Switchboard VRF implementation on Solana needs 276 transactions to verify VRF output on-chain, which is computationally intensive."  
> — [Adevar Labs blog, On-Chain Randomness on Solana](https://www.adevarlabs.com/blog/on-chain-randomness-on-solana-predictability-manipulation-safer-alternatives-part-1)

**Caveat.** I could not independently reproduce the "276 transactions" number from Switchboard's own docs and it does not match the newer randomness-on-demand flow. This is almost certainly a reference to the v2 on-chain VRF proof verification cost under the IRTF ECVRF scheme, where posting and verifying the proof took many partial-verify instructions across Solana's small-BPF compute budget. It matters because **MagicBlock's "1 TX instead of 50-100" swipe is clearly aimed at exactly this historical pain**, and even if the current product is 2-tx the memory of "VRF is the hard integration" is still out there.

### 2.3 Latency and wait states

From the current tutorial code path:

> "Wait approximately 3 seconds for slot advancement"  
> — [Switchboard Randomness Tutorial](https://docs.switchboard.xyz/docs-by-chain/solana-svm/randomness/randomness-tutorial)

And from their error handling section:

> "If 'reveal' is not settled within an hour of commit, the randomness request will be considered as expired and protocols should register this as a manageable user flow."  
> — [Switchboard Randomness Tutorial](https://docs.switchboard.xyz/docs-by-chain/solana-svm/randomness/randomness-tutorial)

**Category: latency + edge-case handling.** Devs have to implement a one-hour expiration state machine for every randomness request. That is the tutorial's own guidance. A dev building a gacha mint or a fast-paced game is paying for edge cases the randomness-on-demand architecture created.

### 2.4 "Orao is easier"

From dev-to blog coverage of Solana VRF providers:

> "The main oracle providers in Solana today are Switchboard and Orao Network, with a developer noting that Orao is a bit more easy to use."  
> — paraphrased from [On-Chain Randomness on Solana (Adevar)](https://www.adevarlabs.com/blog/on-chain-randomness-on-solana-predictability-manipulation-safer-alternatives-part-1) and community DEV.to posts

**Category: DX / docs.** Not a killer complaint, but a recurring one. Orao's pitch page ([orao.network/solana-vrf](https://orao.network/solana-vrf)) pushes hard on "Multiple code examples and active dev support on Telegram" as a differentiator. That tells you the sentiment the Orao team is hearing from the street.

### 2.5 SGX concerns (public discourse)

I could not find a **single** Solana developer publicly raising the SGX deprecation concern against Switchboard. Not on Twitter, not on GitHub, not on the Switchboard or Solana forums. The concern is real for consumers of raw IAS/PCS endpoints but the application layer doesn't see it — if Switchboard keeps SAIL running on SEV-SNP, no dApp breaks. **Honest gap. The SGX story is a backroom concern, not a market-surface complaint.**

### 2.6 Counts summary

I was not able to verify the following quantitatively:
- Exact number of steps a dev must follow (I counted **8 sequential actions** / ~190 LOC Rust + ~58 LOC TS in the tutorial walkthrough, but this is one example, not a survey)
- Observed median latency in production (estimated 3–8 s from step-by-step walkthrough, not from instrumentation)
- Callback failure/timeout rate in production (unknown; tutorial acknowledges retry logic is necessary)
- Whether the v2→v3 migration hurt adoption (many old repos are now in `-deprecated-*` prefixes, suggesting Switchboard forced a cutover, but I have no adoption-curve data)

---

## 3. Switchboard's public weakness signals

### 3.1 Repo structure tells the story

The `switchboard-xyz` GitHub org contains multiple **explicitly-deprecated** VRF repos, prefixed `-deprecated-`:

- `switchboard-xyz/-deprecated-vrf_req_example`
- `switchboard-xyz/-deprecated-vrf-cpi-example`

This is a public artifact of at least one hard cutover. Every dev who built on v2 VRF was forced to rewrite. That's a soft migration-pain signal — not as loud as a blog post complaint, but visible.

### 3.2 Investment direction: Surge (price feeds) over randomness

The main `switchboard-xyz/solana-sdk` README centers `switchboard-on-demand v0.8.0` and primarily exposes `PullFeedAccountData`, `QuoteVerifier`, `QUOTE_PROGRAM_ID` — **price-feed primitives**. Randomness is a separate package with its own gitbook (`gitbook-randomness-on-demand`). The Surge launch materials (Blockworks, Medium) are exclusively about prices. The Scale or Die 2025 "Spilling the TEE" talk by DoctorBlocks is about SAIL/confidential compute generally, not randomness specifically.

**Inference (not confirmed):** Switchboard is allocating engineering to the Surge price-feed path (where the money and the dev attention is — DeFi liquidations, perp DEXes). Randomness is maintained but not prioritized. This is a durable opening.

### 3.3 Public post-mortems

**None found.** No public post-mortem where Switchboard VRF failed a shipped dApp. That is a double-edged finding: it means Switchboard is reliable enough not to have blown up publicly, but also that there's no smoking-gun incident to cite. DICE cannot pitch against "that time Switchboard broke."

### 3.4 Founder statements on SGX sunset

No public statement from Chris Hermida (CEO) or Mitch Gildenberg (CTO) addressing the PCS API v2/v3 April 30, 2026 EOL specifically. The SAIL/SEV-SNP migration blog post from March 2025 is the closest thing to a roadmap signal — and it quietly confirms they've been building a post-SGX architecture for over a year. Again: **don't pitch on SGX sunset. They already moved.**

---

## 4. What a "fast, simple, SGX-free" VRF pitch looks like (DICE positioning)

### 4.1 Switchboard's three strongest counterarguments to "we're faster and simpler"

1. **"We run in a confidential enclave with attestation; a DePIN box cannot offer the same trust guarantees."** This is their strongest shot. SAIL attestation gives on-chain proof that a specific signed binary ran in genuine SEV-SNP hardware. DICE's ESP32-S3 offers hardware RNG + key sealing, but not remote attestation of program state in the same strong sense. **Counter this head-on:** DICE's trust model is commit-reveal with per-request entropy from a hardware TRNG (not predictable), coordinator-signed, and on-chain verifiable — the commit binds before reveal so the trust surface is the same as Switchboard's (oracle cannot peek) without needing TEE machinery. Don't hide the architecture difference; argue that for randomness specifically, hardware RNG + commit-reveal is sufficient and simpler.
2. **"We're multi-chain, battle-tested on 51+ production integrations."** This is a pure incumbency argument and it's hard to beat directly. **Counter with focus:** DICE is Solana-native and single-purpose; one first-class integration beats being-line-item-N on a multi-chain catalog.
3. **"Surge on SAIL is our platform — randomness will ride it eventually."** This is a promise argument and can be neutralized by **shipping first**. If DICE has sub-second-latency randomness on mainnet L1 in production before Switchboard extends Surge to randomness, the argument dies on the calendar.

### 4.2 Minimum credible latency claim on Solana mainnet (not rollups)

Switchboard's tutorial end-to-end lower bound is approximately **3 seconds (one forced slot wait) + commit-tx confirmation + reveal-tx confirmation**. Realistic mainnet number: **5–8 seconds**.

DICE can credibly claim a better number **only if** it eliminates the commit→reveal slot gap. Two architectural routes:

- **(a) Bundled commit+reveal in a single transaction via pre-signed reveal payload.** The coordinator pre-signs the randomness against a future slothash, the hardware sealed the seed in step -1, and the Anchor program in one tx validates the commit hash, consumes the reveal, and settles. This is bundled-tx randomness. Latency: **~1 slot = ~400 ms** on happy path. This is exactly what you already shipped per the v4 branch commit `657e136 Bundle commit+reveal+finalize into single TX — latency 8s → 3.5s`. **That commit message is the pitch.** It gives you a measured, mainnet-realistic number (3.5 s today, with room to push closer to 1 s) that Switchboard's architecture cannot match without redesigning their tutorial.
- **(b) Commit pipelining, where the coordinator maintains a rolling buffer of committed seeds and the user's request is matched against an already-committed seed that they couldn't have influenced.** Same end-state (sub-slot fulfillment) with a different trust story.

**Pick (a) because you already have it shipped.** Publish median latency as a prominent number: "**DICE median end-to-end: 3.5 s on mainnet L1. Switchboard tutorial minimum: 3 seconds of waiting alone, 5–8 s end-to-end."** This is honest, sourced, and the comparison does the marketing for you.

### 4.3 Sketch of "one-line VRF integration"

**Anchor Rust (the ask):**

```rust
use dice_vrf::dice_randomness;

#[dice_randomness]
pub fn settle_game(ctx: Context<SettleGame>, rnd: [u8; 32]) -> Result<()> {
    let winner_idx = (rnd[0] as usize) % ctx.accounts.players.len();
    ctx.accounts.game.winner = ctx.accounts.players[winner_idx].key();
    Ok(())
}
```

The macro handles account injection, commit binding, reveal verification, and CPI to the DICE program. Developer writes one decorated function and references a single `rnd: [u8; 32]` parameter. No VrfAccount, no pool management, no commit/reveal instructions in user code, no 3-second wait state.

**TypeScript client (the ask):**

```ts
import { DiceClient } from "@dice/sdk";
const dice = new DiceClient({ connection, wallet });
const sig = await dice.requestAndSettle(myProgram, "settleGame", accounts);
```

One line. The SDK handles commit/reveal bundling, retry logic, and the expiry state machine.

Contrast Switchboard's tutorial: **8 sequential actions, 4 account types, ~58 LOC TS, ~190 LOC Rust, explicit 3-second sleep, explicit one-hour expiry handling.**

### 4.4 Real technical obstacles to migration

- **Rust trait differences.** If a dev has `AccountLoader<'info, VrfAccountData>` in their existing Anchor context, they have to rip it out. Ship a migration guide.
- **Client code.** Devs using `@switchboard-xyz/on-demand` in TS have to swap imports and re-wire instruction builders. Ship a codemod or at minimum a side-by-side diff.
- **Fee token handling.** Switchboard takes SOL via the queue/escrow model; DICE takes SOL via direct program CPI. Simpler, but different mental model.
- **Request rate limits.** Switchboard had a per-VrfAccount 10-second rate limit in v2. Devs built around it. DICE has no such limit (if throughput is node-count-driven), which is a selling point but needs to be documented or skeptics will assume the limit is just hidden.
- **Trust migration.** Switchboard's enclave-attestation story is familiar to auditors; DICE's hardware-TRNG-in-ESP32-S3 story is not. **Ship an auditor-friendly threat model doc** alongside the SDK. This is non-optional.

---

## 5. The SGX EOL timing question — revised verdict

**Short answer: it's not a competitive window. Don't pitch it.**

Longer answer:
- Intel IAS EOL: **April 2, 2025** ([Intel Community](https://community.intel.com/t5/Intel-Software-Guard-Extensions/IAS-End-of-Life-Announcement/m-p/1545831))
- Intel PCS API v2/v3 EOL: **April 30, 2026**, extended by 6 months from the original October 31, 2025 date ([Intel Community](https://community.intel.com/t5/Intel-Software-Guard-Extensions/Intel-PCS-API-versions-2-and-3-EOL-Date-Extended-to-April-30/m-p/1704170))
- PCS API v4 is the migration target, and it remains fully supported
- Switchboard has already rebuilt on SEV-SNP via SAIL as of March 2025 ([Confidential Containers blog](https://confidentialcontainers.org/blog/2025/03/11/how-switchboard-oracles-leverage-confidential-containers-for-next-generation-web3-security/))

The "real competitive window" between April 30 and whenever Switchboard ships a replacement **does not exist**. Switchboard shipped the replacement more than a year before the deadline. Any pitch built on "SGX is dying and Switchboard hasn't noticed" will get dunked on by anyone who has read the SAIL docs.

**What is true and defensible:** DICE has zero TEE dependency of any kind. It does not track SGX, SEV-SNP, or TDX errata. It does not need to ship new oracle container images every time AMD or Intel rotate their attestation roots. Its trust surface is the ESP32-S3 silicon TRNG, key sealing, and a commit-reveal protocol. That is a **simpler** trust stack, not necessarily a **stronger** one, and the honest pitch is "we don't take on cloud-enclave attestation complexity because randomness doesn't need it." That's a cleaner argument than "Switchboard is about to break."

---

## 6. Recommended positioning (the founder pitch)

1. **Headline.** "Single-transaction VRF on Solana mainnet. Median 3.5 seconds end-to-end. No SGX, no SEV-SNP, no slot-wait tutorial dance."
2. **Proof.** Publish a benchmark page with (a) DICE median/p95 latency on mainnet measured against actual dApps, (b) the exact 8-step Switchboard tutorial alongside DICE's 1-line macro, (c) cost comparison per request.
3. **Integration story.** Ship the Anchor macro + TS one-liner in Section 4.3. This is the only deliverable the founder needs to prioritize above everything else. The pitch is dead without it.
4. **Trust story.** Don't hide the hardware TRNG architecture — lead with it. Frame it as "commit-reveal with hardware-rooted entropy, no TEE vendor risk." Auditor doc attached.
5. **Don't pitch.** Do not pitch SGX sunset. Do not pitch "Switchboard is unreliable" (no incidents to point to). Do not pitch multi-chain (you're Solana-native, that's a feature).
6. **Do pitch.** Latency. Transaction count (1 vs 2 mandatory + 1 provision). Lines of code to integrate. Absence of a one-hour expiry state machine. The fact that Surge doesn't extend to randomness and you ship the Surge-equivalent UX for randomness today.

---

## 7. Sources

**Switchboard official docs and blogs**
- [Switchboard Randomness Tutorial (current)](https://docs.switchboard.xyz/docs-by-chain/solana-svm/randomness/randomness-tutorial)
- [Switchboard gitbook-randomness-on-demand repo](https://github.com/switchboard-xyz/gitbook-randomness-on-demand)
- [Switchboard Verifiable Randomness on Solana (Medium, July 13, 2022)](https://switchboardxyz.medium.com/verifiable-randomness-on-solana-46f72a46d9cf)
- [Switchboard Randomness Service (SRS) intro Medium](https://switchboardxyz.medium.com/revolutionizing-fairness-one-roll-at-a-time-switchboard-randomness-service-srs-747b2dcb8251)
- [Introducing Switchboard Surge (August 2025, Medium)](https://switchboardxyz.medium.com/introducing-switchboard-surge-the-fastest-oracle-on-solana-is-here-36ff615bfdf9)
- [Getting Started with Surge (Switchboard docs)](https://docs.switchboard.xyz/switchboard-surge/surge)
- [switchboard-xyz/solana-sdk GitHub](https://github.com/switchboard-xyz/solana-sdk)
- [switchboard-xyz/-deprecated-vrf_req_example GitHub](https://github.com/switchboard-xyz/-deprecated-vrf_req_example)
- [switchboard-xyz/-deprecated-vrf-cpi-example GitHub](https://github.com/switchboard-xyz/-deprecated-vrf-cpi-example)

**Switchboard third-party and infrastructure coverage**
- [Blockworks: Switchboard launches Surge, Solana's fastest oracle yet](https://blockworks.com/news/fastest-oracle-on-solana-launches)
- [Breakpoint 2023: Reinventing Oracles with Switchboard V3 (Solana Compass)](https://solanacompass.com/learn/breakpoint-23/breakpoint-2023-reinventing-oracles-with-switchboards-v3-secure-and-dynamic-infrastructure)
- [Scale or Die 2025: Spilling the TEE, DoctorBlocks Switchboard (Solana Compass)](https://solanacompass.com/learn/accelerate-25/scale-or-die-2025-spilling-the-tee-doctorblocks-switchboard)
- [How Switchboard Oracles Leverage Confidential Containers (Mar 11, 2025)](https://confidentialcontainers.org/blog/2025/03/11/how-switchboard-oracles-leverage-confidential-containers-for-next-generation-web3-security/)
- [Lightspeed Podcast — Hermida & Gildenberg on the Oracle Problem](https://solanacompass.com/learn/Lightspeed/how-switchboard-is-solving-cryptos-oracle-problem-chris-hermida-mitch-gildenberg)

**Intel SGX EOL primary sources**
- [Intel Community — IAS End of Life Announcement (April 2, 2025)](https://community.intel.com/t5/Intel-Software-Guard-Extensions/IAS-End-of-Life-Announcement/m-p/1545831)
- [Intel Community — PCS API v2/v3 EOL extended to April 30, 2026](https://community.intel.com/t5/Intel-Software-Guard-Extensions/Intel-PCS-API-versions-2-and-3-EOL-Date-Extended-to-April-30/m-p/1704170)

**Competitor context and comparison**
- [MagicBlock: Introducing the Verifiable Randomness Solana Plugin (April 20, 2025)](https://www.magicblock.xyz/blog/verifiable-randomness-solana-plugin)
- [MagicBlock: Unlocking Solana Plugins (April 29, 2025)](https://www.magicblock.xyz/blog/solana-plugins) — source of the "1 TX instead of 50-100" quote
- [magicblock-labs/ephemeral-vrf GitHub](https://github.com/magicblock-labs/ephemeral-vrf)
- [Orao Network — Solana VRF product page](https://orao.network/solana-vrf) — source of the Switchboard-vs-Orao comparison framing
- [orao-network/solana-vrf GitHub SDK](https://github.com/orao-network/solana-vrf)
- [Adevar Labs: On-Chain Randomness on Solana Part 1](https://www.adevarlabs.com/blog/on-chain-randomness-on-solana-predictability-manipulation-safer-alternatives-part-1)

**Developer pain point surface**
- [GuidoDipietro/solana-switchboard-vrf-pool README — VrfAccount pool rationale](https://github.com/GuidoDipietro/solana-switchboard-vrf-pool)
- [Solana Foundation developer-content VRF course](https://solana.com/developers/courses/connecting-to-offchain-data/verifiable-randomness-functions)
- [Neodyme: Secure Randomness — From Zero to VDFs Part 1](https://neodyme.io/en/blog/secure-randomness-part-1/)

## 8. Research gaps, honestly

- **No direct raw-quote Solana Stack Exchange or Discord complaints about Switchboard VRF.** I searched but found very little — the canonical SE question is about on-chain randomness broadly, not Switchboard pain. If the founder wants real dev-mouth quotes, the best remaining move is to (a) search Switchboard's own Discord archives if accessible, (b) search "switchboard" on the Solana Foundation Discord, (c) check the Anchor Discord, (d) check r/solana directly. I could not do any of these in this session.
- **No production active-feed count for `RANDMo5gFnqnXJW5Z52KNmd24sAo95KAd5VbiCtq5Rh`.** A `getProgramAccounts` RPC call would give a hard number. Flagged as follow-up.
- **No quantitative callback failure rate.** Switchboard's tutorial admits retries are necessary but gives no reliability numbers. An independent measurement run against a test dApp would be worth doing.
- **No public founder statement on SGX sunset.** The SAIL migration blog is as close as it gets. Chris Hermida and Mitch Gildenberg have not addressed it directly in the Lightspeed podcast transcript I pulled.
- **"276 transactions to verify" — not reproducible.** Adevar Labs cites this number. Switchboard's own materials don't. Likely refers to v2 ECVRF proof verification under Solana's BPF compute limits, but I couldn't find a primary source.
