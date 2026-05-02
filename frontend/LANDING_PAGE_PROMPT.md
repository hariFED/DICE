# DICE Landing Page — AI Build Brief

> **How to use this file**: Paste this entire document into a frontend-capable AI (v0.dev, Lovable, Cursor agent mode, Bolt, Claude). It contains everything needed to design and build the DICE landing page from scratch: full product context, services, brand direction, section-by-section spec, required assets, tech stack, and hard constraints. No external research required.

---

## 1. Project Snapshot

**DICE** is a hardware-backed verifiable randomness function (VRF) oracle on Solana, built on a distributed network of ESP32-S3 microcontrollers. Each node has a cryptographic key burned into silicon, generates physical entropy, and participates in an on-chain commit-reveal protocol. The bigger picture: DICE is positioning itself as **the hardware witness layer for Solana** — randomness is the first service; trusted time, AI provenance, and DePIN witness infrastructure come next.

| Field | Value |
|---|---|
| **Product name** | DICE (Distributed Infrastructure for Cryptographic Entropy) |
| **Chain** | Solana (devnet live, mainnet soon) |
| **Program ID (devnet)** | `78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv` |
| **Hardware** | ESP32-S3-N16R8 DevKit (~$8 BOM) |
| **Protocol** | Commit-reveal with ECDSA secp256k1 signatures, SHA-256 entropy combination |
| **Node count per round** | 4–50 (4-of-N Byzantine threshold) |
| **Request fee** | 0.002 SOL |
| **Fee split** | 70% node operators / 20% treasury / 10% reserve |
| **End-to-end latency** | ~3.5s median |
| **Trust model** | One honest node guarantees unpredictable output |

---

## 2. What DICE Is — Full Context

### The Problem

Every randomness oracle on Solana today — Switchboard, MagicBlock, Orao, Chainlink — generates entropy on **cloud servers**. A single company. A single machine. Either software VRF proofs or TEE (Intel SGX / AMD SEV-SNP) attestation. Trust is a vendor relationship.

This is fine for most use cases. It is not fine when a lottery dApp holds real money and one random number decides who walks away with it. It is not fine when the integrity of a multi-million-dollar NFT mint depends on a single cloud API call. It is not fine when builders want hardware-tangible trust roots instead of abstract cryptographic proofs from a vendor dashboard.

### The Solution

DICE is a distributed network of $8 ESP32-S3 microcontrollers. Each device:

- Has a **secp256k1 private key burned into hardware** during provisioning, never exported
- Generates **physical entropy** from a floating ADC pin (thermal/EMI noise) combined with the onboard hardware RNG
- Signs every commit and reveal with hardware-resident keys
- Participates in an **on-chain commit-reveal protocol** with 4–7 peers per round (up to 50)
- Runs **immutable firmware** with eFuses burned post-provisioning (secure boot v2, flash encryption, no OTA)

The protocol works in two phases:

1. **Commit phase** — each node generates 32 bytes of entropy, computes `SHA-256(entropy)`, signs the hash with its hardware key, and posts the commit on-chain.
2. **Reveal phase** — once commit deadline passes, each node reveals the raw entropy plus a signature. On-chain verification confirms `SHA-256(entropy) == commit`.
3. **Finalization** — once at least 4 valid reveals land, the final randomness is `SHA-256(entropy_1 || entropy_2 || ... || entropy_n)`, deterministic and verifiable by anyone.

### Why It Matters

- **No cloud vendor** in the trust path.
- **No TEE lock-in**. No SGX. No SEV-SNP. No vendor attestation service that could be deprecated.
- **Hardware-tangible trust root**. You can hold a DICE node in your hand. You cannot hold a TEE enclave.
- **Byzantine-tolerant**. One honest node out of N guarantees unpredictability.
- **Solana-native from day one**. No multi-chain tax. No bridged compromise.
- **Single-transaction latency**. ~3.5s median vs. 5–8s for traditional two-TX commit-reveal designs.

### The Bigger Story — Platform Narrative

VRF is the first thing DICE ships. It is not the last. A distributed fleet of hardware-attested, cryptographically-signing nodes is useful for far more than random numbers:

- **Trusted time** — signed timestamps from a distributed hardware network
- **AI provenance** — hardware witness signatures on model outputs and training data commitments
- **DePIN witness infrastructure** — the "Intel inside" trust layer that other DePIN networks can plug into
- **Signed attestations** — physical seal-break events, insurance claim witness, supply chain verification, regulated gaming audit trails

**Positioning line to lead with**: _"The hardware witness layer for Solana. Randomness is week one."_

---

## 3. Services DICE Provides

### 3.1 On-Demand VRF (v1)
Single-shot verifiable randomness. Developer's Solana program calls `request_randomness` via CPI. DICE program orchestrates the round, then invokes the developer's `dice_callback` instruction with 32 bytes of randomness. 0.002 SOL per request. Good for NFT mints, lottery draws, raffle outcomes.

### 3.2 Channel VRF (v2)
Reusable **DiceChannel** PDAs for high-throughput applications. A channel is initialized once, funded with SOL, and then every subsequent request reuses the same PDA — eliminating per-request account creation overhead. 18× cost reduction vs v1. Callback delivery is decoupled from finalization (a failed callback does not lose the randomness result). Good for on-chain games with frequent randomness needs.

### 3.3 Streaming VRF Feed (v7)
A **RandomnessFeed** is a persistent PDA that the coordinator populates on a cadence — every N slots, a fresh randomness value from the bound DiceChannel is published to the feed. Subscribers read the feed as a plain input account: no transactions, no callbacks, no per-read cost. Maintains a 16-entry rolling history so late-landing transactions can still read the value that was current at build time. Good for dynamic NFT trait mutation, on-chain game loops, passive randomness consumers.

### 3.4 Hardware Node Operation
Anyone with an ESP32-S3 DevKit can join the network. The **NodeVault** system credits earnings from all services (VRF v1, VRF v2, and future services like trusted time or signed attestations) to a single universal payout wallet per device. Wallet binding is hardware-signed — only the physical device can authorize its payout target. Wallet rotation requires dual signatures (device + current wallet owner) and a 24-hour cooldown. Operators earn 70% of all protocol fees from rounds they participate in.

### 3.5 Developer SDK
A Rust crate (`dice-vrf`) that exposes one-line CPI integration from any Solana program. Instruction builders, PDA derivation, account structs, and shared types. Optionally, an off-chain client module for dApps that prefer to request randomness from a backend rather than CPI.

### 3.6 Network Explorer
A live dashboard showing real-time network state: nodes online, total rounds, success rate, average latency, queue depth, individual round history with verification links. This is a separate sub-application (`/explorer`) from the marketing landing page. **Note**: the explorer is the ONLY place where the green accent color (`#00FF85`) is allowed, reserved for live-status indicators like "Finalized" badges and "Online" dots.

---

## 4. Target Customers

### Primary — Who to speak to in the hero
- Solana gaming dApps (dice, coin flip, card games, PvP, turn-based)
- NFT mint and reveal projects where fairness auditability matters
- On-chain lotteries, raffles, prediction markets
- RWA projects running provably fair drawings
- DeFi protocols needing unbiased tiebreaker randomness

### Aspirational — For the roadmap section
- Regulated gaming operators (jurisdictions requiring certified hardware RNG)
- Insurance claim processors needing independent witness co-signing
- Supply chain / cold-chain logistics (ESP32 in container, signed seal-break events)
- DePIN networks needing a plug-in trust attestation layer
- Any Solana project that wants hardware-tangible proof of something physical happening

---

## 5. Positioning & Brand Voice

### Voice
**Grounded confidence. Professional credibility. Long-term vision. Zero hype.** The founders know what they built and why it matters. They're not here to sell you on another Web3 dream. They're here because they hit a problem in their own lottery project and shipped the infrastructure to solve it.

Think: how Apple talks about the M-series chips. How Linear talks about speed. How Raycast talks about developer ergonomics. How io.net talks about compute. Technical specificity, visual minimalism, no crypto-neon.

### Reference Brands
- Apple product pages (macbook, iphone, vision pro) — scroll-driven 3D, macro photography, confident typography
- Linear — restrained motion, glass surfaces, clean hierarchy
- Raycast — monochromatic with selective color, macOS-native polish
- Vercel — dark mode mastery, confident typography
- io.net — premium Web3 infrastructure reference
- Helium — hardware network narrative done right

### Hero Positioning — Pick One
- _"The hardware witness layer for Solana."_
- _"Hardware-rooted randomness. Built on silicon, not servers."_
- _"Verifiable randomness, born in hardware. Built for Solana."_

### Do Not Say
- "Trustless oracle" — that framing is taken and the market is saturated
- "Solve the oracle problem" — Switchboard already owns that positioning
- "Web3 native" — say Solana, be specific
- "Revolutionary" or "game-changing" — show the hardware instead

---

## 6. Design System — Locked In

These are hard constraints. Do not deviate without checking first.

### Palette
| Token | Hex | Usage |
|---|---|---|
| Background | `#000000` | Pure black, everywhere |
| Surface | `#111111` | Cards, elevated glass panels |
| Border | `rgba(255,255,255,0.08)` | Subtle ghost borders on glass |
| Foreground | `#fafafa` | Primary text |
| Muted | `#a1a1aa` | Secondary text, captions |
| Metallic text | linear-gradient silver → white → grey | Headlines, hero, section titles |
| Accent green | `#00FF85` | **EXPLORER STATUS TAGS ONLY — NEVER on the landing page** |

### Critical Design Constraint
**Do not use green on the landing page.** No green buttons. No green glows. No green accents. No green icons. The green `#00FF85` color is reserved exclusively for the separate `/explorer` sub-application's live-status indicators (finalized badges, online dots). A prior iteration of this landing page used green everywhere and was explicitly rejected as "too generic crypto."

All buttons, glows, accents, and highlights on the landing page must be **metallic silver / chrome / white gradients**. Think polished aluminum, brushed steel, liquid mercury.

### Typography
- Primary font: **Space Grotesk** (already configured)
- Headline treatment: metallic gradient (silver → white → grey), optional subtle text-shadow for depth
- Body: `#fafafa` at 90% opacity
- Mono (for code): JetBrains Mono or IBM Plex Mono

### Surfaces
- **Glassmorphism cards**: `backdrop-blur(20px)`, background `rgba(255,255,255,0.03)`, border `rgba(255,255,255,0.08)`
- **Strong glass** variant: `backdrop-blur(40px)`, background `rgba(255,255,255,0.06)`
- **Noise overlay**: SVG static texture at 3–5% opacity, applied to the root layout for film-grain feel
- **Glow utility**: silver / chrome glow using `box-shadow: 0 0 40px rgba(255,255,255,0.08)` — NOT green

### Motion Principles
- Use **Lenis** for global smooth scroll
- Use **GSAP ScrollTrigger** for pinned scroll-driven sequences (hardware showcase, how-it-works)
- Use **Framer Motion** for enter/exit, hover states, whileInView reveals
- Use **React Three Fiber** for 3D (ESP32 model, globe)
- Every interaction should feel weighted. Nothing instant. Nothing bouncy-goofy. Think dampened springs.

---

## 7. Landing Page Sections

**Build mode**: fresh rebuild from scratch. A previous attempt exists in `frontend/components/` — you may reference it for data contracts (API routes, mock data shapes) but do not feel bound to its structure. Reinvent the component architecture.

### Section 1 — Sticky Header
- Logo (monogram + wordmark, metallic gradient)
- Nav links: Product · Services · Explorer · Docs · GitHub
- Right side: "Launch App" button (chrome gradient, NOT green) + wallet connect placeholder
- Behavior: transparent over the hero, transitions to strong-glass backdrop on scroll

### Section 2 — Hero (Full Viewport)
- Headline (metallic gradient, large): _"The hardware witness layer for Solana."_
- Subhead (muted): _"Distributed ESP32 nodes. Physical entropy. Verifiable on-chain. Randomness is week one."_
- Two CTAs: primary (chrome) "Start Building" + secondary (ghost) "See Live Network"
- Background: looping 15–20s WebM video (abstract silver-on-black particle/circuit flow) OR R3F procedural particle field
- Foreground: interactive 3D globe with pulsing node pins (mock data in `frontend/data/nodes.json` has 20 locations)
- Bottom strip: 3 live counters (nodes online, total rounds, success rate) pulling from `/api/v1/stats`

### Section 3 — The Problem
- Two-column or centered narrative, no cards
- Copy: _"Every VRF on Solana runs on a cloud server. One company. One machine. Trust is a vendor relationship. We built a different model."_
- Optional: small animated diagram showing "cloud server → single point of failure" morphing into "distributed hardware network → byzantine tolerance"

### Section 4 — The Hardware (Full-Bleed Scroll-Driven)
- Full-viewport scroll-pinned section
- Centered: photorealistic GLTF ESP32-S3 model (PBR textures: green solder mask, copper pads, black SoC, gold contacts)
- 6 annotation stops as the user scrolls:
  1. **USB-C connector** — "Power and programming. $8 total BOM."
  2. **Status LEDs** — "Live round state, visible to the operator."
  3. **ESP32-S3 SoC** — "The cryptographic key is burned into silicon. It never leaves."
  4. **WiFi antenna trace** — "mTLS to coordinator. Private PKI."
  5. **Component overview** — "Flash, PSRAM, CP2102 bridge, voltage regulator, crystal oscillator."
  6. **Exploded view** — all components separate, floating in space, labeled
- Rotation progresses from 0 to 2π across the scroll; explode ramps from 0 to 1 at ~70% scroll; camera lerps smoothly
- Annotations fade in near the highlighted component

### Section 5 — How It Works (Horizontal Scroll-Pinned)
- GSAP ScrollTrigger horizontal pin
- 4 steps, each with a custom motion-graphic illustration:
  1. **Request** — dApp calls `request_randomness` via CPI. Visual: arrow flowing from a dApp icon to the DICE program.
  2. **Commit** — nodes generate entropy, hash it, sign it, post commits. Visual: 4 node nodes each producing a hash that snaps into an on-chain slot.
  3. **Reveal** — commit deadline passes, nodes publish raw entropy + signatures. Visual: hashes "open" to reveal underlying entropy.
  4. **Finalize** — on-chain SHA-256 combination, callback delivered. Visual: entropies converging into a single randomness value, then routing to the dApp's callback.
- Each step includes: title, 1-line description, motion illustration, subtle timing marker ("~1s", "~1s", "~1.5s")

### Section 6 — Services (3-Card Grid)
Three glass cards, each with:
- Icon (chrome line-art SVG, custom-designed)
- Title + 2-line description
- Code snippet preview (syntax-highlighted, monospace, dimmed)
- "Learn more" ghost link

Cards:
1. **On-Demand VRF** — Single-shot verifiable randomness. 0.002 SOL per request.
2. **Channel VRF** — Reusable rounds, 18× cost reduction, decoupled callbacks.
3. **Streaming Feed** — Persistent randomness PDA. Unlimited passive readers. Zero per-read cost.

### Section 7 — For Developers (Split Layout)
**Left**: Rust code block showing the one-liner CPI integration:
```rust
use dice_vrf::cpi;

let ix = cpi::request_randomness_ix(&accounts, sequence, &callback_program_id);
solana_program::program::invoke(&ix, account_infos)?;
```
Plus install command: `cargo add dice-vrf`

**Right**: Comparison table
| | DICE | Switchboard | MagicBlock | Orao |
|---|---|---|---|---|
| Latency | ~3.5s | 5–8s | <1s | <1s |
| Price (L1) | 0.002 SOL | 0.002 SOL | 0.0005 SOL | 0.001–0.003 SOL |
| Trust model | Hardware TRNG + Byzantine | TEE (SEV-SNP) | Software VRF | EdDSA multisig |
| Hardware-rooted | Yes | No | No | No |

### Section 8 — For Operators
- Hero photograph of an ESP32-S3 node (lifestyle: on a desk, cable plugged in, LEDs visible)
- Right side: earnings calculator — sliders for "rounds per day" and "nodes operated", live-calculated SOL/day and USD/month
- CTA: "Join the Network" button (chrome)
- Secondary link: "Read the operator setup guide"

### Section 9 — Live Network Stats
- 4 animated counters in a row: Nodes Online · Total Rounds · Success Rate · Avg Latency
- NumberTicker-style animation from 0
- Subtle pulse on the background card
- Data source: `/api/v1/stats` (mock fallback in `lib/api.ts`)

### Section 10 — Trusted By / Ecosystem
- Infinite-scroll logo marquee
- Partner logos: Solana, Anchor, Phantom, Helius, Jupiter, Solana Foundation
- Monochrome white SVG, hover reveals full color
- Marquee direction alternates across rows (row 1 left, row 2 right)

### Section 11 — The Platform Roadmap (The Reveal)
This is the moment where the landing page escalates from "VRF product" to "platform vision". Copy: _"Randomness is week one. Here's what comes next."_

Four horizontal cards with custom AI-generated illustrations:
1. **Trusted Time** — Hardware-signed timestamps from a distributed network.
2. **AI Provenance** — Witness signatures on model outputs and training commitments.
3. **DePIN Witness** — The hardware attestation layer other networks plug into.
4. **Signed Attestations** — Physical event signatures for insurance, supply chain, compliance.

Each card: illustration, title, 1-line description, "Coming soon" tag.

### Section 12 — Founder / Origin Story
- Short narrative block (3–4 sentences)
- Based on `shoot/script-6-the-platform.html` voice
- Copy direction: _"We didn't set out to build a VRF oracle. We were building a lottery game on Solana where real money rides on a single random number. When we saw that every randomness provider ran on a cloud server, we decided we could do better. We started with an $8 microcontroller. That was the beginning."_
- Named founders (check project for actual names — script 6 references Hari and Nikhil)
- Optional: black-and-white photo of founders holding an ESP32, or a 30s looped founder b-roll video

### Section 13 — Final CTA
- Full-width section, centered
- Headline: _"Build on hardware-rooted randomness."_
- Two buttons: primary (chrome) "Start Building" → docs, secondary (ghost) "Read the Whitepaper"
- Subtle animated background (particle field or slow metallic gradient flow)

### Section 14 — Footer
- 4-column layout: Product · Developers · Network · Company
- Logo + tagline in the first column
- Social links (Twitter/X, GitHub, Discord)
- Bottom bar: copyright, devnet program ID, "Built on Solana" badge

---

## 8. Required Assets Checklist

The landing page needs all of the following. Some can be generated by AI (Midjourney, Gemini, Runway, Sora). Others need real production.

### 8.1 Brand and Static Graphics
- [ ] **DICE logo** — SVG master file with variants: monogram, wordmark, full lockup. Light-on-dark only. Metallic gradient fill. Should feel like a luxury tech brand mark, not a crypto project.
- [ ] **Favicon pack** — 16×16, 32×32, 192×192, 512×512, apple-touch-icon, maskable PWA icon
- [ ] **OpenGraph and Twitter card images** — 1200×630 PNG per key route (hero, explorer, docs)
- [ ] **Partner ecosystem logos** — Solana, Anchor, Phantom, Helius, Jupiter, Solana Foundation. 6–10 marks, SVG monochrome white.
- [ ] **Service icons** — 3 custom line-art icons for On-Demand VRF, Channel VRF, Streaming Feed. Chrome gradient fill.
- [ ] **Roadmap illustrations** — 4 abstract AI-generated illustrations (Trusted Time, AI Provenance, DePIN Witness, Signed Attestations). Consistent style. Dark metallic aesthetic. Generate via Midjourney or Gemini with identical prompts.
- [ ] **Icon set (utility)** — comparison-table icons (check, cross, clock, lock, etc.) — use Lucide React, already installed
- [ ] **Custom SVG icons** — 4–6 where Lucide does not fit (hardware-specific, protocol-specific)

### 8.2 Hero and Section Backgrounds
- [ ] **Hero background video loop** — 15–20s seamless WebM + MP4 fallback. Abstract silver-on-black particle or circuit flow. No harsh motion. Generate via Runway ML, Pika, Sora, or render in After Effects.
- [ ] **Section divider textures** — 2–3 subtle AI-generated backgrounds (metallic nebula, circuit traces, dark bokeh) for transitional sections
- [ ] **Noise overlay SVG** — film-grain texture at 3–5% opacity, applied to the root layout

### 8.3 Product Photography (Full Premium Production)
- [ ] **ESP32-S3 hero shots (6–10 photographs)**
  - 3/4 angle beauty shot
  - Macro detail of the ESP32-S3 SoC
  - Macro detail of USB-C port
  - Top-down flat lay
  - Side profile
  - LEDs glowing in the dark
  - All on a dark seamless background with rim lighting, shallow depth of field
  - Production options: (a) real photography in a light tent with a ring light and a proper DSLR, (b) AI product render via Gemini product mode or Midjourney
- [ ] **Lifestyle shots (4–6 scenes)**
  - ESP32 plugged into a laptop
  - Multiple ESP32 nodes stacked on a desk
  - ESP32 held in an operator's hand
  - ESP32 running in a rack / mini server setup
  - ESP32 with LEDs pulsing during a live round
- [ ] **Founder photography (2–3 shots)** — founders holding hardware, working at a desk. Black-and-white or low-saturation color. Matches the origin-story section.

### 8.4 3D Assets
- [ ] **ESP32-S3 photorealistic GLTF/GLB model** — PBR textures: green solder mask, copper pads, black ceramic SoC, gold contacts, white silkscreen. Sources: Sketchfab (search "ESP32-S3"), TurboSquid, or commission a custom model. Must replace the procedural Three.js approximation for landing-page fidelity.
- [ ] **Globe with node pins** — use `cobe` library (already installed, lightweight) OR build a custom R3F globe with emissive pins and arc animations between nodes for more polish
- [ ] **Optional: 3D particle field** — R3F instanced-mesh particle system for the hero backdrop as a WebGL alternative to the video loop

### 8.5 Motion Graphics and Animations

All of the following are code-driven (no asset files needed), unless otherwise noted:

- [ ] **Hero entrance choreography** — Framer Motion stagger: headline words fade up in sequence, subhead fades, CTAs scale-in, globe rotates in, stat counters tick from 0
- [ ] **Scroll-pinned how-it-works timeline** — GSAP ScrollTrigger with 4 custom motion-graphic illustrations (commit-hash forming, reveal unveiling, entropy combining, callback delivery)
- [ ] **ESP32 scroll showcase choreography** — rotation 0 to 2π, explode 0 to 1 at 70% scroll, camera position lerp, annotation whileInView fades, LED emissive pulse during highlighted stop
- [ ] **Animated counters** — NumberTicker-style on all stats
- [ ] **Logo marquee** — infinite-loop CSS or Framer Motion
- [ ] **Button hover states** — chrome shimmer sweep, subtle 1-2px lift, focus ring
- [ ] **Page transitions** — Framer Motion AnimatePresence between routes
- [ ] **Magnetic cursor** — subtle magnetic cursor ring that snaps toward interactive elements (use a tiny custom hook, no library needed)
- [ ] **Section reveal animations** — each major section triggers on `whileInView` with a staggered child reveal
- [ ] **After Effects cut-scenes (2–3 clips, optional)** — short motion-graphic transitions between major narrative sections, delivered as WebM with alpha. Examples: a protocol schematic drawing itself, a commit hash materializing, a node network forming. Commissioned or self-produced.

### 8.6 Video Production (Full Premium Scope)

- [ ] **Hero background loop** — 15–20s seamless silver-on-black abstract. See 8.2.
- [ ] **Hardware b-roll** — 10–15s ESP32 turntable + macro pullback + LED pulse. Real footage or 3D render. Used in Section 4 as optional intercut.
- [ ] **Founder origin mini-doc** — 30s narrative clip adapted from `shoot/script-6-the-platform.html`. Founder on camera in a dimly-lit workspace, holds up the ESP32, explains the origin moment. Used in Section 12.
- [ ] **Product explainer (60–90s)** — full narrated walkthrough: problem → solution → hardware reveal → protocol animation → CTA. Embedded via MUX or YouTube player in a dedicated section (can go after Section 11 or replace part of Section 4). Narration script: use `shoot/script-6-the-platform.html` as a starting point.
- [ ] **Social cut-downs (off-landing-page deliverable)** — 15s Twitter/X teaser, 30s LinkedIn version, 60s vertical for Instagram Reels and TikTok. Derived from the main explainer.

### 8.7 Copy and Text Assets (Needs Written)

- [ ] Hero headline + subhead + 2 CTA labels
- [ ] 14 section headlines + supporting body copy
- [ ] 3 service card descriptions (headline + 2 lines + code snippet)
- [ ] 4 how-it-works step descriptions (title + 1 line)
- [ ] Comparison table data (DICE vs Switchboard, MagicBlock, Orao) — latency, price, trust model, integration LOC
- [ ] 4 roadmap card descriptions (title + 1 line + "coming soon" tag)
- [ ] Founder origin paragraph (3–4 sentences, grounded voice)
- [ ] Operator earnings calculator labels and formulas
- [ ] Page metadata: title, description, OG tags, alt text for all images, favicons
- [ ] Footer copy: nav column headings, social link labels, legal line, devnet program ID
- [ ] Error / loading states for live data (coordinator offline, polling error, zero nodes)

---

## 9. Tech Stack — Already Installed

The receiving AI should build inside `C:\Users\Abcom\DICE\frontend\` using the existing toolchain. **Do not install new major libraries without checking** — the stack is deliberately curated.

| Category | Library | Version |
|---|---|---|
| Framework | Next.js (App Router) | 16.2.2 |
| Runtime | React | 19.2.4 |
| Language | TypeScript | strict mode |
| Styling | Tailwind CSS | v4 (CSS variables in `app/globals.css`) |
| UI primitives | shadcn/ui + Radix | latest (button, card, badge, table, tabs) |
| Motion (DOM) | Framer Motion | v12.38 |
| Motion (scroll) | GSAP + @gsap/react | v3.14 |
| Smooth scroll | Lenis | v1.3 |
| 3D engine | @react-three/fiber | v9.5 |
| 3D helpers | @react-three/drei | v10.7 |
| WebGL effects | @react-three/postprocessing | v3.0 |
| Globe | cobe | v2.0 |
| Charts | Recharts | v3.8 |
| Icons | Lucide React | v1.7 |
| Data | @tanstack/react-query | v5.96 |
| Package manager | pnpm | (workspace) |

**Not installed, do not add**: Spline (bloat), Vanta.js (redundant with R3F), Aceternity UI as a package (copy-paste components only if needed), anime.js (redundant with GSAP + Framer).

### Reference data contracts (reuse these)
- Mock nodes: `frontend/data/nodes.json` — 20 locations with lat/lng
- API hooks: `frontend/lib/hooks.ts` — `useStats()`, `useNodes()`, `useRounds()`
- Types: `frontend/lib/types.ts`
- Brand constants: `frontend/lib/constants.ts` — `BRAND`, `NAV_LINKS`, `SOCIAL_LINKS`, `API_URL`
- Graceful fallbacks: `frontend/lib/api.ts` returns mock data when coordinator is offline

---

## 10. Strict Don'ts

These are non-negotiable. Violating any of these means the landing page fails review.

1. **No green accents on the landing page.** Green `#00FF85` is reserved for the `/explorer` sub-app. Use chrome, silver, white, or grey everywhere else.
2. **No emojis** in code, copy, UI, or markdown. Not in headlines, not in buttons, not in code comments.
3. **No generic "Web3 crypto neon" aesthetic**. No glowing purples, no cyan accents, no rainbow gradients, no pixel art.
4. **No multi-chain positioning.** DICE is Solana-native. Own it. Do not hedge with "multi-chain roadmap" or "EVM support coming".
5. **No "trustless oracle" or "solved the oracle problem" rhetoric.** Those positions are taken by Chainlink and Switchboard.
6. **No "SGX is broken" or "TEE is dying" claims.** Switchboard moved to SEV-SNP in 2025. This weapon is dulled. The differentiator is "hardware-tangible, distributed, no vendor dependency", not "TEEs are bad".
7. **No per-request revenue projections or TAM slides** in the marketing copy. The unit economics are thin. Do not invite scrutiny in copy.
8. **No stock photos of generic "Web3 people looking at laptops"**. Either real product photography, AI-generated abstract imagery, or no image.
9. **No dark patterns**: no faux scarcity ("only 100 nodes left"), no countdown timers, no popups. The brand is restraint.
10. **No broken "Launch App" button**. Point it to `/explorer` (the existing route) if mainnet is not live yet.

---

## 11. Deliverable Checklist for the Receiving AI

When you finish, confirm:

- [ ] All 14 sections render in order and scroll smoothly on desktop and mobile
- [ ] Design system honored — no green on landing, metallic everywhere
- [ ] 3D hardware showcase works on scroll (rotation, explode, annotations)
- [ ] Globe renders with node pins
- [ ] Live stats pull from `/api/v1/stats` with graceful mock fallback
- [ ] Comparison table is accurate and sourced from Section 7 above
- [ ] Lighthouse performance >= 85 on mobile, >= 95 on desktop
- [ ] OG image and metadata set correctly
- [ ] All assets from Section 8 are either present or clearly listed as "TODO — awaiting production"
- [ ] No emojis anywhere
- [ ] No green anywhere on the landing page
- [ ] `pnpm dev` runs without errors
- [ ] `pnpm build` completes without errors
- [ ] Mobile responsive down to 375px

---

## 12. Critical File References

If the receiving AI needs to cross-check anything, these are the source-of-truth files in the repo:

| File | Purpose |
|---|---|
| `C:\Users\Abcom\DICE\README.md` | Protocol facts, instruction list, program ID |
| `C:\Users\Abcom\DICE\frontend\app\globals.css` | Design tokens, CSS variables, utility classes |
| `C:\Users\Abcom\DICE\frontend\lib\constants.ts` | Brand name, nav links, API URL |
| `C:\Users\Abcom\DICE\frontend\lib\api.ts` | Live data + mock fallback |
| `C:\Users\Abcom\DICE\frontend\data\nodes.json` | 20 mock node locations |
| `C:\Users\Abcom\DICE\shoot\script-6-the-platform.html` | Brand voice reference, origin story |
| `C:\Users\Abcom\DICE\programs\dice\src\constants.rs` | On-chain constants (fees, timeouts, node limits) |
| `C:\Users\Abcom\DICE\sdk\dice-vrf\src\cpi.rs` | SDK integration surface — the code snippets should look like this |

---

**End of brief. Build the landing page.**
