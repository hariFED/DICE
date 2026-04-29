import { PageTransition } from "@/components/shared/PageTransition"

export default function ExplorerTemplate({
  children,
}: {
  children: React.ReactNode
}) {
  return <PageTransition>{children}</PageTransition>
}
