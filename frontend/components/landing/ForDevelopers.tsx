"use client"

import { motion } from "framer-motion"
import { CornerBox } from "@/components/shared/CornerBox"
import { SectionHeader } from "@/components/landing/HowItWorks"

const COMPARISON = [
  ["Integration",     "2 lines",       "~50 lines",      "~100 lines"],
  ["Cost",            "0.002 SOL",     "Variable + tok", "LINK token"],
  ["Latency",         "< 4s",          "2 – 5s",         "10 – 30s"],
  ["Token Required",  "No",            "SBV2",           "LINK"],
  ["Hardware Backed", "Yes",           "No",             "No"],
] as const

const CODE_LINES = [
  "use dice_vrf::cpi::request_randomness;",
  "",
  "pub fn roll_dice(ctx: Context<RollDice>) -> Result<()> {",
  "    request_randomness(ctx.accounts.dice_ctx(), seq, &crate::ID)?;",
  "    Ok(())",
  "}",
  "",
  "// callback — DICE invokes your program when entropy lands",
  "pub fn dice_callback(",
  "    ctx: Context<Callback>,",
  "    randomness: [u8; 32],",
  ") -> Result<()> {",
  "    let roll = (randomness[0] % 6) + 1;",
  "    msg!(\"You rolled: {}\", roll);",
  "    Ok(())",
  "}",
] as const

export function ForDevelopers() {
  return (
    <section className="relative py-28 border-b border-border">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <SectionHeader
          index="02"
          name="developers"
          title="Two instructions. Full verifiability."
          lead="DICE ships as an Anchor crate. Plug it into your program with one CPI call, handle the callback when randomness lands. No oracles to register, no token balances to top up."
        />

        <div className="mt-12 grid grid-cols-1 lg:grid-cols-12 gap-8 items-start">
          {/* Code panel */}
          <motion.div
            initial={{ opacity: 0, y: 12 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, margin: "-60px" }}
            transition={{ duration: 0.5 }}
            className="lg:col-span-7"
          >
            <CornerBox title="programs/your_dapp/src/lib.rs" tag="rust · cpi" innerClassName="p-0">
              <pre className="overflow-x-auto p-4 text-xs leading-[1.65] font-mono text-foreground">
                {CODE_LINES.map((line, i) => (
                  <div key={i} className="flex">
                    <span className="select-none text-muted-foreground/40 w-8 shrink-0 text-right pr-3 tabular-nums">
                      {String(i + 1).padStart(2, "0")}
                    </span>
                    <span className="whitespace-pre">
                      {line.startsWith("//") ? (
                        <span className="text-muted-foreground italic">{line}</span>
                      ) : (
                        line
                      )}
                    </span>
                  </div>
                ))}
              </pre>
            </CornerBox>
          </motion.div>

          {/* Comparison panel */}
          <motion.div
            initial={{ opacity: 0, y: 12 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, margin: "-60px" }}
            transition={{ duration: 0.5, delay: 0.1 }}
            className="lg:col-span-5"
          >
            <CornerBox title="vs · existing · oracles" innerClassName="p-0">
              <div className="overflow-x-auto">
                <table className="w-full text-xs font-mono">
                  <thead>
                    <tr className="border-b border-border">
                      <th className="text-left px-3 py-2 ascii-label text-[10px]"></th>
                      <th className="text-left px-3 py-2 text-foreground font-semibold">DICE</th>
                      <th className="text-left px-3 py-2 ascii-label text-[10px]">Switch</th>
                      <th className="text-left px-3 py-2 ascii-label text-[10px]">Chainlink</th>
                    </tr>
                  </thead>
                  <tbody>
                    {COMPARISON.map(([feat, dice, sb, link]) => (
                      <tr key={feat} className="border-b border-border last:border-0">
                        <td className="px-3 py-2 text-muted-foreground">{feat}</td>
                        <td className="px-3 py-2 text-foreground">{dice}</td>
                        <td className="px-3 py-2 text-muted-foreground">{sb}</td>
                        <td className="px-3 py-2 text-muted-foreground">{link}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </CornerBox>
          </motion.div>
        </div>
      </div>
    </section>
  )
}

export default ForDevelopers
