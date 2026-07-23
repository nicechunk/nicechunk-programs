import {
  PublicKey,
  SystemProgram,
  TransactionInstruction,
} from "@solana/web3.js";
import { Buffer } from "buffer";
import {
  derivePlayerProfilePda,
  NICECHUNK_PLAYER_PROGRAM_ID,
} from "./nicechunk-player.ts";

const env = typeof process !== "undefined" ? process.env : {};

export const NICECHUNK_BACKPACK_PROGRAM_ID = new PublicKey(
  env.NICECHUNK_BACKPACK_PROGRAM_ID ?? env.NICECHUNK_GAME_PROGRAM_ID ?? "6CurnvneezBuHwPUnrCiFg1QMWeUF67ufQxYebyr2UP7",
);
export const NICECHUNK_GAME_PROGRAM_ID = new PublicKey(
  env.NICECHUNK_GAME_PROGRAM_ID ?? "6CurnvneezBuHwPUnrCiFg1QMWeUF67ufQxYebyr2UP7",
);
export const NICECHUNK_BLUEPRINT_ISSUER = new PublicKey(
  env.NICECHUNK_BLUEPRINT_ISSUER ?? "9XuoVVwqP2jipt3jpJVXCSS2N2jr9vDuV3d6K73FKVud",
);
export const NICECHUNK_BOOTSTRAP_AUTHORITY = new PublicKey(
  env.NICECHUNK_BOOTSTRAP_AUTHORITY ?? "9XuoVVwqP2jipt3jpJVXCSS2N2jr9vDuV3d6K73FKVud",
);
const UNIFIED_GAME_BACKPACK_NAMESPACE = 1;
export const BACKPACK_SEED = "backpack";
export const BACKPACK_MAGIC = "NCKBPK01";
export const BACKPACK_VERSION = 3;
export const BACKPACK_DEFAULT_CAPACITY = 50;
export const BACKPACK_MAX_CAPACITY = 99;
export const BACKPACK_HEADER_LEN = 128;
export const BACKPACK_RESOURCE_RECORD_LEN = 10;
export const BACKPACK_SLOT_RECORD_LEN = 80;
export const BACKPACK_RECORD_LEN = BACKPACK_SLOT_RECORD_LEN;
export const BACKPACK_LEN = BACKPACK_HEADER_LEN + BACKPACK_MAX_CAPACITY * BACKPACK_RECORD_LEN;
export const BACKPACK_SLOT_KIND_BLOCK = 1;
export const BACKPACK_SLOT_KIND_ITEM = 2;
export const BACKPACK_ITEM_CATEGORY_MATERIAL = 1;
export const BACKPACK_ITEM_CATEGORY_FORGED = 2;
export const BACKPACK_ITEM_CATEGORY_BLUEPRINT = 3;
export const BACKPACK_FORGED_ITEM_CODE = 8;
export const BACKPACK_BLUEPRINT_ITEM_CODE = 9;
export const BACKPACK_ITEM_FLAG_UNIQUE = 1;
export const BACKPACK_ITEM_FLAG_MASS_VALID = 1 << 15;
export const BACKPACK_FLAG_TOTAL_MASS_INITIALIZED = 1;
export const BACKPACK_TOTAL_MASS_GRAMS_OFFSET = 90;
export const BACKPACK_LAST_MINE_PRE_MASS_GRAMS_OFFSET = 98;
export const BACKPACK_LAST_MINE_ACTION_ID_OFFSET = 106;
export const BACKPACK_MINE_SEQUENCE_OFFSET = 114;
export const BLUEPRINT_ITEM_SEED = "blueprint-item";
export const BLUEPRINT_ITEM_MAGIC = "NCKBPT01";
export const BLUEPRINT_ITEM_VERSION = 1;
export const BLUEPRINT_ITEM_LEN = 96;
export const BACKPACK_DECORATION_METADATA_MASK = 0xffff;
export const VERIFIED_FORGE_CODE_MAX_BYTES = 640;
export const MATERIAL_PHYSICS_SEED = "material-physics-v1";
export const MATERIAL_PHYSICS_MAGIC = "NCKPHY01";
export const MATERIAL_PHYSICS_VERSION = 1;
export const MATERIAL_PHYSICS_HEADER_LEN = 128;
export const MATERIAL_PHYSICS_RECORD_LEN = 4;
export const MATERIAL_PHYSICS_MAX_RECORDS = 240;
export const MATERIAL_PHYSICS_LEN = MATERIAL_PHYSICS_HEADER_LEN
  + MATERIAL_PHYSICS_MAX_RECORDS * MATERIAL_PHYSICS_RECORD_LEN;

function backpackInstructionData(programId: PublicKey, data: Buffer): Buffer {
  return programId.equals(NICECHUNK_GAME_PROGRAM_ID)
    ? Buffer.concat([Buffer.from([UNIFIED_GAME_BACKPACK_NAMESPACE]), data])
    : data;
}

export interface BackpackResourceRecord {
  worldX: number;
  worldY: number;
  worldZ: number;
}

export interface BackpackSlotRecord {
  kind: number;
  category: number;
  flags: number;
  quantity: number;
  resource: BackpackResourceRecord;
  itemCode: number;
  itemId: bigint;
  itemPda: PublicKey;
  volumeMm3?: number;
  durabilityCurrent?: number;
  durabilityMax?: number;
  grade?: number;
  itemLevel?: number;
  qualityBps?: number;
  metadata?: number;
  massGrams?: number;
}

export interface DecodedBackpack {
  magic: string;
  version: number;
  bump: number;
  initialized: boolean;
  backpackId: bigint;
  owner: PublicKey;
  capacity: number;
  itemCount: number;
  state: number;
  flags: number;
  placed: { x: number; y: number; z: number };
  createdSlot: bigint;
  updatedSlot: bigint;
  createdAt: bigint;
  massInitialized: boolean;
  totalMassGrams: bigint;
  lastMinePreMassGrams: bigint;
  lastMineActionId: bigint;
  mineSequence: bigint;
  records: BackpackResourceRecord[];
  slots: BackpackSlotRecord[];
}

export interface MaterialPhysicsRecord {
  materialId: number;
  densityKgM3: number;
}

export interface DecodedMaterialPhysics {
  magic: string;
  version: number;
  bump: number;
  initialized: boolean;
  authority: PublicKey;
  globalConfig: PublicKey;
  revision: number;
  recordCount: number;
  createdSlot: bigint;
  updatedSlot: bigint;
  createdAt: bigint;
  records: MaterialPhysicsRecord[];
}

export interface BackpackDecorationMetadata {
  ruleId: number;
  decorationId: number;
}

export interface DecodedBlueprintItem {
  magic: string;
  version: number;
  bump: number;
  initialized: boolean;
  itemId: bigint;
  owner: PublicKey;
  issuer: PublicKey;
  createdSlot: bigint;
}

export function encodeBackpackDecorationMetadata({
  ruleId,
  decorationId,
}: BackpackDecorationMetadata): number {
  const normalizedRuleId = Math.max(0, Math.min(0xffff, Math.trunc(Number(ruleId) || 0)));
  const normalizedDecorationId = Math.max(0, Math.min(0xffff, Math.trunc(Number(decorationId) || 0)));
  if (!normalizedRuleId || !normalizedDecorationId) return 0;
  return ((normalizedRuleId << 16) | normalizedDecorationId) >>> 0;
}

export function decodeBackpackDecorationMetadata(metadata: number): BackpackDecorationMetadata | null {
  const value = Math.trunc(Number(metadata) || 0) >>> 0;
  const ruleId = value >>> 16;
  const decorationId = value & BACKPACK_DECORATION_METADATA_MASK;
  return ruleId && decorationId ? { ruleId, decorationId } : null;
}

export function deriveBackpackPda({
  creator,
  backpackId,
  programId = NICECHUNK_BACKPACK_PROGRAM_ID,
}: {
  creator: PublicKey;
  backpackId: bigint | number;
  programId?: PublicKey;
}): [PublicKey, number] {
  const backpackIdBytes = Buffer.alloc(8);
  backpackIdBytes.writeBigUInt64LE(BigInt(backpackId), 0);
  return PublicKey.findProgramAddressSync(
    [Buffer.from(BACKPACK_SEED), creator.toBuffer(), backpackIdBytes],
    programId,
  );
}

export function deriveBlueprintItemPda({
  itemId,
  programId = NICECHUNK_BACKPACK_PROGRAM_ID,
}: {
  itemId: bigint | number;
  programId?: PublicKey;
}): [PublicKey, number] {
  const itemIdBytes = Buffer.alloc(8);
  itemIdBytes.writeBigUInt64LE(BigInt(itemId), 0);
  return PublicKey.findProgramAddressSync(
    [Buffer.from(BLUEPRINT_ITEM_SEED), itemIdBytes],
    programId,
  );
}

export function deriveMaterialPhysicsPda({
  globalConfig,
  programId = NICECHUNK_BACKPACK_PROGRAM_ID,
}: {
  globalConfig: PublicKey;
  programId?: PublicKey;
}): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(MATERIAL_PHYSICS_SEED), globalConfig.toBuffer()],
    programId,
  );
}

export function createInitializeMaterialPhysicsInstruction({
  authority,
  globalConfig,
  backpackProgramId = NICECHUNK_BACKPACK_PROGRAM_ID,
}: {
  authority: PublicKey;
  globalConfig: PublicKey;
  backpackProgramId?: PublicKey;
}): TransactionInstruction {
  const [materialPhysics] = deriveMaterialPhysicsPda({
    globalConfig,
    programId: backpackProgramId,
  });
  return new TransactionInstruction({
    programId: backpackProgramId,
    keys: [
      { pubkey: authority, isSigner: true, isWritable: true },
      { pubkey: materialPhysics, isSigner: false, isWritable: true },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: backpackInstructionData(backpackProgramId, Buffer.from([12])),
  });
}

export function createReplaceMaterialPhysicsInstruction({
  authority,
  globalConfig,
  records,
  backpackProgramId = NICECHUNK_BACKPACK_PROGRAM_ID,
}: {
  authority: PublicKey;
  globalConfig: PublicKey;
  records: MaterialPhysicsRecord[];
  backpackProgramId?: PublicKey;
}): TransactionInstruction {
  const normalized = [...(records ?? [])]
    .map((record) => ({
      materialId: Math.trunc(Number(record.materialId)),
      densityKgM3: Math.trunc(Number(record.densityKgM3)),
    }))
    .sort((left, right) => left.materialId - right.materialId);
  if (!normalized.length || normalized.length > MATERIAL_PHYSICS_MAX_RECORDS) {
    throw new Error(`Material physics requires 1-${MATERIAL_PHYSICS_MAX_RECORDS} records.`);
  }
  normalized.forEach((record, index) => {
    if (record.materialId <= 0
      || record.materialId > 0xffff
      || record.densityKgM3 <= 0
      || record.densityKgM3 > 0xffff
      || (index > 0 && record.materialId === normalized[index - 1].materialId)) {
      throw new Error("Material physics records require unique uint16 IDs and densities.");
    }
  });
  const [materialPhysics] = deriveMaterialPhysicsPda({
    globalConfig,
    programId: backpackProgramId,
  });
  const data = Buffer.alloc(2 + normalized.length * MATERIAL_PHYSICS_RECORD_LEN);
  data.writeUInt8(13, 0);
  data.writeUInt8(normalized.length, 1);
  normalized.forEach((record, index) => {
    const offset = 2 + index * MATERIAL_PHYSICS_RECORD_LEN;
    data.writeUInt16LE(record.materialId, offset);
    data.writeUInt16LE(record.densityKgM3, offset + 2);
  });
  return new TransactionInstruction({
    programId: backpackProgramId,
    keys: [
      { pubkey: authority, isSigner: true, isWritable: false },
      { pubkey: materialPhysics, isSigner: false, isWritable: true },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
    ],
    data: backpackInstructionData(backpackProgramId, data),
  });
}

export function createMigrateBackpackMassInstruction({
  owner,
  backpack,
  globalConfig,
  backpackProgramId = NICECHUNK_BACKPACK_PROGRAM_ID,
}: {
  owner: PublicKey;
  backpack: PublicKey;
  globalConfig: PublicKey;
  backpackProgramId?: PublicKey;
}): TransactionInstruction {
  const [materialPhysics] = deriveMaterialPhysicsPda({
    globalConfig,
    programId: backpackProgramId,
  });
  return new TransactionInstruction({
    programId: backpackProgramId,
    keys: [
      { pubkey: owner, isSigner: true, isWritable: false },
      { pubkey: backpack, isSigner: false, isWritable: true },
      { pubkey: materialPhysics, isSigner: false, isWritable: false },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
    ],
    data: backpackInstructionData(backpackProgramId, Buffer.from([14])),
  });
}

export function createInitializeBackpackInstruction({
  payer,
  backpackId,
  capacity = BACKPACK_DEFAULT_CAPACITY,
  backpackProgramId = NICECHUNK_BACKPACK_PROGRAM_ID,
  playerProgramId = NICECHUNK_PLAYER_PROGRAM_ID,
}: {
  payer: PublicKey;
  backpackId: bigint | number;
  capacity?: number;
  backpackProgramId?: PublicKey;
  playerProgramId?: PublicKey;
}): TransactionInstruction {
  const [backpack] = deriveBackpackPda({ creator: payer, backpackId, programId: backpackProgramId });
  const [playerProfile] = derivePlayerProfilePda(payer, playerProgramId);
  const data = Buffer.alloc(10);
  data.writeUInt8(0, 0);
  data.writeBigUInt64LE(BigInt(backpackId), 1);
  data.writeUInt8(capacity, 9);
  return new TransactionInstruction({
    programId: backpackProgramId,
    keys: [
      { pubkey: payer, isSigner: true, isWritable: true },
      { pubkey: playerProfile, isSigner: false, isWritable: false },
      { pubkey: backpack, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: backpackInstructionData(backpackProgramId, data),
  });
}

export function createAppendSmeltingItemInstruction({
  smeltingAuthority,
  owner,
  backpack,
  slot,
  globalConfig,
  backpackProgramId = NICECHUNK_BACKPACK_PROGRAM_ID,
}: {
  smeltingAuthority: PublicKey;
  owner: PublicKey;
  backpack: PublicKey;
  slot: BackpackSlotRecord;
  globalConfig: PublicKey;
  backpackProgramId?: PublicKey;
}): TransactionInstruction {
  const [materialPhysics] = deriveMaterialPhysicsPda({
    globalConfig,
    programId: backpackProgramId,
  });
  return new TransactionInstruction({
    programId: backpackProgramId,
    keys: [
      { pubkey: smeltingAuthority, isSigner: true, isWritable: false },
      { pubkey: owner, isSigner: false, isWritable: false },
      { pubkey: backpack, isSigner: false, isWritable: true },
      { pubkey: materialPhysics, isSigner: false, isWritable: false },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
    ],
    data: backpackInstructionData(backpackProgramId, Buffer.concat([Buffer.from([5]), encodeBackpackSlotRecord(slot)])),
  });
}

export function createIssueBlueprintInstruction({
  issuer,
  recipient,
  backpack,
  itemId,
  backpackProgramId = NICECHUNK_BACKPACK_PROGRAM_ID,
}: {
  issuer: PublicKey;
  recipient: PublicKey;
  backpack: PublicKey;
  itemId: bigint | number;
  backpackProgramId?: PublicKey;
}): TransactionInstruction {
  const normalizedItemId = BigInt(itemId);
  if (normalizedItemId <= 0n || normalizedItemId > 0xffffffffffffffffn) {
    throw new Error("Blueprint item ID must be an unsigned non-zero 64-bit integer.");
  }
  const [blueprintItem] = deriveBlueprintItemPda({ itemId: normalizedItemId, programId: backpackProgramId });
  const data = Buffer.alloc(9);
  data.writeUInt8(9, 0);
  data.writeBigUInt64LE(normalizedItemId, 1);
  return new TransactionInstruction({
    programId: backpackProgramId,
    keys: [
      { pubkey: issuer, isSigner: true, isWritable: true },
      { pubkey: recipient, isSigner: false, isWritable: false },
      { pubkey: backpack, isSigner: false, isWritable: true },
      { pubkey: blueprintItem, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: backpackInstructionData(backpackProgramId, data),
  });
}

export function createForgeEquipmentInstruction({
  owner,
  backpack,
  itemId,
  codeBytes,
  inputIndexes,
  backpackProgramId = NICECHUNK_BACKPACK_PROGRAM_ID,
  playerProgramId = NICECHUNK_PLAYER_PROGRAM_ID,
}: {
  owner: PublicKey;
  backpack: PublicKey;
  itemId: bigint | number;
  codeBytes: Uint8Array;
  inputIndexes: number[];
  backpackProgramId?: PublicKey;
  playerProgramId?: PublicKey;
}): TransactionInstruction {
  const indexes = Array.from(new Set((inputIndexes ?? [])
    .map((index) => Number(index))
    .filter((index) => Number.isInteger(index) && index >= 0 && index <= BACKPACK_MAX_CAPACITY - 1)));
  if (!indexes.length || indexes.length > 24) {
    throw new Error("Forge equipment requires 1-24 material indexes.");
  }
  const canonicalBytes = Buffer.from(codeBytes ?? []);
  if (!canonicalBytes.length || canonicalBytes.length > VERIFIED_FORGE_CODE_MAX_BYTES) {
    throw new Error(`Forge equipment requires 1-${VERIFIED_FORGE_CODE_MAX_BYTES} canonical NCF1 bytes.`);
  }
  const [playerProfile] = derivePlayerProfilePda(owner, playerProgramId);
  const data = Buffer.alloc(12 + canonicalBytes.length + indexes.length);
  data.writeUInt8(8, 0);
  data.writeBigUInt64LE(BigInt(itemId), 1);
  data.writeUInt16LE(canonicalBytes.length, 9);
  data.writeUInt8(indexes.length, 11);
  canonicalBytes.copy(data, 12);
  indexes.forEach((index, offset) => data.writeUInt8(index, 12 + canonicalBytes.length + offset));
  return new TransactionInstruction({
    programId: backpackProgramId,
    keys: [
      { pubkey: owner, isSigner: true, isWritable: true },
      { pubkey: playerProfile, isSigner: false, isWritable: true },
      { pubkey: backpack, isSigner: false, isWritable: true },
      { pubkey: playerProgramId, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: backpackInstructionData(backpackProgramId, data),
  });
}

export function decodeBackpack(data: Buffer): DecodedBackpack {
  if (data.length !== BACKPACK_LEN) {
    throw new Error(`Invalid Backpack length: expected ${BACKPACK_LEN}, got ${data.length}`);
  }
  const magic = data.subarray(0, 8).toString("utf8");
  if (magic !== BACKPACK_MAGIC) throw new Error(`Invalid Backpack magic: ${magic}`);
  const version = data.readUInt16LE(8);
  if (version !== BACKPACK_VERSION) throw new Error(`Invalid Backpack version: expected ${BACKPACK_VERSION}, got ${version}`);
  const capacity = data.readUInt8(52);
  const itemCount = data.readUInt8(53);
  const records: BackpackResourceRecord[] = [];
  const slots: BackpackSlotRecord[] = [];
  const readableCount = Math.min(itemCount, capacity, BACKPACK_MAX_CAPACITY);
  for (let index = 0; index < readableCount; index += 1) {
    const offset = BACKPACK_HEADER_LEN + index * BACKPACK_SLOT_RECORD_LEN;
    const slot = decodeBackpackSlotRecord(data.subarray(offset, offset + BACKPACK_SLOT_RECORD_LEN));
    slots.push(slot);
    if (slot.kind === BACKPACK_SLOT_KIND_BLOCK) records.push(slot.resource);
  }
  const flags = data.readUInt8(55);
  return {
    magic,
    version,
    bump: data.readUInt8(10),
    initialized: data.readUInt8(11) === 1,
    backpackId: data.readBigUInt64LE(12),
    owner: new PublicKey(data.subarray(20, 52)),
    capacity,
    itemCount,
    state: data.readUInt8(54),
    flags,
    placed: {
      x: data.readInt32LE(56),
      y: data.readInt16LE(60),
      z: data.readInt32LE(62),
    },
    createdSlot: data.readBigUInt64LE(66),
    updatedSlot: data.readBigUInt64LE(74),
    createdAt: data.readBigInt64LE(82),
    massInitialized: (flags & BACKPACK_FLAG_TOTAL_MASS_INITIALIZED) !== 0,
    totalMassGrams: data.readBigUInt64LE(BACKPACK_TOTAL_MASS_GRAMS_OFFSET),
    lastMinePreMassGrams: data.readBigUInt64LE(BACKPACK_LAST_MINE_PRE_MASS_GRAMS_OFFSET),
    lastMineActionId: data.readBigUInt64LE(BACKPACK_LAST_MINE_ACTION_ID_OFFSET),
    mineSequence: data.readBigUInt64LE(BACKPACK_MINE_SEQUENCE_OFFSET),
    records,
    slots,
  };
}

export function decodeMaterialPhysics(data: Buffer): DecodedMaterialPhysics {
  if (data.length !== MATERIAL_PHYSICS_LEN) {
    throw new Error(`Invalid MaterialPhysics length: expected ${MATERIAL_PHYSICS_LEN}, got ${data.length}`);
  }
  const magic = data.subarray(0, 8).toString("utf8");
  if (magic !== MATERIAL_PHYSICS_MAGIC) {
    throw new Error(`Invalid MaterialPhysics magic: ${magic}`);
  }
  const version = data.readUInt16LE(8);
  if (version !== MATERIAL_PHYSICS_VERSION) {
    throw new Error(`Invalid MaterialPhysics version: expected ${MATERIAL_PHYSICS_VERSION}, got ${version}`);
  }
  const recordCount = data.readUInt8(80);
  if (recordCount > MATERIAL_PHYSICS_MAX_RECORDS) {
    throw new Error(`Invalid MaterialPhysics record count: ${recordCount}`);
  }
  const records: MaterialPhysicsRecord[] = [];
  for (let index = 0; index < recordCount; index += 1) {
    const offset = MATERIAL_PHYSICS_HEADER_LEN + index * MATERIAL_PHYSICS_RECORD_LEN;
    const record = {
      materialId: data.readUInt16LE(offset),
      densityKgM3: data.readUInt16LE(offset + 2),
    };
    if (!record.materialId
      || !record.densityKgM3
      || (index > 0 && record.materialId <= records[index - 1].materialId)) {
      throw new Error(`Invalid MaterialPhysics record at index ${index}.`);
    }
    records.push(record);
  }
  return {
    magic,
    version,
    bump: data.readUInt8(10),
    initialized: data.readUInt8(11) === 1,
    authority: new PublicKey(data.subarray(12, 44)),
    globalConfig: new PublicKey(data.subarray(44, 76)),
    revision: data.readUInt32LE(76),
    recordCount,
    createdSlot: data.readBigUInt64LE(84),
    updatedSlot: data.readBigUInt64LE(92),
    createdAt: data.readBigInt64LE(100),
    records,
  };
}

export function decodeBlueprintItem(data: Buffer): DecodedBlueprintItem {
  if (data.length !== BLUEPRINT_ITEM_LEN) {
    throw new Error(`Invalid Blueprint item length: expected ${BLUEPRINT_ITEM_LEN}, got ${data.length}`);
  }
  const magic = data.subarray(0, 8).toString("utf8");
  if (magic !== BLUEPRINT_ITEM_MAGIC) throw new Error(`Invalid Blueprint item magic: ${magic}`);
  const version = data.readUInt16LE(8);
  if (version !== BLUEPRINT_ITEM_VERSION) {
    throw new Error(`Invalid Blueprint item version: expected ${BLUEPRINT_ITEM_VERSION}, got ${version}`);
  }
  const itemId = data.readBigUInt64LE(12);
  if (!itemId) throw new Error("Invalid zero Blueprint item ID.");
  return {
    magic,
    version,
    bump: data.readUInt8(10),
    initialized: data.readUInt8(11) === 1,
    itemId,
    owner: new PublicKey(data.subarray(20, 52)),
    issuer: new PublicKey(data.subarray(52, 84)),
    createdSlot: data.readBigUInt64LE(84),
  };
}

export function backpackSlotFromResource(resource: BackpackResourceRecord): BackpackSlotRecord {
  return {
    kind: BACKPACK_SLOT_KIND_BLOCK,
    category: 0,
    flags: 0,
    quantity: 1,
    resource,
    itemCode: 0,
    itemId: 0n,
    itemPda: PublicKey.default,
    volumeMm3: 0,
    durabilityCurrent: 0,
    durabilityMax: 0,
    grade: 0,
    itemLevel: 0,
    qualityBps: 0,
    metadata: 0,
  };
}

export function decodeBackpackResourceRecord(data: Buffer): BackpackResourceRecord {
  if (data.length !== BACKPACK_RESOURCE_RECORD_LEN) {
    throw new Error(`Invalid Backpack resource length: expected ${BACKPACK_RESOURCE_RECORD_LEN}, got ${data.length}`);
  }
  return {
    worldX: data.readInt32LE(0),
    worldY: data.readInt16LE(4),
    worldZ: data.readInt32LE(6),
  };
}

export function decodeBackpackSlotRecord(data: Buffer): BackpackSlotRecord {
  if (data.length !== BACKPACK_SLOT_RECORD_LEN) {
    throw new Error(`Invalid Backpack slot length: expected ${BACKPACK_SLOT_RECORD_LEN}, got ${data.length}`);
  }
  const kind = data.readUInt8(0);
  const flags = data.readUInt16LE(2);
  return {
    kind,
    category: data.readUInt8(1),
    flags,
    quantity: data.readUInt32LE(4),
    resource: decodeBackpackResourceRecord(data.subarray(8, 18)),
    itemCode: data.readUInt16LE(18),
    itemId: data.readBigUInt64LE(20),
    itemPda: new PublicKey(data.subarray(28, 60)),
    volumeMm3: data.readUInt32LE(60),
    durabilityCurrent: data.readUInt32LE(64),
    durabilityMax: data.readUInt32LE(68),
    grade: data.readUInt8(72),
    itemLevel: data.readUInt8(73),
    qualityBps: data.readUInt16LE(74),
    metadata: data.readUInt32LE(76),
    massGrams: (flags & BACKPACK_ITEM_FLAG_MASS_VALID) !== 0
      ? (kind === BACKPACK_SLOT_KIND_BLOCK ? data.readUInt32LE(64) : data.readUInt32LE(8))
      : undefined,
  };
}

export function encodeBackpackSlotRecord(slot: BackpackSlotRecord): Buffer {
  const data = Buffer.alloc(BACKPACK_SLOT_RECORD_LEN);
  const hasExplicitMass = Number.isFinite(Number(slot.massGrams));
  const flags = (slot.flags ?? 0) | (hasExplicitMass ? BACKPACK_ITEM_FLAG_MASS_VALID : 0);
  const massGrams = Math.max(0, Math.min(0xffffffff, Math.floor(Number(slot.massGrams) || 0)));
  data.writeUInt8(slot.kind, 0);
  data.writeUInt8(slot.category, 1);
  data.writeUInt16LE(flags, 2);
  data.writeUInt32LE(slot.quantity ?? 1, 4);
  if (hasExplicitMass && slot.kind !== BACKPACK_SLOT_KIND_BLOCK) {
    data.writeUInt32LE(massGrams, 8);
  } else {
    data.writeInt32LE(slot.resource?.worldX ?? 0, 8);
  }
  data.writeInt16LE(slot.resource?.worldY ?? 0, 12);
  data.writeInt32LE(slot.resource?.worldZ ?? 0, 14);
  data.writeUInt16LE(slot.itemCode ?? 0, 18);
  data.writeBigUInt64LE(BigInt(slot.itemId ?? 0), 20);
  (slot.itemPda ?? PublicKey.default).toBuffer().copy(data, 28);
  data.writeUInt32LE(Math.max(0, Math.min(0xffffffff, Math.floor(Number(slot.volumeMm3) || 0))), 60);
  data.writeUInt32LE(hasExplicitMass && slot.kind === BACKPACK_SLOT_KIND_BLOCK
    ? massGrams
    : Math.max(0, Math.min(0xffffffff, Math.floor(Number(slot.durabilityCurrent) || 0))), 64);
  data.writeUInt32LE(Math.max(0, Math.min(0xffffffff, Math.floor(Number(slot.durabilityMax) || 0))), 68);
  data.writeUInt8(Math.max(0, Math.min(255, Math.floor(Number(slot.grade) || 0))), 72);
  data.writeUInt8(Math.max(0, Math.min(255, Math.floor(Number(slot.itemLevel) || 0))), 73);
  data.writeUInt16LE(Math.max(0, Math.min(0xffff, Math.floor(Number(slot.qualityBps) || 0))), 74);
  data.writeUInt32LE(Math.max(0, Math.min(0xffffffff, Math.floor(Number(slot.metadata) || 0))), 76);
  return data;
}
