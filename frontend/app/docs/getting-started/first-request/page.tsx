import Link from "next/link"
import type { Metadata } from "next"
import { DocsBreadcrumbs } from "@/components/docs/Breadcrumbs"
import { H1, H2, H3, Lead, P } from "@/components/docs/Prose"
import { CodeBlock } from "@/components/docs/CodeBlock"
import { Callout } from "@/components/docs/Callout"

export const metadata: Metadata = {
  title: "Your first request",
  description: "Hands-on walk-through: install the SDK, configure a wallet, request one random value, and log the result.",
}

export default function FirstRequestPage() {
  return (
    <div className="max-w-[880px]">
      <DocsBreadcrumbs href="/docs/getting-started/first-request" />
      <span className="inline-block font-mono text-[10px] uppercase tracking-[0.2em] text-muted-foreground border border-border rounded-sm px-2 py-1 mb-4">
        beginner · ~10 min · hands-on
      </span>
      <H1>Your first request, end to end</H1>
      <Lead>
        By the time you finish this page, you&apos;ll have called the DICE
        devnet and printed a real 32-byte random value in your terminal.
      </Lead>

      <div className="mt-6 not-prose grid grid-cols-4 gap-0 border border-border">
        {[
          { n: "01", label: "install",  done: false },
          { n: "02", label: "configure", done: false },
          { n: "03", label: "request",  done: false },
          { n: "04", label: "verify",   done: false },
        ].map((s, i) => (
          <div key={s.n} className="p-4 border-r last:border-r-0 border-border">
            <p className="font-pixel text-xs text-muted-foreground">STEP {s.n}</p>
            <p className="font-pixel text-sm text-foreground mt-0.5">{s.label}</p>
          </div>
        ))}
      </div>

      <H2 id="install">01 · Install the SDK</H2>
      <P>
        The SDK is published on npm. If you&apos;re starting fresh, run
        these two commands in a new directory:
      </P>
      <CodeBlock lang="bash" code={`mkdir my-dice-app && cd my-dice-app
npm init -y
npm install @dicelabs/vrf @solana/web3.js bs58`} />

      <H2 id="configure">02 · Configure a devnet wallet</H2>
      <P>
        DICE is live on Solana devnet. You need a funded wallet — the
        devnet faucet drops SOL for free. Save this as{" "}
        <code className="font-mono text-foreground">config.ts</code>:
      </P>
      <CodeBlock lang="ts" code={`import { Keypair, Connection } from "@solana/web3.js";
import fs from "node:fs";

// Generate a keypair on first run, reuse on subsequent runs
const KEYFILE = ".my-wallet.json";
export const wallet = fs.existsSync(KEYFILE)
  ? Keypair.fromSecretKey(Uint8Array.from(JSON.parse(fs.readFileSync(KEYFILE, "utf8"))))
  : (() => {
      const kp = Keypair.generate();
      fs.writeFileSync(KEYFILE, JSON.stringify(Array.from(kp.secretKey)));
      return kp;
    })();

export const connection = new Connection("https://api.devnet.solana.com");

console.log("wallet:", wallet.publicKey.toBase58());`} />

      <Callout type="info">
        After running it once, visit{" "}
        <a href="https://faucet.solana.com" className="underline" target="_blank" rel="noreferrer">faucet.solana.com</a>{" "}
        and airdrop 1 SOL to the address printed above.
      </Callout>

      <H2 id="request">03 · Request one random value</H2>
      <P>
        Save as <code className="font-mono text-foreground">request.ts</code>
        {" "}and run with <code className="font-mono text-foreground">npx tsx request.ts</code>:
      </P>
      <CodeBlock lang="ts" code={`import { DiceClient } from "@dicelabs/vrf";
import { wallet, connection } from "./config.js";

const client = new DiceClient({ connection, cluster: "devnet" });

// Ask for one 32-byte random value. Blocks until the mesh responds
// (usually ~4 seconds).
const round = await client.requestRandomness({ payer: wallet });

console.log("round_id    :", round.id);
console.log("randomness  :", round.value.toString("hex"));
console.log("latency     :", round.latencyMs, "ms");
console.log("nodes       :", round.nodes.length, "/ 6");
console.log("tx          :", round.signature);`} />

      <H3 id="output">What you should see</H3>
      <CodeBlock lang="text" code={`round_id    : 0x4f3a9c1e...
randomness  : d1f7e2b4c9...8a3f (32 bytes / 64 hex chars)
latency     : 3842 ms
nodes       : 6 / 6
tx          : 3xS2y...qK7fQpN8YV3r (on-chain signature)`} />

      <H2 id="verify">04 · Verify on-chain</H2>
      <P>
        Copy the <code className="font-mono text-foreground">tx</code> signature
        from your output and paste it into{" "}
        <a href="https://explorer.solana.com/?cluster=devnet" target="_blank" rel="noreferrer" className="underline">Solana Explorer</a>.
        You&apos;ll see a transaction calling <code className="font-mono text-foreground">finalize_round</code> on
        the DICE program, with the 32-byte randomness value embedded in
        the instruction data.
      </P>

      <Callout type="tip">
        <strong>That&apos;s it.</strong> You just asked six hardware
        devices for an unbiased random value, got it back in 4 seconds,
        and can prove it came from the real DICE mesh. Your dApp is one
        CPI away from doing this too.
      </Callout>

      <H2 id="next">Where to go next</H2>
      <div className="mt-4 space-y-2">
        <Link
          href="/docs/quickstart"
          className="block border border-border p-4 hover:border-foreground transition-colors font-mono text-sm text-foreground"
        >
          → Integrate into an Anchor program (Quickstart)
        </Link>
        <Link
          href="/docs/advanced/verify"
          className="block border border-border p-4 hover:border-foreground transition-colors font-mono text-sm text-foreground"
        >
          → Verify a randomness value yourself (Advanced)
        </Link>
        <Link
          href="/docs/reference/payload"
          className="block border border-border p-4 hover:border-foreground transition-colors font-mono text-sm text-foreground"
        >
          → Full callback payload reference
        </Link>
      </div>
    </div>
  )
}
