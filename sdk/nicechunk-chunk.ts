import {
  PublicKey,
  SystemProgram,
  TransactionInstruction,
} from "@solana/web3.js";
import type { AccountMeta } from "@solana/web3.js";
import {
  deriveGlobalConfigPda,
  NICECHUNK_CORE_PROGRAM_ID,
} from "./nicechunk-core.ts";
import {
  derivePlayerProfilePda,
  derivePlayerSessionPda,
  NICECHUNK_PLAYER_PROGRAM_ID,
} from "./nicechunk-player.ts";

const env = typeof process !== "undefined" ? process.env : {};

export const NICECHUNK_CHUNK_PROGRAM_ID = new PublicKey(
  env.NICECHUNK_CHUNK_PROGRAM_ID ?? "12rCvz9PZ64Uix1TCiHEGU4AN4ZS1h4jv5u7CkqTRdk5",
);
export const CHUNK_SEED = "chunk";
export const CHUNK_BROKEN_SEED = "chunk-broken";
export const MAGICBLOCK_DELEGATION_PROGRAM_ID = new PublicKey(
  "DELeGGvXpWV2fqJUhqcF5ZSYMS4JTLjteaAMARRSaeSh",
);
export const MAGICBLOCK_ROUTER_URL = "https://devnet-router.magicblock.app";
export const MAGICBLOCK_ROUTER_WS_URL = "wss://devnet-router.magicblock.app";
export const CHUNK_LEN = 8280;
export const CHUNK_HEADER_LEN = 88;
export const BLOCK_DELTA_LEN = 64;
export const MAX_BLOCK_DELTAS = 128;
export const CHUNK_MAGIC = "NCKCHK01";
export const CHUNK_BROKEN_MAGIC = "NCBK";
export const CHUNK_BROKEN_HEADER_LEN = 16;
export const CHUNK_BROKEN_RECORD_LEN = 3;
export const CHUNK_BROKEN_INITIAL_CAPACITY = 64;
export const CHUNK_BROKEN_MAX_CAPACITY = 2048;
export const VERIFY_GENERATED_BLOCK_INSPECT_ONLY = 0xffff;
export const BLOCK_AIR = 0;
export const BLOCK_GRASS = 1;
export const BLOCK_DIRT = 2;
export const BLOCK_STONE = 3;
export const BLOCK_DEEP_STONE = 4;
export const BLOCK_BEDROCK = 16;
export const BLOCK_WATER = 17;

export interface BlockChangeInput {
  chunkX: number;
  chunkZ: number;
  localX: number;
  y: number;
  localZ: number;
  previousBlockId: number;
  newBlockId: number;
  action: number;
  toolSlot: number;
}

export interface GeneratedBlockInput {
  chunkX: number;
  chunkZ: number;
  localX: number;
  y: number;
  localZ: number;
  expectedBlockId?: number;
}

export interface MineBlockInput {
  worldX: number;
  worldY: number;
  worldZ: number;
  expectedBlockId?: number;
}

export interface MinimalGlobalConfigForBlockVerification {
  worldSeed: Buffer | Uint8Array;
  chunkSize: number;
  minBuildY: number;
  maxBuildY: number;
  maxTerrainHeight: number;
  seaLevel: number;
}

export interface DecodedBlockDelta {
  sequence: number;
  actor: PublicKey;
  localX: number;
  y: number;
  localZ: number;
  previousBlockId: number;
  newBlockId: number;
  action: number;
  toolSlot: number;
  slot: bigint;
  timestamp: bigint;
}

export interface DecodedChunkState {
  magic: string;
  version: number;
  bump: number;
  initialized: boolean;
  globalConfig: PublicKey;
  worldId: number;
  chunkX: number;
  chunkZ: number;
  changeCount: number;
  storedDeltaCount: number;
  writeCursor: number;
  maxDeltas: number;
  createdSlot: bigint;
  updatedSlot: bigint;
  createdAt: bigint;
  deltas: DecodedBlockDelta[];
}

export interface DecodedBrokenBlock {
  index: number;
  x: number;
  y: number;
  z: number;
  localX: number;
  localZ: number;
  packed: string;
}

export interface DecodedChunkBrokenState {
  magic: string;
  version: number;
  bump: number;
  count: number;
  capacity: number;
  minY: number;
  chunkX: number;
  chunkZ: number;
  brokenBlocks: DecodedBrokenBlock[];
}

export interface ChunkDelegationPdas {
  delegateBuffer: PublicKey;
  delegationRecord: PublicKey;
  delegationMetadata: PublicKey;
}

export function deriveChunkPda({
  globalConfig,
  chunkX,
  chunkZ,
  programId = NICECHUNK_CHUNK_PROGRAM_ID,
}: {
  globalConfig: PublicKey;
  chunkX: number;
  chunkZ: number;
  programId?: PublicKey;
}): [PublicKey, number] {
  const chunkXBytes = Buffer.alloc(4);
  const chunkZBytes = Buffer.alloc(4);
  chunkXBytes.writeInt32LE(chunkX, 0);
  chunkZBytes.writeInt32LE(chunkZ, 0);
  return PublicKey.findProgramAddressSync(
    [Buffer.from(CHUNK_SEED), globalConfig.toBuffer(), chunkXBytes, chunkZBytes],
    programId,
  );
}

export function deriveChunkBrokenPda({
  globalConfig,
  chunkX,
  chunkZ,
  programId = NICECHUNK_CHUNK_PROGRAM_ID,
}: {
  globalConfig: PublicKey;
  chunkX: number;
  chunkZ: number;
  programId?: PublicKey;
}): [PublicKey, number] {
  const chunkXBytes = Buffer.alloc(4);
  const chunkZBytes = Buffer.alloc(4);
  chunkXBytes.writeInt32LE(chunkX, 0);
  chunkZBytes.writeInt32LE(chunkZ, 0);
  return PublicKey.findProgramAddressSync(
    [Buffer.from(CHUNK_BROKEN_SEED), globalConfig.toBuffer(), chunkXBytes, chunkZBytes],
    programId,
  );
}

export function createInitializeChunkInstruction({
  payer,
  chunkX,
  chunkZ,
  chunkProgramId = NICECHUNK_CHUNK_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
}: {
  payer: PublicKey;
  chunkX: number;
  chunkZ: number;
  chunkProgramId?: PublicKey;
  coreProgramId?: PublicKey;
}): TransactionInstruction {
  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  const [chunk] = deriveChunkPda({ globalConfig, chunkX, chunkZ, programId: chunkProgramId });
  const data = Buffer.alloc(9);
  data.writeUInt8(0, 0);
  data.writeInt32LE(chunkX, 1);
  data.writeInt32LE(chunkZ, 5);
  return new TransactionInstruction({
    programId: chunkProgramId,
    keys: [
      { pubkey: payer, isSigner: true, isWritable: true },
      { pubkey: chunk, isSigner: false, isWritable: true },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data,
  });
}

export function deriveChunkDelegationPdas({
  chunk,
  chunkProgramId = NICECHUNK_CHUNK_PROGRAM_ID,
}: {
  chunk: PublicKey;
  chunkProgramId?: PublicKey;
}): ChunkDelegationPdas {
  const [delegateBuffer] = PublicKey.findProgramAddressSync(
    [Buffer.from("buffer"), chunk.toBuffer()],
    chunkProgramId,
  );
  const [delegationRecord] = PublicKey.findProgramAddressSync(
    [Buffer.from("delegation"), chunk.toBuffer()],
    MAGICBLOCK_DELEGATION_PROGRAM_ID,
  );
  const [delegationMetadata] = PublicKey.findProgramAddressSync(
    [Buffer.from("delegation-metadata"), chunk.toBuffer()],
    MAGICBLOCK_DELEGATION_PROGRAM_ID,
  );
  return { delegateBuffer, delegationRecord, delegationMetadata };
}

export function createDelegateChunkInstruction({
  payer,
  chunkX,
  chunkZ,
  commitFrequencyMs = 250,
  chunkProgramId = NICECHUNK_CHUNK_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
}: {
  payer: PublicKey;
  chunkX: number;
  chunkZ: number;
  commitFrequencyMs?: number;
  chunkProgramId?: PublicKey;
  coreProgramId?: PublicKey;
}): TransactionInstruction {
  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  const [chunk] = deriveChunkPda({ globalConfig, chunkX, chunkZ, programId: chunkProgramId });
  const { delegateBuffer, delegationRecord, delegationMetadata } = deriveChunkDelegationPdas({
    chunk,
    chunkProgramId,
  });
  const data = Buffer.alloc(13);
  data.writeUInt8(2, 0);
  data.writeInt32LE(chunkX, 1);
  data.writeInt32LE(chunkZ, 5);
  data.writeUInt32LE(commitFrequencyMs, 9);
  return new TransactionInstruction({
    programId: chunkProgramId,
    keys: [
      { pubkey: payer, isSigner: true, isWritable: true },
      { pubkey: chunk, isSigner: false, isWritable: true },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
      { pubkey: chunkProgramId, isSigner: false, isWritable: false },
      { pubkey: delegateBuffer, isSigner: false, isWritable: true },
      { pubkey: delegationRecord, isSigner: false, isWritable: true },
      { pubkey: delegationMetadata, isSigner: false, isWritable: true },
      { pubkey: MAGICBLOCK_DELEGATION_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data,
  });
}

export function createRecordBlockChangeInstruction({
  authority,
  change,
  chunkProgramId = NICECHUNK_CHUNK_PROGRAM_ID,
  playerProgramId = NICECHUNK_PLAYER_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
}: {
  authority: PublicKey;
  change: BlockChangeInput;
  chunkProgramId?: PublicKey;
  playerProgramId?: PublicKey;
  coreProgramId?: PublicKey;
}): TransactionInstruction {
  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  const [playerProfile] = derivePlayerProfilePda(authority, playerProgramId);
  const [chunk] = deriveChunkPda({
    globalConfig,
    chunkX: change.chunkX,
    chunkZ: change.chunkZ,
    programId: chunkProgramId,
  });
  const data = Buffer.alloc(19);
  data.writeUInt8(1, 0);
  data.writeInt32LE(change.chunkX, 1);
  data.writeInt32LE(change.chunkZ, 5);
  data.writeUInt8(change.localX, 9);
  data.writeInt16LE(change.y, 10);
  data.writeUInt8(change.localZ, 12);
  data.writeUInt16LE(change.previousBlockId, 13);
  data.writeUInt16LE(change.newBlockId, 15);
  data.writeUInt8(change.action, 17);
  data.writeUInt8(change.toolSlot, 18);

  return new TransactionInstruction({
    programId: chunkProgramId,
    keys: [
      { pubkey: authority, isSigner: true, isWritable: true },
      { pubkey: playerProfile, isSigner: false, isWritable: false },
      { pubkey: chunk, isSigner: false, isWritable: true },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data,
  });
}

export function createRecordBlockChangeWithSessionInstruction({
  owner,
  sessionAuthority,
  change,
  chunkProgramId = NICECHUNK_CHUNK_PROGRAM_ID,
  playerProgramId = NICECHUNK_PLAYER_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
}: {
  owner: PublicKey;
  sessionAuthority: PublicKey;
  change: BlockChangeInput;
  chunkProgramId?: PublicKey;
  playerProgramId?: PublicKey;
  coreProgramId?: PublicKey;
}): TransactionInstruction {
  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  const [playerProfile] = derivePlayerProfilePda(owner, playerProgramId);
  const [playerSession] = derivePlayerSessionPda({
    owner,
    sessionAuthority,
    programId: playerProgramId,
  });
  const [chunk] = deriveChunkPda({
    globalConfig,
    chunkX: change.chunkX,
    chunkZ: change.chunkZ,
    programId: chunkProgramId,
  });
  const data = Buffer.alloc(19);
  data.writeUInt8(3, 0);
  data.writeInt32LE(change.chunkX, 1);
  data.writeInt32LE(change.chunkZ, 5);
  data.writeUInt8(change.localX, 9);
  data.writeInt16LE(change.y, 10);
  data.writeUInt8(change.localZ, 12);
  data.writeUInt16LE(change.previousBlockId, 13);
  data.writeUInt16LE(change.newBlockId, 15);
  data.writeUInt8(change.action, 17);
  data.writeUInt8(change.toolSlot, 18);

  return new TransactionInstruction({
    programId: chunkProgramId,
    keys: [
      { pubkey: sessionAuthority, isSigner: true, isWritable: true },
      { pubkey: playerProfile, isSigner: false, isWritable: false },
      { pubkey: playerSession, isSigner: false, isWritable: false },
      { pubkey: chunk, isSigner: false, isWritable: true },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data,
  });
}

export function createVerifyGeneratedBlockInstruction({
  block,
  chunkProgramId = NICECHUNK_CHUNK_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
}: {
  block: GeneratedBlockInput;
  chunkProgramId?: PublicKey;
  coreProgramId?: PublicKey;
}): TransactionInstruction {
  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  const data = Buffer.alloc(15);
  data.writeUInt8(4, 0);
  data.writeInt32LE(block.chunkX, 1);
  data.writeInt32LE(block.chunkZ, 5);
  data.writeUInt8(block.localX, 9);
  data.writeInt16LE(block.y, 10);
  data.writeUInt8(block.localZ, 12);
  data.writeUInt16LE(block.expectedBlockId ?? VERIFY_GENERATED_BLOCK_INSPECT_ONLY, 13);

  const keys: AccountMeta[] = [
    { pubkey: globalConfig, isSigner: false, isWritable: false },
  ];
  return new TransactionInstruction({
    programId: chunkProgramId,
    keys,
    data,
  });
}

export function createMineBlockInstruction({
  payer,
  block,
  chunkProgramId = NICECHUNK_CHUNK_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
  chunkSize = 16,
}: {
  payer: PublicKey;
  block: MineBlockInput;
  chunkProgramId?: PublicKey;
  coreProgramId?: PublicKey;
  chunkSize?: number;
}): TransactionInstruction {
  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  const chunkX = Math.floor(block.worldX / chunkSize);
  const chunkZ = Math.floor(block.worldZ / chunkSize);
  const [chunkBroken] = deriveChunkBrokenPda({
    globalConfig,
    chunkX,
    chunkZ,
    programId: chunkProgramId,
  });
  const data = Buffer.alloc(13);
  data.writeUInt8(5, 0);
  data.writeInt32LE(block.worldX, 1);
  data.writeInt16LE(block.worldY, 5);
  data.writeInt32LE(block.worldZ, 7);
  data.writeUInt16LE(block.expectedBlockId ?? VERIFY_GENERATED_BLOCK_INSPECT_ONLY, 11);

  return new TransactionInstruction({
    programId: chunkProgramId,
    keys: [
      { pubkey: payer, isSigner: true, isWritable: true },
      { pubkey: chunkBroken, isSigner: false, isWritable: true },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data,
  });
}

export function generatedBlockIdAt(
  globalConfig: MinimalGlobalConfigForBlockVerification,
  block: GeneratedBlockInput,
): number {
  const worldX = block.chunkX * globalConfig.chunkSize + block.localX;
  const worldZ = block.chunkZ * globalConfig.chunkSize + block.localZ;
  const surface = generatedSurfaceHeight(globalConfig, worldX, worldZ);
  if (block.y <= globalConfig.minBuildY) return BLOCK_BEDROCK;
  if (block.y > surface) {
    if (block.y <= globalConfig.seaLevel) return BLOCK_WATER;
    return BLOCK_AIR;
  }
  if (block.y === surface) return BLOCK_GRASS;
  if (block.y >= surface - 3) return BLOCK_DIRT;
  if (block.y < globalConfig.minBuildY + 12) return BLOCK_DEEP_STONE;
  return BLOCK_STONE;
}

export function generatedSurfaceHeight(
  globalConfig: MinimalGlobalConfigForBlockVerification,
  worldX: number,
  worldZ: number,
): number {
  const minSurface = globalConfig.minBuildY + 8;
  const maxSurface = Math.max(minSurface, Math.min(globalConfig.maxTerrainHeight, globalConfig.maxBuildY - 1));
  const span = Math.max(1, maxSurface - minSurface + 1);
  const base = hashCoord(globalConfig.worldSeed, worldX, worldZ, 0) % span;
  const detail = (hashCoord(globalConfig.worldSeed, worldX >> 2, worldZ >> 2, 1) % 9) - 4;
  return Math.max(minSurface, Math.min(maxSurface, minSurface + base + detail));
}

function hashCoord(seed: Buffer | Uint8Array, x: number, z: number, salt: number): number {
  let hash = (0x811c9dc5 ^ salt) >>> 0;
  for (const byte of seed) {
    hash ^= byte;
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  const bytes = Buffer.alloc(8);
  bytes.writeInt32LE(x, 0);
  bytes.writeInt32LE(z, 4);
  for (const byte of bytes) {
    hash ^= byte;
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  hash ^= hash >>> 16;
  hash = Math.imul(hash, 0x7feb352d) >>> 0;
  hash ^= hash >>> 15;
  hash = Math.imul(hash, 0x846ca68b) >>> 0;
  hash ^= hash >>> 16;
  return hash >>> 0;
}

export function decodeChunkState(data: Buffer): DecodedChunkState {
  if (data.length !== CHUNK_LEN) {
    throw new Error(`Invalid ChunkState length: expected ${CHUNK_LEN}, got ${data.length}`);
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

  const decoded: DecodedChunkState = {
    magic: bytes(8).toString("utf8"),
    version: u16(),
    bump: u8(),
    initialized: u8() === 1,
    globalConfig: pubkey(),
    worldId: u16(),
    chunkX: i32(),
    chunkZ: i32(),
    changeCount: u32(),
    storedDeltaCount: u16(),
    writeCursor: u16(),
    maxDeltas: u16(),
    createdSlot: u64(),
    updatedSlot: u64(),
    createdAt: i64(),
    deltas: [],
  };

  for (let i = 0; i < decoded.storedDeltaCount; i += 1) {
    const deltaOffset = CHUNK_HEADER_LEN + i * BLOCK_DELTA_LEN;
    const delta = data.subarray(deltaOffset, deltaOffset + BLOCK_DELTA_LEN);
    decoded.deltas.push({
      sequence: delta.readUInt32LE(0),
      actor: new PublicKey(delta.subarray(4, 36)),
      localX: delta.readUInt8(36),
      y: delta.readInt16LE(37),
      localZ: delta.readUInt8(39),
      previousBlockId: delta.readUInt16LE(40),
      newBlockId: delta.readUInt16LE(42),
      action: delta.readUInt8(44),
      toolSlot: delta.readUInt8(45),
      slot: delta.readBigUInt64LE(46),
      timestamp: delta.readBigInt64LE(54),
    });
  }

  if (offset !== CHUNK_HEADER_LEN) {
    throw new Error(`ChunkState header decoder offset mismatch: ${offset}`);
  }
  if (decoded.magic !== CHUNK_MAGIC) {
    throw new Error(`Invalid ChunkState magic: ${decoded.magic}`);
  }
  return decoded;
}

export function decodeChunkBrokenState({
  data,
  chunkX,
  chunkZ,
  chunkSize = 16,
}: {
  data: Buffer;
  chunkX: number;
  chunkZ: number;
  chunkSize?: number;
}): DecodedChunkBrokenState {
  if (data.length < CHUNK_BROKEN_HEADER_LEN) {
    throw new Error(`Invalid ChunkBrokenState length: ${data.length}`);
  }
  const magic = data.subarray(0, 4).toString("utf8");
  if (magic !== CHUNK_BROKEN_MAGIC) {
    throw new Error(`Invalid ChunkBrokenState magic: ${magic}`);
  }
  const version = data.readUInt8(4);
  const bump = data.readUInt8(5);
  const count = data.readUInt16LE(6);
  const capacity = data.readUInt16LE(8);
  const minY = data.readInt16LE(10);
  const expectedLength = CHUNK_BROKEN_HEADER_LEN + capacity * CHUNK_BROKEN_RECORD_LEN;
  if (data.length !== expectedLength || count > capacity) {
    throw new Error(`Invalid ChunkBrokenState size: expected ${expectedLength}, got ${data.length}`);
  }
  const brokenBlocks: DecodedBrokenBlock[] = [];
  for (let index = 0; index < count; index += 1) {
    const offset = CHUNK_BROKEN_HEADER_LEN + index * CHUNK_BROKEN_RECORD_LEN;
    const packed = data.readUIntLE(offset, CHUNK_BROKEN_RECORD_LEN);
    const localX = packed & 0x0f;
    const localZ = (packed >> 4) & 0x0f;
    const yOffset = (packed >> 8) & 0x01ff;
    brokenBlocks.push({
      index,
      x: chunkX * chunkSize + localX,
      y: minY + yOffset,
      z: chunkZ * chunkSize + localZ,
      localX,
      localZ,
      packed: data.subarray(offset, offset + CHUNK_BROKEN_RECORD_LEN).toString("hex"),
    });
  }
  return {
    magic,
    version,
    bump,
    count,
    capacity,
    minY,
    chunkX,
    chunkZ,
    brokenBlocks,
  };
}
