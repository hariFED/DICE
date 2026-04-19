import { Header } from "@/components/shared/Header"
import { Footer } from "@/components/shared/Footer"
import { StatsTicker } from "@/components/landing/StatsTicker"
import { Hero } from "@/components/landing/Hero"
import { ProtocolFlow } from "@/components/landing/ProtocolFlow"
import { Esp32Exploded } from "@/components/landing/Esp32Exploded"
import { Manifesto } from "@/components/landing/Manifesto"
import { Roadmap } from "@/components/landing/Roadmap"
import { UseCases } from "@/components/landing/UseCases"
import { DevQuickstart } from "@/components/landing/DevQuickstart"
import { OperatorPitch } from "@/components/landing/OperatorPitch"
import { Faq } from "@/components/landing/Faq"
import { TrustedBy } from "@/components/landing/TrustedBy"

export default function Home() {
  return (
    <main className="relative">
      <Header />
      <StatsTicker />
      <Hero />
      <ProtocolFlow />
      <Esp32Exploded />
      <Manifesto />
      <Roadmap />
      <UseCases />
      <DevQuickstart />
      <OperatorPitch />
      <Faq />
      <TrustedBy />
      <Footer />
    </main>
  )
}
