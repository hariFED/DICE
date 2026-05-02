#!/usr/bin/env node
/**
 * Build script — walks src/, renders each HTML page to PDF via Chromium.
 *
 * Per-page format hints come from `<meta name="pdf-size" content="...">` and
 * `<meta name="pdf-orientation" content="...">` tags. Defaults: Letter, portrait.
 *
 * Usage:
 *   node build-pdfs.mjs
 *   node build-pdfs.mjs slides   # only build matching pages
 */

import { chromium } from "playwright"
import { mkdir, readFile, readdir, stat } from "node:fs/promises"
import { dirname, join, relative } from "node:path"
import { fileURLToPath, pathToFileURL } from "node:url"

const __dirname = dirname(fileURLToPath(import.meta.url))
const SRC = join(__dirname, "src")
const DIST = join(__dirname, "dist")

const filter = process.argv[2] ?? ""

async function* walk(dir) {
  for (const entry of await readdir(dir)) {
    const full = join(dir, entry)
    const s = await stat(full)
    if (s.isDirectory()) yield* walk(full)
    else if (full.endsWith(".html")) yield full
  }
}

async function readMeta(html) {
  const size = /<meta[^>]+name=["']pdf-size["'][^>]+content=["']([^"']+)/i.exec(html)?.[1] ?? "Letter"
  const orient = /<meta[^>]+name=["']pdf-orientation["'][^>]+content=["']([^"']+)/i.exec(html)?.[1] ?? "portrait"
  return { size, landscape: orient === "landscape" }
}

async function main() {
  await mkdir(DIST, { recursive: true })
  const browser = await chromium.launch()
  const ctx = await browser.newContext()
  const page = await ctx.newPage()

  let made = 0
  for await (const htmlFile of walk(SRC)) {
    const rel = relative(SRC, htmlFile)
    if (filter && !rel.includes(filter)) continue

    const pdfOut = join(DIST, rel.replace(/[\\/]/g, "_").replace(/\.html$/, ".pdf"))
    const html = await readFile(htmlFile, "utf8")
    const { size, landscape } = await readMeta(html)

    process.stdout.write(`  ${rel} → ${relative(__dirname, pdfOut)}  (${size}, ${landscape ? "landscape" : "portrait"}) `)
    await page.goto(pathToFileURL(htmlFile).toString(), { waitUntil: "networkidle" })
    await page.emulateMedia({ media: "print" })
    await page.pdf({
      path: pdfOut,
      format: size,
      landscape,
      printBackground: true,
      margin: { top: 0, right: 0, bottom: 0, left: 0 },
      preferCSSPageSize: true,
    })
    console.log("✓")
    made++
  }

  await browser.close()
  console.log(`\n${made} PDF${made === 1 ? "" : "s"} written to dist/`)
}

main().catch((err) => {
  console.error(err)
  process.exit(1)
})
