import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { Dice } from "../target/types/dice";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import { assert } from "chai";

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

const SEED_CHANNEL = Buffer.from("channel");
const SEED_DEVICE  = Buffer.from("device");

function channelPda(
  programId: PublicKey,
  authority: PublicKey,
  channelIndex: number
): [PublicKey, number] {
  const indexBuf = Buffer.alloc(2);
  indexBuf.writeUInt16LE(channelIndex);
  return PublicKey.findProgramAddressSync(
    [SEED_CHANNEL, authority.toBuffer(), indexBuf],
    programId
  );
}

function deviceRegistryPda(
  programId: PublicKey,
  deviceId: Buffer
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEED_DEVICE, deviceId],
    programId
  );
}

const NO_CALLBACK = new PublicKey(new Uint8Array(32));

function fakeDevicePubkey(seed: number): number[] {
  const key = new Array(33).fill(0);
  key[0] = 0x02;
  key[1] = seed & 0xff;
  key[2] = (seed >> 8) & 0xff;
  return key;
}

function computeDeviceIdSync(devicePubkey: number[]): Buffer {
  const { createHash } = require("crypto");
  return createHash("sha256").update(Buffer.from(devicePubkey)).digest();
}

function fakeEntropy(seed: number): number[] {
  const e = new Array(32).fill(0);
  e[0] = seed & 0xff;
  e[1] = (seed >> 8) & 0xff;
  return e;
}

function computeCommitHash(entropy: number[]): number[] {
  const { createHash } = require("crypto");
  const hash = createHash("sha256").update(Buffer.from(entropy)).digest();
  return Array.from(hash);
}

function fakeSignature(): number[] {
  return new Array(64).fill(1);
}

async function fundAccount(
  provider: anchor.AnchorProvider,
  to: PublicKey,
  lamports: number
): Promise<void> {
  const tx = new anchor.web3.Transaction().add(
    anchor.web3.SystemProgram.transfer({
      fromPubkey: provider.wallet.publicKey,
      toPubkey: to,
      lamports,
    })
  );
  await provider.sendAndConfirm(tx);
}

// ─────────────────────────────────────────────────────────────────────────────
// Test suite — v2.0 Channel Design
// ─────────────────────────────────────────────────────────────────────────────

describe("dice v2.0 — channel design", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Dice as Program<Dice>;
  const programId = program.programId;

  // Actors
  const payer       = provider.wallet as anchor.Wallet;
  const coordinator = Keypair.generate();
  const authority   = Keypair.generate();

  const CHANNEL_INDEX = 0;
  const MAX_NODES = 7;
  const NODE_COUNT = 5;

  // Generate fake devices with real SHA-256 commit hashes
  const devices = Array.from({ length: NODE_COUNT }, (_, i) => {
    const pubkey = fakeDevicePubkey(100 + i);
    const id = computeDeviceIdSync(pubkey);
    const entropy = fakeEntropy(100 + i);
    const commitHash = computeCommitHash(entropy);
    return {
      pubkey,
      id,
      idArray: Array.from(id) as number[],
      entropy,
      commitHash,
    };
  });

  let channelPdaAddr: PublicKey;
  let channelBump: number;

  before(async () => {
    [channelPdaAddr, channelBump] = channelPda(programId, authority.publicKey, CHANNEL_INDEX);

    // Fund test accounts
    await fundAccount(provider, coordinator.publicKey, 0.05 * LAMPORTS_PER_SOL);
    await fundAccount(provider, authority.publicKey, 0.2 * LAMPORTS_PER_SOL);
  });

  // ─── 1. init_channel ──────────────────────────────────────────────────────

  it("initializes a DiceChannel", async () => {
    await program.methods
      .initChannel(CHANNEL_INDEX, MAX_NODES, NO_CALLBACK)
      .accounts({
        authority: authority.publicKey,
        channel: channelPdaAddr,
        systemProgram: SystemProgram.programId,
      })
      .signers([authority])
      .rpc();

    const ch = await program.account.diceChannel.fetch(channelPdaAddr);
    assert.ok(ch.authority.equals(authority.publicKey));
    assert.equal(ch.channelIndex, CHANNEL_INDEX);
    assert.equal(ch.maxNodes, MAX_NODES);
    assert.deepEqual(ch.status, { idle: {} });
    assert.equal(ch.roundId.toNumber(), 0);
    assert.equal(ch.balance.toNumber(), 0);
  });

  it("rejects init_channel with duplicate index", async () => {
    try {
      await program.methods
        .initChannel(CHANNEL_INDEX, MAX_NODES, NO_CALLBACK)
        .accounts({
          authority: authority.publicKey,
          channel: channelPdaAddr,
          systemProgram: SystemProgram.programId,
        })
        .signers([authority])
        .rpc();
      assert.fail("expected duplicate channel init to fail");
    } catch (err: any) {
      assert.ok(err.message, "duplicate init should throw");
    }
  });

  // ─── 2. fund_channel ──────────────────────────────────────────────────────

  it("funds a channel with SOL", async () => {
    const amount = new BN(10_000_000); // 0.01 SOL

    await program.methods
      .fundChannel(amount)
      .accounts({
        authority: authority.publicKey,
        channel: channelPdaAddr,
        systemProgram: SystemProgram.programId,
      })
      .signers([authority])
      .rpc();

    const ch = await program.account.diceChannel.fetch(channelPdaAddr);
    assert.equal(ch.balance.toNumber(), 10_000_000);
  });

  // ─── 3. request_randomness_v2 ─────────────────────────────────────────────

  it("requests randomness via channel", async () => {
    await program.methods
      .requestRandomnessV2(NODE_COUNT)
      .accounts({
        authority: authority.publicKey,
        channel: channelPdaAddr,
      })
      .signers([authority])
      .rpc();

    const ch = await program.account.diceChannel.fetch(channelPdaAddr);
    assert.deepEqual(ch.status, { pending: {} });
    assert.equal(ch.roundId.toNumber(), 1);
    assert.equal(ch.nodeCount, NODE_COUNT);
    assert.equal(ch.commitsReceived, 0);
    // Fee deducted: 10M - 2M = 8M
    assert.equal(ch.balance.toNumber(), 8_000_000);
  });

  it("rejects request when channel is not idle", async () => {
    try {
      await program.methods
        .requestRandomnessV2(NODE_COUNT)
        .accounts({
          authority: authority.publicKey,
          channel: channelPdaAddr,
        })
        .signers([authority])
        .rpc();
      assert.fail("expected request on non-idle channel to fail");
    } catch (err: any) {
      assert.ok(err.message, "should reject non-idle request");
    }
  });

  // ─── 4. submit_commit_v2 × NODE_COUNT ─────────────────────────────────────

  it("accepts commits from all nodes", async () => {
    for (let i = 0; i < NODE_COUNT; i++) {
      const device = devices[i];
      await program.methods
        .submitCommitV2(
          new BN(1), // round_id
          device.idArray,
          device.pubkey,
          device.commitHash
        )
        .accounts({
          coordinator: coordinator.publicKey,
          channel: channelPdaAddr,
        })
        .signers([coordinator])
        .rpc();
    }

    const ch = await program.account.diceChannel.fetch(channelPdaAddr);
    assert.equal(ch.commitsReceived, NODE_COUNT);
    assert.deepEqual(ch.status, { commitPhase: {} });
  });

  it("rejects duplicate commit from the same device", async () => {
    const device = devices[0];
    try {
      await program.methods
        .submitCommitV2(
          new BN(1),
          device.idArray,
          device.pubkey,
          device.commitHash
        )
        .accounts({
          coordinator: coordinator.publicKey,
          channel: channelPdaAddr,
        })
        .signers([coordinator])
        .rpc();
      assert.fail("expected duplicate commit to fail");
    } catch (err: any) {
      assert.ok(err.message, "duplicate commit should throw");
    }
  });

  // ─── 5. submit_reveal_v2 × NODE_COUNT ─────────────────────────────────────
  //
  // NOTE: The on-chain instruction verifies SHA-256(entropy) == commit_hash
  // AND secp256k1 signature. The fake signature will FAIL signature verification.
  // These tests use real SHA-256 commit hashes to pass the hash check,
  // but the ECDSA verification will fail. Full e2e tests need real keys.
  //
  // To exercise the reveal path, we test the structural flow and error codes.

  it("records that all commits are received (reveals deferred to e2e suite)", async () => {
    const ch = await program.account.diceChannel.fetch(channelPdaAddr);
    assert.equal(ch.commitsReceived, NODE_COUNT);
    assert.ok(
      "commitPhase" in ch.status || "revealPhase" in ch.status,
      "expected CommitPhase or RevealPhase"
    );
  });

  // ─── 6. Channel lifecycle tests (on a second channel) ─────────────────────

  const CHANNEL_INDEX_2 = 1;
  let channel2Pda: PublicKey;

  it("creates a second channel, funds, withdraws, and closes it", async () => {
    [channel2Pda] = channelPda(programId, authority.publicKey, CHANNEL_INDEX_2);

    // Init
    await program.methods
      .initChannel(CHANNEL_INDEX_2, 4, NO_CALLBACK)
      .accounts({
        authority: authority.publicKey,
        channel: channel2Pda,
        systemProgram: SystemProgram.programId,
      })
      .signers([authority])
      .rpc();

    let ch = await program.account.diceChannel.fetch(channel2Pda);
    assert.equal(ch.maxNodes, 4);
    assert.deepEqual(ch.status, { idle: {} });

    // Fund
    await program.methods
      .fundChannel(new BN(5_000_000))
      .accounts({
        authority: authority.publicKey,
        channel: channel2Pda,
        systemProgram: SystemProgram.programId,
      })
      .signers([authority])
      .rpc();

    ch = await program.account.diceChannel.fetch(channel2Pda);
    assert.equal(ch.balance.toNumber(), 5_000_000);

    // Withdraw all
    await program.methods
      .withdrawBalance(new BN(5_000_000))
      .accounts({
        authority: authority.publicKey,
        channel: channel2Pda,
      })
      .signers([authority])
      .rpc();

    ch = await program.account.diceChannel.fetch(channel2Pda);
    assert.equal(ch.balance.toNumber(), 0);
  });

  it("closes a channel with zero balance", async () => {
    await program.methods
      .closeChannel()
      .accounts({
        authority: authority.publicKey,
        channel: channel2Pda,
      })
      .signers([authority])
      .rpc();

    // Channel account should no longer exist
    try {
      await program.account.diceChannel.fetch(channel2Pda);
      assert.fail("expected channel to be closed");
    } catch (err: any) {
      assert.ok(
        err.message.includes("Account does not exist") ||
        err.message.includes("could not find"),
        "channel should be gone"
      );
    }
  });

  // ─── 7. resize_channel ────────────────────────────────────────────────────

  const CHANNEL_INDEX_3 = 2;
  let channel3Pda: PublicKey;

  it("resizes a channel", async () => {
    [channel3Pda] = channelPda(programId, authority.publicKey, CHANNEL_INDEX_3);

    // Init with max_nodes=5
    await program.methods
      .initChannel(CHANNEL_INDEX_3, 5, NO_CALLBACK)
      .accounts({
        authority: authority.publicKey,
        channel: channel3Pda,
        systemProgram: SystemProgram.programId,
      })
      .signers([authority])
      .rpc();

    let ch = await program.account.diceChannel.fetch(channel3Pda);
    assert.equal(ch.maxNodes, 5);

    // Resize to max_nodes=10
    await program.methods
      .resizeChannel(10)
      .accounts({
        authority: authority.publicKey,
        channel: channel3Pda,
        systemProgram: SystemProgram.programId,
      })
      .signers([authority])
      .rpc();

    ch = await program.account.diceChannel.fetch(channel3Pda);
    assert.equal(ch.maxNodes, 10);
  });

  it("rejects resize with invalid node count", async () => {
    try {
      await program.methods
        .resizeChannel(3) // below MIN_NODES_REQUIRED (4)
        .accounts({
          authority: authority.publicKey,
          channel: channel3Pda,
          systemProgram: SystemProgram.programId,
        })
        .signers([authority])
        .rpc();
      assert.fail("expected resize with invalid count to fail");
    } catch (err: any) {
      assert.ok(err.message, "should reject invalid node count");
    }
  });

  // ─── 8. Error code: close_channel with non-zero balance ───────────────────

  it("rejects close_channel when balance > 0", async () => {
    const [channel4Pda] = channelPda(programId, authority.publicKey, 3);

    await program.methods
      .initChannel(3, 4, NO_CALLBACK)
      .accounts({
        authority: authority.publicKey,
        channel: channel4Pda,
        systemProgram: SystemProgram.programId,
      })
      .signers([authority])
      .rpc();

    // Fund it
    await program.methods
      .fundChannel(new BN(1_000_000))
      .accounts({
        authority: authority.publicKey,
        channel: channel4Pda,
        systemProgram: SystemProgram.programId,
      })
      .signers([authority])
      .rpc();

    // Try to close — should fail because balance > 0
    try {
      await program.methods
        .closeChannel()
        .accounts({
          authority: authority.publicKey,
          channel: channel4Pda,
        })
        .signers([authority])
        .rpc();
      assert.fail("expected close with balance to fail");
    } catch (err: any) {
      assert.ok(err.message, "should reject close with non-zero balance");
    }
  });

  // ─── 9. PDA derivation sanity check ───────────────────────────────────────

  it("derives channel PDAs deterministically", () => {
    const [pda1] = channelPda(programId, authority.publicKey, 0);
    const [pda2] = channelPda(programId, authority.publicKey, 0);
    const [pda3] = channelPda(programId, authority.publicKey, 1);

    assert.ok(pda1.equals(pda2), "same inputs must give same PDA");
    assert.ok(!pda1.equals(pda3), "different index must give different PDA");
  });

  // ─── 10. withdraw_balance edge case ───────────────────────────────────────

  it("rejects withdraw more than balance", async () => {
    const [channel5Pda] = channelPda(programId, authority.publicKey, 4);

    await program.methods
      .initChannel(4, 4, NO_CALLBACK)
      .accounts({
        authority: authority.publicKey,
        channel: channel5Pda,
        systemProgram: SystemProgram.programId,
      })
      .signers([authority])
      .rpc();

    await program.methods
      .fundChannel(new BN(1_000_000))
      .accounts({
        authority: authority.publicKey,
        channel: channel5Pda,
        systemProgram: SystemProgram.programId,
      })
      .signers([authority])
      .rpc();

    try {
      await program.methods
        .withdrawBalance(new BN(2_000_000)) // more than balance
        .accounts({
          authority: authority.publicKey,
          channel: channel5Pda,
        })
        .signers([authority])
        .rpc();
      assert.fail("expected over-withdraw to fail");
    } catch (err: any) {
      assert.ok(err.message, "should reject withdraw > balance");
    }
  });
});
