import {
  PublicKey,
  SystemProgram,
  TransactionInstruction,
} from "@solana/web3.js";
import {
  deriveGlobalConfigPda,
  NICECHUNK_CORE_PROGRAM_ID,
} from "./nicechunk-core.ts";

export const NICECHUNK_PLAYER_PROGRAM_ID = new PublicKey(
  process.env.NICECHUNK_PLAYER_PROGRAM_ID ?? "oeaRMVnPoV4iENnGCCtaEeRxU5be515s4YYe6aXQuKe",
);
export const PLAYER_PROFILE_SEED = "player";
export const PLAYER_SESSION_SEED = "session";
export const LEGACY_PLAYER_PROFILE_LEN = 417;
export const PLAYER_PROFILE_LEN = 449;
export const PLAYER_SESSION_LEN = 184;
export const PLAYER_PROFILE_MAGIC = "NCKPLY01";
export const PLAYER_SESSION_MAGIC = "NCKSES01";
export const NICECHUNK_BACKPACK_PROGRAM_ID = new PublicKey(
  process.env.NICECHUNK_BACKPACK_PROGRAM_ID ?? "FwTrMDGyRg653L9svvt5aoGii9ZjX1WekSFWcwByjxqt",
);
export const EQUIPMENT_SLOT_COUNT = 9;
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

export function derivePlayerProfilePda(
  owner: PublicKey,
  programId: PublicKey = NICECHUNK_PLAYER_PROGRAM_ID,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(PLAYER_PROFILE_SEED), owner.toBuffer()],
    programId,
  );
}

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
  playerProgramId = NICECHUNK_PLAYER_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
}: {
  payer: PublicKey;
  playerProgramId?: PublicKey;
  coreProgramId?: PublicKey;
}): TransactionInstruction {
  const [playerProfile] = derivePlayerProfilePda(payer, playerProgramId);
  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  return new TransactionInstruction({
    programId: playerProgramId,
    keys: [
      { pubkey: payer, isSigner: true, isWritable: true },
      { pubkey: playerProfile, isSigner: false, isWritable: true },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: Buffer.from([0]),
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
  item,
  playerProgramId = NICECHUNK_PLAYER_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
}: {
  authority: PublicKey;
  slot: number;
  item: PublicKey;
  playerProgramId?: PublicKey;
  coreProgramId?: PublicKey;
}): TransactionInstruction {
  const [playerProfile] = derivePlayerProfilePda(authority, playerProgramId);
  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  const data = Buffer.concat([Buffer.from([2, slot]), item.toBuffer()]);
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
  if (data.length !== PLAYER_PROFILE_LEN && data.length !== LEGACY_PLAYER_PROFILE_LEN) {
    throw new Error(
      `Invalid PlayerProfile length: expected ${PLAYER_PROFILE_LEN} or ${LEGACY_PLAYER_PROFILE_LEN}, got ${data.length}`,
    );
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
  };

  for (let i = 0; i < decoded.equipmentSlotCount; i += 1) {
    decoded.equipment.push(pubkey());
  }
  decoded.backpackStyle = u8();
  decoded.backpackFlags = u8();
  if (data.length === PLAYER_PROFILE_LEN) {
    decoded.equippedBackpack = pubkey();
  }
  decoded.createdSlot = u64();
  decoded.updatedSlot = u64();
  decoded.createdAt = i64();

  if (offset !== data.length) {
    throw new Error(`PlayerProfile decoder offset mismatch: ${offset}`);
  }
  if (decoded.magic !== PLAYER_PROFILE_MAGIC) {
    throw new Error(`Invalid PlayerProfile magic: ${decoded.magic}`);
  }
  return decoded;
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
