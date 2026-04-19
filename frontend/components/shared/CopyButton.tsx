"use client"

import { useState, useCallback } from "react"
import { cn } from "@/lib/utils"

interface CopyButtonProps {
  text: string
  className?: string
}

/**
 * Mono ASCII-style copy button. Glyph swaps from `⧉` (copy) to `✓` (copied),
 * with a small toast above for 2s.
 */
export function CopyButton({ text, className }: CopyButtonProps) {
  const [copied, setCopied] = useState(false)

  const handleCopy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(text)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    } catch {
      const textarea = document.createElement("textarea")
      textarea.value = text
      textarea.style.position = "fixed"
      textarea.style.opacity = "0"
      document.body.appendChild(textarea)
      textarea.select()
      document.execCommand("copy")
      document.body.removeChild(textarea)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    }
  }, [text])

  return (
    <button
      type="button"
      onClick={handleCopy}
      className={cn(
        "relative inline-flex items-center justify-center rounded-sm h-5 w-5 font-mono text-xs leading-none text-muted-foreground hover:text-foreground hover:bg-muted/40 transition-colors",
        className,
      )}
      aria-label="Copy to clipboard"
    >
      <span>{copied ? "✓" : "⧉"}</span>
      {copied && (
        <span className="absolute -top-7 left-1/2 -translate-x-1/2 ascii-label text-[10px] bg-background border border-border px-1.5 py-0.5 whitespace-nowrap">
          copied
        </span>
      )}
    </button>
  )
}
