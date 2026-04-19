import { Header } from "@/components/shared/Header"
import { Footer } from "@/components/shared/Footer"
import { Hero } from "@/components/landing/Hero"
import { StatsTicker } from "@/components/landing/StatsTicker"
import { HowItWorks } from "@/components/landing/HowItWorks"
import { ForDevelopers } from "@/components/landing/ForDevelopers"
import { EspScrollShowcase } from "@/components/landing/EspScrollShowcase"
import { ForOperators } from "@/components/landing/ForOperators"
import { LiveStats } from "@/components/landing/LiveStats"
import { TrustedBy } from "@/components/landing/TrustedBy"

export default function Home() {
  return (
    <main className="relative">
      <Header />
      <StatsTicker />
      <Hero />
      <HowItWorks />
      <ForDevelopers />
      <EspScrollShowcase />
      <ForOperators />
      <LiveStats />
      <TrustedBy />
      <Footer />
    </main>
  )
}
