# VRF Novel Delivery Models — Streaming and Multisig
Research date: 2026-04-10
Scope: is "streaming VRF" or "multisig VRF" a real DICE differentiator, or reinventing the wheel?

---

## Executive Summary (answer first)

**Streaming / push VRF — partially novel, genuinely empty lane on Solana, but the use-case demand is thin.** Every major VRF provider (Chainlink, Switchboard, Orao, Pyth Entropy, Supra, MagicBlock, API3 QRNG) ships request-response. The word "subscription" in Chainlink VRF refers to *billing*, not *delivery* — consumers still call `requestRandomWords` and wait for a callback. The only thing that resembles push randomness today is drand's public beacon (a new round every 3s on quicknet / 30s on mainnet), and it is read-pull off-chain, not native on any Solana contract. Pyth Lazer is a push streaming product but for prices only; there is no Pyth Entropy Lazer. So "push VRF on Solana" is literal white space. The catch: the dApps that would actually consume it (tick-based PvP games, autonomous worlds, continuous lotteries) are small, early, and mostly run on ephemeral rollups where MagicBlock already ships free VRF. DICE could ship it in a week as a thin wrapper over its existing commit-reveal and own the category name — but the revenue case depends on a gaming narrative that has not yet materialized on Solana L1.

**Multisig / threshold VRF — not a credible differentiator. It is table stakes the industry does not market.** drand (threshold BLS, 22 orgs), Supra dVRF (threshold BLS, N≥2T+1), Orao v2 (Byzantine quorum EdDSA), and dcipher (threshold BLS) are all explicitly multi-party. Chainlink has been moving VRF to threshold DON. Calling DICE "the multisig VRF for Solana" would invite comparison with drand/Supra/dcipher — all of whom have true threshold cryptography (single aggregated signature from distributed key shares), whereas DICE's 4-7 node commit-reveal with per-node ECDSA is technically *multi-signer*, not *threshold*. The differentiator buried in here is "hardware-bound keys inside attested physical devices," not the "multisig" framing itself.

---

## Section 1 — Streaming / Push VRF

### 1.1 What every major VRF provider actually ships today

| Provider | Chain | Delivery model | Notes |
|---|---|---|---|
| Chainlink VRF v2.5 | EVM + Solana (via CCIP) | Request-response | "Subscription method" is a *billing account*, not a push stream. Consumers call `requestRandomWords`, wait for `fulfillRandomWords` callback. ([docs.chain.link/vrf](https://docs.chain.link/vrf)) |
| Switchboard Randomness | Solana | Commit-reveal request-response | Slothash + TEE. User commits to a slot, reveals next slot. One request = one value. No subscription. ([docs.switchboard.xyz](https://docs.switchboard.xyz/product-documentation/randomness)) |
| Orao VRF v2 | Solana | Request-response | Byzantine quorum EdDSA, sub-second. Per-request. ([orao.network/solana-vrf](https://orao.network/solana-vrf)) |
| Pyth Entropy | 16 EVM chains (not Solana) | "Pull" request-response over HTTP + on-chain reveal | Provider commits hash chain up-front; user "grabs next number". Still per-request from the dApp's point of view — no push. ([docs.pyth.network/entropy](https://docs.pyth.network/entropy)) |
| Supra dVRF | 80+ chains | Request-response | Threshold BLS partial shares, client aggregates. Per-request. ([docs.supra.com/dvrf](https://docs.supra.com/dvrf)) |
| MagicBlock EphemeralVRF | Solana + ER | Request-response | Free on ER, 0.0005 SOL on L1 settlement. ([magicblock.xyz/blog/unlocking-free-vrfs-on-solana](https://www.magicblock.xyz/blog/unlocking-free-vrfs-on-solana)) |
| API3 QRNG | 13 EVM chains | Airnode request-response (AirnodeRrpV0) | Free but still request-response. ([dapi-docs.api3.org](https://dapi-docs.api3.org/guides/qrng/qrng-remix/)) |
| drand / League of Entropy | Off-chain HTTP beacon | **PUSH beacon** | Threshold BLS. `quicknet` emits one signed random value every 3s, `mainnet` every 30s. This is the only true streaming randomness in production. ([drand.love](https://drand.love/), [docs.drand.love/blog/2023/10/16/quicknet-is-live](https://docs.drand.love/blog/2023/10/16/quicknet-is-live/)) |
| dcipher network | Multi-chain | Request-response + Blocklock conditional reveal | "Extends drand", custom committees. Not a streaming product to dApps. ([docs.dcipher.network](https://docs.dcipher.network/)) |

**Bottom line on prior art:** every on-chain consumer today lives in a request-response world. drand is the only push source but it is an off-chain beacon that dApps have to pull themselves, and it has no native Solana integration. I could not find any evidence of a drand-to-Solana bridge project shipping as of April 2026.

### 1.2 What "subscription" means in Chainlink VRF (to avoid confusion)

From the Chainlink docs ([docs.chain.link/vrf/v2/subscription](https://docs.chain.link/vrf/v2/subscription)): a Chainlink VRF "subscription" is a funding account — you top it up with LINK once and link multiple consumer contracts to it, so requests are billed against the pool instead of per-contract. The *delivery model is still request-response*. Every `requestRandomWords` still triggers a separate oracle fulfillment. This is billing sugar, not streaming.

This matters for DICE's framing: if you market "VRF streams," be explicit that you mean delivery model, not billing, or you will be dismissed as reinventing Chainlink subscriptions.

### 1.3 What IS push on Solana today (relevant adjacents, not VRF)

- **Pyth Lazer** (launched 2024, renamed Pyth Pro in Sept 2025): 1ms price updates pushed to subscriber contracts on SVM and EVM. This is the closest architectural precedent — but it is **prices only**. There is no Pyth Entropy Lazer. ([pyth.network/blog/introducing-pyth-lazer](https://www.pyth.network/blog/introducing-pyth-lazer))
- **Switchboard On-Demand push feeds**: same — prices and generic data, not randomness. Surge WebSocket streaming exists for data but the randomness product remains commit-reveal request-response.
- **Clockwork / cron automation**: a dApp could schedule its own VRF requests on a timer, but this is user-side automation wrapping request-response, not a streaming VRF product.

So on Solana specifically, **no one ships push randomness**. That claim appears safe.

### 1.4 Use cases that actually benefit from streaming VRF

Brainstormed from the onchain gaming literature ([IOSG onchain gaming tech stack](https://medium.com/iosg-ventures/the-tech-stack-of-on-chain-gaming-how-the-game-state-is-synced-1eb349ae2101), [Pirate Nation VRF post](https://piratenation.medium.com/blockchain-gaming-optimized-vrf-5b9e67d45daf)):

1. **Tick-based onchain RPGs / autonomous worlds** — loot drops, crit rolls, enemy AI rolls every tick. Currently these games either batch-pull VRF (slow) or use pseudo-random block hashes (manipulable). A streaming feed they subscribe to once = massive DX win.
2. **Continuous lotteries / number broadcasters** — draws every N slots. drand is the analog; no on-chain Solana version.
3. **Live PvP with periodic reveals** — fog-of-war reveals, trading card games drawing a card per turn without waiting 1-2 slots per draw.
4. **Dynamic NFT traits that mutate on a schedule** — evolving assets that need a fresh verifiable random seed every hour/day.
5. **On-chain procedural generation** — biome/map tile RNG as the player explores.

Caveat: most of these use cases are today **tiny revenue streams**. The big Solana gaming narrative runs on MagicBlock Ephemeral Rollups where VRF is free. Pirate Nation built its own game-specific VRF to sidestep latency. Proof of Play publicly discussed moving to a "Notary" signed-client-action system to bypass VRF altogether for PvP. So the pattern in gaming is *custom in-house randomness*, not subscribing to a third-party streaming feed.

### 1.5 Is streaming VRF technically feasible on Solana?

Yes, and it is not exotic. Sketch:
- A single on-chain `RandomnessFeed` PDA seeded by the DICE coordinator.
- Every N slots, the coordinator (or a cranking account using Clockwork/Triggr) writes a new value + commit-reveal proof into the PDA.
- Subscribers read the PDA as a standard account input in their instruction (no extra account allocation, no callback gas).
- Throughput limit is whatever Solana's write contention on a single account allows — each feed handles ~1 write per slot (~400ms), so one feed = one push per slot.

Costs to watch:
- **Rent**: single PDA of ~200 bytes ≈ 0.0016 SOL once. Cheap.
- **CU budget**: a commit-reveal verification on Solana costs ~30-50k CU per value. At 1 feed per slot that is well within block limits.
- **Sybil / hot-account contention**: if many dApps subscribe to the same feed PDA in the same slot, Solana's account locking will serialize them as readers only, which is fine because readers don't block each other.

It is a weekend's work to prototype on top of DICE's existing commit-reveal.

### 1.6 Verdict on streaming VRF

- **Novel on Solana**: yes. No provider ships it as of April 2026.
- **Novel cross-chain**: partially. drand's beacon is the strongest prior art, and anyone technically sophisticated will say "you're building on-chain drand." That is a defensible framing, not a weakness.
- **Worth shipping for DICE**: yes, as a differentiator and marketing piece — provided you expect thin immediate demand and treat it as a narrative wedge for the gaming vertical. The technical cost is low (one PDA, one cranking job, reuses existing commit-reveal), and "VRF streams" is a cleaner story to sell than "cheaper Switchboard VRF."
- **Risk**: Pyth is an obvious candidate to extend Lazer with an Entropy variant. If they do, DICE loses the category name. Ship before someone else does.

---

## Section 2 — Multisig / Threshold VRF

### 2.1 The terminology matters — and "multisig VRF" is not really a thing

From [Panther Protocol's threshold crypto overview](https://blog.pantherprotocol.io/threshold-cryptography-an-overview/), [dcipher.network docs](https://docs.dcipher.network/), and the Wikipedia VRF entry:

- **Multisig VRF** would mean: each signer has its own full keypair, each publishes its own signature, and the consumer checks M-of-N signatures independently. There is no aggregation. You get N proofs.
- **Threshold VRF (TVRF)** means: the network runs a distributed key generation (DKG) to produce *one* public key shared across all nodes as shares of a private key no one holds. Each node computes a partial evaluation, and any T+1 shares can be combined (Lagrange interpolation on BLS) to produce *one* verifiable output against *one* public key. drand, Supra dVRF, and dcipher all do this.

**Nobody in this space markets as "multisig VRF."** The research literature and every production product use "threshold VRF," "distributed VRF," or "Byzantine quorum" (Orao). "Multisig VRF" is a phrase I could not find in any product-page, whitepaper, or research paper. Claiming it would read as either naive or novel-by-accident — and the honest answer is that DICE's actual protocol (4-7 nodes, each with a hardware ECDSA key, commit-reveal with per-node signatures) is closer to "multisig" in the strict cryptographic sense than "threshold." That is technically accurate but commercially weak because it means *more bytes on chain per proof* and *no elegant single-public-key story*.

### 2.2 What the major players market as their multi-party story

| Provider | Term used | Scheme | Aggregation |
|---|---|---|---|
| drand / League of Entropy | "threshold BLS", "distributed randomness beacon" | BLS t-of-n (currently 12 of 22) | Single aggregated BLS signature |
| Supra dVRF | "Distributed VRF (dVRF)" | BLS threshold, N≥2T+1 | Single aggregated output |
| dcipher network | "threshold signing network", "permissionless threshold" | BLS + async DKG | Single aggregated signature |
| Orao VRF v2 | "Byzantine quorum", "multi-party", "multinode" | EdDSA multi-node | N independent signatures posted on-chain (no threshold aggregation) |
| Chainlink VRF | "DON", "threshold signatures" (v2.5+ roadmap) | Moving to threshold | Aggregated |
| Switchboard Randomness | "TEE oracle" (single-oracle model) | Single SGX/TEE operator, slothash commit-reveal | N/A (single signer) |
| MagicBlock EphemeralVRF | "network of oracles" | Not publicly specified as threshold | Appears single or small-N; not marketed as multi-party |
| Pyth Entropy | "two-party commit-reveal" | Two-party (provider + user) | Not threshold |

### 2.3 Key insight on the Solana market specifically

On Solana today:
- Switchboard is effectively single-operator (TEE-based trust).
- MagicBlock is a small oracle set without an advertised threshold scheme.
- Orao is multi-node Byzantine quorum but markets "quorum" not "threshold."
- Pyth Entropy isn't on Solana.

So there is technically an opening for "the first Solana VRF with N-of-M witness security" framing. **But** the moment anyone digs in and notices that drand (off-chain) and Supra dVRF (80+ chains including many SVM) have true threshold BLS with single aggregated signatures, the DICE story gets visibly weaker.

### 2.4 Can DICE credibly pitch as multisig VRF?

Three options, in order of honesty:

1. **"Hardware-attested multi-witness VRF"** — lean on the physical differentiator (ESP32-S3 with attested keys in secure element), which is orthogonal to whether the crypto is threshold or multisig. This is the angle that no one else owns. Drand, Supra, and dcipher all run on cloud VMs. DICE's hardware root of trust is the actual wedge. Frame the multi-node thing as a natural property, not the headline.

2. **"N-of-M commit-reveal"** — accurate, defensible. Avoid the word "multisig" because it carries wallet-world connotations that will confuse buyers.

3. **"Threshold VRF for Solana"** — only if DICE actually ships threshold BLS DKG (it currently does not, per memory of the protocol — per-node ECDSA is not threshold). Claiming this without shipping it risks getting called out by cryptographers who will immediately ask about the DKG.

**Recommendation:** drop "multisig" as a marketing term. The real moat is "hardware-bound keys attesting each signature, physically distributed nodes," not the signature aggregation scheme. If you want a crisp label, try **"attested-hardware VRF"** or **"physically decentralized VRF"** — both are accurate and neither drand, Supra, nor dcipher can honestly claim them.

### 2.5 Verdict on multisig VRF

- **Recognized product category**: no. "Threshold VRF" is. "Multisig VRF" is not used in the literature and would read as imprecise.
- **Table stakes**: effectively yes. Every serious VRF offering except Switchboard and Pyth Entropy is already multi-party.
- **Differentiator for DICE**: weak on its own. Strong when combined with the hardware-attestation angle.
- **Action**: reframe as "hardware-attested distributed VRF" or similar; do not lead with "multisig."

---

## Section 3 — Bonus: Novel Delivery Models Beyond Request-Response and Streaming

Brief scan of recent work:

1. **Timelock / Blocklock (dcipher + drand tlock)** — the randomness is embedded in a future drand round. A user can encrypt data today that can only be decrypted once drand has emitted a specific future beacon round. Not a push or pull to the dApp directly — it is "conditional reveal on block height." Useful for sealed-bid auctions and hidden votes. ([drand tlock](https://github.com/drand/tlock), [dcipher Blocklock](https://docs.dcipher.network/)) This is the most interesting non-RR, non-streaming pattern in production, and DICE could offer it as an add-on since the commit-reveal model naturally supports delayed reveal.

2. **VRaaS (Gorman et al., IACR 2024/957)** — formalizes verifiable randomness as a service in UC model. Their headline result is that *two transactions* (request + fulfill) are **provably necessary** for any service satisfying their security definition. This is an argument *against* single-transaction or pure-push randomness if you want strict unbiasability. Worth noting: a streaming feed with commit-reveal can still satisfy this if each value has its own commit and reveal phase embedded in the feed timeline. ([eprint 2024/957](https://eprint.iacr.org/2024/957))

3. **Notary-style signed client actions (Proof of Play)** — client signs its own action, oracle countersigns a result derived from a seed. Not quite VRF but positioned as a VRF replacement for PvP latency. Trades some trust for speed. ([proofofplay.com/resources/onchain-vrf-optimized-for-gaming](https://proofofplay.com/resources/onchain-vrf-optimized-for-gaming))

4. **Hash-chain pre-commitment (Pyth Entropy)** — provider pre-commits to an entire chain of values via a root hash, and "grabs next" on each request. This amortizes commitment cost and makes each fulfillment one-transaction-ish. Still request-response from the dApp, but interesting as a throughput optimization. ([docs.pyth.network/entropy/protocol-design](https://docs.pyth.network/entropy/protocol-design))

None of these is "streaming" in the Lazer sense. The delivery-model design space is still genuinely small.

---

## Gaps / Things I Could Not Verify

- **Exact status of Switchboard push feeds for randomness**: Switchboard markets "on-demand" and "push" feeds but all surfaced docs tie push to price/data, not randomness. I could not find a page where Switchboard explicitly offers a randomness push feed. High confidence they do not, but I did not exhaust the docs.
- **MagicBlock oracle set size and threshold status**: the EphemeralVRF repo describes "a network of oracles" but does not publicly specify N or whether there is any threshold scheme. I treated it as small-N single-trust.
- **Chainlink VRF DON threshold rollout status on Solana**: Chainlink is on Solana via CCIP but I could not confirm whether native VRF with threshold signatures is live on Solana or only roadmap.
- **Streaming VRF on Aptos/Sui/Sei**: scope was Solana and chain-agnostic VRF providers, so non-EVM non-Solana alt-chains were not deeply searched. There may be a project in that cluster doing push randomness.
- **On-chain drand bridge to Solana**: not found, but absence of search hits is not proof of absence. If one exists it is niche.

---

## Final Answer to Founder

**Ship streaming VRF.** It is the only clean novel category on Solana that does not require re-architecting DICE, and the narrative story ("Pyth Lazer for randomness," "drand on Solana") writes itself. Expect low immediate revenue but high competitive-moat value. Risk: Pyth could extend Lazer to Entropy any quarter.

**Do not ship "multisig VRF" as a headline.** Reframe the multi-node architecture as "hardware-attested distributed VRF" and let the physical security story carry the weight. Nobody else can claim it, and the "multisig" framing invites unfavorable comparison with drand, Supra dVRF, and dcipher, all of whom have proper threshold BLS that DICE does not.
