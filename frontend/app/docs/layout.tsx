import type { Metadata } from "next"
import { Header } from "@/components/shared/Header"
import { Footer } from "@/components/shared/Footer"
import { DocsSidebar } from "@/components/docs/Sidebar"
import { DocsMobileNav } from "@/components/docs/MobileNav"
import { DocsSearchStub } from "@/components/docs/SearchStub"
import "./docs.css"

export const metadata: Metadata = {
  title: {
    default: "Docs — DICE",
    template: "%s · DICE Docs",
  },
  description:
    "Integrate hardware-backed verifiable randomness into any Solana program with one CPI call.",
}

export default function DocsLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return (
    <>
      <Header />
      <div className="mx-auto w-full max-w-[1400px] px-4 sm:px-6 lg:px-8">
        {/* Docs sub-header — terminal-style with prompt + search */}
        <div className="sticky top-14 z-40 -mx-4 flex items-center gap-3 px-4 py-3 sm:-mx-6 sm:px-6 lg:-mx-8 lg:px-8">
          <DocsMobileNav />
          <span className="hidden sm:inline font-mono text-xs text-muted-foreground">
            {/* <span className="text-muted-foreground/60">$</span>  <span className="text-foreground">v7.7</span> */}
          </span>
          <div className="ml-auto w-full max-w-xs">
            <DocsSearchStub />
          </div>
        </div>

        <div className="flex gap-10 pt-6 pb-16">
          <aside className="hidden lg:block w-60 shrink-0">
            <div className="sticky top-32 max-h-[calc(100vh-9rem)] overflow-y-auto pr-2 pb-8">
              <DocsSidebar />
            </div>
          </aside>

          <main className="min-w-0 flex-1">{children}</main>
        </div>
      </div>
      <Footer />
    </>
  )
}
