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

export const NICECHUNK_BACKPACK_PROGRAM_ID = new PublicKey(
  process.env.NICECHUNK_BACKPACK_PROGRAM_ID ?? "FwTrMDGyRg653L9svvt5aoGii9ZjX1WekSFWcwByjxqt",
);
export const BACKPACK_SEED = "backpack";
export const BACKPACK_MAGIC = "NCKBPK01";
export const BACKPACK_DEFAULT_CAPACITY = 50;
export const BACKPACK_MAX_CAPACITY = 99;
export const BACKPACK_HEADER_LEN = 128;
export const BACKPACK_RECORD_LEN = 10;
export const BACKPACK_LEN = BACKPACK_HEADER_LEN + BACKPACK_MAX_CAPACITY * BACKPACK_RECORD_LEN;

export interface BackpackResourceRecord {
  worldX: number;
  worldY: number;
  worldZ: number;
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

export function decodeBackpack(data: Buffer): DecodedBackpack {
  if (data.length !== BACKPACK_LEN) {
    throw new Error(`Invalid Backpack length: expected ${BACKPACK_LEN}, got ${data.length}`);
  }
  const magic = data.subarray(0, 8).toString("utf8");
  if (magic !== BACKPACK_MAGIC) throw new Error(`Invalid Backpack magic: ${magic}`);
  const capacity = data.readUInt8(52);
  const itemCount = data.readUInt8(53);
  const records: BackpackResourceRecord[] = [];
  const readableCount = Math.min(itemCount, capacity, BACKPACK_MAX_CAPACITY);
  for (let index = 0; index < readableCount; index += 1) {
    const offset = BACKPACK_HEADER_LEN + index * BACKPACK_RECORD_LEN;
    records.push({
      worldX: data.readInt32LE(offset),
      worldY: data.readInt16LE(offset + 4),
      worldZ: data.readInt32LE(offset + 6),
    });
  }
  return {
    magic,
    version: data.readUInt16LE(8),
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
  };
}
