export const BRAND = {
  name: "DICE",
  tagline: "Hardware-Backed Verifiable Randomness for Solana",
  description:
    "20 physical devices. Commit-reveal protocol. 0.002 SOL per request.",
  colors: {
    green: "#00FF85",
    greenDim: "#00cc6a",
    black: "#000000",
  },
} as const

// Alias used by shared components (Header, Footer)
export const SITE = {
  name: BRAND.name,
  tagline: BRAND.tagline,
  launchAppUrl: "/explorer",
} as const

export const API_URL =
  process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080"
export const REFRESH_INTERVAL = 5000 // 5s polling for live data

// NAV_LINKS deliberately omits "Docs" — there is no public docs site yet.
// The interactive HTML explainers in how-it-works/ are not deployed publicly,
// and dead "#" links ship a visible bug. Add the entry back when a real docs
// URL exists.
export const NAV_LINKS = [
  { label: "Home", href: "/" },
  { label: "Explorer", href: "/explorer" },
] as const

// discord is intentionally absent — no server exists yet. Shipping a "#" link
// as a social icon would be a visible bug. Add it back when there's a real URL.
export const SOCIAL_LINKS = {
  github: "https://github.com/dice-network",
  twitter: "https://x.com/dice_network",
} as const
