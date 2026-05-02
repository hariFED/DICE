import {
  PublicKey,
  SystemProgram,
  TransactionInstruction,
} from "@solana/web3.js";
import { createHash } from "node:crypto";

import { DICE_PROGRAM_ID } from "./constants.js";
import { channelPda, feedPda } from "./pda.js";

/**
 * Anchor discriminator for a given instruction name.
 * Equivalent to `SHA-256("global:<name>")[0..8]`.
 */
export function anchorDiscriminator(name: string): Buffer {
  return createHash("sha256").update(`global:${name}`).digest().subarray(0, 8);
}

/**
 * Create a fresh `DiceChannel` bound to this dApp. Must be called once per
 * `(authority, channelIndex)` pair before any round.
 *
 * `callbackProgramId` can be `PublicKey.default` for a no-callback channel
 * (the passive streaming VRF pattern). `coordinator` is the pubkey the
 * DICE coordinator is running under — bound permanently at init time.
 */
export function buildInitChannelIx(params: {
  authority: PublicKey;
  channelIndex: number;
  maxNodes: number;
  callbackProgramId: PublicKey;
  coordinator: PublicKey;
  programId?: PublicKey;
}): TransactionInstruction {
  const programId = params.programId ?? DICE_PROGRAM_ID;
  const [channel] = channelPda(params.authority, params.channelIndex, programId);

  const data = Buffer.concat([
    anchorDiscriminator("init_channel"),
    u16LE(params.channelIndex),
    Buffer.from([params.maxNodes]),
    params.callbackProgramId.toBuffer(),
    params.coordinator.toBuffer(),
  ]);

  return new TransactionInstruction({
    programId,
    keys: [
      { pubkey: params.authority, isSigner: true, isWritable: true },
      { pubkey: channel, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data,
  });
}

/**
 * Top up the channel's internal balance. Each `request_randomness_auto`
 * debits 0.002 SOL (see `REQUEST_FEE_LAMPORTS`), so pre-fund accordingly.
 */
export function buildFundChannelIx(params: {
  authority: PublicKey;
  channelIndex: number;
  amountLamports: bigint;
  programId?: PublicKey;
}): TransactionInstruction {
  const programId = params.programId ?? DICE_PROGRAM_ID;
  const [channel] = channelPda(params.authority, params.channelIndex, programId);

  const data = Buffer.concat([
    anchorDiscriminator("fund_channel"),
    u64LE(params.amountLamports),
  ]);

  return new TransactionInstruction({
    programId,
    keys: [
      { pubkey: params.authority, isSigner: true, isWritable: true },
      { pubkey: channel, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data,
  });
}

/**
 * Kick off a new commit-reveal round on the channel. The coordinator
 * detects this on-chain event, dispatches to live devices, runs the
 * protocol, and submits the on-chain finalize + claim bundle.
 *
 * `nodeCount` must satisfy the channel's min/max (typically 4..=max_nodes).
 */
export function buildRequestRandomnessAutoIx(params: {
  authority: PublicKey;
  channelIndex: number;
  nodeCount: number;
  programId?: PublicKey;
}): TransactionInstruction {
  const programId = params.programId ?? DICE_PROGRAM_ID;
  const [channel] = channelPda(params.authority, params.channelIndex, programId);

  const data = Buffer.concat([
    anchorDiscriminator("request_randomness_auto"),
    Buffer.from([params.nodeCount]),
  ]);

  return new TransactionInstruction({
    programId,
    keys: [
      { pubkey: params.authority, isSigner: true, isWritable: true },
      { pubkey: channel, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data,
  });
}

/**
 * Create a streaming `RandomnessFeed` bound to an existing channel.
 * `publishIntervalSlots` is the target cadence between publishes
 * (clamped on-chain to MIN/MAX by the program).
 *
 * `name` is zero-padded to 32 bytes internally.
 */
export function buildInitFeedIx(params: {
  authority: PublicKey;
  channelIndex: number;
  feedIndex: number;
  publishIntervalSlots: number;
  name: string;
  programId?: PublicKey;
}): TransactionInstruction {
  const programId = params.programId ?? DICE_PROGRAM_ID;
  const [channel] = channelPda(params.authority, params.channelIndex, programId);
  const [feed] = feedPda(params.authority, params.feedIndex, programId);

  const nameBuf = Buffer.alloc(32);
  Buffer.from(params.name, "utf8").copy(nameBuf, 0, 0, Math.min(32, params.name.length));

  const data = Buffer.concat([
    anchorDiscriminator("init_feed"),
    u16LE(params.feedIndex),
    u32LE(params.publishIntervalSlots),
    nameBuf,
  ]);

  return new TransactionInstruction({
    programId,
    keys: [
      { pubkey: params.authority, isSigner: true, isWritable: true },
      { pubkey: channel, isSigner: false, isWritable: false },
      { pubkey: feed, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data,
  });
}

// ── LE helpers ──────────────────────────────────────────────────────────

function u16LE(value: number): Buffer {
  const buf = Buffer.alloc(2);
  buf.writeUInt16LE(value, 0);
  return buf;
}

function u32LE(value: number): Buffer {
  const buf = Buffer.alloc(4);
  buf.writeUInt32LE(value, 0);
  return buf;
}

function u64LE(value: bigint): Buffer {
  const buf = Buffer.alloc(8);
  buf.writeBigUInt64LE(value, 0);
  return buf;
}
