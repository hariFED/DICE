import type { AccountInfo } from "@solana/web3.js";

import { RANDOMNESS_FEED_DISCRIMINATOR } from "./constants.js";

/**
 * Decoded snapshot of a `RandomnessFeed` PDA. Mirrors the layout in
 * `programs/dice/src/state/randomness_feed.rs`.
 *
 * Offsets of the relevant fixed fields:
 *   disc(8) + authority(32) + coordinator(32) + bound_channel(32)
 *   + feed_index(2) + name(32) + publish_interval_slots(4)
 *   + status(1) = 143
 *   + current_randomness(32) = 175
 *   + current_sequence(u64 LE) = 183
 *   + current_slot(u64 LE) = 191
 *   + current_round_id(u64 LE) = 199
 */
export interface FeedSnapshot {
  /** 0 = Active, 1 = Closed. */
  status: number;
  /** 32-byte current randomness value. */
  randomness: Uint8Array;
  /** Monotonic publish counter. 0 = genesis (no value ever published). */
  sequence: bigint;
  /** Solana slot of the latest publish. */
  slot: bigint;
  /** DiceChannel round_id that produced the current value. */
  roundId: bigint;
}

/**
 * Decode a `RandomnessFeed` account buffer into its current snapshot.
 *
 * Accepts the raw account data (typically from
 * `Connection.getAccountInfo(feed_pda).data`). Returns `null` if the
 * buffer is too short or the discriminator doesn't match, so callers
 * can safely pass untrusted input without exceptions.
 *
 * Only the `current_*` fields are decoded — the history ring buffer and
 * previous-value fields are skipped since most subscribers only need
 * the most recent value.
 */
export function decodeFeedSnapshot(data: Uint8Array | Buffer): FeedSnapshot | null {
  const buf = Buffer.isBuffer(data) ? data : Buffer.from(data);

  const CURRENT_RANDOMNESS_OFFSET = 143;
  const CURRENT_SEQUENCE_OFFSET = 175;
  const CURRENT_SLOT_OFFSET = 183;
  const CURRENT_ROUND_ID_OFFSET = 191;
  const STATUS_OFFSET = 142;

  if (buf.byteLength < CURRENT_ROUND_ID_OFFSET + 8) {
    return null;
  }
  if (!buf.subarray(0, 8).equals(RANDOMNESS_FEED_DISCRIMINATOR)) {
    return null;
  }

  const status = buf.readUInt8(STATUS_OFFSET);
  const randomness = Uint8Array.from(
    buf.subarray(CURRENT_RANDOMNESS_OFFSET, CURRENT_RANDOMNESS_OFFSET + 32)
  );
  const sequence = buf.readBigUInt64LE(CURRENT_SEQUENCE_OFFSET);
  const slot = buf.readBigUInt64LE(CURRENT_SLOT_OFFSET);
  const roundId = buf.readBigUInt64LE(CURRENT_ROUND_ID_OFFSET);

  return { status, randomness, sequence, slot, roundId };
}

/**
 * Convenience wrapper: pass a whole `AccountInfo<Buffer>` and get the
 * snapshot (or `null` if the account is missing / not a feed).
 */
export function decodeFeedAccount(
  account: AccountInfo<Buffer> | null
): FeedSnapshot | null {
  if (!account) return null;
  return decodeFeedSnapshot(account.data);
}
