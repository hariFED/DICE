# DICE Frontend

Premium landing page and live network explorer for the
[DICE](https://github.com/dice-network/dice) hardware-backed VRF oracle on
Solana. Built with Next.js 16 (App Router), React 19, framer-motion, GSAP,
shadcn/ui, and `@react-three/fiber` for the 3D ESP32 model and cobe globe.

## Stack

- **Next.js 16** — App Router, React Server Components, static + streaming
- **TypeScript** with strict mode
- **Tailwind CSS v4** for styling, `shadcn/ui` for base components
- **framer-motion** + **GSAP** + **lenis** for scroll and entry animations
- **@react-three/fiber** + **drei** for the procedural ESP32 PCB model
- **cobe** for the WebGL globe with node location markers
- **Tanstack Query** for live coordinator data fetching (5s poll)
- **pnpm** as the package manager (see `pnpm-workspace.yaml`)

## Project structure

```
frontend/
├── app/
│   ├── layout.tsx          # Root layout, theme provider, fonts
│   ├── page.tsx            # Landing page (Hero + sections)
│   └── explorer/
│       ├── layout.tsx
│       ├── page.tsx        # Live network explorer
│       ├── nodes/          # /explorer/nodes — connected ESP32 nodes
│       └── rounds/         # /explorer/rounds — VRF round history
├── components/
│   ├── landing/            # Hero, LiveStats, HowItWorks, ForDevelopers, ...
│   ├── three/              # ESP32Model, Globe, ESP32Scene (R3F)
│   ├── shared/             # Header, Footer, GlowCard, AnimatedCounter, ...
│   └── ui/                 # shadcn primitives
├── lib/
│   ├── api.ts              # Fetch helpers against the coordinator
│   ├── hooks.ts            # Tanstack Query hooks (useNodes, useRounds, ...)
│   ├── constants.ts        # Brand, nav links, API URL
│   └── types.ts            # Coordinator response shapes
└── public/                 # Static assets
```

## Quick start

```bash
# From the repo root:
cd frontend
pnpm install

# Run against a local coordinator (default http://localhost:8080):
pnpm dev

# Open http://localhost:3000
```

## Environment variables

Copy `.env.example` to `.env.local` and adjust:

```bash
cp .env.example .env.local
```

| Variable | Default | Purpose |
|---|---|---|
| `NEXT_PUBLIC_API_URL` | `http://localhost:8080` | Base URL of the DICE coordinator REST API. The frontend polls `/api/v1/stats`, `/nodes`, `/rounds`, and `/queue` from this origin. |

The coordinator must have CORS enabled and be reachable from the browser's
origin. The release v7 coordinator binary applies `CorsLayer::permissive()`
at startup, so any origin works out of the box.

## Connecting to a coordinator

### Local coordinator (simulation mode)

```bash
# In another terminal, from the repo root:
cargo run -p dice-coordinator --release -- --simulation
```

Then `pnpm dev` in `frontend/` and open http://localhost:3000. The
Explorer page will start polling the simulated coordinator immediately.

### Local coordinator (production mode)

If you're running the coordinator with a real PostgreSQL database and
Solana RPC (see the root repo's `.env.example`), the frontend wiring is
identical — just point `NEXT_PUBLIC_API_URL` at the API port.

### Remote coordinator

Set `NEXT_PUBLIC_API_URL=https://your-coordinator.example.com` and
`pnpm build && pnpm start`. The frontend makes only public, unauthenticated
GET requests against the coordinator — no API keys needed in the browser.

## Production build

```bash
pnpm build    # produces .next/ with static assets + server bundle
pnpm start    # runs the production server on :3000
```

## Deploying

### Vercel (one-click)

1. Push this repo to GitHub
2. Import the repo in Vercel
3. Set Root Directory to `frontend`
4. Add environment variable `NEXT_PUBLIC_API_URL` pointing at your
   production coordinator
5. Deploy

Vercel auto-detects Next.js 16 and handles the build + deploy.

### Self-host

```bash
pnpm build
pnpm start                # or use a process manager (pm2, systemd)
```

For a static export, Next 16 supports `output: 'export'` in
`next.config.ts` — useful if you're serving via a CDN and don't need
server components. Note that the Explorer page's client-side polling
still works in static mode, but you lose Next's caching + RSC benefits.

### Troubleshooting

**Stats panel shows "Coordinator offline"** — the browser can't reach
the URL in `NEXT_PUBLIC_API_URL`. Check:
- The URL is correct and includes the scheme (`http://` or `https://`)
- The coordinator is actually running and listening on that address
- If running over HTTPS, the coordinator has a valid TLS cert
- CORS is enabled on the coordinator (the v7 binary does this
  automatically; older builds may not)

**Explorer shows empty nodes/rounds tables** — same root cause as
above, plus: verify the coordinator has the v7 endpoints `/nodes`,
`/rounds`, `/queue`, and `/api/v1/stats`. Pre-v7 coordinators did not
serve `/api/v1/stats` at all.

**3D ESP32 model doesn't render** — this component is client-only
(`dynamic(() => import(...), { ssr: false })`). Check the browser console
for WebGL errors; some restrictive CSPs or older GPUs can break R3F.

## License

See root repo for license.
