/**
 * DICE Devnet On-Chain Setup
 *
 * Registers the real ESP32-S3 device on Solana devnet and tests the full
 * on-chain VRF flow: register_device → request_randomness → verify.
 *
 * Usage: npx ts-node tests/devnet_setup.ts
 */

import {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
  sendAndConfirmTransaction,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import { createHash } from "crypto";
import * as fs from "fs";
import { BN } from "@coral-xyz/anchor";

// ── Config ──────────────────────────────────────────────────────────────────

const PROGRAM_ID = new PublicKey("78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv");
const RPC_URL = "https://api.devnet.solana.com";
const DEVICE_PUBKEY_HEX = "025e62666100d9ee1973a02032dbe41f3e5d7b3e54bb11e9ba9cc839b43c35a01d";

// PDA seeds (must match programs/dice/src/constants.rs)
const SEED_DEVICE  = Buffer.from("device");
const SEED_REQUEST = Buffer.from("request");
const SEED_ESCROW  = Buffer.from("escrow");
const SEED_RESULT  = Buffer.from("result");

// Protocol fee: 0.002 SOL
const PROTOCOL_FEE = 2_000_000;

// ── Helpers ─────────────────────────────────────────────────────────────────

function anchorDiscriminator(name: string): Buffer {
  return createHash("sha256").update(`global:${name}`).digest().subarray(0, 8);
}

function seqBuf(seq: BN): Buffer {
  return seq.toArrayLike(Buffer, "le", 8);
}

function loadKeypair(path: string): Keypair {
  const raw = JSON.parse(fs.readFileSync(path, "utf-8"));
  return Keypair.fromSecretKey(Uint8Array.from(raw));
}

// ── Main ────────────────────────────────────────────────────────────────────

async function main() {
  console.log("=".repeat(60));
  console.log("  DICE DEVNET ON-CHAIN SETUP");
  console.log("=".repeat(60));
  console.log();

  const connection = new Connection(RPC_URL, "confirmed");
  const coordinator = loadKeypair("coordinator-keypair.json");
  console.log(`Coordinator: ${coordinator.publicKey.toBase58()}`);

  const balance = await connection.getBalance(coordinator.publicKey);
  console.log(`Balance: ${balance / LAMPORTS_PER_SOL} SOL`);
  console.log(`Program: ${PROGRAM_ID.toBase58()}`);
  console.log();

  // ── Device identity ────────────────────────────────────────────────────

  const devicePubkeyBytes = Buffer.from(DEVICE_PUBKEY_HEX, "hex"); // 33 bytes
  const deviceId = createHash("sha256").update(devicePubkeyBytes).digest(); // 32 bytes
  console.log(`Device pubkey: ${DEVICE_PUBKEY_HEX}`);
  console.log(`Device ID (SHA-256): ${deviceId.toString("hex")}`);

  // ── PDA derivation ─────────────────────────────────────────────────────

  const [deviceRegistryPda, deviceBump] = PublicKey.findProgramAddressSync(
    [SEED_DEVICE, deviceId],
    PROGRAM_ID
  );
  console.log(`DeviceRegistry PDA: ${deviceRegistryPda.toBase58()}`);
  console.log();

  // ═══════════════════════════════════════════════════════════════════════
  // STEP 1: Register Device
  // ═══════════════════════════════════════════════════════════════════════

  console.log("=".repeat(60));
  console.log("  STEP 1: register_device");
  console.log("=".repeat(60));

  // Check if already registered
  const existingAccount = await connection.getAccountInfo(deviceRegistryPda);
  if (existingAccount) {
    console.log("Device already registered! Skipping.");
    console.log(`  Account size: ${existingAccount.data.length} bytes`);
  } else {
    console.log("Registering device on-chain...");

    // Build instruction data: discriminator + device_id(32) + device_pubkey(33)
    const disc = anchorDiscriminator("register_device");
    const data = Buffer.concat([
      disc,                              // 8 bytes
      deviceId,                          // 32 bytes
      devicePubkeyBytes,                 // 33 bytes (note: Array<u8> in Anchor = 4-byte length prefix + data)
    ]);

    // Actually, Anchor serializes [u8; 32] as raw 32 bytes (fixed array, no length prefix)
    // and [u8; 33] as raw 33 bytes

    const ix = new TransactionInstruction({
      programId: PROGRAM_ID,
      keys: [
        { pubkey: coordinator.publicKey, isSigner: true, isWritable: true },    // authority (payer)
        { pubkey: deviceRegistryPda, isSigner: false, isWritable: true },        // device_registry
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false }, // system_program
      ],
      data,
    });

    try {
      const tx = new Transaction().add(ix);
      const sig = await sendAndConfirmTransaction(connection, tx, [coordinator], {
        commitment: "confirmed",
      });
      console.log(`  TX: ${sig}`);
      console.log(`  Explorer: https://explorer.solana.com/tx/${sig}?cluster=devnet`);
      console.log("  DEVICE REGISTERED ON-CHAIN! ✓");
    } catch (e: any) {
      console.error(`  Error: ${e.message?.substring(0, 300)}`);
      if (e.message?.includes("already in use")) {
        console.log("  (Account already exists — device was previously registered)");
      }
    }
  }

  console.log();

  // ═══════════════════════════════════════════════════════════════════════
  // STEP 2: Request Randomness (creates escrow + request PDAs)
  // ═══════════════════════════════════════════════════════════════════════

  console.log("=".repeat(60));
  console.log("  STEP 2: request_randomness");
  console.log("=".repeat(60));

  const sequence = new BN(Date.now()); // unique sequence

  const [requestPda] = PublicKey.findProgramAddressSync(
    [SEED_REQUEST, coordinator.publicKey.toBuffer(), seqBuf(sequence)],
    PROGRAM_ID
  );
  const [escrowPda] = PublicKey.findProgramAddressSync(
    [SEED_ESCROW, coordinator.publicKey.toBuffer(), seqBuf(sequence)],
    PROGRAM_ID
  );

  console.log(`Sequence: ${sequence.toString()}`);
  console.log(`Request PDA: ${requestPda.toBase58()}`);
  console.log(`Escrow PDA: ${escrowPda.toBase58()}`);

  // Build instruction: discriminator + sequence(u64) + callback_program_id(Pubkey)
  const noCallback = new PublicKey(new Uint8Array(32)); // Pubkey::default() = poll mode
  const reqDisc = anchorDiscriminator("request_randomness");
  const reqData = Buffer.concat([
    reqDisc,                                    // 8 bytes
    seqBuf(sequence),                           // 8 bytes (u64 LE)
    noCallback.toBuffer(),                      // 32 bytes
  ]);

  const reqIx = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: coordinator.publicKey, isSigner: true, isWritable: true },    // requester (payer)
      { pubkey: requestPda, isSigner: false, isWritable: true },              // randomness_request
      { pubkey: escrowPda, isSigner: false, isWritable: true },               // escrow
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false }, // system_program
    ],
    data: reqData,
  });

  try {
    const tx = new Transaction().add(reqIx);
    const sig = await sendAndConfirmTransaction(connection, tx, [coordinator], {
      commitment: "confirmed",
    });
    console.log(`  TX: ${sig}`);
    console.log(`  Explorer: https://explorer.solana.com/tx/${sig}?cluster=devnet`);
    console.log("  REQUEST CREATED ON-CHAIN! ✓");
    console.log(`  Escrow: 0.002 SOL locked`);

    // Verify the escrow has funds
    const escrowBalance = await connection.getBalance(escrowPda);
    console.log(`  Escrow balance: ${escrowBalance / LAMPORTS_PER_SOL} SOL`);
  } catch (e: any) {
    console.error(`  Error: ${e.message?.substring(0, 500)}`);
    console.log();
    console.log("  If this fails, check:");
    console.log("  1. Is the program deployed and matches the latest code?");
    console.log("  2. Does the coordinator have enough SOL?");
    console.log("  3. Are the account sizes correct?");
  }

  console.log();

  // ═══════════════════════════════════════════════════════════════════════
  // STEP 3: Verify state on-chain
  // ═══════════════════════════════════════════════════════════════════════

  console.log("=".repeat(60));
  console.log("  STEP 3: Verify on-chain state");
  console.log("=".repeat(60));

  const reqAccount = await connection.getAccountInfo(requestPda);
  if (reqAccount) {
    console.log(`  RandomnessRequest account: ${reqAccount.data.length} bytes ✓`);
    console.log(`  Owner: ${reqAccount.owner.toBase58()}`);
  } else {
    console.log("  RandomnessRequest account: NOT FOUND ✗");
  }

  const escAccount = await connection.getAccountInfo(escrowPda);
  if (escAccount) {
    console.log(`  Escrow account: ${escAccount.data.length} bytes, ${escAccount.lamports / LAMPORTS_PER_SOL} SOL ✓`);
  } else {
    console.log("  Escrow account: NOT FOUND ✗");
  }

  const devAccount = await connection.getAccountInfo(deviceRegistryPda);
  if (devAccount) {
    console.log(`  DeviceRegistry account: ${devAccount.data.length} bytes ✓`);
  } else {
    console.log("  DeviceRegistry account: NOT FOUND ✗");
  }

  console.log();
  console.log("=".repeat(60));
  console.log("  DONE");
  console.log("=".repeat(60));
  console.log();
  console.log("Next: Start the coordinator and trigger a VRF round.");
  console.log("The coordinator should now be able to submit_commit,");
  console.log("submit_reveal, and finalize_randomness ON-CHAIN.");

  const finalBalance = await connection.getBalance(coordinator.publicKey);
  console.log(`\nFinal coordinator balance: ${finalBalance / LAMPORTS_PER_SOL} SOL`);
}

main().catch(console.error);
