# DICE · Marketing Kit

Print-ready HTML → PDF assets for presentations, packaging,
onboarding cards, and brand docs.

## Layout

```
marketing/
├── src/
│   ├── slides/
│   │   └── deck.html              # 12-slide pitch deck (16:9)
│   ├── cards/
│   │   ├── packages.html          # Starter / Pro / Rack product cards
│   │   └── how-to.html            # Operator + Developer quickstart cards
│   ├── branding/
│   │   └── brandbook.html         # Full brand guidelines
│   └── shared/
│       ├── styles.css             # Brand tokens + print helpers
│       └── cube.svg               # Logo (dark-on-transparent)
├── build-pdfs.mjs                 # Headless Chromium HTML → PDF
├── package.json
└── dist/                          # Generated PDFs land here
```

## Build

```bash
cd marketing
pnpm install
pnpm pdf                          # generate all PDFs into dist/
pnpm pdf:serve                    # serve HTML at localhost:4000 for preview
```

Each HTML file is self-contained — open in a browser to preview
before rendering. The build script uses Playwright Chromium (first
run downloads ~300 MB; subsequent runs are cached).

## What renders

| Source                         | Output                   | Paper size  |
| ------------------------------ | ------------------------ | ----------- |
| `src/slides/deck.html`         | `dist/deck.pdf`          | 16:9, 1080p |
| `src/cards/packages.html`      | `dist/packages.pdf`      | Letter · 4up |
| `src/cards/how-to.html`        | `dist/how-to.pdf`        | Letter · 4up |
| `src/branding/brandbook.html`  | `dist/brandbook.pdf`     | Letter      |

## Customization

Brand tokens live in `src/shared/styles.css` as CSS variables.
Fonts are loaded via Google Fonts — swap `@import` lines to use
local files for fully offline builds.
