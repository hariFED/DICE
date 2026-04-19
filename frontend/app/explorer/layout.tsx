import { Header } from "@/components/shared/Header"
import { Footer } from "@/components/shared/Footer"

export default function ExplorerLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return (
    <>
      <Header />
      <main className="min-h-screen pt-10 px-4 md:px-8 max-w-7xl mx-auto">
        {/* Terminal-style breadcrumb */}
        <nav className="mb-6 font-mono text-xs text-muted-foreground">
          <span className="text-muted-foreground/60">$</span> cd ~ / dice / <span className="text-foreground">explorer</span>
        </nav>
        {children}
      </main>
      <Footer />
    </>
  )
}
