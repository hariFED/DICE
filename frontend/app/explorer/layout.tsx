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
      <main className="min-h-screen pt-28 px-4 md:px-8 max-w-7xl mx-auto">
        {children}
      </main>
      <Footer />
    </>
  )
}
