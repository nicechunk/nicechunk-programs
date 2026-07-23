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

const env = typeof process !== "undefined" ? process.env : {};

export const NICECHUNK_PLAYER_PROGRAM_ID = new PublicKey(
  env.NICECHUNK_PLAYER_PROGRAM_ID ?? "CHZHsBCGn58ih2WrPfKSYhvCEjMPGhArTiYCH7AWWBkB",
);
export const PLAYER_PROFILE_SEED = "player-v7";
export const PLAYER_APPEARANCE_SEED = "appearance-v1";
export const PLAYER_EQUIPMENT_SEED = "player-equipment-v1";
export const PLAYER_SESSION_SEED = "session";
export const PLAYER_PROFILE_LEN = 773;
export const PLAYER_APPEARANCE_LEN = 9612;
export const PLAYER_EQUIPMENT_HEADER_LEN = 128;
export const PLAYER_EQUIPMENT_SLOT_LEN = 768;
export const PLAYER_EQUIPMENT_MODEL_CODE_MAX_BYTES = 640;
export const PLAYER_EQUIPMENT_LEN = PLAYER_EQUIPMENT_HEADER_LEN + 9 * PLAYER_EQUIPMENT_SLOT_LEN;
export const PLAYER_EQUIPMENT_FLAG_MODEL = 1 << 0;
export const PLAYER_EQUIPMENT_FLAG_CUSTODY = 1 << 1;
export const EQUIPMENT_TRANSFER_AUTHORITY_SEED = "equipment-transfer-v1";
export const MATERIAL_PHYSICS_SEED = "material-physics-v1";
export const PLAYER_SESSION_LEN = 184;
export const PLAYER_PROFILE_MAGIC = "NCKPLY01";
export const PLAYER_APPEARANCE_MAGIC = "NCKAPP01";
export const PLAYER_EQUIPMENT_MAGIC = "NCKEQP01";
export const PLAYER_SESSION_MAGIC = "NCKSES01";
export const PLAYER_NAME_MAX_CHARS = 100;
export const PLAYER_NAME_MAX_BYTES = 300;
export const APPEARANCE_TITLE_MAX_BYTES = 96;
export const APPEARANCE_MODEL_CODE_MAX_BYTES = 2048;
export const APPEARANCE_EQUIPMENT_SLOT_COUNT = 12;
export const APPEARANCE_EQUIPMENT_SLOT_LEN = 576;
export const APPEARANCE_EQUIPMENT_CODE_MAX_BYTES = 512;
export const NICECHUNK_BACKPACK_PROGRAM_ID = new PublicKey(
  env.NICECHUNK_BACKPACK_PROGRAM_ID ?? env.NICECHUNK_GAME_PROGRAM_ID ?? "6CurnvneezBuHwPUnrCiFg1QMWeUF67ufQxYebyr2UP7",
);
export const EQUIPMENT_SLOT_COUNT = 9;
export const CLEAR_EQUIPMENT_BACKPACK_INDEX = 255;
export const SESSION_ACTION_BREAK_BLOCK = 1 << 1;
export const SESSION_ACTION_PLACE_BLOCK = 1 << 2;

export interface DecodedPlayerProfile {
  magic: string;
  version: number;
  bump: number;
  initialized: boolean;
  owner: PublicKey;
  globalConfig: PublicKey;
  worldId: number;
  position: { x: number; y: number; z: number };
  attributes: {
    health: number;
    energy: number;
    stamina: number;
    miningPower: number;
    buildPower: number;
    defense: number;
  };
  equipmentSlotCount: number;
  equipment: PublicKey[];
  backpackStyle: number;
  backpackFlags: number;
  equippedBackpack: PublicKey;
  createdSlot: bigint;
  updatedSlot: bigint;
  createdAt: bigint;
  forgingXp: bigint;
  forgedItemCount: number;
  bestForgedGrade: number;
  bestForgedItemLevel: number;
  playerName: string;
}

export interface DecodedAppearanceEquipmentSlot {
  state: number;
  slot: number;
  equipped: boolean;
  flags: number;
  itemPda: PublicKey;
  massGrams: number;
  gripPoint: { x: number; y: number; z: number };
  gripRotation: { x: number; y: number; z: number };
  modelCode: string;
}

export interface DecodedPlayerAppearance {
  magic: string;
  version: number;
  bump: number;
  initialized: boolean;
  owner: PublicKey;
  playerProfile: PublicKey;
  globalConfig: PublicKey;
  treasuryAuthority: PublicKey;
  modelKind: number;
  flags: number;
  displayName: string;
  title: string;
  modelCode: string;
  equipment: DecodedAppearanceEquipmentSlot[];
  createdSlot: bigint;
  updatedSlot: bigint;
  createdAt: bigint;
  updatedAt: bigint;
}

export interface DecodedPlayerSession {
  magic: string;
  version: number;
  bump: number;
  active: boolean;
  owner: PublicKey;
  sessionAuthority: PublicKey;
  playerProfile: PublicKey;
  globalConfig: PublicKey;
  worldId: number;
  allowedActions: number;
  expiresAt: bigint;
  createdSlot: bigint;
  updatedSlot: bigint;
  createdAt: bigint;
  maxActions: number;
  actionCount: number;
}

export interface DecodedPlayerEquipmentSlot {
  state: number;
  slot: number;
  equipped: boolean;
  custodied: boolean;
  backpackIndex: number;
  flags: number;
  backpack: PublicKey;
  backpackSlot: Buffer;
  kindCode: number;
  category: number;
  quantity: number;
  itemCode: number;
  itemId: bigint;
  itemPda: PublicKey;
  metadata: number;
  modelCode: Buffer;
}

export interface DecodedPlayerEquipment {
  magic: string;
  version: number;
  bump: number;
  initialized: boolean;
  owner: PublicKey;
  playerProfile: PublicKey;
  globalConfig: PublicKey;
  slotCount: number;
  createdSlot: bigint;
  updatedSlot: bigint;
  slots: DecodedPlayerEquipmentSlot[];
}

export function derivePlayerProfilePda(
  owner: PublicKey,
  programId: PublicKey = NICECHUNK_PLAYER_PROGRAM_ID,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(PLAYER_PROFILE_SEED), owner.toBuffer()],
    programId,
  );
}

export function derivePlayerAppearancePda(
  owner: PublicKey,
  programId: PublicKey = NICECHUNK_PLAYER_PROGRAM_ID,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(PLAYER_APPEARANCE_SEED), owner.toBuffer()],
    programId,
  );
}

export function derivePlayerEquipmentPda(
  owner: PublicKey,
  programId: PublicKey = NICECHUNK_PLAYER_PROGRAM_ID,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(PLAYER_EQUIPMENT_SEED), owner.toBuffer()],
    programId,
  );
}

export function deriveEquipmentTransferAuthorityPda(
  programId: PublicKey = NICECHUNK_PLAYER_PROGRAM_ID,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(EQUIPMENT_TRANSFER_AUTHORITY_SEED)],
    programId,
  );
}

export const derivePlayerCharacterPda = derivePlayerAppearancePda;

export function derivePlayerSessionPda({
  owner,
  sessionAuthority,
  programId = NICECHUNK_PLAYER_PROGRAM_ID,
}: {
  owner: PublicKey;
  sessionAuthority: PublicKey;
  programId?: PublicKey;
}): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(PLAYER_SESSION_SEED), owner.toBuffer(), sessionAuthority.toBuffer()],
    programId,
  );
}

export function createInitializePlayerInstruction({
  payer,
  playerName = "",
  playerProgramId = NICECHUNK_PLAYER_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
}: {
  payer: PublicKey;
  playerName?: string;
  playerProgramId?: PublicKey;
  coreProgramId?: PublicKey;
}): TransactionInstruction {
  const [playerProfile] = derivePlayerProfilePda(payer, playerProgramId);
  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  const nameBytes = encodePlayerName(playerName);
  return new TransactionInstruction({
    programId: playerProgramId,
    keys: [
      { pubkey: payer, isSigner: true, isWritable: true },
      { pubkey: playerProfile, isSigner: false, isWritable: true },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: Buffer.concat([Buffer.from([0]), nameBytes]),
  });
}

export function createSetPlayerNameInstruction({
  authority,
  playerName,
  playerProgramId = NICECHUNK_PLAYER_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
}: {
  authority: PublicKey;
  playerName: string;
  playerProgramId?: PublicKey;
  coreProgramId?: PublicKey;
}): TransactionInstruction {
  const [playerProfile] = derivePlayerProfilePda(authority, playerProgramId);
  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  return new TransactionInstruction({
    programId: playerProgramId,
    keys: [
      { pubkey: authority, isSigner: true, isWritable: false },
      { pubkey: playerProfile, isSigner: false, isWritable: true },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
    ],
    data: Buffer.concat([Buffer.from([7]), encodePlayerName(playerName)]),
  });
}

export function createUpsertPlayerAppearanceInstruction({
  authority,
  displayName,
  title = "",
  modelKind = 1,
  modelCode,
  playerProgramId = NICECHUNK_PLAYER_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
}: {
  authority: PublicKey;
  displayName: string;
  title?: string;
  modelKind?: number;
  modelCode: string;
  playerProgramId?: PublicKey;
  coreProgramId?: PublicKey;
}): TransactionInstruction {
  const [playerProfile] = derivePlayerProfilePda(authority, playerProgramId);
  const [appearance] = derivePlayerAppearancePda(authority, playerProgramId);
  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  const nameBytes = encodePlayerName(displayName);
  const titleBytes = Buffer.from(String(title ?? "").trim(), "utf8");
  const codeBytes = Buffer.from(String(modelCode ?? "").trim(), "utf8");
  if (titleBytes.length > APPEARANCE_TITLE_MAX_BYTES) {
    throw new Error(`Appearance title is too large: max ${APPEARANCE_TITLE_MAX_BYTES} UTF-8 bytes.`);
  }
  if (!codeBytes.length || codeBytes.length > APPEARANCE_MODEL_CODE_MAX_BYTES || !codeBytes.toString("utf8").startsWith("NCM")) {
    throw new Error(`Invalid appearance model code: max ${APPEARANCE_MODEL_CODE_MAX_BYTES} UTF-8 bytes.`);
  }
  const header = Buffer.alloc(8);
  header.writeUInt8(8, 0);
  header.writeUInt8(modelKind === 2 ? 2 : 1, 1);
  header.writeUInt16LE(nameBytes.length, 2);
  header.writeUInt16LE(titleBytes.length, 4);
  header.writeUInt16LE(codeBytes.length, 6);
  return new TransactionInstruction({
    programId: playerProgramId,
    keys: [
      { pubkey: authority, isSigner: true, isWritable: true },
      { pubkey: playerProfile, isSigner: false, isWritable: true },
      { pubkey: appearance, isSigner: false, isWritable: true },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: Buffer.concat([header, nameBytes, titleBytes, codeBytes]),
  });
}

export function createUpdatePlayerPositionInstruction({
  authority,
  x,
  y,
  z,
  playerProgramId = NICECHUNK_PLAYER_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
}: {
  authority: PublicKey;
  x: number;
  y: number;
  z: number;
  playerProgramId?: PublicKey;
  coreProgramId?: PublicKey;
}): TransactionInstruction {
  const [playerProfile] = derivePlayerProfilePda(authority, playerProgramId);
  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  const data = Buffer.alloc(13);
  data.writeUInt8(1, 0);
  data.writeInt32LE(x, 1);
  data.writeInt32LE(y, 5);
  data.writeInt32LE(z, 9);
  return new TransactionInstruction({
    programId: playerProgramId,
    keys: [
      { pubkey: authority, isSigner: true, isWritable: false },
      { pubkey: playerProfile, isSigner: false, isWritable: true },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
    ],
    data,
  });
}

export function createSetEquipmentSlotInstruction({
  authority,
  slot,
  backpack = null,
  backpackSlotIndex = CLEAR_EQUIPMENT_BACKPACK_INDEX,
  playerProgramId = NICECHUNK_PLAYER_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
}: {
  authority: PublicKey;
  slot: number;
  backpack?: PublicKey | null;
  backpackSlotIndex?: number;
  playerProgramId?: PublicKey;
  coreProgramId?: PublicKey;
}): TransactionInstruction {
  const [playerProfile] = derivePlayerProfilePda(authority, playerProgramId);
  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  const normalizedSlot = Math.max(0, Math.min(255, Math.floor(slot)));
  const normalizedBackpackSlotIndex = Math.max(0, Math.min(255, Math.floor(backpackSlotIndex)));
  const clearsSlot = normalizedBackpackSlotIndex === CLEAR_EQUIPMENT_BACKPACK_INDEX;
  if (!clearsSlot && !backpack) {
    throw new Error("Equipping an item requires a backpack PDA.");
  }
  const data = Buffer.from([2, normalizedSlot, normalizedBackpackSlotIndex]);
  const keys = [
    { pubkey: authority, isSigner: true, isWritable: false },
    { pubkey: playerProfile, isSigner: false, isWritable: true },
    { pubkey: globalConfig, isSigner: false, isWritable: false },
  ];
  if (!clearsSlot && backpack) {
    keys.push({ pubkey: backpack, isSigner: false, isWritable: false });
  }
  return new TransactionInstruction({
    programId: playerProgramId,
    keys,
    data,
  });
}

export function createSetPlayerEquipmentSlotInstruction({
  authority,
  slot,
  backpack = null,
  backpackSlotIndex = CLEAR_EQUIPMENT_BACKPACK_INDEX,
  modelCode = new Uint8Array(),
  playerProgramId = NICECHUNK_PLAYER_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
}: {
  authority: PublicKey;
  slot: number;
  backpack?: PublicKey | null;
  backpackSlotIndex?: number;
  modelCode?: Uint8Array | Buffer;
  playerProgramId?: PublicKey;
  coreProgramId?: PublicKey;
}): TransactionInstruction {
  const [playerProfile] = derivePlayerProfilePda(authority, playerProgramId);
  const [playerEquipment] = derivePlayerEquipmentPda(authority, playerProgramId);
  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  const normalizedSlot = Math.max(0, Math.min(255, Math.floor(slot)));
  const normalizedBackpackSlotIndex = Math.max(0, Math.min(255, Math.floor(backpackSlotIndex)));
  const clearsSlot = normalizedBackpackSlotIndex === CLEAR_EQUIPMENT_BACKPACK_INDEX;
  const codeBytes = Buffer.from(modelCode ?? []);
  if (codeBytes.length > PLAYER_EQUIPMENT_MODEL_CODE_MAX_BYTES) {
    throw new Error(`Equipment model is too large: max ${PLAYER_EQUIPMENT_MODEL_CODE_MAX_BYTES} bytes.`);
  }
  if (clearsSlot && codeBytes.length) throw new Error("Clearing equipment cannot include model bytes.");
  if (!clearsSlot && !backpack) throw new Error("Equipping an item requires a backpack PDA.");
  const header = Buffer.alloc(5);
  header.writeUInt8(12, 0);
  header.writeUInt8(normalizedSlot, 1);
  header.writeUInt8(normalizedBackpackSlotIndex, 2);
  header.writeUInt16LE(codeBytes.length, 3);
  const keys = [
    { pubkey: authority, isSigner: true, isWritable: true },
    { pubkey: playerProfile, isSigner: false, isWritable: true },
    { pubkey: playerEquipment, isSigner: false, isWritable: true },
    { pubkey: globalConfig, isSigner: false, isWritable: false },
    { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
  ];
  if (!clearsSlot && backpack) keys.push({ pubkey: backpack, isSigner: false, isWritable: false });
  return new TransactionInstruction({
    programId: playerProgramId,
    keys,
    data: Buffer.concat([header, codeBytes]),
  });
}

export function createTransferPlayerEquipmentSlotInstruction({
  authority,
  slot,
  backpack,
  backpackSlotIndex = CLEAR_EQUIPMENT_BACKPACK_INDEX,
  modelCode = new Uint8Array(),
  playerProgramId = NICECHUNK_PLAYER_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
  gameProgramId = NICECHUNK_BACKPACK_PROGRAM_ID,
}: {
  authority: PublicKey;
  slot: number;
  backpack: PublicKey;
  backpackSlotIndex?: number;
  modelCode?: Uint8Array | Buffer;
  playerProgramId?: PublicKey;
  coreProgramId?: PublicKey;
  gameProgramId?: PublicKey;
}): TransactionInstruction {
  const [playerProfile] = derivePlayerProfilePda(authority, playerProgramId);
  const [playerEquipment] = derivePlayerEquipmentPda(authority, playerProgramId);
  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  const [transferAuthority] = deriveEquipmentTransferAuthorityPda(playerProgramId);
  const [materialPhysics] = PublicKey.findProgramAddressSync(
    [Buffer.from(MATERIAL_PHYSICS_SEED), globalConfig.toBuffer()],
    gameProgramId,
  );
  const normalizedSlot = Math.max(0, Math.min(255, Math.floor(slot)));
  const normalizedBackpackSlotIndex = Math.max(0, Math.min(255, Math.floor(backpackSlotIndex)));
  const clearsSlot = normalizedBackpackSlotIndex === CLEAR_EQUIPMENT_BACKPACK_INDEX;
  const codeBytes = Buffer.from(modelCode ?? []);
  if (codeBytes.length > PLAYER_EQUIPMENT_MODEL_CODE_MAX_BYTES) {
    throw new Error(`Equipment model is too large: max ${PLAYER_EQUIPMENT_MODEL_CODE_MAX_BYTES} bytes.`);
  }
  if (clearsSlot && codeBytes.length) throw new Error("Clearing equipment cannot include model bytes.");
  const header = Buffer.alloc(5);
  header.writeUInt8(13, 0);
  header.writeUInt8(normalizedSlot, 1);
  header.writeUInt8(normalizedBackpackSlotIndex, 2);
  header.writeUInt16LE(codeBytes.length, 3);
  return new TransactionInstruction({
    programId: playerProgramId,
    keys: [
      { pubkey: authority, isSigner: true, isWritable: true },
      { pubkey: playerProfile, isSigner: false, isWritable: true },
      { pubkey: playerEquipment, isSigner: false, isWritable: true },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
      { pubkey: materialPhysics, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      { pubkey: backpack, isSigner: false, isWritable: true },
      { pubkey: gameProgramId, isSigner: false, isWritable: false },
      { pubkey: transferAuthority, isSigner: false, isWritable: false },
    ],
    data: Buffer.concat([header, codeBytes]),
  });
}

export function createSwapPlayerEquipmentSlotsInstruction({
  authority,
  fromSlot,
  toSlot,
  playerProgramId = NICECHUNK_PLAYER_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
}: {
  authority: PublicKey;
  fromSlot: number;
  toSlot: number;
  playerProgramId?: PublicKey;
  coreProgramId?: PublicKey;
}): TransactionInstruction {
  const [playerProfile] = derivePlayerProfilePda(authority, playerProgramId);
  const [playerEquipment] = derivePlayerEquipmentPda(authority, playerProgramId);
  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  const from = Math.max(0, Math.min(255, Math.floor(fromSlot)));
  const to = Math.max(0, Math.min(255, Math.floor(toSlot)));
  if (from === to) throw new Error("Equipment swap requires two different slots.");
  return new TransactionInstruction({
    programId: playerProgramId,
    keys: [
      { pubkey: authority, isSigner: true, isWritable: false },
      { pubkey: playerProfile, isSigner: false, isWritable: true },
      { pubkey: playerEquipment, isSigner: false, isWritable: true },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
    ],
    data: Buffer.from([14, from, to]),
  });
}

export function createSetBackpackStyleInstruction({
  authority,
  backpackStyle,
  playerProgramId = NICECHUNK_PLAYER_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
}: {
  authority: PublicKey;
  backpackStyle: number;
  playerProgramId?: PublicKey;
  coreProgramId?: PublicKey;
}): TransactionInstruction {
  const [playerProfile] = derivePlayerProfilePda(authority, playerProgramId);
  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  return new TransactionInstruction({
    programId: playerProgramId,
    keys: [
      { pubkey: authority, isSigner: true, isWritable: false },
      { pubkey: playerProfile, isSigner: false, isWritable: true },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
    ],
    data: Buffer.from([3, backpackStyle]),
  });
}

export function createOrRefreshPlayerSessionInstruction({
  owner,
  sessionAuthority,
  expiresAt,
  allowedActions = SESSION_ACTION_BREAK_BLOCK | SESSION_ACTION_PLACE_BLOCK,
  maxActions = 10_000,
  playerProgramId = NICECHUNK_PLAYER_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
}: {
  owner: PublicKey;
  sessionAuthority: PublicKey;
  expiresAt: bigint | number;
  allowedActions?: number;
  maxActions?: number;
  playerProgramId?: PublicKey;
  coreProgramId?: PublicKey;
}): TransactionInstruction {
  const [playerProfile] = derivePlayerProfilePda(owner, playerProgramId);
  const [playerSession] = derivePlayerSessionPda({
    owner,
    sessionAuthority,
    programId: playerProgramId,
  });
  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  const data = Buffer.alloc(15);
  data.writeUInt8(4, 0);
  data.writeBigInt64LE(BigInt(expiresAt), 1);
  data.writeUInt16LE(allowedActions, 9);
  data.writeUInt32LE(maxActions, 11);
  return new TransactionInstruction({
    programId: playerProgramId,
    keys: [
      { pubkey: owner, isSigner: true, isWritable: true },
      { pubkey: sessionAuthority, isSigner: true, isWritable: false },
      { pubkey: playerProfile, isSigner: false, isWritable: false },
      { pubkey: playerSession, isSigner: false, isWritable: true },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data,
  });
}

export function createSetEquippedBackpackInstruction({
  authority,
  backpack,
  playerProgramId = NICECHUNK_PLAYER_PROGRAM_ID,
}: {
  authority: PublicKey;
  backpack: PublicKey;
  playerProgramId?: PublicKey;
}): TransactionInstruction {
  const [playerProfile] = derivePlayerProfilePda(authority, playerProgramId);
  return new TransactionInstruction({
    programId: playerProgramId,
    keys: [
      { pubkey: authority, isSigner: true, isWritable: true },
      { pubkey: playerProfile, isSigner: false, isWritable: true },
      { pubkey: backpack, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: Buffer.from([5]),
  });
}

export function decodePlayerProfile(data: Buffer): DecodedPlayerProfile {
  if (data.length !== PLAYER_PROFILE_LEN) {
    throw new Error(`Invalid PlayerProfile length: expected ${PLAYER_PROFILE_LEN}, got ${data.length}`);
  }

  let offset = 0;
  const bytes = (length: number): Buffer => {
    const value = data.subarray(offset, offset + length);
    offset += length;
    return value;
  };
  const u8 = (): number => data.readUInt8(offset++);
  const u16 = (): number => {
    const value = data.readUInt16LE(offset);
    offset += 2;
    return value;
  };
  const u32 = (): number => {
    const value = data.readUInt32LE(offset);
    offset += 4;
    return value;
  };
  const i32 = (): number => {
    const value = data.readInt32LE(offset);
    offset += 4;
    return value;
  };
  const u64 = (): bigint => {
    const value = data.readBigUInt64LE(offset);
    offset += 8;
    return value;
  };
  const i64 = (): bigint => {
    const value = data.readBigInt64LE(offset);
    offset += 8;
    return value;
  };
  const pubkey = (): PublicKey => new PublicKey(bytes(32));

  const decoded: DecodedPlayerProfile = {
    magic: bytes(8).toString("utf8"),
    version: u16(),
    bump: u8(),
    initialized: u8() === 1,
    owner: pubkey(),
    globalConfig: pubkey(),
    worldId: u16(),
    position: { x: i32(), y: i32(), z: i32() },
    attributes: {
      health: u16(),
      energy: u16(),
      stamina: u16(),
      miningPower: u16(),
      buildPower: u16(),
      defense: u16(),
    },
    equipmentSlotCount: u8(),
    equipment: [],
    backpackStyle: 0,
    backpackFlags: 0,
    equippedBackpack: PublicKey.default,
    createdSlot: 0n,
    updatedSlot: 0n,
    createdAt: 0n,
    forgingXp: 0n,
    forgedItemCount: 0,
    bestForgedGrade: 0,
    bestForgedItemLevel: 0,
    playerName: "",
  };

  for (let i = 0; i < decoded.equipmentSlotCount; i += 1) {
    decoded.equipment.push(pubkey());
  }
  decoded.backpackStyle = u8();
  decoded.backpackFlags = u8();
  decoded.equippedBackpack = pubkey();
  decoded.createdSlot = u64();
  decoded.updatedSlot = u64();
  decoded.createdAt = i64();
  decoded.forgingXp = u64();
  decoded.forgedItemCount = u32();
  decoded.bestForgedGrade = u8();
  decoded.bestForgedItemLevel = u8();
  const playerNameByteLength = u16();
  if (playerNameByteLength > PLAYER_NAME_MAX_BYTES) {
    throw new Error(`Invalid PlayerProfile player name length: ${playerNameByteLength}`);
  }
  decoded.playerName = bytes(playerNameByteLength).toString("utf8");
  bytes(PLAYER_NAME_MAX_BYTES - playerNameByteLength);
  bytes(8);

  if (offset !== data.length) {
    throw new Error(`PlayerProfile decoder offset mismatch: ${offset}`);
  }
  if (decoded.magic !== PLAYER_PROFILE_MAGIC) {
    throw new Error(`Invalid PlayerProfile magic: ${decoded.magic}`);
  }
  return decoded;
}

export function decodePlayerEquipment(data: Buffer): DecodedPlayerEquipment {
  if (data.length !== PLAYER_EQUIPMENT_LEN) {
    throw new Error(`Invalid PlayerEquipment length: expected ${PLAYER_EQUIPMENT_LEN}, got ${data.length}`);
  }
  const magic = data.subarray(0, 8).toString("utf8");
  if (magic !== PLAYER_EQUIPMENT_MAGIC) throw new Error(`Invalid PlayerEquipment magic: ${magic}`);
  const slotCount = data.readUInt8(108);
  if (slotCount !== EQUIPMENT_SLOT_COUNT) throw new Error(`Invalid PlayerEquipment slot count: ${slotCount}`);
  const slots: DecodedPlayerEquipmentSlot[] = [];
  for (let index = 0; index < slotCount; index += 1) {
    const offset = PLAYER_EQUIPMENT_HEADER_LEN + index * PLAYER_EQUIPMENT_SLOT_LEN;
    const modelLength = data.readUInt16LE(offset + 4);
    if (modelLength > PLAYER_EQUIPMENT_MODEL_CODE_MAX_BYTES) {
      throw new Error(`Invalid PlayerEquipment model length at slot ${index}: ${modelLength}`);
    }
    const backpackSlot = Buffer.from(data.subarray(offset + 40, offset + 120));
    const flags = data.readUInt8(offset + 3);
    slots.push({
      state: data.readUInt8(offset),
      slot: data.readUInt8(offset + 1),
      equipped: data.readUInt8(offset) === 1,
      custodied: data.readUInt8(offset) === 1 && (flags & PLAYER_EQUIPMENT_FLAG_CUSTODY) !== 0,
      backpackIndex: data.readUInt8(offset + 2),
      flags,
      backpack: new PublicKey(data.subarray(offset + 8, offset + 40)),
      backpackSlot,
      kindCode: backpackSlot.readUInt8(0),
      category: backpackSlot.readUInt8(1),
      quantity: backpackSlot.readUInt32LE(4),
      itemCode: backpackSlot.readUInt16LE(18),
      itemId: backpackSlot.readBigUInt64LE(20),
      itemPda: new PublicKey(backpackSlot.subarray(28, 60)),
      metadata: backpackSlot.readUInt32LE(76),
      modelCode: Buffer.from(data.subarray(offset + 120, offset + 120 + modelLength)),
    });
  }
  return {
    magic,
    version: data.readUInt16LE(8),
    bump: data.readUInt8(10),
    initialized: data.readUInt8(11) === 1,
    owner: new PublicKey(data.subarray(12, 44)),
    playerProfile: new PublicKey(data.subarray(44, 76)),
    globalConfig: new PublicKey(data.subarray(76, 108)),
    slotCount,
    createdSlot: data.readBigUInt64LE(112),
    updatedSlot: data.readBigUInt64LE(120),
    slots,
  };
}

export function decodePlayerAppearance(data: Buffer): DecodedPlayerAppearance {
  if (data.length !== PLAYER_APPEARANCE_LEN) {
    throw new Error(`Invalid PlayerAppearance length: expected ${PLAYER_APPEARANCE_LEN}, got ${data.length}`);
  }
  const displayNameLength = data.readUInt16LE(143);
  const titleLength = data.readUInt16LE(145);
  const modelCodeLength = data.readUInt16LE(147);
  if (
    displayNameLength > PLAYER_NAME_MAX_BYTES ||
    titleLength > APPEARANCE_TITLE_MAX_BYTES ||
    modelCodeLength > APPEARANCE_MODEL_CODE_MAX_BYTES
  ) {
    throw new Error("Invalid PlayerAppearance string length.");
  }
  const displayNameOffset = 256;
  const titleOffset = displayNameOffset + PLAYER_NAME_MAX_BYTES;
  const modelCodeOffset = titleOffset + APPEARANCE_TITLE_MAX_BYTES;
  const equipmentOffset = modelCodeOffset + APPEARANCE_MODEL_CODE_MAX_BYTES;
  const equipment: DecodedAppearanceEquipmentSlot[] = [];
  for (let index = 0; index < APPEARANCE_EQUIPMENT_SLOT_COUNT; index += 1) {
    const offset = equipmentOffset + index * APPEARANCE_EQUIPMENT_SLOT_LEN;
    const codeLength = data.readUInt16LE(offset + 36);
    equipment.push({
      state: data.readUInt8(offset),
      slot: data.readUInt8(offset + 1),
      equipped: data.readUInt8(offset) === 1,
      flags: data.readUInt16LE(offset + 2),
      itemPda: new PublicKey(data.subarray(offset + 4, offset + 36)),
      massGrams: data.readUInt32LE(offset + 38),
      gripPoint: {
        x: data.readInt16LE(offset + 42),
        y: data.readInt16LE(offset + 44),
        z: data.readInt16LE(offset + 46),
      },
      gripRotation: {
        x: data.readInt16LE(offset + 48),
        y: data.readInt16LE(offset + 50),
        z: data.readInt16LE(offset + 52),
      },
      modelCode: codeLength > 0 && codeLength <= APPEARANCE_EQUIPMENT_CODE_MAX_BYTES
        ? data.subarray(offset + 64, offset + 64 + codeLength).toString("utf8")
        : "",
    });
  }

  const decoded: DecodedPlayerAppearance = {
    magic: data.subarray(0, 8).toString("utf8"),
    version: data.readUInt16LE(8),
    bump: data.readUInt8(10),
    initialized: data.readUInt8(11) === 1,
    owner: new PublicKey(data.subarray(12, 44)),
    playerProfile: new PublicKey(data.subarray(44, 76)),
    globalConfig: new PublicKey(data.subarray(76, 108)),
    treasuryAuthority: new PublicKey(data.subarray(108, 140)),
    modelKind: data.readUInt8(140),
    flags: data.readUInt16LE(141),
    displayName: data.subarray(displayNameOffset, displayNameOffset + displayNameLength).toString("utf8"),
    title: data.subarray(titleOffset, titleOffset + titleLength).toString("utf8"),
    modelCode: data.subarray(modelCodeOffset, modelCodeOffset + modelCodeLength).toString("utf8"),
    equipment,
    createdSlot: data.readBigUInt64LE(150),
    updatedSlot: data.readBigUInt64LE(158),
    createdAt: data.readBigInt64LE(166),
    updatedAt: data.readBigInt64LE(174),
  };
  if (decoded.magic !== PLAYER_APPEARANCE_MAGIC) {
    throw new Error(`Invalid PlayerAppearance magic: ${decoded.magic}`);
  }
  return decoded;
}

function encodePlayerName(playerName: string): Buffer {
  const normalized = String(playerName ?? "").trim();
  if ([...normalized].length > PLAYER_NAME_MAX_CHARS) {
    throw new Error(`Player name is too long: max ${PLAYER_NAME_MAX_CHARS} characters.`);
  }
  if (!/^[\p{Script=Han}A-Za-z0-9_]*$/u.test(normalized)) {
    throw new Error("Player name contains unsupported characters.");
  }
  const bytes = Buffer.from(normalized, "utf8");
  if (bytes.length > PLAYER_NAME_MAX_BYTES) {
    throw new Error(`Player name is too large: max ${PLAYER_NAME_MAX_BYTES} UTF-8 bytes.`);
  }
  return bytes;
}

export function decodePlayerSession(data: Buffer): DecodedPlayerSession {
  if (data.length !== PLAYER_SESSION_LEN) {
    throw new Error(`Invalid PlayerSession length: expected ${PLAYER_SESSION_LEN}, got ${data.length}`);
  }

  let offset = 0;
  const bytes = (length: number): Buffer => {
    const value = data.subarray(offset, offset + length);
    offset += length;
    return value;
  };
  const u8 = (): number => data.readUInt8(offset++);
  const u16 = (): number => {
    const value = data.readUInt16LE(offset);
    offset += 2;
    return value;
  };
  const u32 = (): number => {
    const value = data.readUInt32LE(offset);
    offset += 4;
    return value;
  };
  const u64 = (): bigint => {
    const value = data.readBigUInt64LE(offset);
    offset += 8;
    return value;
  };
  const i64 = (): bigint => {
    const value = data.readBigInt64LE(offset);
    offset += 8;
    return value;
  };
  const pubkey = (): PublicKey => new PublicKey(bytes(32));

  const decoded: DecodedPlayerSession = {
    magic: bytes(8).toString("utf8"),
    version: u16(),
    bump: u8(),
    active: u8() === 1,
    owner: pubkey(),
    sessionAuthority: pubkey(),
    playerProfile: pubkey(),
    globalConfig: pubkey(),
    worldId: u16(),
    allowedActions: u16(),
    expiresAt: i64(),
    createdSlot: u64(),
    updatedSlot: u64(),
    createdAt: i64(),
    maxActions: u32(),
    actionCount: u32(),
  };

  if (offset !== PLAYER_SESSION_LEN) {
    throw new Error(`PlayerSession decoder offset mismatch: ${offset}`);
  }
  if (decoded.magic !== PLAYER_SESSION_MAGIC) {
    throw new Error(`Invalid PlayerSession magic: ${decoded.magic}`);
  }
  return decoded;
}
