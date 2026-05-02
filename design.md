# DICE Network — Design System

Reference document for building consistent pages across the DICE site.
Derived from the home screen (Hero + Header + NetworkGlobe).

---

## 1. Color Palette

### Brand Gradient (Solana)

| Token | Value | Usage |
|-------|-------|-------|
| **Purple** | `#9945FF` | Gradient start, comet tails on globe |
| **Green** | `#14F195` | Gradient end |
| **Gradient CSS** | `linear-gradient(to right, #9945FF, #14F195)` | Hero "Solana" text, accents |

### Dark Theme (Primary)

| Token | CSS Variable | Value |
|-------|-------------|-------|
| Background | `--background` | `#0a0a0a` |
| Foreground | `--foreground` | `#fafafa` |
| Card | `--card` | `#0a0a0a` |
| Secondary | `--secondary` | `#141414` |
| Muted Foreground | `--muted-foreground` | `#a1a1aa` |
| Border | `--border` | `rgba(255, 255, 255, 0.10)` |
| Input | `--input` | `rgba(255, 255, 255, 0.14)` |
| Ring | `--ring` | `rgba(255, 255, 255, 0.40)` |

### Status Colors (Dark Mode)

| Status | Variable | Value |
|--------|----------|-------|
| OK / Finalized | `--status-ok` | `#4ade80` |
| Warn / In-progress | `--status-warn` | `#facc15` |
| Error / Failed | `--status-err` | `#f87171` |

### Texture Tokens

| Token | Value |
|-------|-------|
| `--ascii-bg` | `rgba(255, 255, 255, 0.04)` |
| `--ascii-bg-strong` | `rgba(255, 255, 255, 0.08)` |

---

## 2. Typography

### Font Stack

| Role | CSS Variable | Family | Notes |
|------|-------------|--------|-------|
| **Sans** | `--font-sans` | Geist | Headings (h1–h6), body alt |
| **Mono** | `--font-mono` | Geist Mono | Default body font, labels, inputs |
| **Pixel** | `--font-pixel` | Silkscreen | Chapter numbers, decorative display |

### Font Feature Settings

Applied to `<body>`: `"ss01", "cv11"` (stylistic alternates, character variants).

### Heading Style

```css
h1, h2, h3, h4, h5, h6 {
  font-family: var(--font-sans);
  letter-spacing: -0.01em;
}
```

### Utility: ASCII Label

Monospace, 0.75rem, uppercase, `letter-spacing: 0.06em`, with `[ ]` brackets via pseudo-elements.

```css
.ascii-label {
  font-family: var(--font-mono);
  font-size: 0.75rem;
  letter-spacing: 0.06em;
  color: var(--muted-foreground);
  text-transform: uppercase;
}
```

### Utility: Chapter Number

Large decorative number used as section markers (e.g. "01").

```css
.chapter-num {
  font-family: var(--font-pixel);
  font-size: clamp(48px, 6vw, 92px);
  line-height: 0.85;
  letter-spacing: -0.02em;
  color: var(--foreground);
  opacity: 0.15;
}
```

---

## 3. Liquid Glass Effect

The core visual identity. Two tiers:

### Standard `.liquid-glass`

```css
.liquid-glass {
  background: rgba(255, 255, 255, 0.01);
  background-blend-mode: luminosity;
  backdrop-filter: blur(4px);
  -webkit-backdrop-filter: blur(4px);
  border: none;
  box-shadow: inset 0 1px 1px rgba(255, 255, 255, 0.1);
  position: relative;
  overflow: hidden;
}
```

**Gradient border overlay** (via `::before`):

```css
.liquid-glass::before {
  background: linear-gradient(180deg,
    rgba(255,255,255,0.45) 0%,
    rgba(255,255,255,0.15) 20%,
    rgba(255,255,255,0) 40%,
    rgba(255,255,255,0) 60%,
    rgba(255,255,255,0.15) 80%,
    rgba(255,255,255,0.45) 100%);
}
```

### Strong `.liquid-glass-strong`

Higher blur (50px), added drop shadow. Use for primary CTAs and prominent cards.

```css
.liquid-glass-strong {
  background: rgba(255, 255, 255, 0.01);
  backdrop-filter: blur(50px);
  box-shadow: 4px 4px 4px rgba(0,0,0,0.05),
              inset 0 1px 1px rgba(255,255,255,0.15);
}
```

---

## 4. Texture & Pattern Overlays

Layer these behind content for depth. All use `--ascii-bg` / `--ascii-bg-strong` tokens.

| Class | Pattern | Size |
|-------|---------|------|
| `.dither-fine` | Checkerboard (repeating-conic-gradient) | 3px × 3px |
| `.dither-dots` | Dot matrix (radial-gradient) | 5px × 5px |
| `.scanlines` | Horizontal scanlines (repeating-linear-gradient, overlay blend) | 3px repeat |
| `.bg-grid` | Blueprint grid | 24px × 24px |
| `.bg-grid-fine` | Fine grid | 8px × 8px |
| `.bg-iso-grid` | Isometric grid (30deg / -30deg) | 28px × 16px |
| `.ascii-shade` | 45deg diagonal stripes | 7px repeat |

---

## 5. Border Radius Scale

Base `--radius: 0.25rem`. All others derived:

| Token | Calc | Value |
|-------|------|-------|
| `--radius-sm` | `× 0.6` | `0.15rem` |
| `--radius-md` | `× 0.8` | `0.20rem` |
| `--radius-lg` | `× 1.0` | `0.25rem` |
| `--radius-xl` | `× 1.4` | `0.35rem` |
| `--radius-2xl` | `× 1.8` | `0.45rem` |
| `--radius-3xl` | `× 2.2` | `0.55rem` |
| `--radius-4xl` | `× 2.6` | `0.65rem` |

**Common usage:** `rounded-full` for pills, nav items, CTAs. `rounded-2xl` for cards/panels.

---

## 6. Component Patterns

### Header / Navbar

- Floating nav bar centered at top
- Nav links inside a `liquid-glass rounded-full` pill
- Logo: `liquid-glass` circle, 48×48px (`h-12 w-12`)
- CTA button (right): solid white bg, black text, `rounded-full`
- Mobile menu: `liquid-glass rounded-2xl`

### Hero Section

- **Chapter number**: `01` in pixel font, oversized, 15% opacity
- **Section label**: "THE DICE NETWORK" — mono, uppercase, muted
- **Headline**: Large sans-serif, bold, tight leading. "Solana" rendered with `bg-clip-text text-transparent` + brand gradient
- **Feature list**: Numbered `01`, `02`, `03` items in mono, muted foreground
- **CTA buttons**: Two side-by-side, `rounded-full`:
  - Primary: `liquid-glass-strong`, mono uppercase
  - Secondary: `liquid-glass`, mono uppercase
  - Tertiary text link: "OR SEE THE NETWORK LIVE →"

### Network Globe (Right Side)

- 3D WebGL sphere on dark/transparent canvas
- Dot-matrix earth with land/ocean differentiation
- Wireframe cube markers at city positions (white, low opacity)
- Animated comet trails: white head → `#9945FF` purple tail
- Subtle glass overlay + fresnel rim glow
- Stats overlay: `ENTROPY`, `HEAD`, with live-updating values in mono

### Cards / Panels

- `liquid-glass` or `liquid-glass-strong` background
- `rounded-2xl` or `rounded-lg`
- Optional `corner-ticks` for editorial chrome (12px corner marks, 30% opacity)

### Status Pills

- Outlined, colored text matching status:
  - `.pill-ok` — green (`--status-ok`)
  - `.pill-warn` — yellow (`--status-warn`)
  - `.pill-err` — red (`--status-err`)

### Form Inputs

- Monospace, 13px, `padding: 10px 14px`
- Border radius: 4px
- Focus: foreground-colored border + `box-shadow: 0 0 0 1px var(--foreground)`

---

## 7. Animations

| Name | CSS | Usage |
|------|-----|-------|
| Marquee | `marquee-x` — translateX(0 → -50%), linear infinite | Scrolling tickers |
| Progress Dot | Scale 1→1.5, opacity 0.3→1 on active | Step indicators |

---

## 8. Spacing & Layout Conventions

- **Page background**: `bg-background` (`#0a0a0a`)
- **Max content width**: Contained with padding, hero is full-bleed
- **Section spacing**: Generous vertical padding between sections
- **Grid**: Hero uses a two-column layout — text left, globe right
- **Responsive**: Mobile collapses to single column, globe below text

---

## 9. Design Principles

1. **Dark-first**: Everything is designed for the dark theme. Light mode exists but dark is the identity.
2. **Liquid glass everywhere**: Use `liquid-glass` for any floating UI element (nav, cards, modals, tooltips).
3. **Monospace by default**: Body text is mono. Sans is reserved for headings and display text.
4. **Solana purple-green gradient**: Used sparingly for brand emphasis — gradient text, accent highlights, globe trails. Never as a background fill.
5. **Editorial chrome**: Chapter numbers, bracketed labels, corner ticks, ASCII rules. The site feels like a technical document or spec sheet.
6. **Texture as depth**: Dither patterns, grids, and scanlines add subtle texture — never so strong they distract from content.
7. **Minimal color**: The palette is essentially monochrome (white on black) with the Solana gradient as the only color accent. Status colors appear only in functional contexts.
