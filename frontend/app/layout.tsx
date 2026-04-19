import type { Metadata } from "next"
import { Geist, Geist_Mono, Silkscreen } from "next/font/google"
import { Providers } from "@/components/providers"
import "./globals.css"

const geistSans = Geist({
  subsets: ["latin"],
  display: "swap",
  variable: "--font-sans",
})

const geistMono = Geist_Mono({
  subsets: ["latin"],
  display: "swap",
  variable: "--font-mono",
})

/** Pixel font — used for chapter numbers and big numerical readouts. */
const silkscreen = Silkscreen({
  subsets: ["latin"],
  display: "swap",
  weight: ["400", "700"],
  variable: "--font-pixel",
})

export const metadata: Metadata = {
  title: "DICE — Hardware-Backed VRF Oracle",
  description:
    "Verifiable randomness for Solana powered by physical ESP32-S3 devices. Commit-reveal protocol, hardware-backed entropy, 0.002 SOL per request.",
  keywords: [
    "Solana",
    "VRF",
    "oracle",
    "randomness",
    "hardware",
    "ESP32",
    "blockchain",
  ],
  icons: {
    icon: [
      { url: "/favicon.svg", type: "image/svg+xml" },
      { url: "/favicon.ico", sizes: "any" },
    ],
    shortcut: "/favicon.svg",
    apple: "/favicon.svg",
  },
  openGraph: {
    title: "DICE — Hardware-Backed VRF Oracle",
    description: "Physical ESP32 devices. Commit-reveal. 0.002 SOL per request.",
    type: "website",
  },
  twitter: {
    card: "summary_large_image",
    creator: "@dicelabsnetwork",
  },
}

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode
}>) {
  return (
    <html
      lang="en"
      className={`${geistSans.variable} ${geistMono.variable} ${silkscreen.variable} h-full antialiased`}
      suppressHydrationWarning
    >
      <body className="min-h-full flex flex-col bg-background text-foreground">
        <Providers>{children}</Providers>
      </body>
    </html>
  )
}
