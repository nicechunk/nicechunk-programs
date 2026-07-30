import {
  PublicKey,
  SystemProgram,
  TransactionInstruction,
} from "@solana/web3.js";
import { Buffer } from "buffer";
import {
  BACKPACK_MAX_CAPACITY,
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
import { derivePlayerSkillsPda } from "./nicechunk-skills.ts";

const env = typeof process !== "undefined" ? process.env : {};

export const NICECHUNK_SMELTING_PROGRAM_ID = new PublicKey(
  env.NICECHUNK_SMELTING_PROGRAM_ID ?? env.NICECHUNK_GAME_PROGRAM_ID ?? "6CurnvneezBuHwPUnrCiFg1QMWeUF67ufQxYebyr2UP7",
);
export const NICECHUNK_GAME_PROGRAM_ID = new PublicKey(
  env.NICECHUNK_GAME_PROGRAM_ID ?? "6CurnvneezBuHwPUnrCiFg1QMWeUF67ufQxYebyr2UP7",
);
export const NICECHUNK_SMELTING_RECIPE_AUTHORITY = new PublicKey(
  "9XuoVVwqP2jipt3jpJVXCSS2N2jr9vDuV3d6K73FKVud",
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
const U64_MAX = 0xffffffffffffffffn;

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

export { deriveMaterialPhysicsPda };

export function createInitializeRecipeTableInstruction({
  payer,
  tableId,
  smeltingProgramId = NICECHUNK_SMELTING_PROGRAM_ID,
}: {
  payer: PublicKey;
  tableId: bigint | number;
  smeltingProgramId?: PublicKey;
}): TransactionInstruction {
  assertSmeltingRecipeAuthority(payer);
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
  assertSmeltingRecipeAuthority(authority);
  validateSmeltingRecipeOutputs(recipe, recipeTable);
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
  const selection = normalizeExecuteSmeltingSelection({
    recipeId,
    inputIndexes,
    fuelIndexes,
    batchMultiplier,
  });
  const [smeltingAuthority] = deriveSmeltingAuthorityPda(smeltingProgramId);
  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  const [materialPhysics] = deriveMaterialPhysicsPda({
    globalConfig,
    backpackProgramId,
  });
  const [playerProgress] = derivePlayerProgressPda({
    globalConfig,
    owner,
    programId: smeltingProgramId,
  });
  const [playerSkills] = derivePlayerSkillsPda({ owner, globalConfig });
  const { indexes, fuels, multiplier } = selection;
  const data = Buffer.alloc(13 + indexes.length + fuels.length);
  data.writeUInt8(2, 0);
  data.writeBigUInt64LE(selection.recipeId, 1);
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
      { pubkey: playerSkills, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: smeltingInstructionData(smeltingProgramId, data),
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
  validateSmeltingRecipeOutputs(recipe, recipeTable);
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
  normalizeU64(recipe.recipeId, "Smelting recipe id");
  if (!Array.isArray(recipe.inputs) || !recipe.inputs.length || recipe.inputs.length > RECIPE_MAX_INPUTS) {
    throw new Error(`Smelting recipe inputs must be 1-${RECIPE_MAX_INPUTS}`);
  }
  if (!Array.isArray(recipe.outputs) || !recipe.outputs.length || recipe.outputs.length > RECIPE_MAX_OUTPUTS) {
    throw new Error(`Smelting recipe outputs must be 1-${RECIPE_MAX_OUTPUTS}`);
  }
  const minHeatTier = recipe.minHeatTier ?? 1;
  if (!Number.isInteger(minHeatTier) || minHeatTier < 0 || minHeatTier > 0xff) {
    throw new Error("Smelting recipe heat tier must be an integer from 0 to 255");
  }
  const yieldBps = recipe.yieldBps ?? RECIPE_YIELD_BPS_DENOMINATOR;
  if (!Number.isInteger(yieldBps) || yieldBps < 1 || yieldBps > RECIPE_YIELD_BPS_DENOMINATOR) {
    throw new Error(`Smelting recipe yield must be an integer from 1 to ${RECIPE_YIELD_BPS_DENOMINATOR}`);
  }
  if (recipe.enabled !== undefined && typeof recipe.enabled !== "boolean") {
    throw new Error("Smelting recipe enabled must be a boolean");
  }
}

function validateSmeltingRecipeOutputs(recipe: SmeltingRecipeInput, recipeTable: PublicKey): void {
  validateSmeltingRecipeShape(recipe);
  for (const output of recipe.outputs) {
    if (
      output.kind !== 2
      || output.category !== 1
      || !output.itemPda.equals(recipeTable)
    ) {
      throw new Error("Every smelting output must be a material Item backed by its RecipeTable PDA.");
    }
  }
}

function assertSmeltingRecipeAuthority(authority: PublicKey): void {
  if (!authority.equals(NICECHUNK_SMELTING_RECIPE_AUTHORITY)) {
    throw new Error(`Smelting recipe authority must be ${NICECHUNK_SMELTING_RECIPE_AUTHORITY.toBase58()}.`);
  }
}

function writeSmeltingRecipeHeader(data: Buffer, recipe: SmeltingRecipeInput): void {
  const recipeId = normalizeU64(recipe.recipeId, "Smelting recipe id");
  const minHeatTier = recipe.minHeatTier ?? 1;
  const yieldBps = recipe.yieldBps ?? RECIPE_YIELD_BPS_DENOMINATOR;
  data.writeBigUInt64LE(recipeId, 0);
  data.writeUInt8(recipe.enabled === false ? 0 : 1, 8);
  data.writeUInt8(minHeatTier, 9);
  data.writeUInt8(recipe.inputs.length, 10);
  data.writeUInt8(recipe.outputs.length, 11);
  data.writeUInt16LE(yieldBps, 12);
  data.writeUInt16LE(0, 14);
}

function normalizeExecuteSmeltingSelection({
  recipeId,
  inputIndexes,
  fuelIndexes,
  batchMultiplier,
}: {
  recipeId: bigint | number;
  inputIndexes: number[];
  fuelIndexes: number[];
  batchMultiplier: number;
}): {
  recipeId: bigint;
  indexes: number[];
  fuels: number[];
  multiplier: number;
} {
  const normalizedRecipeId = normalizeU64(recipeId, "Smelting recipe id");
  const indexes = normalizeBackpackIndexList(inputIndexes, "Smelting input indexes");
  const fuels = normalizeBackpackIndexList(fuelIndexes, "Smelting fuel indexes");
  if (!indexes.length) throw new Error("Smelting requires at least one input index");
  if (indexes.length + fuels.length > BACKPACK_MAX_CAPACITY) {
    throw new Error(`Smelting supports at most ${BACKPACK_MAX_CAPACITY} total input and fuel indexes`);
  }
  if (new Set([...indexes, ...fuels]).size !== indexes.length + fuels.length) {
    throw new Error("Smelting input and fuel indexes must be unique");
  }
  if (!Number.isSafeInteger(batchMultiplier) || batchMultiplier < 1 || batchMultiplier > 0xffff) {
    throw new Error("Smelting batch multiplier must be an integer from 1 to 65535");
  }
  return { recipeId: normalizedRecipeId, indexes, fuels, multiplier: batchMultiplier };
}

function normalizeBackpackIndexList(value: number[], label: string): number[] {
  if (!Array.isArray(value)) throw new Error(`${label} must be an array`);
  if (value.some((index) => !Number.isInteger(index) || index < 0 || index >= BACKPACK_MAX_CAPACITY)) {
    throw new Error(`${label} must contain integers from 0 to ${BACKPACK_MAX_CAPACITY - 1}`);
  }
  return [...value];
}

function normalizeU64(value: bigint | number, label: string): bigint {
  if (typeof value === "number" && !Number.isSafeInteger(value)) {
    throw new Error(`${label} must be a safe integer or bigint`);
  }
  let normalized: bigint;
  try {
    normalized = BigInt(value);
  } catch {
    throw new Error(`${label} must be an integer`);
  }
  if (normalized < 1n || normalized > U64_MAX) {
    throw new Error(`${label} must be between 1 and ${U64_MAX}`);
  }
  return normalized;
}
