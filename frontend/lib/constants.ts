export const BRAND = {
  name: "DICE",
  parent: "DICELabs",
  /** Backronym shown under the wordmark on the hero. */
  acronym: {
    D: "Decentralized",
    I: "Incentivized",
    C: "Cryptographic",
    E: "Entropy",
  },
  tagline: "Hardware-Backed Verifiable Randomness for Solana",
  description:
    "20 physical devices. Commit-reveal protocol. 0.002 SOL per request.",
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

export const NAV_LINKS = [
  { label: "Home", href: "/" },
  { label: "Explorer", href: "/explorer" },
  { label: "Docs", href: "/docs" },
] as const

// discord is intentionally absent — no server exists yet. Shipping a "#" link
// as a social icon would be a visible bug. Add it back when there's a real URL.
export const SOCIAL_LINKS = {
  github: "https://github.com/dicelabsnetwork",
  twitter: "https://x.com/dicelabsnetwork",
  twitterHandle: "@dicelabsnetwork",
} as const
