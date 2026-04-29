import { PageTransition } from "@/components/shared/PageTransition"

export default function DocsTemplate({
  children,
}: {
  children: React.ReactNode
}) {
  return <PageTransition>{children}</PageTransition>
}
