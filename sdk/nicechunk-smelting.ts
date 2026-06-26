import {
  PublicKey,
  SystemProgram,
  TransactionInstruction,
} from "@solana/web3.js";
import {
  BACKPACK_SLOT_RECORD_LEN,
  encodeBackpackSlotRecord,
  NICECHUNK_BACKPACK_PROGRAM_ID,
} from "./nicechunk-backpack.ts";
import type { BackpackSlotRecord } from "./nicechunk-backpack.ts";

const env = typeof process !== "undefined" ? process.env : {};

export const NICECHUNK_SMELTING_PROGRAM_ID = new PublicKey(
  env.NICECHUNK_SMELTING_PROGRAM_ID ?? "7imEiNtpiN487HRwrftdLrMFs8TcAnjLE94vKsDgU6L7",
);
export const RECIPE_TABLE_SEED = "smelting-recipes";
export const SMELTING_AUTHORITY_SEED = "smelting-authority";
export const RECIPE_TABLE_MAGIC = "NCKSMR01";
export const RECIPE_TABLE_HEADER_LEN = 96;
export const RECIPE_TABLE_MAX_RECIPES = 12;
export const RECIPE_MAX_INPUTS = 8;
export const RECIPE_MAX_OUTPUTS = 4;
export const RECIPE_RECORD_LEN =
  8 + 1 + 1 + 1 + 1 + RECIPE_MAX_INPUTS * BACKPACK_SLOT_RECORD_LEN + RECIPE_MAX_OUTPUTS * BACKPACK_SLOT_RECORD_LEN + 8;
export const RECIPE_TABLE_LEN = RECIPE_TABLE_HEADER_LEN + RECIPE_TABLE_MAX_RECIPES * RECIPE_RECORD_LEN;
export const UPSERT_RECIPE_ARGS_LEN =
  8 + 1 + 1 + 1 + 1 + RECIPE_MAX_INPUTS * BACKPACK_SLOT_RECORD_LEN + RECIPE_MAX_OUTPUTS * BACKPACK_SLOT_RECORD_LEN;

export interface SmeltingRecipeInput {
  recipeId: bigint | number;
  enabled?: boolean;
  minHeatTier?: number;
  inputs: BackpackSlotRecord[];
  outputs: BackpackSlotRecord[];
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
    data,
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
    data: Buffer.concat([Buffer.from([1]), encodeSmeltingRecipeArgs(recipe)]),
  });
}

export function createExecuteSmeltingInstruction({
  owner,
  recipeTable,
  backpack,
  recipeId,
  inputIndexes,
  fuelIndexes,
  smeltingProgramId = NICECHUNK_SMELTING_PROGRAM_ID,
  backpackProgramId = NICECHUNK_BACKPACK_PROGRAM_ID,
}: {
  owner: PublicKey;
  recipeTable: PublicKey;
  backpack: PublicKey;
  recipeId: bigint | number;
  inputIndexes: number[];
  fuelIndexes: number[];
  smeltingProgramId?: PublicKey;
  backpackProgramId?: PublicKey;
}): TransactionInstruction {
  const [smeltingAuthority] = deriveSmeltingAuthorityPda(smeltingProgramId);
  const indexes = inputIndexes.map((index) => Number(index));
  const fuels = fuelIndexes.map((index) => Number(index));
  const data = Buffer.alloc(11 + indexes.length + fuels.length);
  data.writeUInt8(2, 0);
  data.writeBigUInt64LE(BigInt(recipeId), 1);
  data.writeUInt8(indexes.length, 9);
  data.writeUInt8(fuels.length, 10);
  indexes.forEach((index, offset) => data.writeUInt8(index, 11 + offset));
  fuels.forEach((index, offset) => data.writeUInt8(index, 11 + indexes.length + offset));
  return new TransactionInstruction({
    programId: smeltingProgramId,
    keys: [
      { pubkey: owner, isSigner: true, isWritable: true },
      { pubkey: recipeTable, isSigner: false, isWritable: false },
      { pubkey: backpack, isSigner: false, isWritable: true },
      { pubkey: smeltingAuthority, isSigner: false, isWritable: false },
      { pubkey: backpackProgramId, isSigner: false, isWritable: false },
    ],
    data,
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
    data: Buffer.from([3]),
  });
}

export function encodeSmeltingRecipeArgs(recipe: SmeltingRecipeInput): Buffer {
  if (!recipe.inputs.length || recipe.inputs.length > RECIPE_MAX_INPUTS) {
    throw new Error(`Smelting recipe inputs must be 1-${RECIPE_MAX_INPUTS}`);
  }
  if (!recipe.outputs.length || recipe.outputs.length > RECIPE_MAX_OUTPUTS) {
    throw new Error(`Smelting recipe outputs must be 1-${RECIPE_MAX_OUTPUTS}`);
  }
  const data = Buffer.alloc(UPSERT_RECIPE_ARGS_LEN);
  data.writeBigUInt64LE(BigInt(recipe.recipeId), 0);
  data.writeUInt8(recipe.enabled === false ? 0 : 1, 8);
  data.writeUInt8(recipe.minHeatTier ?? 1, 9);
  data.writeUInt8(recipe.inputs.length, 10);
  data.writeUInt8(recipe.outputs.length, 11);
  let offset = 12;
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
