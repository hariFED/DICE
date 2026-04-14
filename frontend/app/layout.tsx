import type { Metadata } from "next"
import { Space_Grotesk } from "next/font/google"
import { Geist_Mono } from "next/font/google"
import { Providers } from "@/components/providers"
import "./globals.css"

const spaceGrotesk = Space_Grotesk({
  subsets: ["latin"],
  display: "swap",
  variable: "--font-sans",
})

const geistMono = Geist_Mono({
  subsets: ["latin"],
  display: "swap",
  variable: "--font-geist-mono",
})

export const metadata: Metadata = {
  title: "DICE — Hardware-Backed VRF Oracle",
  description:
    "Verifiable randomness for Solana powered by 20 physical ESP32-S3 devices. Commit-reveal protocol, hardware-backed entropy, 0.002 SOL per request.",
  keywords: [
    "Solana",
    "VRF",
    "oracle",
    "randomness",
    "hardware",
    "ESP32",
    "blockchain",
  ],
}

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode
}>) {
  return (
    <html
      lang="en"
      className={`${spaceGrotesk.variable} ${geistMono.variable} dark h-full antialiased`}
      suppressHydrationWarning
    >
      <body className="min-h-full flex flex-col bg-background text-foreground">
        <Providers>{children}</Providers>
      </body>
    </html>
  )
}
