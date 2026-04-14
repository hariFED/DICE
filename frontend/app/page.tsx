import { Header } from "@/components/shared/Header"
import { Footer } from "@/components/shared/Footer"
import { Hero } from "@/components/landing/Hero"
import { HowItWorks } from "@/components/landing/HowItWorks"
import { ForDevelopers } from "@/components/landing/ForDevelopers"
import { ForOperators } from "@/components/landing/ForOperators"
import { LiveStats } from "@/components/landing/LiveStats"
import { TrustedBy } from "@/components/landing/TrustedBy"

export default function Home() {
  return (
    <main className="relative">
      <Header />
      <Hero />
      <HowItWorks />
      <ForDevelopers />
      <ForOperators />
      <LiveStats />
      <TrustedBy />
      <Footer />
    </main>
  )
}
