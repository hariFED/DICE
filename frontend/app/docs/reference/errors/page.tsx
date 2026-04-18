import type { Metadata } from "next"
import { DocsPageShell } from "@/components/docs/PageShell"
import { H2, P, Code } from "@/components/docs/Prose"

export const metadata: Metadata = {
  title: "Errors",
  description:
    "Every DiceError variant from programs/dice/src/error.rs with a plain-English root cause.",
}

const TOC = [
  { id: "round-protocol", label: "Round protocol errors", level: 2 as const },
  { id: "callback-errors", label: "Callback errors", level: 2 as const },
  { id: "feed-errors", label: "Streaming feed errors", level: 2 as const },
  { id: "vault-errors", label: "Node vault errors", level: 2 as const },
  { id: "deprecated", label: "Deprecated", level: 2 as const },
]

type Row = { name: string; msg: string; cause: string }

const ROUND: Row[] = [
  {
    name: "InsufficientNodes",
    msg: "minimum 4 required",
    cause: "Fewer than 4 nodes online at request time. Retry or reduce node_count.",
  },
  {
    name: "InvalidNodeCount",
    msg: "must be between 4 and 50",
    cause: "You passed node_count < 4 or > 50. Clamp client-side.",
  },
  {
    name: "RoundTimedOut",
    msg: "Round has timed out",
    cause: "Nodes didn't reveal within the round window. Rare; coordinator retries automatically.",
  },
  {
    name: "RoundAlreadyFinalized",
    msg: "already been finalized",
    cause: "Double finalisation attempted. Usually a coordinator bug, not caller-visible.",
  },
  {
    name: "RoundNotComplete",
    msg: "not yet complete",
    cause: "Attempted to finalise before all reveals are in. Transient.",
  },
  {
    name: "EscrowInsufficient",
    msg: "Escrow has insufficient funds",
    cause: "Channel is out of SOL. Call fund_channel.",
  },
  {
    name: "UnauthorizedCoordinator",
    msg: "not the authorized coordinator for this channel",
    cause: "The TX signer doesn't match the channel's bound coordinator pubkey.",
  },
]

const SIG: Row[] = [
  {
    name: "AlreadyCommitted",
    msg: "Node has already committed for this round",
    cause: "Duplicate submit_round for the same device + round.",
  },
  {
    name: "AlreadyRevealed",
    msg: "Node has already revealed for this round",
    cause: "Duplicate submit_reveal for the same device + round.",
  },
  {
    name: "RevealMismatch",
    msg: "hash(entropy) does not match commit",
    cause: "A node's reveal doesn't hash to its earlier commit. Hardware fault or malice.",
  },
  {
    name: "InvalidSignature",
    msg: "Invalid ECDSA signature",
    cause: "The reveal signature didn't verify against the device's registered pubkey.",
  },
  {
    name: "UnauthorizedNode",
    msg: "not authorized for this round",
    cause: "A device tried to submit for a round it wasn't selected in.",
  },
  {
    name: "InvalidDeviceId",
    msg: "device_id does not match SHA-256(device_pubkey)",
    cause: "Provisioning integrity check failed. The device is misregistered.",
  },
]

const CB: Row[] = [
  {
    name: "CallbackProgramMissing",
    msg: "Callback program missing from remaining accounts",
    cause: "Coordinator didn't include the callback program in the finalisation TX. Ops issue.",
  },
  {
    name: "CallbackProgramMismatch",
    msg: "Callback program ID does not match request",
    cause: "A different program was supplied than the channel's configured callback. Usually a client bug.",
  },
  {
    name: "CallbackFailed",
    msg: "CPI callback to developer program failed",
    cause: "Your dice_callback reverted. See Callback handler → replay.",
  },
]

const FEED: Row[] = [
  {
    name: "FeedPublishTooSoon",
    msg: "before the configured interval has elapsed",
    cause: "Feed crank ran faster than publish_interval_slots. Retry next interval.",
  },
  {
    name: "FeedWrongCoordinator",
    msg: "not the coordinator bound to this feed",
    cause: "The signer doesn't match the feed's coordinator pubkey.",
  },
  {
    name: "FeedInvalidInterval",
    msg: "between MIN and MAX_PUBLISH_INTERVAL_SLOTS",
    cause: "Interval config out of bounds. Choose a value in the allowed range.",
  },
  {
    name: "FeedNotActive",
    msg: "closed or paused",
    cause: "Feed has been paused by its authority.",
  },
  {
    name: "FeedChannelMismatch",
    msg: "does not match the feed's configured bound_channel",
    cause: "The channel passed to publish doesn't match the feed's binding.",
  },
  {
    name: "FeedChannelNotFinalized",
    msg: "not in Finalized status",
    cause: "Attempted to publish from a still-running round.",
  },
  {
    name: "FeedRoundIdMismatch",
    msg: "round_id does not match",
    cause: "The round_id argument disagrees with the channel's last finalised round.",
  },
  {
    name: "FeedRandomnessMismatch",
    msg: "randomness does not match",
    cause: "The randomness argument disagrees with the channel's stored result.",
  },
]

const VAULT: Row[] = [
  {
    name: "VaultAlreadyBound",
    msg: "already bound to a payout wallet",
    cause: "bind_payout_wallet called twice. Use rotate_payout_wallet instead.",
  },
  {
    name: "VaultNotBound",
    msg: "no payout wallet bound yet",
    cause: "Withdraw attempted before binding. Call bind_payout_wallet first.",
  },
  {
    name: "VaultWrongPayoutWallet",
    msg: "not the vault's current payout wallet",
    cause: "Signer doesn't match the vault's bound wallet.",
  },
  {
    name: "VaultInsufficientBalance",
    msg: "exceeds vault credited balance",
    cause: "Withdraw amount > credited. Reduce the amount.",
  },
  {
    name: "VaultZeroAmount",
    msg: "must be greater than zero",
    cause: "Withdraw of 0 lamports.",
  },
  {
    name: "VaultRotationCooldown",
    msg: "before cooldown elapsed",
    cause: "Tried to rotate payout wallet before the cooldown window ended.",
  },
  {
    name: "VaultBindingSignatureInvalid",
    msg: "failed secp256k1 verification",
    cause: "Binding message signature didn't verify. Check firmware key.",
  },
  {
    name: "VaultCreditOverflow",
    msg: "would overflow vault totals",
    cause: "Theoretical u64 overflow on credit. Should not happen in practice.",
  },
  {
    name: "VaultUnknownServiceId",
    msg: "Unknown service ID",
    cause: "Credit path passed an unrecognised service tag.",
  },
  {
    name: "VaultFrozen",
    msg: "Vault is frozen",
    cause: "Vault has been frozen by governance. Contact DICE ops.",
  },
]

const DEP: Row[] = [
  {
    name: "V1ClaimRewardsDeprecated",
    msg: "use claim_rewards_v2 (v2 channel path)",
    cause: "The v1 claim path had unfixable per-node payout semantics. Migrate to v2.",
  },
]

function ErrorTable({ rows }: { rows: Row[] }) {
  return (
    <div className="my-4 overflow-x-auto rounded-xl border border-white/[0.08]">
      <table className="w-full text-left text-[13.5px]">
        <thead className="bg-white/[0.03] text-zinc-300">
          <tr>
            <th className="px-3 py-2 font-medium">Variant</th>
            <th className="px-3 py-2 font-medium">Message</th>
            <th className="px-3 py-2 font-medium">Root cause</th>
          </tr>
        </thead>
        <tbody className="text-zinc-400">
          {rows.map((r) => (
            <tr key={r.name} className="border-t border-white/[0.04]">
              <td className="px-3 py-2 font-mono text-[#7cf9c1] align-top">
                {r.name}
              </td>
              <td className="px-3 py-2 align-top">{r.msg}</td>
              <td className="px-3 py-2 align-top">{r.cause}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

export default function ErrorsPage() {
  return (
    <DocsPageShell
      href="/docs/reference/errors"
      title="Errors"
      lead="The full DiceError enum, grouped by subsystem. If you see one of these in an Anchor log, find it here."
      toc={TOC}
    >
      <P>
        All variants live in <Code>programs/dice/src/error.rs</Code>. Anchor
        returns them as <Code>Custom(code)</Code> in TX logs — the variant
        name and message are what you see in the explorer.
      </P>

      <H2 id="round-protocol">Round protocol errors</H2>
      <P>
        The errors your client code is most likely to encounter when
        driving rounds.
      </P>
      <ErrorTable rows={ROUND} />
      <ErrorTable rows={SIG} />

      <H2 id="callback-errors">Callback errors</H2>
      <P>
        These surface when DICE tries to CPI into your program.
      </P>
      <ErrorTable rows={CB} />

      <H2 id="feed-errors">Streaming feed errors</H2>
      <P>
        Relevant only if you're consuming the streaming VRF feed PDA. See{" "}
        <a
          href="/docs/advanced/streaming"
          className="text-[#00FF85] underline decoration-[#00FF85]/30 underline-offset-4"
        >
          Streaming feed
        </a>
        .
      </P>
      <ErrorTable rows={FEED} />

      <H2 id="vault-errors">Node vault errors</H2>
      <P>
        You'll see these only if you operate a node. Integrators don't call
        vault instructions.
      </P>
      <ErrorTable rows={VAULT} />

      <H2 id="deprecated">Deprecated</H2>
      <ErrorTable rows={DEP} />
    </DocsPageShell>
  )
}
