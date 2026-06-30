import {
  PublicKey,
  SystemProgram,
  TransactionInstruction,
} from "@solana/web3.js";
import {
  derivePlayerProfilePda,
  derivePlayerSessionPda,
  NICECHUNK_PLAYER_PROGRAM_ID,
} from "./nicechunk-player.ts";

const env = typeof process !== "undefined" ? process.env : {};

export const NICECHUNK_BACKPACK_PROGRAM_ID = new PublicKey(
  env.NICECHUNK_BACKPACK_PROGRAM_ID ?? "FwTrMDGyRg653L9svvt5aoGii9ZjX1WekSFWcwByjxqt",
);
export const BACKPACK_SEED = "backpack";
export const BACKPACK_MAGIC = "NCKBPK01";
export const BACKPACK_LEGACY_VERSION = 1;
export const BACKPACK_VERSION = 2;
export const BACKPACK_DEFAULT_CAPACITY = 50;
export const BACKPACK_MAX_CAPACITY = 99;
export const BACKPACK_HEADER_LEN = 128;
export const BACKPACK_LEGACY_RECORD_LEN = 10;
export const BACKPACK_RESOURCE_RECORD_LEN = 10;
export const BACKPACK_SLOT_RECORD_LEN = 64;
export const BACKPACK_RECORD_LEN = BACKPACK_SLOT_RECORD_LEN;
export const BACKPACK_LEGACY_LEN = BACKPACK_HEADER_LEN + BACKPACK_MAX_CAPACITY * BACKPACK_LEGACY_RECORD_LEN;
export const BACKPACK_LEN = BACKPACK_HEADER_LEN + BACKPACK_MAX_CAPACITY * BACKPACK_RECORD_LEN;
export const BACKPACK_SLOT_KIND_BLOCK = 1;
export const BACKPACK_SLOT_KIND_ITEM = 2;
export const BACKPACK_ITEM_CATEGORY_MATERIAL = 1;
export const BACKPACK_ITEM_CATEGORY_FORGED = 2;

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
  records: BackpackResourceRecord[];
  slots: BackpackSlotRecord[];
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
    data,
  });
}

export function createAppendMinedResourceInstruction({
  owner,
  sessionAuthority,
  backpack,
  record,
  backpackProgramId = NICECHUNK_BACKPACK_PROGRAM_ID,
  playerProgramId = NICECHUNK_PLAYER_PROGRAM_ID,
}: {
  owner: PublicKey;
  sessionAuthority: PublicKey;
  backpack: PublicKey;
  record: BackpackResourceRecord;
  backpackProgramId?: PublicKey;
  playerProgramId?: PublicKey;
}): TransactionInstruction {
  const [playerProfile] = derivePlayerProfilePda(owner, playerProgramId);
  const [playerSession] = derivePlayerSessionPda({ owner, sessionAuthority, programId: playerProgramId });
  const data = Buffer.alloc(11);
  data.writeUInt8(1, 0);
  data.writeInt32LE(record.worldX, 1);
  data.writeInt16LE(record.worldY, 5);
  data.writeInt32LE(record.worldZ, 7);
  return new TransactionInstruction({
    programId: backpackProgramId,
    keys: [
      { pubkey: sessionAuthority, isSigner: true, isWritable: false },
      { pubkey: playerProfile, isSigner: false, isWritable: false },
      { pubkey: playerSession, isSigner: false, isWritable: false },
      { pubkey: backpack, isSigner: false, isWritable: true },
    ],
    data,
  });
}

export function createAppendSmeltingItemInstruction({
  smeltingAuthority,
  owner,
  backpack,
  slot,
  backpackProgramId = NICECHUNK_BACKPACK_PROGRAM_ID,
}: {
  smeltingAuthority: PublicKey;
  owner: PublicKey;
  backpack: PublicKey;
  slot: BackpackSlotRecord;
  backpackProgramId?: PublicKey;
}): TransactionInstruction {
  return new TransactionInstruction({
    programId: backpackProgramId,
    keys: [
      { pubkey: smeltingAuthority, isSigner: true, isWritable: false },
      { pubkey: owner, isSigner: false, isWritable: false },
      { pubkey: backpack, isSigner: false, isWritable: true },
    ],
    data: Buffer.concat([Buffer.from([5]), encodeBackpackSlotRecord(slot)]),
  });
}

export function decodeBackpack(data: Buffer): DecodedBackpack {
  if (data.length !== BACKPACK_LEN && data.length !== BACKPACK_LEGACY_LEN) {
    throw new Error(`Invalid Backpack length: expected ${BACKPACK_LEN} or ${BACKPACK_LEGACY_LEN}, got ${data.length}`);
  }
  const magic = data.subarray(0, 8).toString("utf8");
  if (magic !== BACKPACK_MAGIC) throw new Error(`Invalid Backpack magic: ${magic}`);
  const version = data.readUInt16LE(8);
  const recordLen = version === BACKPACK_LEGACY_VERSION ? BACKPACK_LEGACY_RECORD_LEN : BACKPACK_SLOT_RECORD_LEN;
  const capacity = data.readUInt8(52);
  const itemCount = data.readUInt8(53);
  const records: BackpackResourceRecord[] = [];
  const slots: BackpackSlotRecord[] = [];
  const readableCount = Math.min(itemCount, capacity, BACKPACK_MAX_CAPACITY);
  for (let index = 0; index < readableCount; index += 1) {
    const offset = BACKPACK_HEADER_LEN + index * recordLen;
    const slot = recordLen === BACKPACK_LEGACY_RECORD_LEN
      ? backpackSlotFromResource(decodeBackpackResourceRecord(data.subarray(offset, offset + recordLen)))
      : decodeBackpackSlotRecord(data.subarray(offset, offset + recordLen));
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
    flags: data.readUInt8(55),
    placed: {
      x: data.readInt32LE(56),
      y: data.readInt16LE(60),
      z: data.readInt32LE(62),
    },
    createdSlot: data.readBigUInt64LE(66),
    updatedSlot: data.readBigUInt64LE(74),
    createdAt: data.readBigInt64LE(82),
    records,
    slots,
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
  return {
    kind: data.readUInt8(0),
    category: data.readUInt8(1),
    flags: data.readUInt16LE(2),
    quantity: data.readUInt32LE(4),
    resource: decodeBackpackResourceRecord(data.subarray(8, 18)),
    itemCode: data.readUInt16LE(18),
    itemId: data.readBigUInt64LE(20),
    itemPda: new PublicKey(data.subarray(28, 60)),
    volumeMm3: data.readUInt32LE(60),
  };
}

export function encodeBackpackSlotRecord(slot: BackpackSlotRecord): Buffer {
  const data = Buffer.alloc(BACKPACK_SLOT_RECORD_LEN);
  data.writeUInt8(slot.kind, 0);
  data.writeUInt8(slot.category, 1);
  data.writeUInt16LE(slot.flags ?? 0, 2);
  data.writeUInt32LE(slot.quantity ?? 1, 4);
  data.writeInt32LE(slot.resource?.worldX ?? 0, 8);
  data.writeInt16LE(slot.resource?.worldY ?? 0, 12);
  data.writeInt32LE(slot.resource?.worldZ ?? 0, 14);
  data.writeUInt16LE(slot.itemCode ?? 0, 18);
  data.writeBigUInt64LE(BigInt(slot.itemId ?? 0), 20);
  (slot.itemPda ?? PublicKey.default).toBuffer().copy(data, 28);
  data.writeUInt32LE(Math.max(0, Math.min(0xffffffff, Math.floor(Number(slot.volumeMm3) || 0))), 60);
  return data;
}
