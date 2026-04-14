"use client"

import { useState, useCallback } from "react"
import { cn } from "@/lib/utils"

interface CopyButtonProps {
  text: string
  className?: string
}

export function CopyButton({ text, className }: CopyButtonProps) {
  const [copied, setCopied] = useState(false)

  const handleCopy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(text)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    } catch {
      // Fallback for non-secure contexts
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
        "relative inline-flex items-center justify-center rounded-md p-1.5 text-zinc-500 hover:text-zinc-300 hover:bg-white/[0.06] transition-colors duration-200",
        className
      )}
      aria-label="Copy to clipboard"
    >
      {copied ? (
        <>
          {/* Check icon */}
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="#00FF85"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <polyline points="20 6 9 17 4 12" />
          </svg>
          <span className="absolute -top-8 left-1/2 -translate-x-1/2 rounded bg-[#00FF85]/20 px-2 py-0.5 text-xs font-medium text-[#00FF85] whitespace-nowrap border border-[#00FF85]/30">
            Copied!
          </span>
        </>
      ) : (
        /* Copy icon */
        <svg
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
          <path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1" />
        </svg>
      )}
    </button>
  )
}
