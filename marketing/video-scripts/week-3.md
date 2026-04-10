# Week 3 — "Use Cases & Social Proof" (Credibility)

Show real applications, real transactions, real developer experience.

---

## DAY 15 — "Every Game Needs a Dice Roll"

**Theme:** Gaming use case. Every on-chain game needs randomness — loot drops, matchmaking, card draws.

**Voiceover Script (18 sec):**
> Loot drops. Card draws. Critical hits. Boss spawns.
> Every game needs a dice roll.
> On-chain games can't use Math-dot-random.
> They need randomness that's verifiable, tamper-proof, and fast.
> DICE delivers it in under two seconds.
> Roll the dice. Trust the result.

**Visual Breakdown:**

| Time | Visual | Text Overlay |
|------|--------|-------------|
| 0-4s | Fast montage: RPG loot chest opening, card flip, dice rolling, sword swing (game footage / stock) | — |
| 4-6s | Code snippet: `Math.random()` with a red X over it (motion) | "Can't use Math.random" |
| 6-9s | DICE node green LED → Solana tx confirmation (composite) | "Verifiable" |
| 9-13s | Game UI receiving a randomness callback, result appearing (mockup) | "Under 2 seconds" |
| 13-17s | Player celebrating a rare loot drop (game footage / stock) | — |
| 17-22s | Dice rolling → lands → DICE logo | "Trust the result" |

**Music:** Gaming soundtrack energy, 8-bit meets modern
**SFX:** Loot chest sound, card flip, dice clatter

---

## DAY 16 — "The Prediction Market"

**Theme:** Show the prediction market example dApp — real code, real transactions on devnet.

**Voiceover Script (17 sec):**
> A prediction market. Fully on-chain.
> Players bet on outcomes. The market resolves with DICE randomness.
> No house edge. No manipulation. Verifiable by anyone.
> This isn't a concept — it's running on Solana Devnet right now.
> Open source. Go build on it.

**Visual Breakdown:**

| Time | Visual | Text Overlay |
|------|--------|-------------|
| 0-3s | Stock trading floor / betting screens, intense (stock) | "Prediction market" |
| 3-6s | Code scrolling: prediction-market smart contract (screen rec) | "Fully on-chain" |
| 6-9s | Solana Explorer showing resolve transaction (screen rec) | "DICE randomness" |
| 9-13s | Terminal output showing VRF round completing (screen rec) | "Running right now" |
| 13-17s | GitHub repo page for DICE (screen rec) | "Open source" |
| 17-22s | DICE logo + "programs/prediction-market" | "Go build on it" |

**Music:** Intense, trading-floor energy
**SFX:** Stock ticker sounds, transaction confirmation ping

---

## DAY 17 — "Lucky Wheel"

**Theme:** The lucky wheel dApp — visual, fun, easy to understand. Show it working.

**Voiceover Script (16 sec):**
> Spin the wheel. Win the prize.
> But who decides where it lands?
> Not a server. Not an admin. Not a seed you can't verify.
> A DICE node. Hardware entropy. On-chain proof.
> The wheel is provably fair. Every. Single. Spin.

**Visual Breakdown:**

| Time | Visual | Text Overlay |
|------|--------|-------------|
| 0-3s | Colorful wheel spinning (game show footage / stock) | — |
| 3-5s | Finger pointing accusingly / suspicious look (stock) | "Who decides?" |
| 5-8s | Server rack with red X → DICE node with green check (motion) | — |
| 8-12s | VRF round executing on terminal (screen rec) | "Hardware entropy" |
| 12-16s | Wheel landing, result highlighted (mockup / animation) | "Provably fair" |
| 16-22s | Wheel morphs into DICE logo | "Every single spin" |

**Music:** Game show theme, playful but techy
**SFX:** Wheel spin/click sound, crowd cheer

---

## DAY 18 — "NFT Mints That Can't Be Sniped"

**Theme:** NFT minting with verifiable randomness — no more insider rarity manipulation.

**Voiceover Script (19 sec):**
> The rarest NFT in the collection — 
> who decided it was yours?
> In most mints, the team knows. The insiders know.
> The metadata is set before you click.
> With DICE — rarity is assigned after mint, by hardware randomness.
> No one knows. No one can. Not even the team.

**Visual Breakdown:**

| Time | Visual | Text Overlay |
|------|--------|-------------|
| 0-3s | Rare NFT reveal moment, shiny/golden (stock / screen rec) | "The rarest one" |
| 3-6s | People whispering / shadowy deal (stock) | "Insiders know" |
| 6-9s | JSON metadata file on screen (screen rec) | "Set before you click" |
| 9-13s | DICE node processing, green LED blink (you film) | "After mint" |
| 13-17s | NFT metadata shuffling randomly, landing on traits (animation/mockup) | "Hardware randomness" |
| 17-22s | DICE logo + "Fair mints. Verified." | "Not even the team" |

**Music:** Mysterious, then resolving to clarity
**SFX:** Shuffle/card dealing sounds

---

## DAY 19 — "545 Rounds. Zero Crashes."

**Theme:** Real test results from the actual hardware. Numbers don't lie.

**Voiceover Script (18 sec):**
> Five hundred and forty-five VRF rounds.
> Real ESP32 hardware. Real Solana transactions.
> Average latency — one-point-seven seconds.
> Crashes? Zero.
> Failures? Zero.
> This isn't a testnet demo. This is production-grade infrastructure.
> Built. Tested. Ready.

**Visual Breakdown:**

| Time | Visual | Text Overlay |
|------|--------|-------------|
| 0-3s | Counter rapidly ticking up to 545 (motion graphics) | "545 rounds" |
| 3-5s | Real ESP32 board powered on, LED active (you film) | "Real hardware" |
| 5-8s | Solana Explorer transactions scrolling (screen rec) | "Real transactions" |
| 8-10s | Speedometer / latency gauge hitting 1.7s (motion) | "1.7s average" |
| 10-13s | Big "ZERO" appearing twice (motion, impactful) | "Crashes: 0 / Failures: 0" |
| 13-17s | Test results terminal output (screen rec from actual test runs) | "Production-grade" |
| 17-22s | DICE logo + "Built. Tested. Ready." | — |

**Music:** Confident, data-driven tech feel
**SFX:** Counter clicking, industrial clunk on "ZERO"

---

## DAY 20 — "The Developer Experience"

**Theme:** Show what it actually looks like to integrate DICE into a Solana smart contract.

**Voiceover Script (18 sec):**
> Step one — add dice-vrf to your Cargo.toml.
> Step two — call request-randomness in your instruction.
> Step three — there is no step three.
> Your contract gets a callback with verified random bytes.
> No dashboard signup. No token purchase. No oracle configuration.
> Just code.

**Visual Breakdown:**

| Time | Visual | Text Overlay |
|------|--------|-------------|
| 0-4s | VS Code: typing `dice-vrf = "0.1"` in Cargo.toml (screen rec) | "Step 1" |
| 4-8s | VS Code: typing the CPI call (screen rec) | "Step 2" |
| 8-10s | Blank/empty screen, pause | "Step 3: ..." (then nothing) |
| 10-14s | Terminal: `anchor build` succeeding, `anchor deploy` (screen rec) | "Just code" |
| 14-18s | Solana Explorer: randomness result account with data (screen rec) | "Verified random bytes" |
| 18-22s | DICE logo + "docs.dice.network" | — |

**Music:** Lo-fi coding music, clean keystrokes
**SFX:** Keyboard typing, build success chime

---

## DAY 21 — "Open Source Everything"

**Theme:** The entire stack is open source — firmware, coordinator, smart contract, SDK. Transparency.

**Voiceover Script (17 sec):**
> Firmware — open source.
> Smart contract — open source.
> Coordinator — open source.
> SDK — open source.
> Every line of code that touches your randomness is auditable.
> We don't ask you to trust us. We ask you to read the code.
> DICE. Verify everything.

**Visual Breakdown:**

| Time | Visual | Text Overlay |
|------|--------|-------------|
| 0-2s | GitHub file tree: firmware/ folder (screen rec) | "Firmware — OS" |
| 2-4s | programs/dice/ folder (screen rec) | "Contract — OS" |
| 4-6s | coordinator/ folder (screen rec) | "Coordinator — OS" |
| 6-8s | sdk/ folder (screen rec) | "SDK — OS" |
| 8-12s | Full repo view scrolling, showing stars/forks (screen rec) | "Every line. Auditable." |
| 12-17s | Code diff view, someone reviewing a PR (screen rec) | "Read the code" |
| 17-22s | DICE logo + GitHub stars counter | "Verify everything" |

**Music:** Clean, open, airy — trust vibes
**SFX:** Git commit sounds, subtle paper turning
