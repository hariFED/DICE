"use client"

import dynamic from "next/dynamic"
import { Header } from "@/components/shared/Header"
import { Footer } from "@/components/shared/Footer"
import { Hero } from "@/components/landing/Hero"

/* Below-fold sections — code-split into separate chunks so the
   initial page load only pays for Header + Hero. Each section
   loads as the user scrolls or the browser idles. */
const ProtocolFlow = dynamic(
  () => import("@/components/landing/ProtocolFlow").then((m) => ({ default: m.ProtocolFlow })),
)
const Esp32Exploded = dynamic(
  () => import("@/components/landing/Esp32Exploded").then((m) => ({ default: m.Esp32Exploded })),
)
const Manifesto = dynamic(
  () => import("@/components/landing/Manifesto").then((m) => ({ default: m.Manifesto })),
)
const Roadmap = dynamic(
  () => import("@/components/landing/Roadmap").then((m) => ({ default: m.Roadmap })),
)
const UseCases = dynamic(
  () => import("@/components/landing/UseCases").then((m) => ({ default: m.UseCases })),
)
const DevQuickstart = dynamic(
  () => import("@/components/landing/DevQuickstart").then((m) => ({ default: m.DevQuickstart })),
)
const Faq = dynamic(
  () => import("@/components/landing/Faq").then((m) => ({ default: m.Faq })),
)

export default function Home() {
  return (
    <main className="relative">
      <Header />
      <Hero />
      <ProtocolFlow />
      <Esp32Exploded />
      <Manifesto />
      <Roadmap />
      <UseCases />
      <DevQuickstart />
      <Faq />
      <Footer />
    </main>
  )
}
