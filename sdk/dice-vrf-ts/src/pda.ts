import { PublicKey } from "@solana/web3.js";
import { createHash } from "node:crypto";

import {
  DICE_PROGRAM_ID,
  SEED_CHANNEL,
  SEED_FEED,
  SEED_NODE_VAULT,
} from "./constants.js";

/**
 * Derive the `DiceChannel` PDA for a given dApp authority + channel index.
 *
 * Seeds: `["channel", authority, channel_index (u16 LE)]`
 */
export function channelPda(
  authority: PublicKey,
  channelIndex: number,
  programId: PublicKey = DICE_PROGRAM_ID
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEED_CHANNEL, authority.toBuffer(), u16LE(channelIndex)],
    programId
  );
}

/**
 * Derive the `RandomnessFeed` PDA for a given authority + feed index.
 *
 * Seeds: `["feed", authority, feed_index (u16 LE)]`
 */
export function feedPda(
  authority: PublicKey,
  feedIndex: number,
  programId: PublicKey = DICE_PROGRAM_ID
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEED_FEED, authority.toBuffer(), u16LE(feedIndex)],
    programId
  );
}

/**
 * Derive the `NodeVault` PDA for a given device.
 *
 * Seeds: `["node_vault", device_id]` where
 * `device_id = SHA-256(device_pubkey_33_bytes)`.
 *
 * Solana seed slots are capped at 32 bytes — the device_id hash is what
 * lets us key the PDA by a 33-byte compressed secp256k1 pubkey.
 */
export function nodeVaultPda(
  devicePubkey33: Uint8Array,
  programId: PublicKey = DICE_PROGRAM_ID
): [PublicKey, number] {
  if (devicePubkey33.byteLength !== 33) {
    throw new Error(
      `device pubkey must be 33 bytes compressed secp256k1, got ${devicePubkey33.byteLength}`
    );
  }
  const deviceId = sha256(Buffer.from(devicePubkey33));
  return PublicKey.findProgramAddressSync([SEED_NODE_VAULT, deviceId], programId);
}

function u16LE(value: number): Buffer {
  if (value < 0 || value > 0xffff || !Number.isInteger(value)) {
    throw new Error(`channel/feed index must fit in u16, got ${value}`);
  }
  const buf = Buffer.alloc(2);
  buf.writeUInt16LE(value, 0);
  return buf;
}

function sha256(input: Buffer): Buffer {
  return createHash("sha256").update(input).digest();
}
