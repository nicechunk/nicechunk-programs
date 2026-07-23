import {
  PublicKey,
  SystemProgram,
  TransactionInstruction,
} from "@solana/web3.js";
import { Buffer } from "buffer";
import {
  deriveGlobalConfigPda,
  NICECHUNK_CORE_PROGRAM_ID,
} from "./nicechunk-core.ts";
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
  env.NICECHUNK_BOOTSTRAP_AUTHORITY ?? NICECHUNK_BLUEPRINT_ISSUER.toBase58(),
);
const UNIFIED_GAME_BACKPACK_NAMESPACE = 1;
export const BACKPACK_SEED = "backpack";
export const BACKPACK_MAGIC = "NCKBPK01";
export const BACKPACK_VERSION = 4;
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
export const BACKPACK_FLAG_MASS_STATE_VALID = 1;
export const BACKPACK_FLAG_TOTAL_MASS_INITIALIZED = BACKPACK_FLAG_MASS_STATE_VALID;
export const BACKPACK_TOTAL_MASS_GRAMS_OFFSET = 90;
export const BACKPACK_LAST_MINE_PRE_MASS_GRAMS_OFFSET = 98;
export const BACKPACK_LAST_MINE_ACTION_ID_OFFSET = 106;
export const BACKPACK_MINE_SEQUENCE_OFFSET = 114;
export const MATERIAL_PHYSICS_SEED = "material-physics-v2";
export const MATERIAL_PHYSICS_MAGIC = "NCKPHY02";
export const MATERIAL_PHYSICS_VERSION = 2;
export const MATERIAL_PHYSICS_HEADER_LEN = 16;
export const MATERIAL_PHYSICS_RULE_LEN = 8;
export const MATERIAL_PHYSICS_MAX_RULES = 128;
export const MATERIAL_PHYSICS_LEN = MATERIAL_PHYSICS_HEADER_LEN
  + MATERIAL_PHYSICS_MAX_RULES * MATERIAL_PHYSICS_RULE_LEN;
export const MATERIAL_PHYSICS_ITEM_KEY_MASK = 1 << 15;
export const BLUEPRINT_ITEM_SEED = "blueprint-item";
export const BLUEPRINT_ITEM_MAGIC = "NCKBPT01";
export const BLUEPRINT_ITEM_VERSION = 1;
export const BLUEPRINT_ITEM_LEN = 96;
export const BACKPACK_DECORATION_METADATA_MASK = 0xffff;
export const VERIFIED_FORGE_CODE_MAX_BYTES = 640;

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
  totalMassGrams: bigint;
  lastMinePreMassGrams: bigint;
  lastMineActionId: bigint;
  mineSequence: bigint;
  records: BackpackResourceRecord[];
  slots: BackpackSlotRecord[];
}

export interface MaterialPhysicsRule {
  kind: "block" | "item";
  id: number;
  name?: string;
  densityKgM3: number;
  standardVolumeMm3: number;
}

export interface DecodedMaterialPhysicsTable {
  magic: string;
  version: number;
  bump: number;
  revision: number;
  ruleCount: number;
  rules: MaterialPhysicsRule[];
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
  backpackProgramId = NICECHUNK_BACKPACK_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
}: {
  globalConfig?: PublicKey;
  backpackProgramId?: PublicKey;
  coreProgramId?: PublicKey;
} = {}): [PublicKey, number] {
  const config = globalConfig ?? deriveGlobalConfigPda(coreProgramId)[0];
  return PublicKey.findProgramAddressSync(
    [Buffer.from(MATERIAL_PHYSICS_SEED), config.toBuffer()],
    backpackProgramId,
  );
}

export function createConfigureMaterialPhysicsInstruction({
  authority,
  revision,
  rules,
  globalConfig,
  backpackProgramId = NICECHUNK_BACKPACK_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
}: {
  authority: PublicKey;
  revision: number;
  rules: MaterialPhysicsRule[];
  globalConfig?: PublicKey;
  backpackProgramId?: PublicKey;
  coreProgramId?: PublicKey;
}): TransactionInstruction {
  const normalizedRevision = checkedUnsignedInteger(revision, 0xffffffff, "MaterialPhysics revision");
  if (normalizedRevision === 0) throw new Error("MaterialPhysics revision must be non-zero.");
  const normalizedRules = normalizeMaterialPhysicsRules(rules);
  const config = globalConfig ?? deriveGlobalConfigPda(coreProgramId)[0];
  const [materialPhysics] = deriveMaterialPhysicsPda({
    globalConfig: config,
    backpackProgramId,
    coreProgramId,
  });
  const data = Buffer.alloc(6 + normalizedRules.length * MATERIAL_PHYSICS_RULE_LEN);
  data.writeUInt8(12, 0);
  data.writeUInt32LE(normalizedRevision, 1);
  data.writeUInt8(normalizedRules.length, 5);
  normalizedRules.forEach((rule, index) => {
    const offset = 6 + index * MATERIAL_PHYSICS_RULE_LEN;
    data.writeUInt16LE(materialPhysicsRuleKey(rule), offset);
    data.writeUInt16LE(rule.densityKgM3, offset + 2);
    data.writeUInt32LE(rule.standardVolumeMm3, offset + 4);
  });
  return new TransactionInstruction({
    programId: backpackProgramId,
    keys: [
      { pubkey: authority, isSigner: true, isWritable: true },
      { pubkey: config, isSigner: false, isWritable: false },
      { pubkey: materialPhysics, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: backpackInstructionData(backpackProgramId, data),
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
  backpackProgramId = NICECHUNK_BACKPACK_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
}: {
  smeltingAuthority: PublicKey;
  owner: PublicKey;
  backpack: PublicKey;
  slot: BackpackSlotRecord;
  backpackProgramId?: PublicKey;
  coreProgramId?: PublicKey;
}): TransactionInstruction {
  const [materialPhysics] = deriveMaterialPhysicsPda({ backpackProgramId, coreProgramId });
  return new TransactionInstruction({
    programId: backpackProgramId,
    keys: [
      { pubkey: smeltingAuthority, isSigner: true, isWritable: false },
      { pubkey: owner, isSigner: false, isWritable: false },
      { pubkey: backpack, isSigner: false, isWritable: true },
      { pubkey: materialPhysics, isSigner: false, isWritable: false },
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
  const flags = data.readUInt8(55);
  if ((flags & BACKPACK_FLAG_MASS_STATE_VALID) === 0) {
    throw new Error("Invalid Backpack mass state.");
  }
  const records: BackpackResourceRecord[] = [];
  const slots: BackpackSlotRecord[] = [];
  const readableCount = Math.min(itemCount, capacity, BACKPACK_MAX_CAPACITY);
  for (let index = 0; index < readableCount; index += 1) {
    const offset = BACKPACK_HEADER_LEN + index * BACKPACK_SLOT_RECORD_LEN;
    const slot = decodeBackpackSlotRecord(data.subarray(offset, offset + BACKPACK_SLOT_RECORD_LEN));
    if (slot.massGrams === undefined) throw new Error(`Backpack slot ${index} has no authoritative mass.`);
    slots.push(slot);
    if (slot.kind === BACKPACK_SLOT_KIND_BLOCK) records.push(slot.resource);
  }
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
    totalMassGrams: data.readBigUInt64LE(BACKPACK_TOTAL_MASS_GRAMS_OFFSET),
    lastMinePreMassGrams: data.readBigUInt64LE(BACKPACK_LAST_MINE_PRE_MASS_GRAMS_OFFSET),
    lastMineActionId: data.readBigUInt64LE(BACKPACK_LAST_MINE_ACTION_ID_OFFSET),
    mineSequence: data.readBigUInt64LE(BACKPACK_MINE_SEQUENCE_OFFSET),
    records,
    slots,
  };
}

export function decodeMaterialPhysicsTable(data: Buffer | Uint8Array): DecodedMaterialPhysicsTable {
  const bytes = Buffer.from(data);
  if (bytes.length !== MATERIAL_PHYSICS_LEN) {
    throw new Error(`Invalid MaterialPhysics length: expected ${MATERIAL_PHYSICS_LEN}, got ${bytes.length}`);
  }
  const magic = bytes.subarray(0, 8).toString("utf8");
  if (magic !== MATERIAL_PHYSICS_MAGIC) throw new Error(`Invalid MaterialPhysics magic: ${magic}`);
  const version = bytes.readUInt8(8);
  if (version !== MATERIAL_PHYSICS_VERSION) {
    throw new Error(`Invalid MaterialPhysics version: expected ${MATERIAL_PHYSICS_VERSION}, got ${version}`);
  }
  const ruleCount = bytes.readUInt8(10);
  if (ruleCount < 1 || ruleCount > MATERIAL_PHYSICS_MAX_RULES) {
    throw new Error(`Invalid MaterialPhysics rule count: ${ruleCount}`);
  }
  const rules: MaterialPhysicsRule[] = [];
  let previousKey = -1;
  for (let index = 0; index < ruleCount; index += 1) {
    const offset = MATERIAL_PHYSICS_HEADER_LEN + index * MATERIAL_PHYSICS_RULE_LEN;
    const key = bytes.readUInt16LE(offset);
    if (key <= previousKey) throw new Error("MaterialPhysics rules must use unique ascending keys.");
    previousKey = key;
    const kind = (key & MATERIAL_PHYSICS_ITEM_KEY_MASK) !== 0 ? "item" : "block";
    const id = key & ~MATERIAL_PHYSICS_ITEM_KEY_MASK;
    const densityKgM3 = bytes.readUInt16LE(offset + 2);
    const standardVolumeMm3 = bytes.readUInt32LE(offset + 4);
    if (id === 0 || densityKgM3 === 0 || standardVolumeMm3 === 0) {
      throw new Error(`Invalid MaterialPhysics rule at index ${index}.`);
    }
    rules.push({ kind, id, densityKgM3, standardVolumeMm3 });
  }
  return {
    magic,
    version,
    bump: bytes.readUInt8(9),
    revision: bytes.readUInt32LE(12),
    ruleCount,
    rules,
  };
}

export function materialPhysicsMassGrams(
  rule: Pick<MaterialPhysicsRule, "densityKgM3">,
  volumeMm3: number,
): number {
  const density = checkedUnsignedInteger(rule.densityKgM3, 0xffff, "MaterialPhysics density");
  const volume = checkedUnsignedInteger(volumeMm3, 0xffffffff, "MaterialPhysics volume");
  if (density === 0 || volume === 0) throw new Error("MaterialPhysics density and volume must be non-zero.");
  const mass = (BigInt(volume) * BigInt(density) + 500_000n) / 1_000_000n;
  if (mass > 0xffffffffn) throw new Error("MaterialPhysics mass exceeds u32.");
  return Number(mass);
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
  const massGramsValue = slot.massGrams;
  const hasMass = massGramsValue !== undefined;
  const flags = (slot.flags ?? 0) | (hasMass ? BACKPACK_ITEM_FLAG_MASS_VALID : 0);
  data.writeUInt8(slot.kind, 0);
  data.writeUInt8(slot.category, 1);
  data.writeUInt16LE(flags, 2);
  data.writeUInt32LE(slot.quantity ?? 1, 4);
  data.writeInt32LE(slot.resource?.worldX ?? 0, 8);
  data.writeInt16LE(slot.resource?.worldY ?? 0, 12);
  data.writeInt32LE(slot.resource?.worldZ ?? 0, 14);
  data.writeUInt16LE(slot.itemCode ?? 0, 18);
  data.writeBigUInt64LE(BigInt(slot.itemId ?? 0), 20);
  (slot.itemPda ?? PublicKey.default).toBuffer().copy(data, 28);
  data.writeUInt32LE(Math.max(0, Math.min(0xffffffff, Math.floor(Number(slot.volumeMm3) || 0))), 60);
  data.writeUInt32LE(Math.max(0, Math.min(0xffffffff, Math.floor(Number(slot.durabilityCurrent) || 0))), 64);
  data.writeUInt32LE(Math.max(0, Math.min(0xffffffff, Math.floor(Number(slot.durabilityMax) || 0))), 68);
  data.writeUInt8(Math.max(0, Math.min(255, Math.floor(Number(slot.grade) || 0))), 72);
  data.writeUInt8(Math.max(0, Math.min(255, Math.floor(Number(slot.itemLevel) || 0))), 73);
  data.writeUInt16LE(Math.max(0, Math.min(0xffff, Math.floor(Number(slot.qualityBps) || 0))), 74);
  data.writeUInt32LE(Math.max(0, Math.min(0xffffffff, Math.floor(Number(slot.metadata) || 0))), 76);
  if (hasMass) {
    const massGrams = checkedUnsignedInteger(massGramsValue, 0xffffffff, "Backpack slot mass");
    data.writeUInt32LE(massGrams, slot.kind === BACKPACK_SLOT_KIND_BLOCK ? 64 : 8);
  }
  return data;
}

function normalizeMaterialPhysicsRules(rules: MaterialPhysicsRule[]): MaterialPhysicsRule[] {
  if (!Array.isArray(rules) || rules.length < 1 || rules.length > MATERIAL_PHYSICS_MAX_RULES) {
    throw new Error(`MaterialPhysics requires 1-${MATERIAL_PHYSICS_MAX_RULES} rules.`);
  }
  const normalized = rules.map((rule) => ({
    ...rule,
    kind: rule.kind,
    id: checkedUnsignedInteger(rule.id, MATERIAL_PHYSICS_ITEM_KEY_MASK - 1, "MaterialPhysics rule ID"),
    densityKgM3: checkedUnsignedInteger(rule.densityKgM3, 0xffff, "MaterialPhysics density"),
    standardVolumeMm3: checkedUnsignedInteger(rule.standardVolumeMm3, 0xffffffff, "MaterialPhysics standard volume"),
  }));
  for (const rule of normalized) {
    if ((rule.kind !== "block" && rule.kind !== "item")
      || rule.id === 0
      || rule.densityKgM3 === 0
      || rule.standardVolumeMm3 === 0) {
      throw new Error("Invalid MaterialPhysics rule.");
    }
  }
  normalized.sort((left, right) => materialPhysicsRuleKey(left) - materialPhysicsRuleKey(right));
  for (let index = 1; index < normalized.length; index += 1) {
    if (materialPhysicsRuleKey(normalized[index - 1]) === materialPhysicsRuleKey(normalized[index])) {
      throw new Error(`Duplicate MaterialPhysics rule key: ${materialPhysicsRuleKey(normalized[index])}`);
    }
  }
  return normalized;
}

function materialPhysicsRuleKey(rule: Pick<MaterialPhysicsRule, "kind" | "id">): number {
  return rule.kind === "item" ? rule.id | MATERIAL_PHYSICS_ITEM_KEY_MASK : rule.id;
}

function checkedUnsignedInteger(value: number, maximum: number, label: string): number {
  const normalized = Number(value);
  if (!Number.isInteger(normalized) || normalized < 0 || normalized > maximum) {
    throw new Error(`${label} must be an integer from 0 to ${maximum}.`);
  }
  return normalized;
}
