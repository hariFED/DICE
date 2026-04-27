# DICE Pre-Launch Narrative — Device-First, Pre-Register Edition

> **Status:** Spec / working document. Author: CEO + design.
> **Audience:** Frontend implementer, marketing copy, design QA.
> **Goal:** Capture the maximum traction signal (emails, intent volume, virality)
> from a device-first landing page with a free pre-register CTA, **without
> taking money** and **without overpromising**.
> **Out of scope today:** Payment processing, full e-commerce checkout,
> the developer-facing landing page (separate doc), shipping logistics.

---

## 0. Why this doc exists

We have two products: **the device** and **the VRF service**. They serve two
audiences that don't speak the same language. The strategic call (per
`marketing/strategy/two-lane-launch.md` — to be written): lead the public
brand with the device, run dApp integrations behind the scenes.

Pre-register (free, no payment, just email) is the right opening move for the
device side because:

1. We don't yet know how much demand exists for hardware-backed VRF mining.
   Pre-register *is* the experiment.
2. Pre-orders with money trigger refund liability, fulfillment expectations,
   and securities-law sensitivity ("buying an investment that earns yield").
   Pre-register avoids all three.
3. A waitlist is socially fungible: a viral pre-register list (e.g. Solana
   Saga) becomes a pitch deck slide for VCs, a press hook for journalists,
   and credibility for the dApp BD pipeline simultaneously.
4. We can convert the list to a paid pre-order *later*, on our own timing,
   when we have audit, mainnet program, and ≥3 dApp integrations to point at.

**Single success metric:** number of unique email signups in the first 30 days.

**Anti-metric (do NOT optimize for):** Twitter follower count, page views,
celebrity retweets without conversion. Vanity.

---

## 1. The audience model

The page must work for FOUR distinct readers without writing four pages.
Each one arrives in a different mental state:

| Reader | Mental state on arrival | What they need to convert |
|---|---|---|
| **The Crypto Believer** | Already buys DePIN hardware (Helium hotspots, DIMO, Pollen). Skims for "what's the rate, when does it ship." | Quick "how it works," scarcity cue, low-friction form. |
| **The Curious Newbie** | Heard "Solana" and "passive income" in the same sentence. Doesn't know what VRF means. | Plain-English explanation. The mining metaphor. Trust signals. |
| **The Builder / Skeptic** | Solana dev or DePIN engineer. Came to vet the project. Will leave if anything seems hand-wavy. | Live network proof (`/explorer`), technical mention, "for developers" link. |
| **The Investor / Reporter** | Looking for the story angle. "Is this big enough to write about / fund?" | Founder credibility, traction signal (visible signup count, partners), narrative weight. |

Design constraint: **same one-page flow serves all four.** Each finds their
own "lane" by scrolling to the section that resonates. The page must reward
all four reading paths.

---

## 2. The narrative arc

Top to bottom, the page is a three-act story:

```
ACT I  — The promise        (hero)
ACT II — The proof          (device, mining loop, live network, social)
ACT III — The ask           (pre-register form, FAQ, footer)
```

Each act answers a question:

- **Act I** answers: *"What is this?"* in 8 seconds.
- **Act II** answers: *"Why should I believe it?"* in 90 seconds.
- **Act III** answers: *"What do you want from me?"* in 15 seconds.

The CTA appears **three times**:
1. Primary in the hero (visible above the fold)
2. Mid-page, after the mining loop diagram (mid-conviction)
3. End-of-page, after the FAQ (closer for stragglers)

Same destination, same form. Don't fragment.

---

## 3. Page-by-page spec

### Section 0 — Top bar / nav (already exists in `Header.tsx`)

Keep as-is. The pre-register page is `/preorder` (existing route — repurpose).
Don't rename to `/pre-register` yet; it'd break shared links and the nav from
the hero CTA. Treat the URL slug as legacy; the page content is what matters.

**Copy on the nav:**
- Logo (existing)
- "Network" → `/explorer`
- "Docs" → `/docs`
- "Pre-register" → `/preorder` (new nav item — make it the visually loudest)
- Theme toggle (existing)

---

### Section 1 — Hero (`components/landing/Hero.tsx`) ✅ already shipped

What landed today:
- H1: **Mine VRF.** *While you* **sleep.**
- Subline: "A real box on your shelf mining verifiable randomness for Solana.
  One-time purchase. No fans. No diminishing returns. No electricity tax."
- 3 pillars: hardware on shelf · earn from every request · plug-in-once
- Primary CTA: `[ Pre-order_Your_Node ]` → `/preorder`
- Right column: rotating dotted globe (entropy mesh)

**Two changes still needed for the pre-register pivot:**

1. **CTA label**: rename `Pre-order_Your_Node` → `Pre-register_Your_Node`.
   "Pre-order" implies payment; "pre-register" sets expectation correctly.
   The user is signing up for a wait-list, not buying yet.
2. **Subline tweak**: drop "One-time purchase." (implies cost commitment).
   Replace with "Built for the patient. Ships when the network's ready."

Keep everything else.

---

### Section 2 — Banner strip: "Pre-register, no payment yet"

A thin (~48 px) horizontal band immediately under the hero. Existing
`AsciiDivider.tsx` aesthetic — dashed border top + bottom, mono small caps.

```
┌──────────────────────────────────────────────────────────────────┐
│  [ EARLY ACCESS ]  Pre-register is free. We'll notify you when   │
│  the first batch ships — and your spot in the queue is reserved. │
└──────────────────────────────────────────────────────────────────┘
```

Why: removes the #1 objection ("how much?") in the first 2 seconds of
scrolling. Buyer commits emotionally before checking price.

Visual: pure ASCII chrome. No emoji. No exclamation marks.

---

### Section 3 — "What is it?" (one-screen explainer)

A two-column section: left = plain-English explanation, right = device hero shot.

**Copy (left column):**

```
WHAT IT IS

DICE is a small box you plug into your home WiFi. It draws true 
randomness from physical noise — not from a software seed — and 
sells that randomness, on chain, to apps on Solana that need it.

You don't run servers. You don't manage anything. You don't even 
keep the box on a desk; it lives next to your router.

Every time the network needs a random number, your box helps 
generate one and gets paid for it. Automatically. To a wallet 
you control.

THAT'S IT.
```

**Visual treatment:**
- Mono body, sans-serif headings (existing pattern)
- Section number `02 / WHAT IT IS` in pixel font, top-left
- The phrase "true randomness from physical noise" gets a hover tooltip:
  "ESP32-S3 hardware TRNG — bit-stream pulled from electrical thermal noise."
- "THAT'S IT." in foreground color, capped, sits as an emotional anchor.

**Right column: the device.**

We have multiple ESP32 components in the codebase already:
`AsciiEsp32.tsx`, `Esp32Blueprint.tsx`, `Esp32Exploded.tsx`,
`EspScrollShowcase.tsx`. Use **`Esp32Blueprint.tsx`** — it's the architectural
drawing style, matches the editorial brand, and is the right "this is real
engineering" tonal cue.

Below the blueprint, a four-stat readout chip strip:

```
[ BOARD ]      [ POWER ]        [ CONNECTIVITY ]    [ FOOTPRINT ]
ESP32-S3       ~1 W (USB)       2.4 GHz WiFi        51 × 27 × 8 mm
```

NO numerical earnings claims. Just hardware specs. Specs are facts; rates are promises.

---

### Section 4 — The mining loop (the differentiator)

This is the single section that decides whether crypto-Twitter screenshots
the page. It needs to be visually iconic and copy-light.

**Headline (sans, large):**

```
Bitcoin asks $3,000/year in electricity for shrinking rewards.

DICE asks for less than your fridge magnet.
```

(Yes, "less than your fridge magnet" is the punchline. ESP32-S3 draws ~1W;
a fridge magnet does work zero. The hyperbole reads.)

**Visual: a side-by-side comparison panel.** No table. Two stylized device
silhouettes — left is a Bitcoin ASIC, right is the DICE box — drawn in the
same blueprint line-art style as `Esp32Blueprint.tsx`. Same scale, but the
DICE box is comically smaller. Annotated like an engineer's diagram.

```
LEFT PANEL                    RIGHT PANEL

[ BITCOIN ASIC ]              [ DICE NODE ]

3,500 W                       ~1 W
$3,000/yr electricity         less than that
2-3 yr lifespan               5-10 yr lifespan
80 dB cooling fans            silent
returns halve every 4y        scales with dApp adoption
```

Beneath, in small mono caption:

```
// passive income, without the carbon footprint or the noise complaint
```

**Why this works:** every reader gets a story to tell their friend in one
sentence. ("It's like Bitcoin mining but the box is the size of a credit
card and uses no electricity.")

**Anti-pattern:** Do NOT include exact dollar earnings. We don't know them
yet. The Bitcoin number is *cost*, not earnings — comparing costs is honest.

---

### Section 5 — How earning works (mechanism)

Now we've earned the right to explain HOW it works, because the reader is
emotionally bought in. Time to deliver substance.

**Headline:**

```
Where the money comes from.
```

(Direct. The reader is asking this question; answer it.)

**Below: a 4-step horizontal flow diagram.** Not a video, not an animation,
not WebGL — just clean blueprint-style boxes connected by dashed arrows.
Same vocabulary as `ProtocolFlow.tsx`. Reuse that component if shape fits.

```
[ 01 ]              [ 02 ]              [ 03 ]              [ 04 ]
A Solana app        Your DICE node      The app gets        Your wallet 
needs a random      (with a few         the random          gets credited
number              others) generates   number              with a share
                    it together                              of the fee
                                        ↘ on chain ↙
```

Each box has a one-line caption. No jargon ("VRF", "commit-reveal", "PDA"
all banned). The reader leaves understanding **(a)** there's real demand
upstream, **(b)** their box is one of several that splits the work, **(c)**
payouts are automatic and on-chain.

**Below the diagram — a single sentence in italic muted-foreground:**

```
We split each fee: 70% to the contributing nodes, 20% to the protocol 
treasury, 10% to a network reserve. No token, no staking, no lock-ups.
```

That sentence is a **trust signal**. It says "we're not going to launch a
token and dump it on you" without saying it. Read by skeptics, ignored by
believers, exactly as intended.

---

### Section 6 — Live network (proof, not promise)

This is the section that converts the skeptic. Embed real numbers from
`/explorer` so the reader can verify the network exists.

**Headline:**

```
We're already running. On devnet. Right now.
```

**Below: 4 live-pulled stat cards** (re-use `LiveStats.tsx` component, point
it at `/api/v1/stats` already wired):

```
[ NODES · ONLINE ]    [ ROUNDS · 24H ]    [ AVG · LATENCY ]    [ NETWORK ]
4                     ~100                 6.0 s                Solana devnet
```

(The "approximate" tilde on rounds/24h is honest — exact count fluctuates,
ballpark is what matters.)

**Below the cards: a single text link in mono small-caps:**

```
→ See every round live in the explorer
```

Goes to `/explorer`. Skeptics will click it, see real data, and be
converted (or churn — better to lose them here than after they've
pre-registered with bad expectations).

---

### Section 7 — Mid-page CTA (second of three)

Now the reader has been emotionally hooked (Section 4), intellectually
satisfied (Section 5), and shown proof (Section 6). They're ready.

A full-bleed banded section, dark background with subtle grid:

```
                   [ Pre-register — no payment yet ]

           Reserve your spot for the first batch.
              We'll email when shipping opens.

                  [  PRE-REGISTER NOW  ]
                          ↓
                 (form scrolls into view)
```

Use `BracketLink` styling already in the codebase. Outline button, white-on-
transparent, hovers fill. The action: scroll to Section 9 (form) — don't
navigate to a new page. Form lives on the same page; this CTA is a smooth
scroll.

---

### Section 8 — Who's already building on DICE

Even if these are aspirational / partnership letters of intent, list them
honestly. **Empty social-proof sections are worse than no section at all** —
they highlight what you don't have.

**If we have ≥ 3 confirmed integration partners by launch:**

```
EARLY INTEGRATION PARTNERS

[ Logo ]    [ Logo ]    [ Logo ]
PartnerA    PartnerB    PartnerC

Live on devnet. Mainnet integrations open Q3.
```

**If we have 0–2 confirmed partners (likely current state):**

Skip this section entirely. Replace with a self-quote / vision statement:

```
WHO IT'S FOR

Solana games that need fair dice rolls. Lottery dApps that need 
verifiable winners. Prediction markets that need unbiased event
outcomes. Anywhere on Solana where randomness has to be trusted, 
not just generated.

If that's you, [start here →]   (link to /docs)
```

The link routes prospective dApp builders to the dev funnel — captures the
secondary audience without distorting the primary narrative.

---

### Section 9 — The form (the conversion event)

This is what we're optimizing for. Every gram of friction kills 30% of the
remaining funnel. Three rules:

1. **Fewer fields than feel right.** Email + ONE optional field is the
   maximum. Resist the temptation to ask for shipping address, payment
   info, NFT verification, anything else. Every additional field is one
   reason to bounce.
2. **No account creation.** No password. No wallet connect. No social SSO.
   Just an email input and a button.
3. **Submit instantly.** No 5-second skeleton. Optimistic update — user
   types email, hits Reserve, sees confirmation in <100 ms.

**Form spec:**

```
[ Section header ]
RESERVE YOUR DICE NODE

[ Subhead ]
We'll email when shipping opens. No payment yet — pre-register only.

[ Field 1 — required ]
Email address                                 [ you@example.com           ]

[ Field 2 — optional, single-select pills ]
How many would you want?
[ 1 ]   [ 2-3 ]   [ 4+ ]   [ I'm a developer integrating DICE ]
                                       ↑ secret door for the dev funnel

[ Field 3 — optional, single-select pills, only renders if Field 2 ≠ developer ]
Why are you interested?
[ Passive income ]   [ DePIN curiosity ]   [ Hardware tinkering ]   [ Other ]

[ Submit button, full-width, primary ]
[  RESERVE_MY_SPOT →  ]

[ Sub-button caption, mono small-caps muted ]
no payment · no commitment · unsubscribe anytime
```

**Backend spec:**
- Form submits to existing BFF route `frontend/app/api/v1/preorder/route.ts`
  (already exists — just confirm shape matches).
- Server adds: timestamp, source URL, UTM tags from `?utm_*` query params,
  geographic country (from request headers, no IP storage).
- Pipes to: existing storage (Formspree / Supabase / coord DB — check what's
  wired).
- Returns: queue position number (computed: `count(emails before this one)`).

**Validation:**
- Email regex (basic, server-side) — reject obviously fake formats
- No CAPTCHA on first launch (kills conversion). Add only if spam exceeds
  10% of submissions.
- One signup per email (server-side dedup).

---

### Section 10 — Confirmation experience (post-submit)

The ~5 seconds after the user submits is the most viral window. Get this right.

**Inline confirmation (replaces the form on the same page, no navigation):**

```
[ ✓  RESERVED ]

You're #247 in line.

We'll email you when the first batch ships. Until then, watch
the network grow live in the explorer →

──────────────────────────────────────────────────

Help us get there faster.
Share this with one friend who'd plug in a node.

[  COPY_REFERRAL_LINK  ]   [  SHARE_ON_TWITTER  ]
```

Three things this confirmation must do:

1. **Position number** ("#247 in line") — gives them ownership. They have a
   number now. Numbers are sticky. Helium did this; Saga did this.
2. **Link to `/explorer`** — keeps the curiosity loop alive. They're now
   invested in the network's growth.
3. **Referral surface** — the moment of highest commitment is the moment to
   ask for a share. **Don't ask before the form is submitted; the friction
   kills.**

**Twitter share template:**

```
Just pre-registered for a DICE node. It's like a Bitcoin miner the size
of a credit card that earns from Solana randomness requests. No fans,
no diminishing returns. https://dicelabs.net/preorder?ref=USER_ID
```

(280 chars exactly. Test on a phone. The `ref` param attribution lets us
later show "you got X friends to register" leaderboard if we want viral
mechanics.)

**Email confirmation (sent within 60 sec of signup):**

```
Subject: You're in. (DICE Node #247)

Body:
Hey.

You're in line for one of the first DICE Nodes. No payment yet —
we'll email when shipping opens.

While you wait, three things you can poke at:

→ See the network running live
   https://dicelabs.net/explorer

→ Read what dApps are doing with it
   https://dicelabs.net/docs

→ Forward this to one friend who'd run a node
   https://dicelabs.net/preorder?ref=USER_REF

— DICE Labs
```

Plain-text first; HTML version optional (and never required — plenty of
crypto-natives prefer plain text and consider HTML newsletters spam-coded).

---

### Section 11 — FAQ

Pre-empt the doubts. Six questions. Real, blunt answers.

```
WHEN DOES IT SHIP?
Honestly: when the network is real. We're targeting Q3 2026
post-mainnet audit. We won't ship hardware before mainnet has 
demand to fulfill — that would be selling a paperweight.

HOW MUCH WILL IT COST?
We don't know yet. Hardware BOM is locked; final retail depends 
on volume and shipping. We'll email a price before we ask for
payment, and you can decline without losing your spot.

HOW MUCH WILL I EARN?
Network demand is unproven on mainnet. We're not promising 
numbers we can't back. Pre-register is for people who want
to watch this experiment from the front row, not investors
expecting yield.

WHO RUNS DICE?
[Founder name + 1 sentence + LinkedIn link.]
[ Built in [country], based in [country]. ]

IS THIS A SECURITY?
We don't think so — pre-register is free, hardware is a one-time 
purchase of a real device, and earnings come from on-chain 
service fees, not token speculation. We are not lawyers; do 
your own homework. We're talking to lawyers.

WHAT IF I CHANGE MY MIND?
Reply to any of our emails with "remove me." Your data is 
deleted within 7 days. No drama.
```

The bluntness is the credibility. Generic-FAQ-language reads like a sales
deck. Founder-voice reads like a person.

---

### Section 12 — Footer

Standard. Links to `/docs`, `/explorer`, `/preorder`, contact email,
Twitter/X, GitHub. Keep it sparse. No newsletter signup in the footer —
**we want one and only one CTA on this page.**

---

## 4. Visual / copy / design guidelines

### Brand reuse

- All sections use the existing dark blueprint aesthetic: pure black bg,
  off-white text, dotted grid, dashed dividers, bracketed `[ LABEL ]` chrome,
  corner ticks on cards, sans for headings, mono for body, pixel font for
  big numerals.
- Color: zero green except status indicators on `/explorer`. Matches the
  brand correction memo from 2026-04 (`feedback_design_correction.md`).
- Typography: Geist Sans (headings), Geist Mono (body), Silkscreen (numerals).
  All already wired in `app/layout.tsx`.

### Copy voice

Three rules:

1. **No exclamation marks.** Anywhere. Including in copy you really really
   want one in. The brand voice is confident, dry, slightly amused. Like a
   blueprint tagline, not a TV commercial.
2. **Specific over general.** "Plugs into your router" beats "easy to set
   up." "Fits in your hand" beats "compact." Use concrete nouns.
3. **No future-perfect tense.** Don't write "will be the leader in." Write
   present-tense and let proof carry the claim.

### Things to NEVER write on this page

- "Revolutionary" / "game-changing" / "the next generation"
- "Powered by AI" (it isn't and doesn't need to be)
- "Earn $X / month" — even with "up to" qualifier
- Any number that has a "+" suffix ("100+ partners", "1000+ rounds") if
  the actual number is below 10
- Any reference to "ROI" / "investment" / "yield" — securities law landmines
- "Our token" / "$DICE" — there isn't one and won't be one for a long while
- "We're better than [competitor]" — compare *categories* (hardware vs
  software VRF), not vendors

---

## 5. Mobile

~70% of pre-register traffic will arrive on phones. Design mobile-first:

- Hero collapses to single column, headline + CTA visible above the fold.
- Mining loop comparison stacks vertically (Bitcoin ASIC ABOVE DICE box).
  Don't shrink the diagram — replace with a tighter mobile version.
- Live network stat cards become a 2×2 grid.
- Form fields full-width, taps not pinches. iOS auto-zoom suppressed (input
  font-size ≥ 16 px — the same gotcha we hit on the captive portal).

---

## 6. Anti-patterns to consciously avoid

| Pattern | Why we don't do it | What we do instead |
|---|---|---|
| Big celebrity testimonial above the fold | We don't have one. Faking it is worse than nothing. | Live network stats — proof beats endorsement. |
| Countdown timer ("ships in 14 days") | We don't know when. Lying about it = trust nuke. | "When shipping opens." Pure honesty. |
| Email field in the hero | Too early. The buyer hasn't bought into anything yet. | First CTA scrolls to form, doesn't BE the form. |
| Newsletter signup in the footer | Splits CTA attention. | One CTA on the page. Pre-register or leave. |
| WebGL device renderer | Slow on mobile, doesn't tell more story than a blueprint. | Existing `Esp32Blueprint.tsx` SVG — tells the engineering story. |
| Auto-playing video | Modern browsers mute it; users hate it; mobile data spike. | Static blueprint + scroll-cued reveal. |
| Live chat widget / Intercom popup | Friction that destroys conversion. | One static `mailto:` in footer. |
| Specific earnings calculator | Premature; speculative; legally fragile. | "Earnings depend on network demand, which is what we're proving." |

---

## 7. Metrics to instrument

Add these to whatever analytics you already wire (Vercel Analytics is
already set up; add basic event tracking).

**Pre-register funnel:**
1. Hero CTA click → `cta_hero_click`
2. Mid-page CTA click → `cta_midpage_click`
3. End-of-page CTA click → `cta_endpage_click`
4. Form viewed (in viewport) → `form_view`
5. Form started (first character typed) → `form_start`
6. Form submitted → `form_submit`
7. Confirmation viewed → `confirmation_view`
8. Referral link copied → `referral_copy`
9. Twitter share clicked → `referral_twitter`

**Funnel conversion targets to hit by week 4:**

| Stage | Bench | Stretch |
|---|---|---|
| Hero CTA CTR | 8% | 15% |
| Form completion (started → submitted) | 65% | 80% |
| Referral copy rate (post-submit) | 15% | 30% |
| Email confirmation open rate | 50% | 70% |

If hero CTA CTR < 5%, the headline isn't landing — A/B test new copy.
If form-completion < 40%, the form is too long or too late — strip a field
or move the form earlier.

---

## 8. A/B tests to run after launch

Don't launch with these. Run them weeks 2–4.

| Test | Variant A | Variant B | Hypothesis |
|---|---|---|---|
| Hero headline | "Mine VRF. While you sleep." | "Plug in. Earn." | Shorter wins on mobile. |
| Mining-loop punchline | "Less than your fridge magnet" | "Cheaper than your phone charger" | Both work; data picks. |
| Form field count | 1 (email only) | 3 (email + qty + reason) | Removing fields lifts conversion. |
| Confirmation queue position | "#247 in line" | "First 250 cohort" (cohort framing) | Cohort feels more exclusive. |
| Mid-page CTA copy | "Pre-register — no payment yet" | "Save your spot — free" | Different framing of same idea. |

Use Vercel Edge Config or PostHog feature flags to split. Don't use a third
party A/B tool that adds 200 ms to every request.

---

## 9. Implementation order (what the dev should build first)

If we have one frontend dev × 1 week, this is the order. Each stage is
shippable independently.

**Day 1 — copy swap (existing components, just text):**
- Hero CTA: "Pre-order" → "Pre-register"
- Hero subline: drop "One-time purchase"
- Replace existing `/preorder` page contents with new sections 2-9
  (some sections may already exist; re-skin where possible)

**Day 2 — Section 4 mining loop:**
- New component: `BitcoinVsDiceComparison.tsx`
- Reuse `Esp32Blueprint.tsx` for the right side; build a similar SVG ASIC
  silhouette for the left.
- Test on mobile (this is the most-shared section).

**Day 3 — Section 5 earning loop:**
- New or modified `ProtocolFlow.tsx` — make it readable to a non-engineer.
- 4 boxes, plain English, no jargon.

**Day 4 — Form + confirmation:**
- Re-skin existing `/preorder` form to spec
- Wire confirmation page with queue position
- Email cadence (just the confirmation; drip campaign comes later)
- Wire referral param `?ref=`

**Day 5 — Polish + analytics:**
- Mobile pass
- A/B test scaffolding (don't activate yet)
- Event tracking
- FAQ section
- QA on three devices, three browsers

**Day 6 — Soft launch:**
- Push to Vercel via direct CLI deploy (the GitHub webhook is currently
  dead — see ops notes).
- Share with a small audience (10 friends, 3 Solana Discord channels).
- Watch funnel for 24h, fix the worst-converting stage.

**Day 7 — Public launch:**
- Submit to Product Hunt, DePIN aggregators, founder's personal Twitter.

---

## 10. Reference shots

A few comparable sites whose pre-register flow this should *feel* like:

- **Helium hotspot 2019** — the original "buy a box, mine a network" story
- **Solana Saga** — pre-register without payment, viral queue position
- **Light Phone II pre-order** — minimal aesthetic, blueprint-driven
- **North Focals pre-launch** — spec-table-as-art, hardware credibility
- **Sandy Munro's tear-down channel** — the engineer-as-storyteller voice

What we are NOT trying to feel like:
- Bored Ape / NFT mint sites (overwhelming, FOMO-hostile, brand-incongruent)
- ICO landing pages 2017-vintage (token tilt, securities red flags)
- Generic SaaS waitlist (Bento, Linear, etc — those are software, we're hardware)

---

## 11. Glossary (for copy reviewers)

When the copy uses these terms, this is what we mean:

| Term | Meaning |
|---|---|
| **Mine VRF** | Use a DICE node to participate in randomness rounds. The verb is intentional — borrows mental model from Bitcoin mining. |
| **Verifiable randomness** | The output is provably random (commit-reveal protocol, on-chain audit trail). |
| **Hardware-backed** | The randomness comes from physical entropy (TRNG), not a software seed. |
| **A round** | One randomness request being fulfilled by the network. ~6 sec on devnet. |
| **A node** | One DICE box. Synonym for "device" in operator copy. |
| **The network** | All currently-connected nodes + the coordinator + the Solana program. |
| **Pre-register** | Free signup. Email + name only. No payment, no commitment. |
| **The first batch** | The initial production run we ship. Size TBD. Use this term to imply scarcity without committing to a number. |

---

## 12. What this doc is NOT

- Not a copywriter's brief — it's a designer + dev brief. A copywriter
  may rewrite individual sentences. Spec the structure first.
- Not a final design. The visual treatment is described, not pixel-specced.
- Not a launch plan. PR / influencer outreach / community amp lives in a
  separate doc (`marketing/launch-week.md`).
- Not a fundraising deck. The pre-register list is the *result* of this
  page — the deck uses the result, not the page itself.

---

## 13. The single sentence

If the implementer reads only one line of this doc, this:

> **Sell the box, prove the network, capture the email — in that order, on one page, with no exclamation marks.**

Everything else is execution.

---

*Last updated: 2026-04-25*
*Owner: CEO*
*Implementer: frontend lead*
