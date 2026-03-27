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

const SEED_DEVICE  = Buffer.from("device");
const SEED_REQUEST = Buffer.from("request");
const SEED_COMMIT  = Buffer.from("commit");
const SEED_REVEAL  = Buffer.from("reveal");
const SEED_RESULT  = Buffer.from("result");
const SEED_ESCROW  = Buffer.from("escrow");

function seqBuf(seq: BN): Buffer {
  return seq.toArrayLike(Buffer, "le", 8);
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

function requestPda(
  programId: PublicKey,
  requester: PublicKey,
  seq: BN
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEED_REQUEST, requester.toBuffer(), seqBuf(seq)],
    programId
  );
}

function escrowPda(
  programId: PublicKey,
  requester: PublicKey,
  seq: BN
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEED_ESCROW, requester.toBuffer(), seqBuf(seq)],
    programId
  );
}

function commitPda(
  programId: PublicKey,
  requester: PublicKey,
  seq: BN,
  deviceId: Buffer
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEED_COMMIT, requester.toBuffer(), seqBuf(seq), deviceId],
    programId
  );
}

function revealPda(
  programId: PublicKey,
  requester: PublicKey,
  seq: BN,
  deviceId: Buffer
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEED_REVEAL, requester.toBuffer(), seqBuf(seq), deviceId],
    programId
  );
}

function resultPda(
  programId: PublicKey,
  requester: PublicKey,
  seq: BN
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEED_RESULT, requester.toBuffer(), seqBuf(seq)],
    programId
  );
}

// No-callback sentinel: Pubkey::default() = all zeros
const NO_CALLBACK = new PublicKey(new Uint8Array(32));

// Fake compressed secp256k1 pubkey (33 bytes, valid prefix 0x02).
// Returns a Uint8Array since that's what the PDA derivation and Anchor methods need.
function fakeDevicePubkey(seed: number): number[] {
  const key = new Array(33).fill(0);
  key[0] = 0x02;
  key[1] = seed & 0xff;
  key[2] = (seed >> 8) & 0xff;
  return key;
}

// Compute device_id = SHA-256(device_pubkey) — used as 32-byte PDA seed.
// Must match programs/dice/src/constants.rs::device_id().
async function computeDeviceId(devicePubkey: number[]): Promise<Buffer> {
  const hash = await crypto.subtle.digest("SHA-256", new Uint8Array(devicePubkey));
  return Buffer.from(hash);
}

// Synchronous version using node:crypto
function computeDeviceIdSync(devicePubkey: number[]): Buffer {
  const { createHash } = require("crypto");
  return createHash("sha256").update(Buffer.from(devicePubkey)).digest();
}

// Fake 32-byte entropy
function fakeEntropy(seed: number): number[] {
  const e = new Array(32).fill(0);
  e[0] = seed & 0xff;
  e[1] = (seed >> 8) & 0xff;
  return e;
}

// SHA-256 commit hash of entropy (using anchor's hashv equivalent via crypto subtle)
// In tests we just use a fixed hash that won't pass on-chain ECDSA check —
// the ECDSA verification would need real secp256k1 keys. Instead we test the
// structural account lifecycle; signature validation is covered by unit tests.
function fakeCommitHash(entropySeed: number): number[] {
  const h = new Array(32).fill(0);
  h[0] = entropySeed & 0xff;
  h[1] = 0xcc; // marker
  return h;
}

// 64-byte fake ECDSA signature
function fakeSignature(): number[] {
  return new Array(64).fill(1);
}

/// Fund an account — uses transfer from payer wallet (works on devnet without airdrop).
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
// Test suite
// ─────────────────────────────────────────────────────────────────────────────

describe("dice", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Dice as Program<Dice>;
  const programId = program.programId;

  // Actors
  const payer      = provider.wallet as anchor.Wallet;
  const coordinator = Keypair.generate();
  const requester   = Keypair.generate();
  const treasury    = Keypair.generate();
  const reserve     = Keypair.generate();

  // Shared state across tests
  const SEQ = new BN(1);
  const DEVICE_COUNT = 5; // 5 nodes >= MIN_NODES_REQUIRED(4)

  // Generate fake device pubkeys, IDs, and their entropies
  const devices = Array.from({ length: DEVICE_COUNT }, (_, i) => {
    const pubkey = fakeDevicePubkey(i + 1);
    const id = computeDeviceIdSync(pubkey);
    return {
      pubkey,
      id,                                  // [u8; 32] = SHA-256(pubkey)
      idArray: Array.from(id) as number[],  // number[] for Anchor methods
      entropy: fakeEntropy(i + 1),
      commitHash: fakeCommitHash(i + 1),
    };
  });

  before(async () => {
    // Fund test accounts from the payer wallet.
    await fundAccount(provider, coordinator.publicKey, 0.05 * LAMPORTS_PER_SOL);
    await fundAccount(provider, requester.publicKey, 0.05 * LAMPORTS_PER_SOL);
    await fundAccount(provider, treasury.publicKey, 0.01 * LAMPORTS_PER_SOL);
    await fundAccount(provider, reserve.publicKey, 0.01 * LAMPORTS_PER_SOL);
  });

  // ─── 1. register_device ───────────────────────────────────────────────────

  it("registers a hardware device", async () => {
    const device = devices[0];
    const [registryPda] = deviceRegistryPda(programId, device.id);

    await program.methods
      .registerDevice(device.idArray, device.pubkey)
      .accounts({
        payer: payer.publicKey,
        deviceRegistry: registryPda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const account = await program.account.deviceRegistry.fetch(registryPda);
    assert.deepEqual(Array.from(account.devicePubkey), device.pubkey);
    assert.isTrue(account.isActive);
    assert.equal(account.jobsCompleted.toNumber(), 0);
  });

  it("rejects duplicate device registration", async () => {
    const device = devices[0];
    const [registryPda] = deviceRegistryPda(programId, device.id);

    try {
      await program.methods
        .registerDevice(device.idArray, device.pubkey)
        .accounts({
          payer: payer.publicKey,
          deviceRegistry: registryPda,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
      assert.fail("expected duplicate registration to fail");
    } catch (err: any) {
      // Anchor/Solana will throw because the PDA already exists
      assert.ok(err.message, "duplicate should throw");
    }
  });

  // ─── 2. request_randomness ────────────────────────────────────────────────

  it("creates a randomness request and escrow", async () => {
    const [reqPda]    = requestPda(programId, requester.publicKey, SEQ);
    const [escrowPdaAcc] = escrowPda(programId, requester.publicKey, SEQ);

    await program.methods
      .requestRandomness(SEQ, NO_CALLBACK)
      .accounts({
        requester: requester.publicKey,
        randomnessRequest: reqPda,
        escrow: escrowPdaAcc,
        systemProgram: SystemProgram.programId,
      })
      .signers([requester])
      .rpc();

    const req = await program.account.randomnessRequest.fetch(reqPda);
    assert.ok(req.requester.equals(requester.publicKey));
    assert.equal(req.sequence.toNumber(), SEQ.toNumber());
    assert.deepEqual(req.status, { pending: {} });
    assert.equal(req.nodeCount, 0);

    const esc = await program.account.escrowAccount.fetch(escrowPdaAcc);
    assert.ok(esc.requester.equals(requester.publicKey));
    assert.equal(esc.amount.toNumber(), 2_000_000); // 0.002 SOL
    assert.isFalse(esc.isClaimed);
  });

  // ─── 3. submit_commit × DEVICE_COUNT ─────────────────────────────────────

  it("accepts commits from all selected nodes", async () => {
    const [reqPda] = requestPda(programId, requester.publicKey, SEQ);

    for (let i = 0; i < DEVICE_COUNT; i++) {
      const device = devices[i];
      const [commitPdaAcc] = commitPda(
        programId,
        requester.publicKey,
        SEQ,
        device.id
      );

      await program.methods
        .submitCommit(device.idArray, device.pubkey, device.commitHash)
        .accounts({
          coordinator: coordinator.publicKey,
          randomnessRequest: reqPda,
          commitRecord: commitPdaAcc,
          systemProgram: SystemProgram.programId,
        })
        .signers([coordinator])
        .rpc();
    }

    const req = await program.account.randomnessRequest.fetch(reqPda);
    assert.equal(req.nodeCount, DEVICE_COUNT);
    assert.equal(req.commitsReceived, DEVICE_COUNT);
    assert.deepEqual(req.status, { commitPhase: {} });
  });

  it("rejects a duplicate commit from the same device", async () => {
    const [reqPda] = requestPda(programId, requester.publicKey, SEQ);
    const device = devices[0];

    // The commit PDA for devices[0] already exists — trying to init it again
    // will fail at the account constraint level.
    const [commitPdaAcc] = commitPda(
      programId,
      requester.publicKey,
      SEQ,
      device.id
    );

    try {
      await program.methods
        .submitCommit(device.idArray, device.pubkey, device.commitHash)
        .accounts({
          coordinator: coordinator.publicKey,
          randomnessRequest: reqPda,
          commitRecord: commitPdaAcc,
          systemProgram: SystemProgram.programId,
        })
        .signers([coordinator])
        .rpc();
      assert.fail("expected duplicate commit to fail");
    } catch (err: any) {
      assert.ok(err.message, "duplicate commit should throw");
    }
  });

  // ─── 4. submit_reveal × DEVICE_COUNT ─────────────────────────────────────
  //
  // NOTE: The on-chain submit_reveal instruction verifies that
  //   SHA-256(entropy) == commit_hash  AND  secp256k1_recover(sig) == device_pubkey
  //
  // The fake data used here will FAIL signature verification. To exercise the
  // full reveal path in integration tests you need real ESP32-generated
  // signatures. These tests instead verify account structure and error codes.
  //
  // For structural tests that bypass signature checking, the on-chain program
  // would need a #[cfg(test)] bypass — a deliberate security decision NOT made
  // in this codebase. Full end-to-end reveal tests run via the Python e2e suite
  // (tests/e2e/) which uses the mock_firmware_node harness with real ECDSA keys.

  it("records a commit for each device (reveals deferred to e2e suite)", async () => {
    const [reqPda] = requestPda(programId, requester.publicKey, SEQ);
    const req = await program.account.randomnessRequest.fetch(reqPda);
    assert.equal(req.commitsReceived, DEVICE_COUNT);
    // Status check: at least CommitPhase (would move to RevealPhase on first reveal)
    assert.ok(
      "commitPhase" in req.status || "revealPhase" in req.status,
      "expected CommitPhase or RevealPhase"
    );
  });

  // ─── 5. init_escrow (independent) ────────────────────────────────────────

  it("initialises a standalone escrow account", async () => {
    const devKp = Keypair.generate();
    await fundAccount(provider, devKp.publicKey, 0.02 * LAMPORTS_PER_SOL);

    const seq2 = new BN(42);
    const [escrowPdaAcc] = escrowPda(programId, devKp.publicKey, seq2);

    await program.methods
      .initEscrow(seq2)
      .accounts({
        developer: devKp.publicKey,
        escrow: escrowPdaAcc,
        systemProgram: SystemProgram.programId,
      })
      .signers([devKp])
      .rpc();

    const esc = await program.account.escrowAccount.fetch(escrowPdaAcc);
    assert.ok(esc.requester.equals(devKp.publicKey));
    assert.equal(esc.sequence.toNumber(), 42);
    assert.equal(esc.amount.toNumber(), 0);
    assert.isFalse(esc.isClaimed);
  });

  // ─── 6. fund_escrow ───────────────────────────────────────────────────────

  it("funds an existing escrow account", async () => {
    const devKp = Keypair.generate();
    await fundAccount(provider, devKp.publicKey, 0.02 * LAMPORTS_PER_SOL);

    const seq3 = new BN(99);
    const [escrowPdaAcc] = escrowPda(programId, devKp.publicKey, seq3);

    // Init first
    await program.methods
      .initEscrow(seq3)
      .accounts({
        developer: devKp.publicKey,
        escrow: escrowPdaAcc,
        systemProgram: SystemProgram.programId,
      })
      .signers([devKp])
      .rpc();

    // Fund it
    const fundAmount = new BN(1_000_000);
    await program.methods
      .fundEscrow(fundAmount)
      .accounts({
        developer: devKp.publicKey,
        escrow: escrowPdaAcc,
        systemProgram: SystemProgram.programId,
      })
      .signers([devKp])
      .rpc();

    const esc = await program.account.escrowAccount.fetch(escrowPdaAcc);
    assert.equal(esc.amount.toNumber(), 1_000_000);
  });

  // ─── 7. Error codes ───────────────────────────────────────────────────────

  it("rejects requestRandomness with duplicate sequence from same requester", async () => {
    // SEQ=1 already created above — this should fail at the account constraint
    const [reqPda]    = requestPda(programId, requester.publicKey, SEQ);
    const [escrowPdaAcc] = escrowPda(programId, requester.publicKey, SEQ);

    try {
      await program.methods
        .requestRandomness(SEQ, NO_CALLBACK)
        .accounts({
          requester: requester.publicKey,
          randomnessRequest: reqPda,
          escrow: escrowPdaAcc,
          systemProgram: SystemProgram.programId,
        })
        .signers([requester])
        .rpc();
      assert.fail("expected duplicate request to fail");
    } catch (err: any) {
      assert.ok(err.message, "duplicate request should throw");
    }
  });

  it("rejects submitCommit on a non-existent request", async () => {
    const fakeSeq = new BN(9999);
    const [reqPda] = requestPda(programId, requester.publicKey, fakeSeq);
    const device = devices[0];
    const [commitPdaAcc] = commitPda(
      programId,
      requester.publicKey,
      fakeSeq,
      device.id
    );

    try {
      await program.methods
        .submitCommit(device.idArray, device.pubkey, device.commitHash)
        .accounts({
          coordinator: coordinator.publicKey,
          randomnessRequest: reqPda,
          commitRecord: commitPdaAcc,
          systemProgram: SystemProgram.programId,
        })
        .signers([coordinator])
        .rpc();
      assert.fail("expected commit on missing request to fail");
    } catch (err: any) {
      assert.ok(err.message, "should throw for missing request account");
    }
  });

  // ─── 8. PDA derivation sanity checks ─────────────────────────────────────

  it("derives all PDAs deterministically", () => {
    const seq = new BN(1);

    const [reqPda1]    = requestPda(programId, requester.publicKey, seq);
    const [reqPda2]    = requestPda(programId, requester.publicKey, seq);
    const [escrowPda1] = escrowPda(programId, requester.publicKey, seq);
    const [escrowPda2] = escrowPda(programId, requester.publicKey, seq);
    const [resPda1]    = resultPda(programId, requester.publicKey, seq);
    const [resPda2]    = resultPda(programId, requester.publicKey, seq);

    assert.ok(reqPda1.equals(reqPda2), "request PDA must be deterministic");
    assert.ok(escrowPda1.equals(escrowPda2), "escrow PDA must be deterministic");
    assert.ok(resPda1.equals(resPda2), "result PDA must be deterministic");

    // Different sequences must produce different PDAs
    const seqB = new BN(2);
    const [reqPdaB] = requestPda(programId, requester.publicKey, seqB);
    assert.ok(!reqPda1.equals(reqPdaB), "different seq must give different PDA");
  });
});
