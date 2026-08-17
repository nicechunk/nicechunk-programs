import assert from "node:assert/strict";
import test from "node:test";

import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { Buffer } from "buffer";

import {
  NICECHUNK_DEVNET_NCK_MINT,
  NICECHUNK_GAME_PROGRAM_ID,
  NICECHUNK_MARKET_TREASURY,
  createConfigureTreasurySwapInstruction,
  createInitializeTreasurySwapInstruction,
  createTreasurySwapInstruction,
  createTreasurySwapNckLiquidityInstruction,
  createTreasurySwapSolLiquidityInstruction,
  decodeTreasurySwapState,
  deriveTreasurySwapPdas,
  quoteTreasurySwap,
} from "../sdk/nicechunk-market.ts";

const config = Object.freeze({
  lamportsPerNck: 25_000_000n,
  minimumNckUnits: 1_000_000n,
  maximumNckUnits: 100_000_000n,
  feeBps: 100,
});

test("Treasury Swap administrator instructions pin every privileged account", () => {
  const pdas = deriveTreasurySwapPdas();
  const initialize = createInitializeTreasurySwapInstruction({
    admin: NICECHUNK_MARKET_TREASURY,
    config,
  });
  assert.equal(initialize.programId.toBase58(), NICECHUNK_GAME_PROGRAM_ID.toBase58());
  assert.equal(initialize.data.length, 28);
  assert.deepEqual([...initialize.data.subarray(0, 2)], [4, 8]);
  assert.equal(initialize.data.readBigUInt64LE(2), config.lamportsPerNck);
  assert.equal(initialize.data.readBigUInt64LE(10), config.minimumNckUnits);
  assert.equal(initialize.data.readBigUInt64LE(18), config.maximumNckUnits);
  assert.equal(initialize.data.readUInt16LE(26), config.feeBps);
  assert.deepEqual(initialize.keys.map((entry) => entry.pubkey.toBase58()), [
    NICECHUNK_MARKET_TREASURY.toBase58(),
    pdas.state[0].toBase58(),
    pdas.solVault[0].toBase58(),
    pdas.nckVault[0].toBase58(),
    pdas.authority[0].toBase58(),
    NICECHUNK_DEVNET_NCK_MINT.toBase58(),
    SystemProgram.programId.toBase58(),
    TOKEN_PROGRAM_ID.toBase58(),
  ]);

  const pause = createConfigureTreasurySwapInstruction({
    admin: NICECHUNK_MARKET_TREASURY,
    config,
    paused: true,
  });
  const activate = createConfigureTreasurySwapInstruction({
    admin: NICECHUNK_MARKET_TREASURY,
    config,
    paused: false,
  });
  assert.equal(pause.keys.length, 2, "emergency pause must not depend on reserve accounts");
  assert.equal(activate.keys.length, 4, "activation must provide both reserves for liquidity checks");
  assert.deepEqual(activate.keys.slice(2).map((entry) => entry.pubkey.toBase58()), [
    pdas.solVault[0].toBase58(),
    pdas.nckVault[0].toBase58(),
  ]);
  assert.equal(pause.data.at(-1), 1);
  assert.equal(activate.data.at(-1), 0);

  assert.throws(() => createInitializeTreasurySwapInstruction({
    admin: Keypair.generate().publicKey,
    config,
  }), /admin must be/);
  assert.throws(() => createInitializeTreasurySwapInstruction({
    admin: NICECHUNK_MARKET_TREASURY,
    nckMint: Keypair.generate().publicKey,
    config,
  }), /NCK mint must be/);
  assert.throws(() => createInitializeTreasurySwapInstruction({
    admin: NICECHUNK_MARKET_TREASURY,
    config: { ...config, lamportsPerNck: Number.MAX_SAFE_INTEGER + 1 },
  }), /positive u64/);
});

test("Treasury Swap reserve instructions keep deposits and withdrawals treasury-only", () => {
  const pdas = deriveTreasurySwapPdas();
  const treasuryNckToken = Keypair.generate().publicKey;
  const depositSol = createTreasurySwapSolLiquidityInstruction({
    admin: NICECHUNK_MARKET_TREASURY,
    amountLamports: 1_000_000_000n,
  });
  const withdrawSol = createTreasurySwapSolLiquidityInstruction({
    admin: NICECHUNK_MARKET_TREASURY,
    amountLamports: 1n,
    withdraw: true,
  });
  assert.deepEqual([...depositSol.data.subarray(0, 2)], [4, 10]);
  assert.deepEqual([...withdrawSol.data.subarray(0, 2)], [4, 11]);
  assert.equal(depositSol.keys.at(-1).pubkey.toBase58(), SystemProgram.programId.toBase58());
  assert.equal(withdrawSol.keys.length, 3);

  const depositNck = createTreasurySwapNckLiquidityInstruction({
    admin: NICECHUNK_MARKET_TREASURY,
    adminNckToken: treasuryNckToken,
    amountNckUnits: 20_000_000n,
  });
  const withdrawNck = createTreasurySwapNckLiquidityInstruction({
    admin: NICECHUNK_MARKET_TREASURY,
    adminNckToken: treasuryNckToken,
    amountNckUnits: 1n,
    withdraw: true,
  });
  assert.equal(depositNck.data.readUInt8(1), 12);
  assert.equal(withdrawNck.data.readUInt8(1), 13);
  assert.equal(depositNck.keys[3].pubkey.toBase58(), pdas.nckVault[0].toBase58());
  assert.equal(withdrawNck.keys[2].pubkey.toBase58(), pdas.authority[0].toBase58());
});

test("Treasury Swap user instructions commit direction, slippage, revision, and deadline", () => {
  const pdas = deriveTreasurySwapPdas();
  const user = Keypair.generate().publicKey;
  const userNckToken = Keypair.generate().publicKey;
  const common = {
    user,
    userNckToken,
    amountIn: 100_000_000n,
    minimumAmountOut: 3_960_000n,
    expectedRevision: 7n,
    deadlineSlot: 9_999n,
  };
  const solToNck = createTreasurySwapInstruction({ ...common, direction: "SOL_TO_NCK" });
  assert.deepEqual([...solToNck.data.subarray(0, 2)], [4, 14]);
  assert.equal(solToNck.data.readBigUInt64LE(2), common.amountIn);
  assert.equal(solToNck.data.readBigUInt64LE(10), common.minimumAmountOut);
  assert.equal(solToNck.data.readBigUInt64LE(18), common.expectedRevision);
  assert.equal(solToNck.data.readBigUInt64LE(26), common.deadlineSlot);
  assert.deepEqual(solToNck.keys.map((entry) => entry.pubkey.toBase58()), [
    user.toBase58(),
    pdas.state[0].toBase58(),
    pdas.solVault[0].toBase58(),
    pdas.authority[0].toBase58(),
    pdas.nckVault[0].toBase58(),
    userNckToken.toBase58(),
    NICECHUNK_DEVNET_NCK_MINT.toBase58(),
    SystemProgram.programId.toBase58(),
    TOKEN_PROGRAM_ID.toBase58(),
  ]);

  const nckToSol = createTreasurySwapInstruction({
    ...common,
    direction: "NCK_TO_SOL",
    amountIn: 4_000_000n,
    minimumAmountOut: 99_000_000n,
  });
  assert.equal(nckToSol.data.readUInt8(1), 15);
  assert.equal(nckToSol.keys.length, 7);
  assert.throws(() => createTreasurySwapInstruction({ ...common, direction: "BAD" }), /direction/);
  assert.throws(() => createTreasurySwapInstruction({ ...common, direction: "SOL_TO_NCK", deadlineSlot: 0n }), /positive u64/);
});

test("Treasury Swap decoder rejects forged identity and noncanonical reserved bytes", () => {
  const data = validStateData();
  const decoded = decodeTreasurySwapState(data);
  assert.equal(decoded.lamportsPerNck, config.lamportsPerNck);
  assert.equal(decoded.feeBps, config.feeBps);

  const forgedAdmin = Buffer.from(data);
  Keypair.generate().publicKey.toBuffer().copy(forgedAdmin, 24);
  assert.throws(() => decodeTreasurySwapState(forgedAdmin), /authority, mint, or revision/);
  const forgedMint = Buffer.from(data);
  Keypair.generate().publicKey.toBuffer().copy(forgedMint, 56);
  assert.throws(() => decodeTreasurySwapState(forgedMint), /authority, mint, or revision/);
  for (const offset of [15, 18, 23]) {
    const noncanonical = Buffer.from(data);
    noncanonical[offset] = 1;
    assert.throws(() => decodeTreasurySwapState(noncanonical), /layout/);
  }
});

test("fixed-price floor rounding and output fees never create round-trip profit", () => {
  const state = {
    lamportsPerNck: 33_333_333n,
    minimumNckUnits: 1n,
    maximumNckUnits: 10_000_000_000n,
    feeBps: 25,
  };
  let seed = 0x1234_5678n;
  for (let index = 0; index < 10_000; index += 1) {
    seed = (seed * 1_103_515_245n + 12_345n) & 0xffff_ffffn;
    const lamports = 1_000_000n + seed;
    const nck = quoteTreasurySwap({ direction: "SOL_TO_NCK", amountIn: lamports, state }).amountOut;
    const returned = quoteTreasurySwap({ direction: "NCK_TO_SOL", amountIn: nck, state }).amountOut;
    assert.ok(returned <= lamports, `${returned} exceeds ${lamports}`);
  }
});

function validStateData() {
  const pdas = deriveTreasurySwapPdas();
  const data = Buffer.alloc(160);
  data.write("NCKSWP01", 0, "ascii");
  data.writeUInt16LE(1, 8);
  data.writeUInt8(pdas.state[1], 10);
  data.writeUInt8(pdas.authority[1], 11);
  data.writeUInt8(pdas.solVault[1], 12);
  data.writeUInt8(pdas.nckVault[1], 13);
  data.writeUInt8(1, 14);
  data.writeUInt16LE(config.feeBps, 16);
  NICECHUNK_MARKET_TREASURY.toBuffer().copy(data, 24);
  NICECHUNK_DEVNET_NCK_MINT.toBuffer().copy(data, 56);
  data.writeBigUInt64LE(config.lamportsPerNck, 88);
  data.writeBigUInt64LE(config.minimumNckUnits, 96);
  data.writeBigUInt64LE(config.maximumNckUnits, 104);
  data.writeBigUInt64LE(1n, 112);
  return data;
}
