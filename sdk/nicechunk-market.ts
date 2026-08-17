import {
  PublicKey,
  SystemProgram,
  TransactionInstruction,
  type PublicKeyInitData,
} from "@solana/web3.js";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { Buffer } from "buffer";

export const NICECHUNK_GAME_PROGRAM_ID = new PublicKey(
  "6CurnvneezBuHwPUnrCiFg1QMWeUF67ufQxYebyr2UP7",
);
export const NICECHUNK_MARKET_TREASURY = new PublicKey(
  "CtPV2vmqNNwUSfMu5nz58ZtMPy6ZvxL4LyNdPHVW7WvF",
);
export const NICECHUNK_DEVNET_NCK_MINT = new PublicKey(
  "HSnWF5kjkWVrceW2SaSskScuLveUZE4gpthZ2ZXRPQPo",
);
export const TREASURY_SWAP_STATE_SEED = "treasury-swap-v1";
export const TREASURY_SWAP_AUTHORITY_SEED = "treasury-swap-authority-v1";
export const TREASURY_SWAP_SOL_VAULT_SEED = "treasury-swap-sol-v1";
export const TREASURY_SWAP_NCK_VAULT_SEED = "treasury-swap-nck-v1";
export const TREASURY_SWAP_MAGIC = "NCKSWP01";
export const TREASURY_SWAP_VERSION = 1;
export const TREASURY_SWAP_STATE_LEN = 160;
export const TREASURY_SWAP_MAX_FEE_BPS = 1_000;

const MARKET_NAMESPACE = 4;
const U64_MAX = 0xffff_ffff_ffff_ffffn;
const NCK_BASE_UNITS = 1_000_000n;
const BPS_DENOMINATOR = 10_000n;

export type TreasurySwapDirection = "SOL_TO_NCK" | "NCK_TO_SOL";

export interface TreasurySwapConfig {
  lamportsPerNck: bigint | number | string;
  minimumNckUnits: bigint | number | string;
  maximumNckUnits: bigint | number | string;
  feeBps: number;
}

export interface TreasurySwapPdas {
  state: [PublicKey, number];
  authority: [PublicKey, number];
  solVault: [PublicKey, number];
  nckVault: [PublicKey, number];
}

export interface DecodedTreasurySwapState {
  stateBump: number;
  authorityBump: number;
  solVaultBump: number;
  nckVaultBump: number;
  paused: boolean;
  feeBps: number;
  admin: PublicKey;
  nckMint: PublicKey;
  lamportsPerNck: bigint;
  minimumNckUnits: bigint;
  maximumNckUnits: bigint;
  revision: bigint;
  updatedSlot: bigint;
  totalSolToNckLamports: bigint;
  totalSolToNckUnits: bigint;
  totalNckToSolUnits: bigint;
  totalNckToSolLamports: bigint;
}

export function deriveTreasurySwapPdas(
  programId: PublicKeyInitData = NICECHUNK_GAME_PROGRAM_ID,
): TreasurySwapPdas {
  const selectedProgram = new PublicKey(programId);
  return {
    state: PublicKey.findProgramAddressSync([Buffer.from(TREASURY_SWAP_STATE_SEED)], selectedProgram),
    authority: PublicKey.findProgramAddressSync([Buffer.from(TREASURY_SWAP_AUTHORITY_SEED)], selectedProgram),
    solVault: PublicKey.findProgramAddressSync([Buffer.from(TREASURY_SWAP_SOL_VAULT_SEED)], selectedProgram),
    nckVault: PublicKey.findProgramAddressSync([Buffer.from(TREASURY_SWAP_NCK_VAULT_SEED)], selectedProgram),
  };
}

export function decodeTreasurySwapState(data: Uint8Array): DecodedTreasurySwapState {
  const bytes = Buffer.from(data);
  if (bytes.length !== TREASURY_SWAP_STATE_LEN
    || decodeAscii(bytes.subarray(0, 8)) !== TREASURY_SWAP_MAGIC
    || bytes.readUInt16LE(8) !== TREASURY_SWAP_VERSION
    || bytes.readUInt8(14) > 1
    || bytes.readUInt8(15) !== 0
    || bytes.subarray(18, 24).some((byte) => byte !== 0)) {
    throw new Error("Invalid TreasurySwapState layout");
  }
  const state: DecodedTreasurySwapState = {
    stateBump: bytes.readUInt8(10),
    authorityBump: bytes.readUInt8(11),
    solVaultBump: bytes.readUInt8(12),
    nckVaultBump: bytes.readUInt8(13),
    paused: bytes.readUInt8(14) === 1,
    feeBps: bytes.readUInt16LE(16),
    admin: new PublicKey(bytes.subarray(24, 56)),
    nckMint: new PublicKey(bytes.subarray(56, 88)),
    lamportsPerNck: readU64Le(bytes, 88),
    minimumNckUnits: readU64Le(bytes, 96),
    maximumNckUnits: readU64Le(bytes, 104),
    revision: readU64Le(bytes, 112),
    updatedSlot: readU64Le(bytes, 120),
    totalSolToNckLamports: readU64Le(bytes, 128),
    totalSolToNckUnits: readU64Le(bytes, 136),
    totalNckToSolUnits: readU64Le(bytes, 144),
    totalNckToSolLamports: readU64Le(bytes, 152),
  };
  validateSwapConfig(state);
  if (!state.admin.equals(NICECHUNK_MARKET_TREASURY)
    || !state.nckMint.equals(NICECHUNK_DEVNET_NCK_MINT)
    || state.revision === 0n) {
    throw new Error("Invalid TreasurySwapState authority, mint, or revision");
  }
  return state;
}

export function quoteTreasurySwap({
  direction,
  amountIn,
  state,
}: {
  direction: TreasurySwapDirection;
  amountIn: bigint | number | string;
  state: Pick<DecodedTreasurySwapState, "lamportsPerNck" | "minimumNckUnits" | "maximumNckUnits" | "feeBps">;
}): { amountOut: bigint; grossAmountOut: bigint; feeAmount: bigint } {
  const normalizedAmount = positiveU64(amountIn, "amountIn");
  if (direction !== "SOL_TO_NCK" && direction !== "NCK_TO_SOL") {
    throw new Error("Invalid Treasury Swap direction");
  }
  validateSwapConfig(state);
  const grossAmountOut = direction === "SOL_TO_NCK"
    ? normalizedAmount * NCK_BASE_UNITS / state.lamportsPerNck
    : normalizedAmount * state.lamportsPerNck / NCK_BASE_UNITS;
  const nckSideAmount = direction === "SOL_TO_NCK" ? grossAmountOut : normalizedAmount;
  if (nckSideAmount < state.minimumNckUnits || nckSideAmount > state.maximumNckUnits) {
    throw new Error("Treasury Swap amount is outside configured limits");
  }
  const amountOut = grossAmountOut * (BPS_DENOMINATOR - BigInt(state.feeBps)) / BPS_DENOMINATOR;
  if (amountOut <= 0n || amountOut > U64_MAX || grossAmountOut > U64_MAX) {
    throw new Error("Treasury Swap quote is outside u64 range");
  }
  return { amountOut, grossAmountOut, feeAmount: grossAmountOut - amountOut };
}

export function createInitializeTreasurySwapInstruction({
  admin,
  nckMint = NICECHUNK_DEVNET_NCK_MINT,
  config,
  programId = NICECHUNK_GAME_PROGRAM_ID,
  unifiedGame = true,
}: {
  admin: PublicKeyInitData;
  nckMint?: PublicKeyInitData;
  config: TreasurySwapConfig;
  programId?: PublicKeyInitData;
  unifiedGame?: boolean;
}): TransactionInstruction {
  const selectedProgram = new PublicKey(programId);
  const selectedAdmin = new PublicKey(admin);
  const selectedNckMint = new PublicKey(nckMint);
  requireTreasuryAdmin(selectedAdmin);
  requireDevnetNckMint(selectedNckMint);
  const pdas = deriveTreasurySwapPdas(selectedProgram);
  return new TransactionInstruction({
    programId: selectedProgram,
    keys: [
      { pubkey: selectedAdmin, isSigner: true, isWritable: true },
      { pubkey: pdas.state[0], isSigner: false, isWritable: true },
      { pubkey: pdas.solVault[0], isSigner: false, isWritable: true },
      { pubkey: pdas.nckVault[0], isSigner: false, isWritable: true },
      { pubkey: pdas.authority[0], isSigner: false, isWritable: false },
      { pubkey: selectedNckMint, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    ],
    data: marketInstructionData(8, encodeConfig(config), unifiedGame),
  });
}

export function createConfigureTreasurySwapInstruction({
  admin,
  config,
  paused,
  programId = NICECHUNK_GAME_PROGRAM_ID,
  unifiedGame = true,
}: {
  admin: PublicKeyInitData;
  config: TreasurySwapConfig;
  paused: boolean;
  programId?: PublicKeyInitData;
  unifiedGame?: boolean;
}): TransactionInstruction {
  const selectedProgram = new PublicKey(programId);
  const selectedAdmin = new PublicKey(admin);
  requireTreasuryAdmin(selectedAdmin);
  const pdas = deriveTreasurySwapPdas(selectedProgram);
  const payload = Buffer.concat([encodeConfig(config), Buffer.from([paused ? 1 : 0])]);
  const keys = [
    { pubkey: selectedAdmin, isSigner: true, isWritable: true },
    { pubkey: pdas.state[0], isSigner: false, isWritable: true },
  ];
  if (!paused) {
    keys.push(
      { pubkey: pdas.solVault[0], isSigner: false, isWritable: true },
      { pubkey: pdas.nckVault[0], isSigner: false, isWritable: true },
    );
  }
  return new TransactionInstruction({
    programId: selectedProgram,
    keys,
    data: marketInstructionData(9, payload, unifiedGame),
  });
}

export function createTreasurySwapSolLiquidityInstruction({
  admin,
  amountLamports,
  withdraw = false,
  programId = NICECHUNK_GAME_PROGRAM_ID,
  unifiedGame = true,
}: {
  admin: PublicKeyInitData;
  amountLamports: bigint | number | string;
  withdraw?: boolean;
  programId?: PublicKeyInitData;
  unifiedGame?: boolean;
}): TransactionInstruction {
  const selectedProgram = new PublicKey(programId);
  const selectedAdmin = new PublicKey(admin);
  requireTreasuryAdmin(selectedAdmin);
  const pdas = deriveTreasurySwapPdas(selectedProgram);
  const keys = [
    { pubkey: selectedAdmin, isSigner: true, isWritable: true },
    { pubkey: pdas.state[0], isSigner: false, isWritable: false },
    { pubkey: pdas.solVault[0], isSigner: false, isWritable: true },
  ];
  if (!withdraw) keys.push({ pubkey: SystemProgram.programId, isSigner: false, isWritable: false });
  return new TransactionInstruction({
    programId: selectedProgram,
    keys,
    data: marketInstructionData(withdraw ? 11 : 10, encodeAmount(amountLamports), unifiedGame),
  });
}

export function createTreasurySwapNckLiquidityInstruction({
  admin,
  adminNckToken,
  amountNckUnits,
  withdraw = false,
  nckMint = NICECHUNK_DEVNET_NCK_MINT,
  programId = NICECHUNK_GAME_PROGRAM_ID,
  unifiedGame = true,
}: {
  admin: PublicKeyInitData;
  adminNckToken: PublicKeyInitData;
  amountNckUnits: bigint | number | string;
  withdraw?: boolean;
  nckMint?: PublicKeyInitData;
  programId?: PublicKeyInitData;
  unifiedGame?: boolean;
}): TransactionInstruction {
  const selectedProgram = new PublicKey(programId);
  const selectedAdmin = new PublicKey(admin);
  const selectedNckMint = new PublicKey(nckMint);
  requireTreasuryAdmin(selectedAdmin);
  requireDevnetNckMint(selectedNckMint);
  const pdas = deriveTreasurySwapPdas(selectedProgram);
  const keys = withdraw
    ? [
        { pubkey: selectedAdmin, isSigner: true, isWritable: true },
        { pubkey: pdas.state[0], isSigner: false, isWritable: false },
        { pubkey: pdas.authority[0], isSigner: false, isWritable: false },
        { pubkey: new PublicKey(adminNckToken), isSigner: false, isWritable: true },
        { pubkey: pdas.nckVault[0], isSigner: false, isWritable: true },
        { pubkey: selectedNckMint, isSigner: false, isWritable: false },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      ]
    : [
        { pubkey: selectedAdmin, isSigner: true, isWritable: true },
        { pubkey: pdas.state[0], isSigner: false, isWritable: false },
        { pubkey: new PublicKey(adminNckToken), isSigner: false, isWritable: true },
        { pubkey: pdas.nckVault[0], isSigner: false, isWritable: true },
        { pubkey: selectedNckMint, isSigner: false, isWritable: false },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      ];
  return new TransactionInstruction({
    programId: selectedProgram,
    keys,
    data: marketInstructionData(withdraw ? 13 : 12, encodeAmount(amountNckUnits), unifiedGame),
  });
}

export function createTreasurySwapInstruction({
  user,
  userNckToken,
  direction,
  amountIn,
  minimumAmountOut,
  expectedRevision,
  deadlineSlot,
  nckMint = NICECHUNK_DEVNET_NCK_MINT,
  programId = NICECHUNK_GAME_PROGRAM_ID,
  unifiedGame = true,
}: {
  user: PublicKeyInitData;
  userNckToken: PublicKeyInitData;
  direction: TreasurySwapDirection;
  amountIn: bigint | number | string;
  minimumAmountOut: bigint | number | string;
  expectedRevision: bigint | number | string;
  deadlineSlot: bigint | number | string;
  nckMint?: PublicKeyInitData;
  programId?: PublicKeyInitData;
  unifiedGame?: boolean;
}): TransactionInstruction {
  if (direction !== "SOL_TO_NCK" && direction !== "NCK_TO_SOL") {
    throw new Error("Invalid Treasury Swap direction");
  }
  const selectedProgram = new PublicKey(programId);
  const selectedUser = new PublicKey(user);
  const selectedNckMint = new PublicKey(nckMint);
  requireDevnetNckMint(selectedNckMint);
  const pdas = deriveTreasurySwapPdas(selectedProgram);
  const payload = Buffer.alloc(32);
  writeU64Le(payload, 0, positiveU64(amountIn, "amountIn"));
  writeU64Le(payload, 8, positiveU64(minimumAmountOut, "minimumAmountOut"));
  writeU64Le(payload, 16, positiveU64(expectedRevision, "expectedRevision"));
  writeU64Le(payload, 24, positiveU64(deadlineSlot, "deadlineSlot"));
  const sharedKeys = [
    { pubkey: selectedUser, isSigner: true, isWritable: true },
    { pubkey: pdas.state[0], isSigner: false, isWritable: true },
    { pubkey: pdas.solVault[0], isSigner: false, isWritable: true },
  ];
  const keys = direction === "SOL_TO_NCK"
    ? [
        ...sharedKeys,
        { pubkey: pdas.authority[0], isSigner: false, isWritable: false },
        { pubkey: pdas.nckVault[0], isSigner: false, isWritable: true },
        { pubkey: new PublicKey(userNckToken), isSigner: false, isWritable: true },
        { pubkey: selectedNckMint, isSigner: false, isWritable: false },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      ]
    : [
        ...sharedKeys,
        { pubkey: pdas.nckVault[0], isSigner: false, isWritable: true },
        { pubkey: new PublicKey(userNckToken), isSigner: false, isWritable: true },
        { pubkey: selectedNckMint, isSigner: false, isWritable: false },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      ];
  return new TransactionInstruction({
    programId: selectedProgram,
    keys,
    data: marketInstructionData(direction === "SOL_TO_NCK" ? 14 : 15, payload, unifiedGame),
  });
}

function encodeConfig(config: TreasurySwapConfig): Buffer {
  const normalized = {
    lamportsPerNck: positiveU64(config.lamportsPerNck, "lamportsPerNck"),
    minimumNckUnits: positiveU64(config.minimumNckUnits, "minimumNckUnits"),
    maximumNckUnits: positiveU64(config.maximumNckUnits, "maximumNckUnits"),
    feeBps: config.feeBps,
  };
  validateSwapConfig(normalized);
  const data = Buffer.alloc(26);
  writeU64Le(data, 0, normalized.lamportsPerNck);
  writeU64Le(data, 8, normalized.minimumNckUnits);
  writeU64Le(data, 16, normalized.maximumNckUnits);
  data.writeUInt16LE(normalized.feeBps, 24);
  return data;
}

function encodeAmount(value: bigint | number | string): Buffer {
  const data = Buffer.alloc(8);
  writeU64Le(data, 0, positiveU64(value, "amount"));
  return data;
}

function readU64Le(data: Uint8Array, offset: number): bigint {
  let value = 0n;
  for (let index = 7; index >= 0; index -= 1) {
    value = (value << 8n) | BigInt(data[offset + index]);
  }
  return value;
}

function writeU64Le(data: Uint8Array, offset: number, value: bigint): void {
  let remaining = value;
  for (let index = 0; index < 8; index += 1) {
    data[offset + index] = Number(remaining & 0xffn);
    remaining >>= 8n;
  }
}

function decodeAscii(data: Uint8Array): string {
  return String.fromCharCode(...data);
}

function marketInstructionData(tag: number, payload: Uint8Array, unifiedGame: boolean): Buffer {
  const marketData = Buffer.concat([Buffer.from([tag]), Buffer.from(payload)]);
  return unifiedGame ? Buffer.concat([Buffer.from([MARKET_NAMESPACE]), marketData]) : marketData;
}

function validateSwapConfig(config: {
  lamportsPerNck: bigint;
  minimumNckUnits: bigint;
  maximumNckUnits: bigint;
  feeBps: number;
}): void {
  if (config.lamportsPerNck <= 0n
    || config.lamportsPerNck > U64_MAX
    || config.minimumNckUnits <= 0n
    || config.minimumNckUnits > U64_MAX
    || config.maximumNckUnits > U64_MAX
    || config.maximumNckUnits < config.minimumNckUnits
    || !Number.isInteger(config.feeBps)
    || config.feeBps < 0
    || config.feeBps > TREASURY_SWAP_MAX_FEE_BPS) {
    throw new Error("Invalid Treasury Swap configuration");
  }
}

function positiveU64(value: bigint | number | string, label: string): bigint {
  let normalized: bigint;
  if (typeof value === "number" && (!Number.isSafeInteger(value) || value <= 0)) {
    throw new Error(`${label} must be a positive u64`);
  }
  try {
    normalized = BigInt(value);
  } catch {
    throw new Error(`${label} must be a positive u64`);
  }
  if (normalized <= 0n || normalized > U64_MAX) throw new Error(`${label} must be a positive u64`);
  return normalized;
}

function requireTreasuryAdmin(admin: PublicKey): void {
  if (!admin.equals(NICECHUNK_MARKET_TREASURY)) {
    throw new Error(`Treasury Swap admin must be ${NICECHUNK_MARKET_TREASURY.toBase58()}`);
  }
}

function requireDevnetNckMint(mint: PublicKey): void {
  if (!mint.equals(NICECHUNK_DEVNET_NCK_MINT)) {
    throw new Error(`Treasury Swap NCK mint must be ${NICECHUNK_DEVNET_NCK_MINT.toBase58()}`);
  }
}
