"use client"

import { useState, useCallback } from "react"
import { cn } from "@/lib/utils"

/**
 * Lightweight code block with language label, copy-to-clipboard button and
 * a hand-rolled token highlighter for `rust`, `typescript`, `bash` and
 * `toml`. No external deps — keeps the bundle tiny.
 *
 * The highlighter is intentionally naive: it's good enough for doc-sized
 * snippets and it handles the exact patterns we need (keywords, types,
 * strings, comments, numbers, attributes). For serious highlighting we'd
 * reach for shiki, but that doubles our server bundle.
 */
export function CodeBlock({
  code,
  lang = "text",
  filename,
}: {
  code: string
  lang?: "rust" | "typescript" | "ts" | "bash" | "toml" | "text"
  filename?: string
}) {
  const [copied, setCopied] = useState(false)
  const handleCopy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(code)
      setCopied(true)
      setTimeout(() => setCopied(false), 1800)
    } catch {
      // noop — clipboard may be unavailable
    }
  }, [code])

  const highlighted = highlight(code, lang)

  return (
    <div className="group/code relative my-4 overflow-hidden border border-border bg-background">
      {(filename || lang !== "text") && (
        <div className="flex items-center justify-between border-b border-border bg-muted/30 px-4 py-1.5 text-[11px] font-mono">
          <span className="ascii-label text-[10px]">
            {filename ?? langLabel(lang)}
          </span>
          <button
            type="button"
            onClick={handleCopy}
            className="rounded-sm px-2 py-0.5 text-muted-foreground hover:text-foreground transition-colors uppercase tracking-wider"
            aria-label="Copy code"
          >
            {copied ? "✓ COPIED" : "⧉ COPY"}
          </button>
        </div>
      )}
      <pre className="overflow-x-auto px-4 py-4 text-[13px] leading-[1.65] font-mono text-foreground">
        <code
          className={cn("language-" + lang)}
          dangerouslySetInnerHTML={{ __html: highlighted }}
        />
      </pre>
    </div>
  )
}

function langLabel(l: string) {
  if (l === "typescript" || l === "ts") return "TypeScript"
  if (l === "rust") return "Rust"
  if (l === "bash") return "Shell"
  if (l === "toml") return "TOML"
  return l
}

/* ------------------------ naive highlighter ----------------------------- */

const RUST_KW = new Set([
  "pub",
  "fn",
  "struct",
  "enum",
  "impl",
  "use",
  "mod",
  "let",
  "mut",
  "match",
  "if",
  "else",
  "return",
  "for",
  "in",
  "while",
  "loop",
  "as",
  "const",
  "static",
  "crate",
  "self",
  "Self",
  "super",
  "where",
  "async",
  "await",
  "move",
  "ref",
  "dyn",
  "trait",
  "type",
  "Result",
  "Ok",
  "Err",
  "Some",
  "None",
])
const RUST_TYPES = new Set([
  "u8",
  "u16",
  "u32",
  "u64",
  "u128",
  "i8",
  "i16",
  "i32",
  "i64",
  "i128",
  "usize",
  "isize",
  "bool",
  "char",
  "str",
  "String",
  "Vec",
  "Option",
  "Result",
  "Pubkey",
  "Context",
  "Account",
  "Accounts",
  "UncheckedAccount",
  "Signer",
  "SystemAccount",
  "DiceResult",
  "DiceChannel",
  "GameState",
  "CpiContext",
])

const TS_KW = new Set([
  "const",
  "let",
  "var",
  "function",
  "return",
  "if",
  "else",
  "for",
  "while",
  "import",
  "export",
  "from",
  "as",
  "async",
  "await",
  "new",
  "class",
  "interface",
  "type",
  "extends",
  "implements",
  "public",
  "private",
  "protected",
  "readonly",
  "true",
  "false",
  "null",
  "undefined",
  "void",
  "this",
  "throw",
  "try",
  "catch",
  "finally",
])

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
}

function wrap(cls: string, text: string): string {
  return `<span class="${cls}">${text}</span>`
}

function highlight(code: string, lang: string): string {
  if (lang === "rust") return highlightRust(code)
  if (lang === "typescript" || lang === "ts") return highlightTs(code)
  if (lang === "bash") return highlightBash(code)
  if (lang === "toml") return highlightToml(code)
  return escapeHtml(code)
}

/**
 * Tokenize with a single-pass regex. Order of alternatives matters:
 *   comment → string → attribute → ident-with-macro → ident → number.
 */
function highlightRust(code: string): string {
  // NB: no `s` flag — we need es2017 compatibility. `[\s\S]` covers it.
  const re =
    /(\/\/[^\n]*|\/\*[\s\S]*?\*\/)|("([^"\\]|\\.)*")|(#\[[^\]]*\])|([A-Za-z_][A-Za-z0-9_]*!?)|(\b\d[\d_]*\b)|([\s\S])/g
  return code.replace(
    re,
    (m, comment, str, _s2, attr, ident, num) => {
      if (comment) return wrap("tk-c", escapeHtml(comment))
      if (str) return wrap("tk-s", escapeHtml(str))
      if (attr) return wrap("tk-a", escapeHtml(attr))
      if (ident) {
        const esc = escapeHtml(ident)
        if (ident.endsWith("!")) return wrap("tk-m", esc)
        if (RUST_KW.has(ident)) return wrap("tk-k", esc)
        if (RUST_TYPES.has(ident)) return wrap("tk-t", esc)
        if (/^[A-Z]/.test(ident)) return wrap("tk-t", esc)
        return esc
      }
      if (num) return wrap("tk-n", escapeHtml(num))
      return escapeHtml(m)
    },
  )
}

function highlightTs(code: string): string {
  const re =
    /(\/\/[^\n]*|\/\*[\s\S]*?\*\/)|("([^"\\]|\\.)*"|'([^'\\]|\\.)*'|`([^`\\]|\\.)*`)|([A-Za-z_$][A-Za-z0-9_$]*)|(\b\d[\d_.]*n?\b)|([\s\S])/g
  return code.replace(
    re,
    (m, comment, str, _s1, _s2, _s3, ident, num) => {
      if (comment) return wrap("tk-c", escapeHtml(comment))
      if (str) return wrap("tk-s", escapeHtml(str))
      if (ident) {
        const esc = escapeHtml(ident)
        if (TS_KW.has(ident)) return wrap("tk-k", esc)
        if (/^[A-Z]/.test(ident)) return wrap("tk-t", esc)
        return esc
      }
      if (num) return wrap("tk-n", escapeHtml(num))
      return escapeHtml(m)
    },
  )
}

function highlightBash(code: string): string {
  return code
    .split("\n")
    .map((line) => {
      if (line.trimStart().startsWith("#"))
        return wrap("tk-c", escapeHtml(line))
      // crude: first word is the command
      const match = line.match(/^(\s*)(\S+)(.*)$/)
      if (!match) return escapeHtml(line)
      const [, pre, cmd, rest] = match
      return (
        escapeHtml(pre) +
        wrap("tk-k", escapeHtml(cmd)) +
        escapeHtml(rest)
      )
    })
    .join("\n")
}

function highlightToml(code: string): string {
  return code
    .split("\n")
    .map((line) => {
      if (line.trimStart().startsWith("#"))
        return wrap("tk-c", escapeHtml(line))
      if (/^\s*\[/.test(line)) return wrap("tk-t", escapeHtml(line))
      const m = line.match(/^(\s*)([\w\-]+)(\s*=\s*)(.*)$/)
      if (m) {
        const [, pre, key, eq, val] = m
        return (
          escapeHtml(pre) +
          wrap("tk-k", escapeHtml(key)) +
          escapeHtml(eq) +
          wrap("tk-s", escapeHtml(val))
        )
      }
      return escapeHtml(line)
    })
    .join("\n")
}
