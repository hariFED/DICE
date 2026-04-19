"use client"

import { useState } from "react"
import { BracketLink } from "@/components/shared/BracketButton"

/**
 * Dev quickstart — single install command + one code sample + link out.
 * Two tabs: Rust and TypeScript. Copy buttons on each block.
 */

const TS_CODE = `import { DiceClient } from "@dicelabs/vrf";

const client = new DiceClient({ cluster: "devnet" });

// Request one 32-byte random value for 0.002 SOL
const round = await client.requestRandomness({
  payer: wallet,
  callback: handleResult,
});

console.log(round.signature);`

const RUST_CODE = `use anchor_lang::prelude::*;
use dice_vrf::cpi::request_randomness_auto;

#[program]
pub mod my_game {
    pub fn roll(ctx: Context<Roll>) -> Result<()> {
        request_randomness_auto(
            ctx.accounts.dice_ctx(),
            callback_seed!("roll"),
        )?;
        Ok(())
    }
}`

export function DevQuickstart() {
  const [lang, setLang] = useState<"ts" | "rust">("ts")
  const [copied, setCopied] = useState(false)

  const code = lang === "ts" ? TS_CODE : RUST_CODE
  const installCmd = lang === "ts" ? "npm i @dicelabs/vrf" : "cargo add dice-vrf"

  const handleCopy = () => {
    navigator.clipboard.writeText(code)
    setCopied(true)
    setTimeout(() => setCopied(false), 1500)
  }

  return (
    <section className="relative border-b border-border">
      <div className="mx-auto w-full max-w-[1400px] px-4 sm:px-6 lg:px-8 py-20 md:py-24">
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-8 mb-12">
          <div className="lg:col-span-3">
            <span className="chapter-num">07</span>
            <p className="mt-2 font-mono text-[11px] uppercase tracking-[0.2em] text-muted-foreground">
              quickstart · dev
            </p>
          </div>
          <div className="lg:col-span-6 lg:col-start-5">
            <h2 className="font-sans text-3xl md:text-4xl lg:text-5xl tracking-[-0.02em] leading-[1.05] text-foreground">
              Drop in. <span className="italic font-light text-muted-foreground">Request entropy.</span>
            </h2>
            <p className="mt-5 font-mono text-sm text-muted-foreground max-w-md leading-relaxed">
              The SDK ships typed clients for Rust and TypeScript.
              One import, one method, one transaction.
            </p>
          </div>
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
          {/* Code panel */}
          <div className="lg:col-span-8 relative border border-border bg-muted/20">
            {/* Tab strip */}
            <div className="flex items-center justify-between border-b border-border font-mono text-[11px] uppercase tracking-wider">
              <div className="flex">
                {(["ts", "rust"] as const).map((l) => (
                  <button
                    key={l}
                    onClick={() => setLang(l)}
                    className={`px-4 py-2 border-r border-border transition-colors ${
                      lang === l
                        ? "bg-foreground text-background"
                        : "text-muted-foreground hover:text-foreground"
                    }`}
                  >
                    {l === "ts" ? "TypeScript" : "Rust · anchor"}
                  </button>
                ))}
              </div>
              <button
                onClick={handleCopy}
                className="px-4 py-2 text-muted-foreground hover:text-foreground border-l border-border transition-colors"
              >
                {copied ? "✓ copied" : "[ copy ]"}
              </button>
            </div>

            {/* Install row */}
            <div className="px-4 py-3 border-b border-dashed border-border font-mono text-[12px] text-muted-foreground flex items-center gap-3">
              <span className="text-foreground">$</span>
              <span>{installCmd}</span>
            </div>

            {/* Code */}
            <pre className="p-6 font-mono text-[13px] leading-relaxed text-foreground overflow-x-auto whitespace-pre">
              {code}
            </pre>
          </div>

          {/* Side panel */}
          <div className="lg:col-span-4 flex flex-col gap-4">
            <div className="border border-border p-6">
              <p className="font-pixel text-[11px] text-muted-foreground mb-3">
                RETURN SHAPE
              </p>
              <ul className="font-mono text-[12px] space-y-2 text-muted-foreground">
                <li className="flex justify-between"><span>round_id</span><span className="text-foreground">u64</span></li>
                <li className="flex justify-between"><span>value</span><span className="text-foreground">[u8; 32]</span></li>
                <li className="flex justify-between"><span>nodes</span><span className="text-foreground">Pubkey[6]</span></li>
                <li className="flex justify-between"><span>proof</span><span className="text-foreground">secp256k1</span></li>
                <li className="flex justify-between"><span>latency</span><span className="text-foreground">~3.9 s</span></li>
              </ul>
            </div>

            <div className="border border-border p-6">
              <p className="font-pixel text-[11px] text-muted-foreground mb-3">
                PRICING
              </p>
              <p className="font-pixel text-[40px] leading-none text-foreground mb-1">0.002</p>
              <p className="font-mono text-[11px] text-muted-foreground">SOL per request. No subscription. No token.</p>
            </div>

            <BracketLink href="/docs" variant="primary">Read_SDK_docs</BracketLink>
          </div>
        </div>
      </div>
    </section>
  )
}

export default DevQuickstart
