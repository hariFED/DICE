# @dice-vrf/sdk

TypeScript SDK for DICE — hardware-backed VRF oracle on Solana. This is
the v2 channel path only; v1 is deprecated on chain (see the Rust crate
[`dice-vrf`](../dice-vrf/README.md) for the full v1 vs v2 breakdown).

## Install

```bash
npm install @dice-vrf/sdk @solana/web3.js
```

`@solana/web3.js` is a peer dependency — bring your own version.

## Minimal request flow

```ts
import {
  Connection,
  Keypair,
  SystemProgram,
  Transaction,
  PublicKey,
} from "@solana/web3.js";
import {
  DICE_PROGRAM_ID,
  channelPda,
  buildInitChannelIx,
  buildFundChannelIx,
  buildRequestRandomnessAutoIx,
  REQUEST_FEE_LAMPORTS,
} from "@dice-vrf/sdk";

const connection = new Connection("https://api.devnet.solana.com", "confirmed");
const authority = Keypair.generate(); // dApp authority
const coordinator = new PublicKey("...the configured DICE coordinator...");

const channelIndex = 0;
const [channel] = channelPda(authority.publicKey, channelIndex);

// 1) init the channel once
const init = buildInitChannelIx({
  authority: authority.publicKey,
  channelIndex,
  maxNodes: 7,
  callbackProgramId: PublicKey.default, // no CPI callback
  coordinator,
});

// 2) fund it (cover at least one round)
const fund = buildFundChannelIx({
  authority: authority.publicKey,
  channelIndex,
  amountLamports: REQUEST_FEE_LAMPORTS * 5n, // five rounds
});

// 3) request a fresh round
const request = buildRequestRandomnessAutoIx({
  authority: authority.publicKey,
  channelIndex,
  nodeCount: 4,
});

const tx = new Transaction().add(init, fund, request);
await connection.sendTransaction(tx, [authority]);
```

After the transaction lands, the coordinator takes over: it sees the
Pending channel, dispatches to live hardware nodes, runs commit-reveal,
and submits the on-chain `finalize_v2 + claim_rewards_v2` bundle.
Subscribers can poll the channel PDA (its `randomness` field holds the
32-byte result when `status == Finalized`) or, better, use the streaming
VRF pattern below.

## Streaming VRF (passive subscriber)

For a continuously-fresh randomness source — price feeds, loot drops,
draw ticks — subscribe to a `RandomnessFeed` PDA. The coordinator's feed
crank pushes new values every time the bound channel finalizes. dApps
read the feed as a readonly account and pay zero per-request fee.

```ts
import { Connection } from "@solana/web3.js";
import { feedPda, decodeFeedAccount } from "@dice-vrf/sdk";

const [feed] = feedPda(authority, 0);
const account = await connection.getAccountInfo(feed);
const snap = decodeFeedAccount(account);

if (!snap || snap.sequence === 0n) {
  console.log("feed not yet published");
} else {
  console.log(`seq=${snap.sequence} slot=${snap.slot} roundId=${snap.roundId}`);
  console.log("randomness:", Buffer.from(snap.randomness).toString("hex"));
}
```

To create the feed, call `buildInitFeedIx` against an existing channel.
See `programs/pulse` in the DICE repo for a full example dApp that
consumes a feed from on-chain.

## What's in the SDK

| Export                        | Purpose                                  |
| ----------------------------- | ---------------------------------------- |
| `DICE_PROGRAM_ID`             | canonical program `PublicKey`            |
| `REQUEST_FEE_LAMPORTS`        | 0.002 SOL per round, as a `bigint`       |
| `channelPda`                  | derive a `DiceChannel` PDA               |
| `feedPda`                     | derive a `RandomnessFeed` PDA            |
| `nodeVaultPda`                | derive a per-device `NodeVault` PDA      |
| `buildInitChannelIx`          | create a channel                         |
| `buildFundChannelIx`          | top up a channel's internal balance      |
| `buildRequestRandomnessAutoIx`| kick off a new round                     |
| `buildInitFeedIx`             | create a streaming randomness feed       |
| `decodeFeedAccount`           | decode a feed PDA snapshot               |
| `anchorDiscriminator`         | low-level helper for custom instructions |

## What's NOT here

- `submit_commit_v2` / `submit_reveal_v2` / `finalize_v2` /
  `claim_rewards_v2`: coordinator-only. A dApp frontend never needs
  them.
- `deliver_callback`: usually submitted by the coordinator after
  finalize. Build it yourself only if your dApp is driving the round.
- v1 `RandomnessRequest` path: deprecated on chain; `claim_rewards`
  returns `V1ClaimRewardsDeprecated`. Use the v2 channel path.
