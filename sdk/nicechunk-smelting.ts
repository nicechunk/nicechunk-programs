import {
  PublicKey,
  SystemProgram,
  TransactionInstruction,
} from "@solana/web3.js";
import { Buffer } from "buffer";
import {
  BACKPACK_SLOT_RECORD_LEN,
  deriveMaterialPhysicsPda,
  encodeBackpackSlotRecord,
  NICECHUNK_BACKPACK_PROGRAM_ID,
} from "./nicechunk-backpack.ts";
import type { BackpackSlotRecord } from "./nicechunk-backpack.ts";
import {
  deriveCivilizationAdapterAuthorityPda,
  NICECHUNK_CIVILIZATION_PROGRAM_ID,
} from "./nicechunk-civilization.ts";
import { deriveGlobalConfigPda, NICECHUNK_CORE_PROGRAM_ID } from "./nicechunk-core.ts";
import { derivePlayerProgressPda } from "./nicechunk-chunk.ts";

const env = typeof process !== "undefined" ? process.env : {};

export const NICECHUNK_SMELTING_PROGRAM_ID = new PublicKey(
  env.NICECHUNK_SMELTING_PROGRAM_ID ?? env.NICECHUNK_GAME_PROGRAM_ID ?? "6CurnvneezBuHwPUnrCiFg1QMWeUF67ufQxYebyr2UP7",
);
export const NICECHUNK_GAME_PROGRAM_ID = new PublicKey(
  env.NICECHUNK_GAME_PROGRAM_ID ?? "6CurnvneezBuHwPUnrCiFg1QMWeUF67ufQxYebyr2UP7",
);
export const RECIPE_TABLE_SEED = "smelting-recipes";
export const SMELTING_AUTHORITY_SEED = "smelting-authority";
const UNIFIED_GAME_SMELTING_NAMESPACE = 3;
export const RECIPE_TABLE_MAGIC = "NCKSMR01";
export const RECIPE_TABLE_HEADER_LEN = 96;
export const RECIPE_TABLE_MAX_RECIPES = 10;
export const RECIPE_MAX_INPUTS = 8;
export const RECIPE_MAX_OUTPUTS = 4;
export const RECIPE_YIELD_BPS_DENOMINATOR = 10_000;
export const RECIPE_RECORD_LEN =
  8 + 1 + 1 + 1 + 1 + 2 + 2 + RECIPE_MAX_INPUTS * BACKPACK_SLOT_RECORD_LEN + RECIPE_MAX_OUTPUTS * BACKPACK_SLOT_RECORD_LEN + 8;
export const RECIPE_TABLE_LEN = RECIPE_TABLE_HEADER_LEN + RECIPE_TABLE_MAX_RECIPES * RECIPE_RECORD_LEN;
export const UPSERT_RECIPE_ARGS_LEN =
  8 + 1 + 1 + 1 + 1 + 2 + 2 + RECIPE_MAX_INPUTS * BACKPACK_SLOT_RECORD_LEN + RECIPE_MAX_OUTPUTS * BACKPACK_SLOT_RECORD_LEN;

function smeltingInstructionData(programId: PublicKey, data: Buffer): Buffer {
  return programId.equals(NICECHUNK_GAME_PROGRAM_ID)
    ? Buffer.concat([Buffer.from([UNIFIED_GAME_SMELTING_NAMESPACE]), data])
    : data;
}

export interface SmeltingRecipeInput {
  recipeId: bigint | number;
  enabled?: boolean;
  minHeatTier?: number;
  yieldBps?: number;
  inputs: BackpackSlotRecord[];
  outputs: BackpackSlotRecord[];
}

export interface ApplyCivilizationSmeltingRecipeInput {
  executor: PublicKey;
  recipeTable: PublicKey;
  ruleBook: PublicKey;
  tally: PublicKey;
  receipt: PublicKey;
  recipe: SmeltingRecipeInput;
  smeltingProgramId?: PublicKey;
  civilizationProgramId?: PublicKey;
}

export function deriveRecipeTablePda({
  tableId,
  programId = NICECHUNK_SMELTING_PROGRAM_ID,
}: {
  tableId: bigint | number;
  programId?: PublicKey;
}): [PublicKey, number] {
  const tableIdBytes = Buffer.alloc(8);
  tableIdBytes.writeBigUInt64LE(BigInt(tableId), 0);
  return PublicKey.findProgramAddressSync(
    [Buffer.from(RECIPE_TABLE_SEED), tableIdBytes],
    programId,
  );
}

export function deriveSmeltingAuthorityPda(
  programId: PublicKey = NICECHUNK_SMELTING_PROGRAM_ID,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync([Buffer.from(SMELTING_AUTHORITY_SEED)], programId);
}

export function createInitializeRecipeTableInstruction({
  payer,
  tableId,
  smeltingProgramId = NICECHUNK_SMELTING_PROGRAM_ID,
}: {
  payer: PublicKey;
  tableId: bigint | number;
  smeltingProgramId?: PublicKey;
}): TransactionInstruction {
  const [recipeTable] = deriveRecipeTablePda({ tableId, programId: smeltingProgramId });
  const data = Buffer.alloc(9);
  data.writeUInt8(0, 0);
  data.writeBigUInt64LE(BigInt(tableId), 1);
  return new TransactionInstruction({
    programId: smeltingProgramId,
    keys: [
      { pubkey: payer, isSigner: true, isWritable: true },
      { pubkey: recipeTable, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: smeltingInstructionData(smeltingProgramId, data),
  });
}

export function createUpsertSmeltingRecipeInstruction({
  authority,
  recipeTable,
  recipe,
  smeltingProgramId = NICECHUNK_SMELTING_PROGRAM_ID,
}: {
  authority: PublicKey;
  recipeTable: PublicKey;
  recipe: SmeltingRecipeInput;
  smeltingProgramId?: PublicKey;
}): TransactionInstruction {
  return new TransactionInstruction({
    programId: smeltingProgramId,
    keys: [
      { pubkey: authority, isSigner: true, isWritable: false },
      { pubkey: recipeTable, isSigner: false, isWritable: true },
    ],
    data: smeltingInstructionData(smeltingProgramId, Buffer.concat([Buffer.from([1]), encodeSmeltingRecipeArgs(recipe)])),
  });
}

export function createDisableSmeltingRecipeInstruction({
  authority,
  recipeTable,
  recipe,
  smeltingProgramId = NICECHUNK_SMELTING_PROGRAM_ID,
}: {
  authority: PublicKey;
  recipeTable: PublicKey;
  recipe: SmeltingRecipeInput;
  smeltingProgramId?: PublicKey;
}): TransactionInstruction {
  return createUpsertSmeltingRecipeInstruction({
    authority,
    recipeTable,
    recipe: { ...recipe, enabled: false },
    smeltingProgramId,
  });
}

export function createExecuteSmeltingInstruction({
  owner,
  recipeTable,
  backpack,
  recipeId,
  inputIndexes,
  fuelIndexes,
  batchMultiplier = 1,
  smeltingProgramId = NICECHUNK_SMELTING_PROGRAM_ID,
  backpackProgramId = NICECHUNK_BACKPACK_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
}: {
  owner: PublicKey;
  recipeTable: PublicKey;
  backpack: PublicKey;
  recipeId: bigint | number;
  inputIndexes: number[];
  fuelIndexes: number[];
  batchMultiplier?: number;
  smeltingProgramId?: PublicKey;
  backpackProgramId?: PublicKey;
  coreProgramId?: PublicKey;
}): TransactionInstruction {
  const [smeltingAuthority] = deriveSmeltingAuthorityPda(smeltingProgramId);
  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  const [materialPhysics] = deriveMaterialPhysicsPda({
    globalConfig,
    programId: backpackProgramId,
  });
  const [playerProgress] = derivePlayerProgressPda({
    globalConfig,
    owner,
    programId: smeltingProgramId,
  });
  const indexes = inputIndexes.map((index) => Number(index));
  const fuels = fuelIndexes.map((index) => Number(index));
  const multiplier = Math.max(1, Math.min(0xffff, Math.floor(Number(batchMultiplier) || 1)));
  const data = Buffer.alloc(13 + indexes.length + fuels.length);
  data.writeUInt8(2, 0);
  data.writeBigUInt64LE(BigInt(recipeId), 1);
  data.writeUInt8(indexes.length, 9);
  data.writeUInt8(fuels.length, 10);
  data.writeUInt16LE(multiplier, 11);
  indexes.forEach((index, offset) => data.writeUInt8(index, 13 + offset));
  fuels.forEach((index, offset) => data.writeUInt8(index, 13 + indexes.length + offset));
  return new TransactionInstruction({
    programId: smeltingProgramId,
    keys: [
      { pubkey: owner, isSigner: true, isWritable: true },
      { pubkey: recipeTable, isSigner: false, isWritable: false },
      { pubkey: backpack, isSigner: false, isWritable: true },
      { pubkey: playerProgress, isSigner: false, isWritable: true },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
      { pubkey: materialPhysics, isSigner: false, isWritable: false },
      { pubkey: smeltingAuthority, isSigner: false, isWritable: false },
      { pubkey: backpackProgramId, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: smeltingInstructionData(smeltingProgramId, data),
  });
}

export function createSetRecipeTableAuthorityInstruction({
  authority,
  recipeTable,
  newAuthority,
  smeltingProgramId = NICECHUNK_SMELTING_PROGRAM_ID,
}: {
  authority: PublicKey;
  recipeTable: PublicKey;
  newAuthority: PublicKey;
  smeltingProgramId?: PublicKey;
}): TransactionInstruction {
  return new TransactionInstruction({
    programId: smeltingProgramId,
    keys: [
      { pubkey: authority, isSigner: true, isWritable: false },
      { pubkey: recipeTable, isSigner: false, isWritable: true },
      { pubkey: newAuthority, isSigner: false, isWritable: false },
    ],
    data: smeltingInstructionData(smeltingProgramId, Buffer.from([3])),
  });
}

export function createApplyCivilizationSmeltingRecipeInstruction({
  executor,
  recipeTable,
  ruleBook,
  tally,
  receipt,
  recipe,
  smeltingProgramId = NICECHUNK_SMELTING_PROGRAM_ID,
  civilizationProgramId = NICECHUNK_CIVILIZATION_PROGRAM_ID,
}: ApplyCivilizationSmeltingRecipeInput): TransactionInstruction {
  const [adapterAuthority] = deriveCivilizationAdapterAuthorityPda({
    ruleBook,
    targetProgram: smeltingProgramId,
  });
  return new TransactionInstruction({
    programId: smeltingProgramId,
    keys: [
      { pubkey: executor, isSigner: true, isWritable: true },
      { pubkey: recipeTable, isSigner: false, isWritable: true },
      { pubkey: civilizationProgramId, isSigner: false, isWritable: false },
      { pubkey: ruleBook, isSigner: false, isWritable: true },
      { pubkey: tally, isSigner: false, isWritable: false },
      { pubkey: receipt, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      { pubkey: adapterAuthority, isSigner: false, isWritable: false },
    ],
    data: smeltingInstructionData(
      smeltingProgramId,
      Buffer.concat([Buffer.from([4]), encodeCivilizationSmeltingRecipePatch(recipe)]),
    ),
  });
}

export function encodeCivilizationSmeltingRecipePatch(recipe: SmeltingRecipeInput): Buffer {
  validateSmeltingRecipeShape(recipe);
  const data = Buffer.alloc(16 + recipe.inputs.length * BACKPACK_SLOT_RECORD_LEN + recipe.outputs.length * BACKPACK_SLOT_RECORD_LEN);
  writeSmeltingRecipeHeader(data, recipe);
  let offset = 16;
  for (const slot of recipe.inputs) {
    encodeBackpackSlotRecord(slot).copy(data, offset);
    offset += BACKPACK_SLOT_RECORD_LEN;
  }
  for (const slot of recipe.outputs) {
    encodeBackpackSlotRecord(slot).copy(data, offset);
    offset += BACKPACK_SLOT_RECORD_LEN;
  }
  return data;
}

export function encodeSmeltingRecipeArgs(recipe: SmeltingRecipeInput): Buffer {
  validateSmeltingRecipeShape(recipe);
  const data = Buffer.alloc(UPSERT_RECIPE_ARGS_LEN);
  writeSmeltingRecipeHeader(data, recipe);
  let offset = 16;
  for (let index = 0; index < RECIPE_MAX_INPUTS; index += 1) {
    const slot = recipe.inputs[index] ?? recipe.inputs[0];
    encodeBackpackSlotRecord(slot).copy(data, offset);
    offset += BACKPACK_SLOT_RECORD_LEN;
  }
  for (let index = 0; index < RECIPE_MAX_OUTPUTS; index += 1) {
    const slot = recipe.outputs[index] ?? recipe.outputs[0];
    encodeBackpackSlotRecord(slot).copy(data, offset);
    offset += BACKPACK_SLOT_RECORD_LEN;
  }
  return data;
}

function validateSmeltingRecipeShape(recipe: SmeltingRecipeInput): void {
  if (!recipe.inputs.length || recipe.inputs.length > RECIPE_MAX_INPUTS) {
    throw new Error(`Smelting recipe inputs must be 1-${RECIPE_MAX_INPUTS}`);
  }
  if (!recipe.outputs.length || recipe.outputs.length > RECIPE_MAX_OUTPUTS) {
    throw new Error(`Smelting recipe outputs must be 1-${RECIPE_MAX_OUTPUTS}`);
  }
}

function writeSmeltingRecipeHeader(data: Buffer, recipe: SmeltingRecipeInput): void {
  data.writeBigUInt64LE(BigInt(recipe.recipeId), 0);
  data.writeUInt8(recipe.enabled === false ? 0 : 1, 8);
  data.writeUInt8(recipe.minHeatTier ?? 1, 9);
  data.writeUInt8(recipe.inputs.length, 10);
  data.writeUInt8(recipe.outputs.length, 11);
  const yieldBps = Math.max(1, Math.min(RECIPE_YIELD_BPS_DENOMINATOR, Math.floor(Number(recipe.yieldBps) || RECIPE_YIELD_BPS_DENOMINATOR)));
  data.writeUInt16LE(yieldBps, 12);
  data.writeUInt16LE(0, 14);
}
