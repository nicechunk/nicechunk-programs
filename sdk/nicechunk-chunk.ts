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
  derivePlayerSessionPda,
  NICECHUNK_PLAYER_PROGRAM_ID,
} from "./nicechunk-player.ts";
import {
  deriveMaterialPhysicsPda,
  NICECHUNK_BACKPACK_PROGRAM_ID,
} from "./nicechunk-backpack.ts";
import {
  deriveCivilizationAdapterAuthorityPda,
  NICECHUNK_CIVILIZATION_PROGRAM_ID,
} from "./nicechunk-civilization.ts";

const env = typeof process !== "undefined" ? process.env : {};

export const NICECHUNK_CHUNK_PROGRAM_ID = new PublicKey(
  env.NICECHUNK_CHUNK_PROGRAM_ID ?? "GnVKn442KDTDgCyjVG7SEtCQQLjaCiLvrEZDWSU13wbj",
);
export const NICECHUNK_GAME_PROGRAM_ID = new PublicKey(
  env.NICECHUNK_GAME_PROGRAM_ID ?? "6CurnvneezBuHwPUnrCiFg1QMWeUF67ufQxYebyr2UP7",
);
const UNIFIED_GAME_CHUNK_NAMESPACE = 2;
export const CHUNK_BROKEN_SEED = "chunk-broken";
export const RESOURCE_DROP_TABLE_SEED = "resource-drops-v2";
export const SURFACE_DECORATION_TABLE_SEED = "surface-decor-v1";
export const PLAYER_PROGRESS_SEED = "player-progress";
export const FOUNDATION_SEED = "foundation";
export const FOUNDATION_CHUNK_SEED = "foundation-chunk";
export const CHUNK_BROKEN_MAGIC = "NCBK";
export const CHUNK_BROKEN_HEADER_LEN = 16;
export const CHUNK_BROKEN_RECORD_LEN = 3;
export const CHUNK_BROKEN_INITIAL_CAPACITY = 64;
export const CHUNK_BROKEN_MAX_CAPACITY = 2048;
export const RESOURCE_DROP_RULE_LEN = 23;
export const SURFACE_DECORATION_TABLE_MAGIC = "NCKDEC01";
export const SURFACE_DECORATION_TABLE_VERSION = 1;
export const SURFACE_DECORATION_TABLE_HEADER_LEN = 16;
export const SURFACE_DECORATION_RULE_LEN = 20;
export const SURFACE_DECORATION_RULE_MAX_COUNT = 128;
export const SURFACE_DECORATION_TABLE_LEN =
  SURFACE_DECORATION_TABLE_HEADER_LEN + SURFACE_DECORATION_RULE_MAX_COUNT * SURFACE_DECORATION_RULE_LEN;
export const SURFACE_DECORATION_ROLL_DENOMINATOR = 10_000;
export const FOUNDATION_MAGIC = "NCKFND01";
export const FOUNDATION_VERSION = 1;
export const FOUNDATION_LEN = 112;
export const FOUNDATION_CHUNK_MAGIC = "NCKFCI01";
export const FOUNDATION_CHUNK_VERSION = 1;
export const FOUNDATION_CHUNK_HEADER_LEN = 52;
export const FOUNDATION_CHUNK_RECORD_LEN = 52;
export const FOUNDATION_CHUNK_CAPACITY = 32;
export const FOUNDATION_CHUNK_LEN =
  FOUNDATION_CHUNK_HEADER_LEN + FOUNDATION_CHUNK_CAPACITY * FOUNDATION_CHUNK_RECORD_LEN;
export const FOUNDATION_MIN_SIZE = 2;
export const FOUNDATION_MAX_SIZE = 16;
export const FOUNDATION_MAX_CHUNKS = 4;
export const BUILD_SITE_SEED = "build-site-v1";
export const BUILDING_MANIFEST_SEED = "building-v2";
export const BUILDING_SHARD_SEED = "building-data-v1";
export const BUILD_SITE_MAGIC = "NCKSITE1";
export const BUILD_SITE_VERSION = 1;
export const BUILD_SITE_LEN = 136;
export const BUILDING_MANIFEST_MAGIC = "NCKBLD02";
export const BUILDING_MANIFEST_VERSION = 2;
export const BUILDING_MANIFEST_LEN = 160;
export const BUILDING_SHARD_MAGIC = "NCKBDT01";
export const BUILDING_SHARD_VERSION = 1;
export const BUILDING_SHARD_HEADER_LEN = 64;
export const BUILDING_SHARD_PAYLOAD_LEN = 8192;
export const BUILDING_MAX_PAYLOAD_LEN = 65535;
export const BUILDING_MAX_WRITE_LEN = 700;
export const VERIFY_GENERATED_BLOCK_INSPECT_ONLY = 0xffff;
export const BLOCK_AIR = 0;
export const BLOCK_GRASS = 1;
export const BLOCK_DIRT = 2;
export const BLOCK_STONE = 3;
export const BLOCK_DEEP_STONE = 4;
export const BLOCK_SAND = 5;
export const BLOCK_GRAVEL = 6;
export const BLOCK_CLAY = 7;
export const BLOCK_MUD = 8;
export const BLOCK_DRY_DIRT = 9;
export const BLOCK_SALT_FLAT = 10;
export const BLOCK_SNOW = 11;
export const BLOCK_FROZEN_SOIL = 13;
export const BLOCK_BASALT = 14;
export const BLOCK_ASH = 15;
export const BLOCK_BEDROCK = 16;
export const BLOCK_WATER = 17;
export const BLOCK_QUICKSAND = 21;
export const BLOCK_TRUNK = 22;
export const BLOCK_LEAVES = 23;
export const BLOCK_PINE_TRUNK = 24;
export const BLOCK_PINE_LEAVES = 25;
export const BLOCK_CACTUS = 32;
export const BLOCK_MOSS = 37;
export const BLOCK_SHELL_BED = 46;
export const BLOCK_COAL = 47;
export const BATCH_MINE_MAX_BLOCKS = 2;
export const BATCH_MINE_MODE_DEBUG = 1;
export const RANGE_MINE_MAX_BLOCKS = 640;
export const RANGE_MINE_MODE_DEBUG = 1;
const TREE_MAX_LEAF_RADIUS = 2;
const MAX_WATER_LEVEL_ABOVE_SEA = 6;

export const CANONICAL_CHUNK_WORLD_CONFIG = Object.freeze({
  worldSeedHex: "6e6963656368756e6b2d6d61696e6e65742d3030310000000000000000000000",
  chunkSize: 16,
  minBuildY: -32,
  maxBuildY: 320,
  maxTerrainHeight: 240,
  seaLevel: 96,
  canonicalSource: "chunk-v4",
});

function chunkInstructionData(programId: PublicKey, data: Buffer): Buffer {
  return programId.equals(NICECHUNK_GAME_PROGRAM_ID)
    ? Buffer.concat([Buffer.from([UNIFIED_GAME_CHUNK_NAMESPACE]), data])
    : data;
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

export interface FoundationInput {
  minX: number;
  minZ: number;
  surfaceY: number;
  width: number;
  depth: number;
}

export interface DecodedBuildSite extends FoundationInput {
  magic: string;
  version: number;
  bump: number;
  status: number;
  owner: PublicKey;
  globalConfig: PublicKey;
  foundationId: bigint;
  activeRevision: number;
  pendingRevision: number;
  createdSlot: bigint;
  updatedSlot: bigint;
}

export interface DecodedFoundationRecord extends FoundationInput {
  owner: PublicKey;
  foundationId: bigint;
}

export interface DecodedFoundationState extends DecodedFoundationRecord {
  magic: string;
  version: number;
  bump: number;
  status: number;
  chunkCount: number;
  globalConfig: PublicKey;
  createdSlot: bigint;
}

export interface DecodedFoundationChunkState {
  magic: string;
  version: number;
  bump: number;
  count: number;
  globalConfig: PublicKey;
  chunkX: number;
  chunkZ: number;
  records: DecodedFoundationRecord[];
}

export interface ResourceDropRuleInput {
  sourceBlockId: number;
  dropBlockId: number;
  chanceBps: number;
  minAltitude: number;
  maxAltitude: number;
  minDepth: number;
  maxDepth: number;
  salt: number;
  minVolumeMm3: number;
  maxVolumeMm3: number;
}

export interface SurfaceDecorationRuleInput {
  ruleId: number;
  decorationId: number;
  surfaceBlockId: number;
  dropBlockId: number;
  rollStartBps: number;
  rollEndBps: number;
  minY: number;
  maxY: number;
  salt: number;
  variant: number;
  flags: number;
}

export interface DecodedSurfaceDecorationTable {
  magic: string;
  version: number;
  bump: number;
  revision: number;
  rules: SurfaceDecorationRuleInput[];
}

export interface SurfaceDecorationMatch extends SurfaceDecorationRuleInput {
  surfaceY: number;
  roll: number;
}

export interface MinimalGlobalConfigForBlockVerification {
  worldSeed: Buffer | Uint8Array;
  chunkSize: number;
  minBuildY: number;
  maxBuildY: number;
  maxTerrainHeight: number;
  seaLevel: number;
}

export function canonicalChunkWorldConfig(
  globalConfig: Partial<MinimalGlobalConfigForBlockVerification> = {},
): MinimalGlobalConfigForBlockVerification {
  return {
    ...globalConfig,
    worldSeed: Buffer.from(CANONICAL_CHUNK_WORLD_CONFIG.worldSeedHex, "hex"),
    chunkSize: CANONICAL_CHUNK_WORLD_CONFIG.chunkSize,
    minBuildY: CANONICAL_CHUNK_WORLD_CONFIG.minBuildY,
    maxBuildY: CANONICAL_CHUNK_WORLD_CONFIG.maxBuildY,
    maxTerrainHeight: CANONICAL_CHUNK_WORLD_CONFIG.maxTerrainHeight,
    seaLevel: CANONICAL_CHUNK_WORLD_CONFIG.seaLevel,
  };
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

export function deriveFoundationPda({
  globalConfig,
  owner,
  foundationId,
  programId = NICECHUNK_CHUNK_PROGRAM_ID,
}: {
  globalConfig: PublicKey;
  owner: PublicKey;
  foundationId: bigint | number | string;
  programId?: PublicKey;
}): [PublicKey, number] {
  const foundationIdBytes = Buffer.alloc(8);
  foundationIdBytes.writeBigUInt64LE(normalizeFoundationId(foundationId));
  return PublicKey.findProgramAddressSync(
    [Buffer.from(FOUNDATION_SEED), globalConfig.toBuffer(), owner.toBuffer(), foundationIdBytes],
    programId,
  );
}

export function deriveFoundationChunkPda({
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
  chunkXBytes.writeInt32LE(requireI32(chunkX, "chunkX"), 0);
  chunkZBytes.writeInt32LE(requireI32(chunkZ, "chunkZ"), 0);
  return PublicKey.findProgramAddressSync(
    [Buffer.from(FOUNDATION_CHUNK_SEED), globalConfig.toBuffer(), chunkXBytes, chunkZBytes],
    programId,
  );
}

export function deriveBuildSitePda({
  globalConfig,
  foundationId,
  programId = NICECHUNK_CHUNK_PROGRAM_ID,
}: {
  globalConfig: PublicKey;
  foundationId: bigint | number | string;
  programId?: PublicKey;
}): [PublicKey, number] {
  const foundationIdBytes = Buffer.alloc(8);
  foundationIdBytes.writeBigUInt64LE(normalizeFoundationId(foundationId));
  return PublicKey.findProgramAddressSync(
    [Buffer.from(BUILD_SITE_SEED), globalConfig.toBuffer(), foundationIdBytes],
    programId,
  );
}

export function deriveBuildingManifestPda({
  globalConfig,
  foundationId,
  revision,
  programId = NICECHUNK_CHUNK_PROGRAM_ID,
}: {
  globalConfig: PublicKey;
  foundationId: bigint | number | string;
  revision: number;
  programId?: PublicKey;
}): [PublicKey, number] {
  const foundationIdBytes = Buffer.alloc(8);
  const revisionBytes = Buffer.alloc(4);
  foundationIdBytes.writeBigUInt64LE(normalizeFoundationId(foundationId));
  revisionBytes.writeUInt32LE(requireU32(revision, "revision"));
  return PublicKey.findProgramAddressSync(
    [Buffer.from(BUILDING_MANIFEST_SEED), globalConfig.toBuffer(), foundationIdBytes, revisionBytes],
    programId,
  );
}

export function deriveBuildingShardPda({
  globalConfig,
  foundationId,
  revision,
  shardIndex,
  programId = NICECHUNK_CHUNK_PROGRAM_ID,
}: {
  globalConfig: PublicKey;
  foundationId: bigint | number | string;
  revision: number;
  shardIndex: number;
  programId?: PublicKey;
}): [PublicKey, number] {
  const foundationIdBytes = Buffer.alloc(8);
  const revisionBytes = Buffer.alloc(4);
  foundationIdBytes.writeBigUInt64LE(normalizeFoundationId(foundationId));
  revisionBytes.writeUInt32LE(requireU32(revision, "revision"));
  return PublicKey.findProgramAddressSync(
    [
      Buffer.from(BUILDING_SHARD_SEED),
      globalConfig.toBuffer(),
      foundationIdBytes,
      revisionBytes,
      Buffer.from([clampInt(shardIndex, 0, 255)]),
    ],
    programId,
  );
}

export function deriveResourceDropTablePda({
  globalConfig,
  programId = NICECHUNK_CHUNK_PROGRAM_ID,
}: {
  globalConfig: PublicKey;
  programId?: PublicKey;
}): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(RESOURCE_DROP_TABLE_SEED), globalConfig.toBuffer()],
    programId,
  );
}

export function deriveSurfaceDecorationTablePda({
  globalConfig,
  programId = NICECHUNK_CHUNK_PROGRAM_ID,
}: {
  globalConfig: PublicKey;
  programId?: PublicKey;
}): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(SURFACE_DECORATION_TABLE_SEED), globalConfig.toBuffer()],
    programId,
  );
}

export function derivePlayerProgressPda({
  globalConfig,
  owner,
  programId = NICECHUNK_CHUNK_PROGRAM_ID,
}: {
  globalConfig: PublicKey;
  owner: PublicKey;
  programId?: PublicKey;
}): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(PLAYER_PROGRESS_SEED), globalConfig.toBuffer(), owner.toBuffer()],
    programId,
  );
}

export function createMineBlockInstruction({
  payer,
  owner,
  block,
  sessionAuthority = payer,
  chunkProgramId = NICECHUNK_CHUNK_PROGRAM_ID,
  playerProgramId = NICECHUNK_PLAYER_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
  chunkSize = 16,
}: {
  payer: PublicKey;
  owner: PublicKey;
  block: MineBlockInput;
  sessionAuthority?: PublicKey;
  chunkProgramId?: PublicKey;
  playerProgramId?: PublicKey;
  coreProgramId?: PublicKey;
  chunkSize?: number;
}): TransactionInstruction {
  if (block.expectedBlockId === undefined) {
    throw new Error("expectedBlockId is required for canonical mining");
  }
  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  const [playerProfile] = derivePlayerProfilePda(owner, playerProgramId);
  const [playerSession] = derivePlayerSessionPda({
    owner,
    sessionAuthority,
    programId: playerProgramId,
  });
  const chunkX = Math.floor(block.worldX / chunkSize);
  const chunkZ = Math.floor(block.worldZ / chunkSize);
  const [chunkBroken] = deriveChunkBrokenPda({
    globalConfig,
    chunkX,
    chunkZ,
    programId: chunkProgramId,
  });
  const [foundationChunk] = deriveFoundationChunkPda({
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
  data.writeUInt16LE(block.expectedBlockId, 11);

  return new TransactionInstruction({
    programId: chunkProgramId,
    keys: [
      { pubkey: payer, isSigner: true, isWritable: true },
      { pubkey: playerProfile, isSigner: false, isWritable: false },
      { pubkey: playerSession, isSigner: false, isWritable: false },
      { pubkey: chunkBroken, isSigner: false, isWritable: true },
      { pubkey: foundationChunk, isSigner: false, isWritable: false },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: chunkInstructionData(chunkProgramId, data),
  });
}

export function createMineBlockWithRewardsInstruction({
  payer,
  owner,
  block,
  backpack,
  sessionAuthority = payer,
  chunkProgramId = NICECHUNK_CHUNK_PROGRAM_ID,
  playerProgramId = NICECHUNK_PLAYER_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
  chunkSize = 16,
}: {
  payer: PublicKey;
  owner: PublicKey;
  block: MineBlockInput;
  backpack: PublicKey;
  sessionAuthority?: PublicKey;
  chunkProgramId?: PublicKey;
  playerProgramId?: PublicKey;
  coreProgramId?: PublicKey;
  chunkSize?: number;
}): TransactionInstruction {
  if (block.expectedBlockId === undefined) {
    throw new Error("expectedBlockId is required for canonical mining");
  }
  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  const [playerProfile] = derivePlayerProfilePda(owner, playerProgramId);
  const [playerSession] = derivePlayerSessionPda({
    owner,
    sessionAuthority,
    programId: playerProgramId,
  });
  const chunkX = Math.floor(block.worldX / chunkSize);
  const chunkZ = Math.floor(block.worldZ / chunkSize);
  const [chunkBroken] = deriveChunkBrokenPda({
    globalConfig,
    chunkX,
    chunkZ,
    programId: chunkProgramId,
  });
  const [foundationChunk] = deriveFoundationChunkPda({
    globalConfig,
    chunkX,
    chunkZ,
    programId: chunkProgramId,
  });
  const [resourceDropTable] = deriveResourceDropTablePda({ globalConfig, programId: chunkProgramId });
  const [surfaceDecorationTable] = deriveSurfaceDecorationTablePda({ globalConfig, programId: chunkProgramId });
  const [playerProgress] = derivePlayerProgressPda({ globalConfig, owner, programId: chunkProgramId });
  const [materialPhysics] = deriveMaterialPhysicsPda({
    globalConfig,
    programId: NICECHUNK_BACKPACK_PROGRAM_ID,
  });
  const data = Buffer.alloc(13);
  data.writeUInt8(8, 0);
  data.writeInt32LE(block.worldX, 1);
  data.writeInt16LE(block.worldY, 5);
  data.writeInt32LE(block.worldZ, 7);
  data.writeUInt16LE(block.expectedBlockId, 11);

  return new TransactionInstruction({
    programId: chunkProgramId,
    keys: [
      { pubkey: payer, isSigner: true, isWritable: true },
      { pubkey: playerProfile, isSigner: false, isWritable: false },
      { pubkey: playerSession, isSigner: false, isWritable: false },
      { pubkey: playerProgress, isSigner: false, isWritable: true },
      { pubkey: chunkBroken, isSigner: false, isWritable: true },
      { pubkey: foundationChunk, isSigner: false, isWritable: false },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
      { pubkey: resourceDropTable, isSigner: false, isWritable: false },
      { pubkey: surfaceDecorationTable, isSigner: false, isWritable: false },
      { pubkey: NICECHUNK_BACKPACK_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: backpack, isSigner: false, isWritable: true },
      { pubkey: materialPhysics, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: chunkInstructionData(chunkProgramId, data),
  });
}

export function createBatchMineWithRewardsInstruction({
  payer,
  owner,
  blocks,
  backpack,
  mode = BATCH_MINE_MODE_DEBUG,
  sessionAuthority = payer,
  chunkProgramId = NICECHUNK_CHUNK_PROGRAM_ID,
  playerProgramId = NICECHUNK_PLAYER_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
  chunkSize = 16,
}: {
  payer: PublicKey;
  owner: PublicKey;
  blocks: MineBlockInput[];
  backpack: PublicKey;
  mode?: number;
  sessionAuthority?: PublicKey;
  chunkProgramId?: PublicKey;
  playerProgramId?: PublicKey;
  coreProgramId?: PublicKey;
  chunkSize?: number;
}): TransactionInstruction {
  if (!owner) throw new Error("owner is required for batch mining");
  if (!backpack) throw new Error("backpack is required for batch mining");
  if (mode !== BATCH_MINE_MODE_DEBUG) throw new Error("unsupported batch mining mode");
  if (!Array.isArray(blocks) || blocks.length < 1 || blocks.length > BATCH_MINE_MAX_BLOCKS) {
    throw new Error(`batch mining requires 1-${BATCH_MINE_MAX_BLOCKS} blocks`);
  }
  const firstChunkX = Math.floor(blocks[0].worldX / chunkSize);
  const firstChunkZ = Math.floor(blocks[0].worldZ / chunkSize);
  for (const block of blocks) {
    if (block.expectedBlockId === undefined) {
      throw new Error("expectedBlockId is required for canonical batch mining");
    }
    if (Math.floor(block.worldX / chunkSize) !== firstChunkX || Math.floor(block.worldZ / chunkSize) !== firstChunkZ) {
      throw new Error("all batch mining blocks must belong to one chunk");
    }
  }

  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  const [playerProfile] = derivePlayerProfilePda(owner, playerProgramId);
  const [playerSession] = derivePlayerSessionPda({ owner, sessionAuthority, programId: playerProgramId });
  const [playerProgress] = derivePlayerProgressPda({ globalConfig, owner, programId: chunkProgramId });
  const [chunkBroken] = deriveChunkBrokenPda({
    globalConfig,
    chunkX: firstChunkX,
    chunkZ: firstChunkZ,
    programId: chunkProgramId,
  });
  const [foundationChunk] = deriveFoundationChunkPda({
    globalConfig,
    chunkX: firstChunkX,
    chunkZ: firstChunkZ,
    programId: chunkProgramId,
  });
  const [resourceDropTable] = deriveResourceDropTablePda({ globalConfig, programId: chunkProgramId });
  const [surfaceDecorationTable] = deriveSurfaceDecorationTablePda({ globalConfig, programId: chunkProgramId });
  const [materialPhysics] = deriveMaterialPhysicsPda({
    globalConfig,
    programId: NICECHUNK_BACKPACK_PROGRAM_ID,
  });
  const data = Buffer.alloc(3 + blocks.length * 12);
  data.writeUInt8(20, 0);
  data.writeUInt8(mode, 1);
  data.writeUInt8(blocks.length, 2);
  blocks.forEach((block, index) => {
    const offset = 3 + index * 12;
    data.writeInt32LE(block.worldX, offset);
    data.writeInt16LE(block.worldY, offset + 4);
    data.writeInt32LE(block.worldZ, offset + 6);
    data.writeUInt16LE(block.expectedBlockId!, offset + 10);
  });

  return new TransactionInstruction({
    programId: chunkProgramId,
    keys: [
      { pubkey: payer, isSigner: true, isWritable: true },
      { pubkey: playerProfile, isSigner: false, isWritable: false },
      { pubkey: playerSession, isSigner: false, isWritable: false },
      { pubkey: playerProgress, isSigner: false, isWritable: true },
      { pubkey: chunkBroken, isSigner: false, isWritable: true },
      { pubkey: foundationChunk, isSigner: false, isWritable: false },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
      { pubkey: resourceDropTable, isSigner: false, isWritable: false },
      { pubkey: surfaceDecorationTable, isSigner: false, isWritable: false },
      { pubkey: NICECHUNK_BACKPACK_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: backpack, isSigner: false, isWritable: true },
      { pubkey: materialPhysics, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: chunkInstructionData(chunkProgramId, data),
  });
}

export function createRangeMineWithRewardsInstruction({
  payer,
  owner,
  blocks,
  backpack,
  mode = RANGE_MINE_MODE_DEBUG,
  sessionAuthority = payer,
  chunkProgramId = NICECHUNK_CHUNK_PROGRAM_ID,
  playerProgramId = NICECHUNK_PLAYER_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
  chunkSize = 16,
}: {
  payer: PublicKey;
  owner: PublicKey;
  blocks: MineBlockInput[];
  backpack: PublicKey;
  mode?: number;
  sessionAuthority?: PublicKey;
  chunkProgramId?: PublicKey;
  playerProgramId?: PublicKey;
  coreProgramId?: PublicKey;
  chunkSize?: number;
}): TransactionInstruction {
  if (!owner) throw new Error("owner is required for range mining");
  if (!backpack) throw new Error("backpack is required for range mining");
  if (mode !== RANGE_MINE_MODE_DEBUG) throw new Error("unsupported range mining mode");
  const data = encodeRangeMineInstructionData(blocks, mode);
  const firstChunkX = Math.floor(blocks[0].worldX / chunkSize);
  const firstChunkZ = Math.floor(blocks[0].worldZ / chunkSize);
  for (const block of blocks) {
    if (Math.floor(block.worldX / chunkSize) !== firstChunkX || Math.floor(block.worldZ / chunkSize) !== firstChunkZ) {
      throw new Error("all range mining blocks must belong to one chunk");
    }
  }

  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  const [playerProfile] = derivePlayerProfilePda(owner, playerProgramId);
  const [playerSession] = derivePlayerSessionPda({ owner, sessionAuthority, programId: playerProgramId });
  const [playerProgress] = derivePlayerProgressPda({ globalConfig, owner, programId: chunkProgramId });
  const [chunkBroken] = deriveChunkBrokenPda({
    globalConfig,
    chunkX: firstChunkX,
    chunkZ: firstChunkZ,
    programId: chunkProgramId,
  });
  const [foundationChunk] = deriveFoundationChunkPda({
    globalConfig,
    chunkX: firstChunkX,
    chunkZ: firstChunkZ,
    programId: chunkProgramId,
  });
  const [resourceDropTable] = deriveResourceDropTablePda({ globalConfig, programId: chunkProgramId });
  const [surfaceDecorationTable] = deriveSurfaceDecorationTablePda({ globalConfig, programId: chunkProgramId });
  const [materialPhysics] = deriveMaterialPhysicsPda({
    globalConfig,
    programId: NICECHUNK_BACKPACK_PROGRAM_ID,
  });

  return new TransactionInstruction({
    programId: chunkProgramId,
    keys: [
      { pubkey: payer, isSigner: true, isWritable: true },
      { pubkey: playerProfile, isSigner: false, isWritable: false },
      { pubkey: playerSession, isSigner: false, isWritable: false },
      { pubkey: playerProgress, isSigner: false, isWritable: true },
      { pubkey: chunkBroken, isSigner: false, isWritable: true },
      { pubkey: foundationChunk, isSigner: false, isWritable: false },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
      { pubkey: resourceDropTable, isSigner: false, isWritable: false },
      { pubkey: surfaceDecorationTable, isSigner: false, isWritable: false },
      { pubkey: NICECHUNK_BACKPACK_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: backpack, isSigner: false, isWritable: true },
      { pubkey: materialPhysics, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: chunkInstructionData(chunkProgramId, data),
  });
}

export function encodeRangeMineInstructionData(
  sourceBlocks: MineBlockInput[],
  mode = RANGE_MINE_MODE_DEBUG,
): Buffer {
  if (mode !== RANGE_MINE_MODE_DEBUG) throw new Error("unsupported range mining mode");
  const blocks = sourceBlocks.map((block) => ({
    worldX: requiredInteger(block.worldX, "worldX"),
    worldY: requiredInteger(block.worldY, "worldY"),
    worldZ: requiredInteger(block.worldZ, "worldZ"),
    expectedBlockId: requiredInteger(block.expectedBlockId, "expectedBlockId"),
  }));
  if (!blocks.length || blocks.length > RANGE_MINE_MAX_BLOCKS) {
    throw new Error(`range mining requires 1-${RANGE_MINE_MAX_BLOCKS} selected blocks`);
  }
  const minX = Math.min(...blocks.map((block) => block.worldX));
  const maxX = Math.max(...blocks.map((block) => block.worldX));
  const minY = Math.min(...blocks.map((block) => block.worldY));
  const maxY = Math.max(...blocks.map((block) => block.worldY));
  const minZ = Math.min(...blocks.map((block) => block.worldZ));
  const maxZ = Math.max(...blocks.map((block) => block.worldZ));
  const sizeX = maxX - minX + 1;
  const sizeY = maxY - minY + 1;
  const sizeZ = maxZ - minZ + 1;
  const volume = sizeX * sizeY * sizeZ;
  if (sizeX < 1 || sizeX > 16 || sizeY < 1 || sizeY > 0xffff || sizeZ < 1 || sizeZ > 16 || volume > RANGE_MINE_MAX_BLOCKS) {
    throw new Error(`range mining volume exceeds ${RANGE_MINE_MAX_BLOCKS} cells`);
  }
  const byCoordinate = new Map<string, (typeof blocks)[number]>();
  for (const block of blocks) {
    const blockId = block.expectedBlockId;
    if (blockId < 1 || blockId > 63 || blockId === BLOCK_BEDROCK || blockId === BLOCK_WATER) {
      throw new Error(`invalid range mining block id: ${blockId}`);
    }
    const key = `${block.worldX},${block.worldY},${block.worldZ}`;
    if (byCoordinate.has(key)) throw new Error("range mining blocks must be unique");
    byCoordinate.set(key, block);
  }
  const bitmap = Buffer.alloc(Math.ceil(volume / 8));
  const blockIds: number[] = [];
  let volumeIndex = 0;
  for (let y = minY; y <= maxY; y += 1) {
    for (let z = minZ; z <= maxZ; z += 1) {
      for (let x = minX; x <= maxX; x += 1) {
        const block = byCoordinate.get(`${x},${y},${z}`);
        if (block) {
          bitmap[volumeIndex >> 3] |= 1 << (volumeIndex & 7);
          blockIds.push(block.expectedBlockId);
        }
        volumeIndex += 1;
      }
    }
  }
  const packedIds = Buffer.alloc(Math.ceil(blockIds.length * 6 / 8));
  blockIds.forEach((blockId, index) => {
    const bitIndex = index * 6;
    const byteIndex = bitIndex >> 3;
    const shift = bitIndex & 7;
    const packed = blockId << shift;
    packedIds[byteIndex] |= packed & 0xff;
    if (byteIndex + 1 < packedIds.length) packedIds[byteIndex + 1] |= (packed >> 8) & 0xff;
  });
  const data = Buffer.alloc(16 + bitmap.length + packedIds.length);
  data.writeUInt8(21, 0);
  data.writeUInt8(mode, 1);
  data.writeInt32LE(minX, 2);
  data.writeInt16LE(minY, 6);
  data.writeInt32LE(minZ, 8);
  data.writeUInt8(sizeX, 12);
  data.writeUInt16LE(sizeY, 13);
  data.writeUInt8(sizeZ, 15);
  bitmap.copy(data, 16);
  packedIds.copy(data, 16 + bitmap.length);
  return data;
}

function requiredInteger(value: unknown, label: string): number {
  const number = Number(value);
  if (!Number.isInteger(number)) throw new Error(`${label} must be an integer`);
  return number;
}

export function createBuildSiteInstruction({
  payer,
  owner,
  foundationId,
  foundation,
  sessionAuthority = payer,
  chunkProgramId = NICECHUNK_CHUNK_PROGRAM_ID,
  playerProgramId = NICECHUNK_PLAYER_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
}: {
  payer: PublicKey;
  owner: PublicKey;
  foundationId: bigint | number | string;
  foundation: FoundationInput;
  sessionAuthority?: PublicKey;
  chunkProgramId?: PublicKey;
  playerProgramId?: PublicKey;
  coreProgramId?: PublicKey;
}): TransactionInstruction {
  const normalized = normalizeBuildSiteInput(foundation);
  const normalizedFoundationId = normalizeFoundationId(foundationId);
  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  const [playerProfile] = derivePlayerProfilePda(owner, playerProgramId);
  const [playerSession] = derivePlayerSessionPda({ owner, sessionAuthority, programId: playerProgramId });
  const [buildSite] = deriveBuildSitePda({ globalConfig, foundationId: normalizedFoundationId, programId: chunkProgramId });
  const data = Buffer.alloc(27);
  data.writeUInt8(15, 0);
  data.writeBigUInt64LE(normalizedFoundationId, 1);
  data.writeInt32LE(normalized.minX, 9);
  data.writeInt16LE(normalized.surfaceY, 13);
  data.writeInt32LE(normalized.minZ, 15);
  data.writeUInt32LE(normalized.width, 19);
  data.writeUInt32LE(normalized.depth, 23);
  return new TransactionInstruction({
    programId: chunkProgramId,
    keys: [
      { pubkey: payer, isSigner: true, isWritable: true },
      { pubkey: playerProfile, isSigner: false, isWritable: false },
      { pubkey: playerSession, isSigner: false, isWritable: false },
      { pubkey: buildSite, isSigner: false, isWritable: true },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: chunkInstructionData(chunkProgramId, data),
  });
}

export function createBeginBuildingInstruction({
  payer,
  owner,
  foundationId,
  revision,
  quarterTurns,
  payloadLen,
  expectedHash,
  sessionAuthority = payer,
  chunkProgramId = NICECHUNK_CHUNK_PROGRAM_ID,
  playerProgramId = NICECHUNK_PLAYER_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
}: {
  payer: PublicKey;
  owner: PublicKey;
  foundationId: bigint | number | string;
  revision: number;
  quarterTurns: number;
  payloadLen: number;
  expectedHash: Buffer | Uint8Array;
  sessionAuthority?: PublicKey;
  chunkProgramId?: PublicKey;
  playerProgramId?: PublicKey;
  coreProgramId?: PublicKey;
}): TransactionInstruction {
  const id = normalizeFoundationId(foundationId);
  const safeRevision = requireU32(revision, "revision");
  const safePayloadLen = clampInt(payloadLen, 1, BUILDING_MAX_PAYLOAD_LEN);
  const hash = Buffer.from(expectedHash);
  if (hash.length !== 32) throw new Error("expectedHash must contain 32 bytes");
  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  const [playerProfile] = derivePlayerProfilePda(owner, playerProgramId);
  const [playerSession] = derivePlayerSessionPda({ owner, sessionAuthority, programId: playerProgramId });
  const [buildSite] = deriveBuildSitePda({ globalConfig, foundationId: id, programId: chunkProgramId });
  const [manifest] = deriveBuildingManifestPda({ globalConfig, foundationId: id, revision: safeRevision, programId: chunkProgramId });
  const data = Buffer.alloc(50);
  data.writeUInt8(16, 0);
  data.writeBigUInt64LE(id, 1);
  data.writeUInt32LE(safeRevision, 9);
  data.writeUInt8(clampInt(quarterTurns, 0, 3), 13);
  data.writeUInt32LE(safePayloadLen, 14);
  hash.copy(data, 18);
  return new TransactionInstruction({
    programId: chunkProgramId,
    keys: [
      { pubkey: payer, isSigner: true, isWritable: true },
      { pubkey: playerProfile, isSigner: false, isWritable: false },
      { pubkey: playerSession, isSigner: false, isWritable: false },
      { pubkey: buildSite, isSigner: false, isWritable: true },
      { pubkey: manifest, isSigner: false, isWritable: true },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: chunkInstructionData(chunkProgramId, data),
  });
}

export function createWriteBuildingShardInstruction({
  payer,
  owner,
  foundationId,
  revision,
  shardIndex,
  offset,
  bytes,
  sessionAuthority = payer,
  chunkProgramId = NICECHUNK_CHUNK_PROGRAM_ID,
  playerProgramId = NICECHUNK_PLAYER_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
}: {
  payer: PublicKey;
  owner: PublicKey;
  foundationId: bigint | number | string;
  revision: number;
  shardIndex: number;
  offset: number;
  bytes: Buffer | Uint8Array;
  sessionAuthority?: PublicKey;
  chunkProgramId?: PublicKey;
  playerProgramId?: PublicKey;
  coreProgramId?: PublicKey;
}): TransactionInstruction {
  const id = normalizeFoundationId(foundationId);
  const safeRevision = requireU32(revision, "revision");
  const safeShardIndex = clampInt(shardIndex, 0, 255);
  const safeOffset = clampInt(offset, 0, 0xffff);
  const payload = Buffer.from(bytes);
  if (!payload.length || payload.length > BUILDING_MAX_WRITE_LEN) throw new Error("Invalid building write length");
  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  const [playerProfile] = derivePlayerProfilePda(owner, playerProgramId);
  const [playerSession] = derivePlayerSessionPda({ owner, sessionAuthority, programId: playerProgramId });
  const [buildSite] = deriveBuildSitePda({ globalConfig, foundationId: id, programId: chunkProgramId });
  const [manifest] = deriveBuildingManifestPda({ globalConfig, foundationId: id, revision: safeRevision, programId: chunkProgramId });
  const [shard] = deriveBuildingShardPda({ globalConfig, foundationId: id, revision: safeRevision, shardIndex: safeShardIndex, programId: chunkProgramId });
  const data = Buffer.alloc(16 + payload.length);
  data.writeUInt8(17, 0);
  data.writeBigUInt64LE(id, 1);
  data.writeUInt32LE(safeRevision, 9);
  data.writeUInt8(safeShardIndex, 13);
  data.writeUInt16LE(safeOffset, 14);
  payload.copy(data, 16);
  return new TransactionInstruction({
    programId: chunkProgramId,
    keys: [
      { pubkey: payer, isSigner: true, isWritable: true },
      { pubkey: playerProfile, isSigner: false, isWritable: false },
      { pubkey: playerSession, isSigner: false, isWritable: false },
      { pubkey: buildSite, isSigner: false, isWritable: false },
      { pubkey: manifest, isSigner: false, isWritable: true },
      { pubkey: shard, isSigner: false, isWritable: true },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: chunkInstructionData(chunkProgramId, data),
  });
}

export function createFinalizeBuildingInstruction({
  payer,
  owner,
  foundationId,
  revision,
  shardCount,
  sessionAuthority = payer,
  chunkProgramId = NICECHUNK_CHUNK_PROGRAM_ID,
  playerProgramId = NICECHUNK_PLAYER_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
}: {
  payer: PublicKey;
  owner: PublicKey;
  foundationId: bigint | number | string;
  revision: number;
  shardCount: number;
  sessionAuthority?: PublicKey;
  chunkProgramId?: PublicKey;
  playerProgramId?: PublicKey;
  coreProgramId?: PublicKey;
}): TransactionInstruction {
  const id = normalizeFoundationId(foundationId);
  const safeRevision = requireU32(revision, "revision");
  const safeShardCount = clampInt(shardCount, 1, Math.ceil(BUILDING_MAX_PAYLOAD_LEN / BUILDING_SHARD_PAYLOAD_LEN));
  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  const [playerProfile] = derivePlayerProfilePda(owner, playerProgramId);
  const [playerSession] = derivePlayerSessionPda({ owner, sessionAuthority, programId: playerProgramId });
  const [buildSite] = deriveBuildSitePda({ globalConfig, foundationId: id, programId: chunkProgramId });
  const [manifest] = deriveBuildingManifestPda({ globalConfig, foundationId: id, revision: safeRevision, programId: chunkProgramId });
  const data = Buffer.alloc(13);
  data.writeUInt8(18, 0);
  data.writeBigUInt64LE(id, 1);
  data.writeUInt32LE(safeRevision, 9);
  return new TransactionInstruction({
    programId: chunkProgramId,
    keys: [
      { pubkey: payer, isSigner: true, isWritable: true },
      { pubkey: playerProfile, isSigner: false, isWritable: false },
      { pubkey: playerSession, isSigner: false, isWritable: false },
      { pubkey: buildSite, isSigner: false, isWritable: true },
      { pubkey: manifest, isSigner: false, isWritable: true },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ...Array.from({ length: safeShardCount }, (_unused, index) => ({
        pubkey: deriveBuildingShardPda({ globalConfig, foundationId: id, revision: safeRevision, shardIndex: index, programId: chunkProgramId })[0],
        isSigner: false,
        isWritable: false,
      })),
    ],
    data: chunkInstructionData(chunkProgramId, data),
  });
}

export function createCancelBuildingUploadInstruction({
  payer,
  owner,
  foundationId,
  revision,
  shardCount,
  sessionAuthority = payer,
  chunkProgramId = NICECHUNK_CHUNK_PROGRAM_ID,
  playerProgramId = NICECHUNK_PLAYER_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
}: {
  payer: PublicKey;
  owner: PublicKey;
  foundationId: bigint | number | string;
  revision: number;
  shardCount: number;
  sessionAuthority?: PublicKey;
  chunkProgramId?: PublicKey;
  playerProgramId?: PublicKey;
  coreProgramId?: PublicKey;
}): TransactionInstruction {
  const id = normalizeFoundationId(foundationId);
  const safeRevision = requireU32(revision, "revision");
  const safeShardCount = clampInt(shardCount, 1, Math.ceil(BUILDING_MAX_PAYLOAD_LEN / BUILDING_SHARD_PAYLOAD_LEN));
  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  const [playerProfile] = derivePlayerProfilePda(owner, playerProgramId);
  const [playerSession] = derivePlayerSessionPda({ owner, sessionAuthority, programId: playerProgramId });
  const [buildSite] = deriveBuildSitePda({ globalConfig, foundationId: id, programId: chunkProgramId });
  const [manifest] = deriveBuildingManifestPda({ globalConfig, foundationId: id, revision: safeRevision, programId: chunkProgramId });
  const data = Buffer.alloc(13);
  data.writeUInt8(19, 0);
  data.writeBigUInt64LE(id, 1);
  data.writeUInt32LE(safeRevision, 9);
  return new TransactionInstruction({
    programId: chunkProgramId,
    keys: [
      { pubkey: payer, isSigner: true, isWritable: true },
      { pubkey: playerProfile, isSigner: false, isWritable: false },
      { pubkey: playerSession, isSigner: false, isWritable: false },
      { pubkey: buildSite, isSigner: false, isWritable: true },
      { pubkey: manifest, isSigner: false, isWritable: true },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ...Array.from({ length: safeShardCount }, (_unused, index) => ({
        pubkey: deriveBuildingShardPda({ globalConfig, foundationId: id, revision: safeRevision, shardIndex: index, programId: chunkProgramId })[0],
        isSigner: false,
        isWritable: true,
      })),
    ],
    data: chunkInstructionData(chunkProgramId, data),
  });
}

export function createFoundationInstruction({
  payer,
  owner,
  foundationId,
  foundation,
  sessionAuthority = payer,
  chunkProgramId = NICECHUNK_CHUNK_PROGRAM_ID,
  playerProgramId = NICECHUNK_PLAYER_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
  chunkSize = CANONICAL_CHUNK_WORLD_CONFIG.chunkSize,
}: {
  payer: PublicKey;
  owner: PublicKey;
  foundationId: bigint | number | string;
  foundation: FoundationInput;
  sessionAuthority?: PublicKey;
  chunkProgramId?: PublicKey;
  playerProgramId?: PublicKey;
  coreProgramId?: PublicKey;
  chunkSize?: number;
}): TransactionInstruction {
  const normalized = normalizeFoundationInput(foundation);
  const normalizedFoundationId = normalizeFoundationId(foundationId);
  const safeChunkSize = requirePositiveInt(chunkSize, "chunkSize");
  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  const [playerProfile] = derivePlayerProfilePda(owner, playerProgramId);
  const [playerSession] = derivePlayerSessionPda({
    owner,
    sessionAuthority,
    programId: playerProgramId,
  });
  const [foundationPda] = deriveFoundationPda({
    globalConfig,
    owner,
    foundationId: normalizedFoundationId,
    programId: chunkProgramId,
  });
  const chunks = foundationChunks(normalized, safeChunkSize);
  const data = Buffer.alloc(21);
  data.writeUInt8(14, 0);
  data.writeBigUInt64LE(normalizedFoundationId, 1);
  data.writeInt32LE(normalized.minX, 9);
  data.writeInt16LE(normalized.surfaceY, 13);
  data.writeInt32LE(normalized.minZ, 15);
  data.writeUInt8(normalized.width, 19);
  data.writeUInt8(normalized.depth, 20);

  return new TransactionInstruction({
    programId: chunkProgramId,
    keys: [
      { pubkey: payer, isSigner: true, isWritable: true },
      { pubkey: playerProfile, isSigner: false, isWritable: false },
      { pubkey: playerSession, isSigner: false, isWritable: false },
      { pubkey: foundationPda, isSigner: false, isWritable: true },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ...chunks.map(({ chunkX, chunkZ }) => ({
        pubkey: deriveFoundationChunkPda({
          globalConfig,
          chunkX,
          chunkZ,
          programId: chunkProgramId,
        })[0],
        isSigner: false,
        isWritable: true,
      })),
    ],
    data: chunkInstructionData(chunkProgramId, data),
  });
}

export function createInitializeResourceDropTableInstruction({
  payer,
  rules,
  chunkProgramId = NICECHUNK_CHUNK_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
}: {
  payer: PublicKey;
  rules: ResourceDropRuleInput[];
  chunkProgramId?: PublicKey;
  coreProgramId?: PublicKey;
}): TransactionInstruction {
  if (!rules.length || rules.length > 64) {
    throw new Error(`Invalid resource drop rule count: ${rules.length}`);
  }
  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  const [resourceDropTable] = deriveResourceDropTablePda({ globalConfig, programId: chunkProgramId });
  const data = Buffer.alloc(2 + rules.length * RESOURCE_DROP_RULE_LEN);
  data.writeUInt8(7, 0);
  data.writeUInt8(rules.length, 1);
  rules.forEach((rule, index) => {
    const offset = 2 + index * RESOURCE_DROP_RULE_LEN;
    data.writeUInt16LE(rule.sourceBlockId, offset);
    data.writeUInt16LE(rule.dropBlockId, offset + 2);
    data.writeUInt16LE(rule.chanceBps, offset + 4);
    data.writeInt16LE(rule.minAltitude, offset + 6);
    data.writeInt16LE(rule.maxAltitude, offset + 8);
    data.writeInt16LE(rule.minDepth, offset + 10);
    data.writeInt16LE(rule.maxDepth, offset + 12);
    data.writeUInt8(rule.salt, offset + 14);
    data.writeUInt32LE(rule.minVolumeMm3, offset + 15);
    data.writeUInt32LE(rule.maxVolumeMm3, offset + 19);
  });
  return new TransactionInstruction({
    programId: chunkProgramId,
    keys: [
      { pubkey: payer, isSigner: true, isWritable: true },
      { pubkey: resourceDropTable, isSigner: false, isWritable: true },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: chunkInstructionData(chunkProgramId, data),
  });
}

export function encodeCivilizationResourceDropRulesPatch(rules: ResourceDropRuleInput[]): Buffer {
  if (!rules.length || rules.length > 64) {
    throw new Error(`Invalid resource drop rule count: ${rules.length}`);
  }
  const data = Buffer.alloc(1 + rules.length * RESOURCE_DROP_RULE_LEN);
  data.writeUInt8(rules.length, 0);
  rules.forEach((rule, index) => {
    writeResourceDropRule(data, 1 + index * RESOURCE_DROP_RULE_LEN, rule);
  });
  return data;
}

export function createApplyCivilizationResourceDropRulesInstruction({
  executor,
  resourceDropTable,
  globalConfig,
  ruleBook,
  tally,
  receipt,
  rules,
  chunkProgramId = NICECHUNK_CHUNK_PROGRAM_ID,
  civilizationProgramId = NICECHUNK_CIVILIZATION_PROGRAM_ID,
}: {
  executor: PublicKey;
  resourceDropTable: PublicKey;
  globalConfig: PublicKey;
  ruleBook: PublicKey;
  tally: PublicKey;
  receipt: PublicKey;
  rules: ResourceDropRuleInput[];
  chunkProgramId?: PublicKey;
  civilizationProgramId?: PublicKey;
}): TransactionInstruction {
  const patch = encodeCivilizationResourceDropRulesPatch(rules);
  const [adapterAuthority] = deriveCivilizationAdapterAuthorityPda({
    ruleBook,
    targetProgram: chunkProgramId,
  });
  return new TransactionInstruction({
    programId: chunkProgramId,
    keys: [
      { pubkey: executor, isSigner: true, isWritable: true },
      { pubkey: resourceDropTable, isSigner: false, isWritable: true },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
      { pubkey: ruleBook, isSigner: false, isWritable: true },
      { pubkey: tally, isSigner: false, isWritable: false },
      { pubkey: receipt, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      { pubkey: civilizationProgramId, isSigner: false, isWritable: false },
      { pubkey: adapterAuthority, isSigner: false, isWritable: false },
    ],
    data: chunkInstructionData(chunkProgramId, Buffer.concat([Buffer.from([10]), patch])),
  });
}

function writeResourceDropRule(data: Buffer, offset: number, rule: ResourceDropRuleInput): void {
  data.writeUInt16LE(clampInt(rule.sourceBlockId, 0, 0xffff), offset);
  data.writeUInt16LE(clampInt(rule.dropBlockId, 0, 0xffff), offset + 2);
  data.writeUInt16LE(clampInt(rule.chanceBps, 0, 10_000), offset + 4);
  data.writeInt16LE(clampInt(rule.minAltitude, -0x8000, 0x7fff), offset + 6);
  data.writeInt16LE(clampInt(rule.maxAltitude, -0x8000, 0x7fff), offset + 8);
  data.writeInt16LE(clampInt(rule.minDepth, -0x8000, 0x7fff), offset + 10);
  data.writeInt16LE(clampInt(rule.maxDepth, -0x8000, 0x7fff), offset + 12);
  data.writeUInt8(clampInt(rule.salt, 0, 0xff), offset + 14);
  data.writeUInt32LE(clampInt(rule.minVolumeMm3, 0, 0xffffffff), offset + 15);
  data.writeUInt32LE(clampInt(rule.maxVolumeMm3, 0, 0xffffffff), offset + 19);
}

export function encodeSurfaceDecorationRules(rules: SurfaceDecorationRuleInput[]): Buffer {
  const normalized = normalizeSurfaceDecorationRules(rules);
  const data = Buffer.alloc(1 + normalized.length * SURFACE_DECORATION_RULE_LEN);
  data.writeUInt8(normalized.length, 0);
  normalized.forEach((rule, index) => {
    writeSurfaceDecorationRule(data, 1 + index * SURFACE_DECORATION_RULE_LEN, rule);
  });
  return data;
}

export function decodeSurfaceDecorationTable(data: Buffer | Uint8Array): DecodedSurfaceDecorationTable {
  const bytes = Buffer.from(data);
  if (bytes.length !== SURFACE_DECORATION_TABLE_LEN) {
    throw new Error(`Invalid SurfaceDecorationTable length: ${bytes.length}`);
  }
  const magic = bytes.subarray(0, 8).toString("utf8");
  const version = bytes.readUInt8(8);
  const bump = bytes.readUInt8(9);
  const count = bytes.readUInt8(10);
  const revision = bytes.readUInt32LE(12);
  if (
    magic !== SURFACE_DECORATION_TABLE_MAGIC ||
    version !== SURFACE_DECORATION_TABLE_VERSION ||
    count === 0 ||
    count > SURFACE_DECORATION_RULE_MAX_COUNT
  ) {
    throw new Error("Invalid SurfaceDecorationTable header");
  }
  const rules: SurfaceDecorationRuleInput[] = [];
  for (let index = 0; index < count; index += 1) {
    const offset = SURFACE_DECORATION_TABLE_HEADER_LEN + index * SURFACE_DECORATION_RULE_LEN;
    rules.push(readSurfaceDecorationRule(bytes, offset));
  }
  normalizeSurfaceDecorationRules(rules);
  return { magic, version, bump, revision, rules };
}

export function createInitializeSurfaceDecorationTableInstruction({
  payer,
  rules,
  chunkProgramId = NICECHUNK_CHUNK_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
}: {
  payer: PublicKey;
  rules: SurfaceDecorationRuleInput[];
  chunkProgramId?: PublicKey;
  coreProgramId?: PublicKey;
}): TransactionInstruction {
  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  const [surfaceDecorationTable] = deriveSurfaceDecorationTablePda({
    globalConfig,
    programId: chunkProgramId,
  });
  return new TransactionInstruction({
    programId: chunkProgramId,
    keys: [
      { pubkey: payer, isSigner: true, isWritable: true },
      { pubkey: surfaceDecorationTable, isSigner: false, isWritable: true },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: chunkInstructionData(
      chunkProgramId,
      Buffer.concat([Buffer.from([11]), encodeSurfaceDecorationRules(rules)]),
    ),
  });
}

export function createVerifySurfaceDecorationInstruction({
  worldX,
  worldZ,
  expectedSurfaceBlockId,
  expectedDecorationId = 0,
  expectedRuleId = 0,
  chunkProgramId = NICECHUNK_CHUNK_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
}: {
  worldX: number;
  worldZ: number;
  expectedSurfaceBlockId: number;
  expectedDecorationId?: number;
  expectedRuleId?: number;
  chunkProgramId?: PublicKey;
  coreProgramId?: PublicKey;
}): TransactionInstruction {
  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  const [surfaceDecorationTable] = deriveSurfaceDecorationTablePda({
    globalConfig,
    programId: chunkProgramId,
  });
  const data = Buffer.alloc(15);
  data.writeUInt8(12, 0);
  data.writeInt32LE(clampInt(worldX, -0x80000000, 0x7fffffff), 1);
  data.writeInt32LE(clampInt(worldZ, -0x80000000, 0x7fffffff), 5);
  data.writeUInt16LE(clampInt(expectedSurfaceBlockId, 0, 0xffff), 9);
  data.writeUInt16LE(clampInt(expectedDecorationId, 0, 0xffff), 11);
  data.writeUInt16LE(clampInt(expectedRuleId, 0, 0xffff), 13);
  return new TransactionInstruction({
    programId: chunkProgramId,
    keys: [
      { pubkey: surfaceDecorationTable, isSigner: false, isWritable: false },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
    ],
    data: chunkInstructionData(chunkProgramId, data),
  });
}

export function createApplyCivilizationSurfaceDecorationRulesInstruction({
  executor,
  surfaceDecorationTable,
  globalConfig,
  ruleBook,
  tally,
  receipt,
  rules,
  chunkProgramId = NICECHUNK_CHUNK_PROGRAM_ID,
  civilizationProgramId = NICECHUNK_CIVILIZATION_PROGRAM_ID,
}: {
  executor: PublicKey;
  surfaceDecorationTable: PublicKey;
  globalConfig: PublicKey;
  ruleBook: PublicKey;
  tally: PublicKey;
  receipt: PublicKey;
  rules: SurfaceDecorationRuleInput[];
  chunkProgramId?: PublicKey;
  civilizationProgramId?: PublicKey;
}): TransactionInstruction {
  const payload = encodeSurfaceDecorationRules(rules);
  const [adapterAuthority] = deriveCivilizationAdapterAuthorityPda({
    ruleBook,
    targetProgram: chunkProgramId,
  });
  return new TransactionInstruction({
    programId: chunkProgramId,
    keys: [
      { pubkey: executor, isSigner: true, isWritable: true },
      { pubkey: surfaceDecorationTable, isSigner: false, isWritable: true },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
      { pubkey: ruleBook, isSigner: false, isWritable: true },
      { pubkey: tally, isSigner: false, isWritable: false },
      { pubkey: receipt, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      { pubkey: civilizationProgramId, isSigner: false, isWritable: false },
      { pubkey: adapterAuthority, isSigner: false, isWritable: false },
    ],
    data: chunkInstructionData(
      chunkProgramId,
      Buffer.concat([Buffer.from([13]), payload]),
    ),
  });
}

export function resolveSurfaceDecorationAt(
  globalConfig: MinimalGlobalConfigForBlockVerification,
  rules: SurfaceDecorationRuleInput[],
  worldX: number,
  worldZ: number,
): SurfaceDecorationMatch | null {
  const config = canonicalChunkWorldConfig(globalConfig);
  const x = Math.trunc(worldX);
  const z = Math.trunc(worldZ);
  const surfaceY = canonicalSurfaceHeight(config, x, z);
  const waterY = canonicalWaterLevel(config, x, z, surfaceY);
  if (waterY !== null && waterY > surfaceY) return null;
  if (canonicalTreeBlockIdAt(config, x, surfaceY + 1, z) !== BLOCK_AIR) return null;
  const surfaceBlockId = canonicalSurfaceBlockId(config, x, z, surfaceY);
  const normalized = normalizeSurfaceDecorationRules(rules);
  let activeSalt = -1;
  let roll = -1;
  for (const rule of normalized) {
    if (rule.surfaceBlockId !== surfaceBlockId || surfaceY < rule.minY || surfaceY > rule.maxY) continue;
    if (rule.salt !== activeSalt) {
      activeSalt = rule.salt;
      roll = hashCoord3(config.worldSeed, x, surfaceY + 1, z, 1200 + rule.salt) % SURFACE_DECORATION_ROLL_DENOMINATOR;
    }
    if (roll < rule.rollStartBps || roll >= rule.rollEndBps) continue;
    return { ...rule, surfaceY, roll };
  }
  return null;
}

function normalizeSurfaceDecorationRules(rules: SurfaceDecorationRuleInput[]): SurfaceDecorationRuleInput[] {
  if (!Array.isArray(rules) || !rules.length || rules.length > SURFACE_DECORATION_RULE_MAX_COUNT) {
    throw new Error(`Invalid surface decoration rule count: ${rules?.length ?? 0}`);
  }
  const ids = new Set<number>();
  return rules.map((input, index) => {
    const rule = {
      ruleId: clampInt(input.ruleId, 0, 0xffff),
      decorationId: clampInt(input.decorationId, 0, 0xffff),
      surfaceBlockId: clampInt(input.surfaceBlockId, 0, 0xffff),
      dropBlockId: clampInt(input.dropBlockId, 0, 0xffff),
      rollStartBps: clampInt(input.rollStartBps, 0, SURFACE_DECORATION_ROLL_DENOMINATOR),
      rollEndBps: clampInt(input.rollEndBps, 0, SURFACE_DECORATION_ROLL_DENOMINATOR),
      minY: clampInt(input.minY, -0x8000, 0x7fff),
      maxY: clampInt(input.maxY, -0x8000, 0x7fff),
      salt: clampInt(input.salt, 0, 0xffff),
      variant: clampInt(input.variant, 0, 0xff),
      flags: clampInt(input.flags, 0, 0xff),
    };
    if (
      rule.ruleId === 0 ||
      rule.decorationId === 0 ||
      rule.surfaceBlockId === BLOCK_AIR ||
      rule.surfaceBlockId === BLOCK_WATER ||
      rule.surfaceBlockId === BLOCK_BEDROCK ||
      rule.dropBlockId === BLOCK_AIR ||
      rule.dropBlockId === BLOCK_WATER ||
      rule.dropBlockId === BLOCK_BEDROCK ||
      rule.rollStartBps >= rule.rollEndBps ||
      rule.minY > rule.maxY ||
      ids.has(rule.ruleId)
    ) {
      throw new Error(`Invalid surface decoration rule at index ${index}`);
    }
    ids.add(rule.ruleId);
    return rule;
  });
}

function writeSurfaceDecorationRule(data: Buffer, offset: number, rule: SurfaceDecorationRuleInput): void {
  data.writeUInt16LE(rule.ruleId, offset);
  data.writeUInt16LE(rule.decorationId, offset + 2);
  data.writeUInt16LE(rule.surfaceBlockId, offset + 4);
  data.writeUInt16LE(rule.dropBlockId, offset + 6);
  data.writeUInt16LE(rule.rollStartBps, offset + 8);
  data.writeUInt16LE(rule.rollEndBps, offset + 10);
  data.writeInt16LE(rule.minY, offset + 12);
  data.writeInt16LE(rule.maxY, offset + 14);
  data.writeUInt16LE(rule.salt, offset + 16);
  data.writeUInt8(rule.variant, offset + 18);
  data.writeUInt8(rule.flags, offset + 19);
}

function readSurfaceDecorationRule(data: Buffer, offset: number): SurfaceDecorationRuleInput {
  return {
    ruleId: data.readUInt16LE(offset),
    decorationId: data.readUInt16LE(offset + 2),
    surfaceBlockId: data.readUInt16LE(offset + 4),
    dropBlockId: data.readUInt16LE(offset + 6),
    rollStartBps: data.readUInt16LE(offset + 8),
    rollEndBps: data.readUInt16LE(offset + 10),
    minY: data.readInt16LE(offset + 12),
    maxY: data.readInt16LE(offset + 14),
    salt: data.readUInt16LE(offset + 16),
    variant: data.readUInt8(offset + 18),
    flags: data.readUInt8(offset + 19),
  };
}

export function generatedBlockIdAt(
  globalConfig: MinimalGlobalConfigForBlockVerification,
  block: GeneratedBlockInput,
): number {
  const canonicalConfig = canonicalChunkWorldConfig(globalConfig);
  const worldX = block.chunkX * canonicalConfig.chunkSize + block.localX;
  const worldZ = block.chunkZ * canonicalConfig.chunkSize + block.localZ;
  return canonicalBlockIdAt(canonicalConfig, worldX, block.y, worldZ);
}

export function generatedSurfaceHeight(
  globalConfig: MinimalGlobalConfigForBlockVerification,
  worldX: number,
  worldZ: number,
): number {
  return canonicalSurfaceHeight(canonicalChunkWorldConfig(globalConfig), worldX, worldZ);
}

function canonicalBlockIdAt(
  globalConfig: MinimalGlobalConfigForBlockVerification,
  x: number,
  y: number,
  z: number,
): number {
  if (y <= globalConfig.minBuildY) return BLOCK_BEDROCK;
  if (y > globalConfig.maxBuildY) return BLOCK_AIR;
  const surface = canonicalSurfaceHeight(globalConfig, x, z);
  if (y > surface) {
    if (y <= globalConfig.seaLevel + MAX_WATER_LEVEL_ABOVE_SEA) {
      const waterLevel = canonicalWaterLevel(globalConfig, x, z, surface);
      if (waterLevel !== null && y <= waterLevel) return BLOCK_WATER;
    }
    const treeBlock = canonicalTreeBlockIdAt(globalConfig, x, y, z);
    if (treeBlock !== BLOCK_AIR) return treeBlock;
    return BLOCK_AIR;
  }
  if (y === surface) return canonicalSurfaceBlockId(globalConfig, x, z, surface);
  const depth = surface - y;
  if (depth <= 3) return canonicalSubsurfaceBlockId(globalConfig, x, z, surface);
  if (depth >= 8 && canonicalCoalSeamAt(globalConfig, x, y, z, surface)) return BLOCK_COAL;
  if (y <= globalConfig.minBuildY + 40 || depth >= 52) return BLOCK_DEEP_STONE;
  if (canonicalVolcanicAt(globalConfig, x, z) > 238 && hashCoord3(globalConfig.worldSeed, x, y, z, 601) > 210) {
    return BLOCK_BASALT;
  }
  return BLOCK_STONE;
}

function canonicalSurfaceHeight(
  globalConfig: MinimalGlobalConfigForBlockVerification,
  x: number,
  z: number,
): number {
  const maxSurface = Math.max(globalConfig.minBuildY + 8, Math.min(globalConfig.maxTerrainHeight, globalConfig.maxBuildY - 1));
  const desiredMinSurface = Math.max(globalConfig.minBuildY + 8, globalConfig.seaLevel - 28);
  const minSurface = Math.min(desiredMinSurface, maxSurface);
  const terrain = canonicalTerrainFactors(globalConfig, x, z);
  const { wx, wz, shelf, inland, waterMask, valleyMask, floodplainMask, lake, valleySoftness, openRiver } = terrain;

  const ocean =
    globalConfig.seaLevel - 16 +
    Math.trunc((valueNoise2(globalConfig.worldSeed, wx, wz, 96, 24) - 128) * 5 / 128) +
    Math.trunc((valueNoise2(globalConfig.worldSeed, wx, wz, 36, 25) - 128) * 2 / 128);
  const coast = globalConfig.seaLevel - 3 + Math.trunc(shelf * 8 / 1024);
  const plains = Math.trunc((valueNoise2(globalConfig.worldSeed, wx, wz, 120, 26) - 128) * 4 / 128);
  const hills = Math.trunc((valueNoise2(globalConfig.worldSeed, wx, wz, 56, 27) - 128) * 7 / 128);
  const rolling = Math.trunc((valueNoise2(globalConfig.worldSeed, wx, wz, 28, 28) - 128) * 2 / 128);
  const roughness = smoothRangeFixed(Math.abs(valueNoise2(globalConfig.worldSeed, wx, wz, 180, 40) - 128), 54, 122);

  const mountainRange = scaleByFixed(smoothRangeFixed(valueNoise2(globalConfig.worldSeed, wx, wz, 360, 30), 136, 226), inland);
  const highland = scaleByFixed(34, scaleByFixed(smoothRangeFixed(valueNoise2(globalConfig.worldSeed, wx, wz, 620, 46), 116, 206), inland));
  const ridgeLine = 128 - Math.abs(valueNoise2(globalConfig.worldSeed, wx, wz, 92, 29) - 128);
  const ridgeLift = smoothRangeFixed(ridgeLine, 44, 126);
  const peakMask = scaleByFixed(smoothRangeFixed(valueNoise2(globalConfig.worldSeed, wx, wz, 176, 47), 176, 242), mountainRange);
  const crag = scaleByFixed(smoothRangeFixed(Math.abs(valueNoise2(globalConfig.worldSeed, wx, wz, 52, 48) - 128), 48, 126), mountainRange);
  const mountain = highland + scaleByFixed(24 + scaleByFixed(72, ridgeLift) + scaleByFixed(24, crag), mountainRange) + scaleByFixed(34, peakMask);

  const land = globalConfig.seaLevel + 7 + Math.trunc(inland * 8 / 1024) + scaleByFixed(plains + scaleByFixed(hills + rolling, roughness), inland) + mountain;
  let shapedLand = Math.max(coast, land);
  if (floodplainMask > 0) {
    const flatNoise = Math.trunc((valueNoise2(globalConfig.worldSeed, wx, wz, 54, 38) - 128) / 128);
    const floodplainLift = 2 + Math.trunc((1024 - openRiver) * 2 / 1024);
    const floodplainFloor = globalConfig.seaLevel + floodplainLift + flatNoise;
    const floodplainBlend = Math.min(1024, Math.trunc(floodplainMask * (720 + Math.trunc(openRiver * 420 / 1024)) / 1024));
    shapedLand = lerpIntFixed(shapedLand, Math.min(shapedLand, floodplainFloor), floodplainBlend);
  }
  if (valleyMask > 0) {
    const bedNoise = Math.trunc((valueNoise2(globalConfig.worldSeed, wx, wz, 32, 39) - 128) * 2 / 128);
    const slopeNoise = valueNoise2(globalConfig.worldSeed, wx, wz, 150, 42);
    const canyon = scaleByFixed(smoothRangeFixed(slopeNoise, 190, 252), 1024 - scaleByFixed(openRiver, 640));
    const gentle = 1024 - scaleByFixed(openRiver, 1024 - canyon);
    const slopeStrength = 220 + scaleByFixed(360, canyon) + scaleByFixed(90, gentle);
    const valleyBlend = Math.min(1024, Math.trunc(valleyMask * slopeStrength / 1024));
    const bankLift = 2 + Math.trunc((255 - valleySoftness) * 4 / 255);
    const valleyCut = Math.trunc(valleyMask * (1 + Math.trunc(slopeNoise / 86)) / 1024);
    const bankFloor = globalConfig.seaLevel + bankLift - valleyCut + bedNoise;
    shapedLand = lerpIntFixed(shapedLand, Math.min(shapedLand, bankFloor), valleyBlend);

    const coreStart = 84 + Math.trunc((255 - slopeNoise) * 172 / 255);
    const coreBlend = smoothRangeFixed(waterMask, coreStart, 1024);
    if (coreBlend > 0) {
      const waterBed = globalConfig.seaLevel - 1 - Math.trunc(waterMask * 4 / 1024) - Math.trunc(lake * 3 / 1024) + bedNoise;
      shapedLand = lerpIntFixed(shapedLand, waterBed, coreBlend);
    }
  }

  return clampInt(lerpIntFixed(ocean, shapedLand, shelf), minSurface, maxSurface);
}

function canonicalSurfaceBlockId(
  globalConfig: MinimalGlobalConfigForBlockVerification,
  x: number,
  z: number,
  surface: number,
): number {
  const waterLevel = canonicalWaterLevel(globalConfig, x, z, surface);
  const underwater = waterLevel !== null && surface < waterLevel;
  const moisture = canonicalMoistureAt(globalConfig, x, z);
  const desert = canonicalDesertScoreAt(globalConfig, x, z);
  const gravelPatch = valueNoise2(globalConfig.worldSeed, x, z, 44, 103);
  const clayPatch = valueNoise2(globalConfig.worldSeed, x, z, 52, 104);

  if (underwater || surface <= globalConfig.seaLevel + 1) {
    if (moisture > 190 && clayPatch > 148) return BLOCK_CLAY;
    if (gravelPatch > 218) return BLOCK_GRAVEL;
    if (valueNoise2(globalConfig.worldSeed, x, z, 96, 105) > 236) return BLOCK_SHELL_BED;
    return BLOCK_SAND;
  }
  if (canonicalVolcanicAt(globalConfig, x, z) > 246) {
    return valueNoise2(globalConfig.worldSeed, x, z, 64, 106) > 180 ? BLOCK_BASALT : BLOCK_ASH;
  }
  if (canonicalColdAt(globalConfig, x, z, surface)) {
    return surface > globalConfig.seaLevel + 34 || valueNoise2(globalConfig.worldSeed, x, z, 72, 107) > 164
      ? BLOCK_SNOW
      : BLOCK_FROZEN_SOIL;
  }
  if (desert > 178) {
    if (desert > 226 && valueNoise2(globalConfig.worldSeed, x, z, 88, 108) > 188) return BLOCK_SALT_FLAT;
    return desert > 204 ? BLOCK_SAND : BLOCK_DRY_DIRT;
  }
  if (moisture > 188) {
    return moisture > 208 ? BLOCK_MUD : BLOCK_GRASS;
  }
  if (surface >= globalConfig.seaLevel + 36) return BLOCK_STONE;
  return BLOCK_GRASS;
}

function canonicalSubsurfaceBlockId(
  globalConfig: MinimalGlobalConfigForBlockVerification,
  x: number,
  z: number,
  surface: number,
): number {
  const top = canonicalSurfaceBlockId(globalConfig, x, z, surface);
  if ([BLOCK_SAND, BLOCK_SALT_FLAT, BLOCK_QUICKSAND].includes(top)) return BLOCK_SAND;
  if ([BLOCK_MUD, BLOCK_CLAY, BLOCK_MOSS].includes(top)) {
    return hashCoord3(globalConfig.worldSeed, x, surface - 1, z, 121) > 112 ? BLOCK_CLAY : BLOCK_MUD;
  }
  if ([BLOCK_SNOW, BLOCK_FROZEN_SOIL].includes(top)) return BLOCK_FROZEN_SOIL;
  if ([BLOCK_BASALT, BLOCK_ASH].includes(top)) return BLOCK_BASALT;
  if (top === BLOCK_STONE) return BLOCK_STONE;
  return BLOCK_DIRT;
}

function canonicalCoalSeamAt(
  globalConfig: MinimalGlobalConfigForBlockVerification,
  x: number,
  y: number,
  z: number,
  surface: number,
): boolean {
  const depth = surface - y;
  if (depth < 10 || y <= globalConfig.minBuildY + 4) return false;
  if (depth > 92 && y < globalConfig.minBuildY + 12) return false;

  const layerY = divFloor(y - globalConfig.minBuildY, 6);
  const cellX = divFloor(x, 10);
  const cellZ = divFloor(z, 10);
  const band = hashCoord3(globalConfig.worldSeed, cellX, layerY, cellZ, 301) & 255;
  if (band < 214) return false;
  if (depth < 18 || depth > 76) return false;

  const lens = hashCoord3(globalConfig.worldSeed, x + layerY * 17, y, z - layerY * 13, 302) & 255;
  const vein = hashCoord3(globalConfig.worldSeed, divFloor(x + y * 2, 4), layerY, divFloor(z - y * 3, 4), 303) & 255;
  return lens + Math.trunc(vein / 2) >= 228;
}

function canonicalTreeBlockIdAt(
  globalConfig: MinimalGlobalConfigForBlockVerification,
  x: number,
  y: number,
  z: number,
): number {
  for (let treeZ = z - TREE_MAX_LEAF_RADIUS; treeZ <= z + TREE_MAX_LEAF_RADIUS; treeZ += 1) {
    for (let treeX = x - TREE_MAX_LEAF_RADIUS; treeX <= x + TREE_MAX_LEAF_RADIUS; treeX += 1) {
      const surface = canonicalSurfaceHeight(globalConfig, treeX, treeZ);
      if (!canonicalCanGrowTree(globalConfig, treeX, treeZ, surface)) continue;
      const tree = canonicalTreeAt(globalConfig, treeX, treeZ, surface);
      if (!tree.exists) continue;
      const block = canonicalTreeVolumeBlock(globalConfig, tree, x, y, z);
      if (block !== BLOCK_AIR) return block;
    }
  }
  return BLOCK_AIR;
}

function canonicalCanGrowTree(
  globalConfig: MinimalGlobalConfigForBlockVerification,
  x: number,
  z: number,
  surface: number,
): boolean {
  if (surface <= globalConfig.seaLevel + 1) return false;
  const waterLevel = canonicalWaterLevel(globalConfig, x, z, surface);
  if (waterLevel !== null && surface < waterLevel) return false;
  if (canonicalDesertAt(globalConfig, x, z) || canonicalVolcanicAt(globalConfig, x, z) > 236) return false;
  return true;
}

function canonicalTreeAt(globalConfig: MinimalGlobalConfigForBlockVerification, x: number, z: number, surface: number) {
  const growth = canonicalTreeGrowthProfile(globalConfig, x, z, surface);
  const density = growth.density;
  const cellSize = growth.cellSize;
  const cellX = divFloor(x, cellSize);
  const cellZ = divFloor(z, cellSize);
  const originX = cellX * cellSize;
  const originZ = cellZ * cellSize;
  const inner = Math.max(1, cellSize - 2);
  const treeX = originX + 1 + (hashCoord3(globalConfig.worldSeed, cellX, 0, cellZ, 401) % inner);
  const treeZ = originZ + 1 + (hashCoord3(globalConfig.worldSeed, cellX, 0, cellZ, 402) % inner);
  const roll = hashCoord3(globalConfig.worldSeed, cellX, 0, cellZ, 403) & 255;
  return {
    ...canonicalTreeFromCandidate(globalConfig, x, z, surface),
    exists: x === treeX && z === treeZ && roll > density,
  };
}

function canonicalTreeFromCandidate(
  globalConfig: MinimalGlobalConfigForBlockVerification,
  x: number,
  z: number,
  surface: number,
) {
  const pine = canonicalTreeGrowthProfile(globalConfig, x, z, surface).pine;
  const trunkHeight = (pine ? 5 : 4) + (hashCoord3(globalConfig.worldSeed, x, surface, z, 405) % 3);
  return { exists: true, x, z, baseY: surface + 1, trunkHeight, pine };
}

function canonicalTreeVolumeBlock(
  globalConfig: MinimalGlobalConfigForBlockVerification,
  tree: { x: number; z: number; baseY: number; trunkHeight: number; pine: boolean },
  x: number,
  y: number,
  z: number,
): number {
  const top = tree.baseY + tree.trunkHeight;
  if (x === tree.x && z === tree.z && y >= tree.baseY && y < top) return tree.pine ? BLOCK_PINE_TRUNK : BLOCK_TRUNK;
  if (tree.pine) {
    const dy = y - top;
    const layer = dy === -4 ? [2, 158, 501]
      : dy === -3 ? [2, 188, 502]
      : dy === -2 ? [1, 218, 503]
      : dy === -1 ? [1, 184, 504]
      : dy === 0 ? [1, 138, 505]
      : null;
    if (layer && leafLayerContainsAtY(globalConfig, tree.x, tree.z, x, y, z, layer[0], layer[1], layer[2])) {
      return BLOCK_PINE_LEAVES;
    }
    if (dy === 1 && x === tree.x && z === tree.z) return BLOCK_PINE_LEAVES;
    return BLOCK_AIR;
  }
  const dy = y - top;
  const layer = dy === -2 ? [2, 174, 511]
    : dy === -1 ? [2, 214, 512]
    : dy === 0 ? [2, 148, 513]
    : dy === 1 ? [1, 194, 514]
    : null;
  if (layer && leafLayerContainsAtY(globalConfig, tree.x, tree.z, x, y, z, layer[0], layer[1], layer[2])) {
    return BLOCK_LEAVES;
  }
  return BLOCK_AIR;
}

function leafLayerContainsAtY(
  globalConfig: MinimalGlobalConfigForBlockVerification,
  centerX: number,
  centerZ: number,
  x: number,
  y: number,
  z: number,
  radius: number,
  density: number,
  salt: number,
): boolean {
  const dx = x - centerX;
  const dz = z - centerZ;
  if (Math.abs(dx) > radius || Math.abs(dz) > radius) return false;
  if (Math.abs(dx) + Math.abs(dz) > radius + 1) return false;
  const corner = Math.abs(dx) === radius && Math.abs(dz) === radius;
  const roll = hashCoord3(globalConfig.worldSeed, centerX + dx * 23, y, centerZ + dz * 29, salt) & 255;
  if (corner && roll < 178) return false;
  return roll <= density;
}

function canonicalColdAt(globalConfig: MinimalGlobalConfigForBlockVerification, x: number, z: number, surface: number): boolean {
  const snowLine = canonicalSnowLineAt(globalConfig, x, z);
  return surface >= snowLine
    || (surface >= snowLine - 7 && valueNoise2(globalConfig.worldSeed, x, z, 160, 201) < 28);
}

function canonicalSnowLineAt(globalConfig: MinimalGlobalConfigForBlockVerification, x: number, z: number): number {
  return globalConfig.seaLevel + 58 + Math.trunc((valueNoise2(globalConfig.worldSeed, x, z, 220, 202) - 128) * 8 / 128);
}

function canonicalTreeGrowthProfile(globalConfig: MinimalGlobalConfigForBlockVerification, x: number, z: number, surface: number) {
  const top = canonicalSurfaceBlockId(globalConfig, x, z, surface);
  if (top === BLOCK_SAND || top === BLOCK_SALT_FLAT || top === BLOCK_ASH || top === BLOCK_BASALT) {
    return { cellSize: 14, density: 255, pine: false };
  }
  const moisture = canonicalMoistureAt(globalConfig, x, z);
  const altitude = surface - globalConfig.seaLevel;
  const snowLine = canonicalSnowLineAt(globalConfig, x, z);
  const terrain = canonicalTerrainFactors(globalConfig, x, z);
  let cellSize = 9;
  let density = 218;
  if (moisture > 214 && altitude <= 44) {
    cellSize = 6;
    density = 136;
  } else if (moisture > 188 && altitude <= 54) {
    cellSize = 6;
    density = 154;
  } else if (moisture < 116) {
    cellSize = 11;
    density = 226;
  } else if (moisture < 150) {
    cellSize = 9;
    density = 210;
  } else {
    cellSize = 7;
    density = 184;
  }

  if (altitude <= 6) {
    cellSize += 2;
    density += 22;
  } else if (altitude <= 18 && terrain.floodplainMask > 360) {
    cellSize += 1;
    density += 10;
  }
  if (terrain.floodplainMask > 620 && terrain.openRiver > 520) density += 10;
  if (altitude >= 36) {
    cellSize += 1;
    density += 12;
  }
  if (altitude >= 54) {
    cellSize += 1;
    density += 14;
  }
  if (surface >= snowLine - 10) {
    cellSize += 1;
    density += 12;
  }
  if (surface >= snowLine) {
    cellSize += 1;
    density += 12;
  }
  if (top === BLOCK_STONE || top === BLOCK_GRAVEL) {
    cellSize += 1;
    density += 14;
  } else if (top === BLOCK_FROZEN_SOIL || top === BLOCK_SNOW) {
    cellSize += 1;
    density += 8;
  } else if (top === BLOCK_MUD || top === BLOCK_CLAY) {
    density -= 8;
  }

  const patch = valueNoise2(globalConfig.worldSeed, x, z, 260, 406);
  if (patch > 204) {
    cellSize -= 1;
    density -= 24;
  } else if (patch < 58) {
    cellSize += 1;
    density += 14;
  }
  cellSize = Math.max(6, Math.min(14, cellSize));
  density = Math.max(128, Math.min(250, density));
  const pine = surface >= snowLine - 18
    || altitude >= 46
    || (altitude >= 26 && moisture < 168)
    || (hashCoord3(globalConfig.worldSeed, x, surface, z, 404) & 255) > 218;
  return { cellSize, density, pine };
}

function canonicalDesertAt(globalConfig: MinimalGlobalConfigForBlockVerification, x: number, z: number): boolean {
  return canonicalDesertScoreAt(globalConfig, x, z) > 178;
}

function canonicalVolcanicAt(globalConfig: MinimalGlobalConfigForBlockVerification, x: number, z: number): number {
  return valueNoise2(globalConfig.worldSeed, x, z, 192, 205);
}

function canonicalTerrainFactors(globalConfig: MinimalGlobalConfigForBlockVerification, x: number, z: number) {
  const warpX = Math.trunc((valueNoise2(globalConfig.worldSeed, x, z, 160, 31) - 128) * 22 / 128);
  const warpZ = Math.trunc((valueNoise2(globalConfig.worldSeed, x, z, 160, 32) - 128) * 22 / 128);
  const wx = x + warpX;
  const wz = z + warpZ;
  const continent =
    Math.trunc((valueNoise2(globalConfig.worldSeed, wx, wz, 520, 21) - 128) * 86 / 128) +
    Math.trunc((valueNoise2(globalConfig.worldSeed, wx, wz, 220, 22) - 128) * 42 / 128) +
    Math.trunc((valueNoise2(globalConfig.worldSeed, wx, wz, 96, 23) - 128) * 14 / 128) +
    46;
  const shelf = smoothRangeFixed(continent, -50, 34);
  const inland = smoothRangeFixed(continent, -8, 78);
  const riverWarpX = Math.trunc((valueNoise2(globalConfig.worldSeed, wx, wz, 128, 33) - 128) * 36 / 128);
  const riverWarpZ = Math.trunc((valueNoise2(globalConfig.worldSeed, wx, wz, 128, 34) - 128) * 36 / 128);
  const riverLine = 128 - Math.abs(valueNoise2(globalConfig.worldSeed, wx + riverWarpX, wz + riverWarpZ, 104, 35) - 128);
  const lakeNoise = valueNoise2(globalConfig.worldSeed, wx, wz, 220, 37);
  const widthNoise = Math.trunc((valueNoise2(globalConfig.worldSeed, wx, wz, 420, 43) * 2 + valueNoise2(globalConfig.worldSeed, wx, wz, 96, 44)) / 3);
  const canyonNoise = valueNoise2(globalConfig.worldSeed, wx, wz, 340, 47);
  const broadPlain = smoothRangeFixed(valueNoise2(globalConfig.worldSeed, wx, wz, 760, 49), 144, 224);
  const riverWidth = Math.min(255, widthNoise + Math.trunc(broadPlain * 64 / 1024));
  const lakeWidth = valueNoise2(globalConfig.worldSeed, wx, wz, 520, 45);
  const openRiver = scaleByFixed(smoothRangeFixed(riverWidth, 72, 198), 1024 - smoothRangeFixed(canyonNoise, 190, 252));
  const riverValleyStart = 104 - Math.trunc(riverWidth * 88 / 255);
  const riverTerraceStart = Math.max(0, riverValleyStart - 64 - Math.trunc(riverWidth * 42 / 255));
  const riverFloodplainStart = Math.max(0, riverTerraceStart - 44 - Math.trunc(riverWidth * 32 / 255));
  const riverCoreStart = 122 - Math.trunc(riverWidth * 44 / 255);
  const riverTerrace = scaleByFixed(smoothRangeFixed(riverLine, riverTerraceStart, 128), 220 + Math.trunc(riverWidth * 430 / 255));
  const riverFloodplain = scaleByFixed(smoothRangeFixed(riverLine, riverFloodplainStart, 128), openRiver);
  const river = scaleByFixed(smoothRangeFixed(riverLine, riverCoreStart, 128), inland);
  const riverValley = scaleByFixed(Math.max(smoothRangeFixed(riverLine, riverValleyStart, 128), riverTerrace), inland);
  const lakeCoreStart = 226 - Math.trunc(lakeWidth * 28 / 255);
  const lake = scaleByFixed(smoothRangeFixed(lakeNoise, lakeCoreStart, 242), inland);
  const lakeValleyStart = 194 - Math.trunc(lakeWidth * 74 / 255);
  const lakeTerrace = scaleByFixed(smoothRangeFixed(lakeNoise, Math.max(0, lakeValleyStart - 42), 242), 180 + Math.trunc(lakeWidth * 260 / 255));
  const lakeValley = scaleByFixed(Math.max(smoothRangeFixed(lakeNoise, lakeValleyStart, 242), lakeTerrace), inland);
  const floodplainMask = scaleByFixed(Math.max(riverFloodplain, lakeTerrace), inland);
  return { wx, wz, shelf, inland, waterMask: Math.max(river, lake), river, lake, riverValley, lakeValley, valleyMask: Math.max(riverValley, lakeValley), floodplainMask, valleySoftness: Math.max(riverWidth, lakeWidth), openRiver };
}

function canonicalWaterLevel(
  globalConfig: MinimalGlobalConfigForBlockVerification,
  x: number,
  z: number,
  surface: number,
): number | null {
  if (surface < globalConfig.seaLevel) return globalConfig.seaLevel;
  return null;
}

function canonicalMoistureAt(globalConfig: MinimalGlobalConfigForBlockVerification, x: number, z: number): number {
  return Math.trunc((
    valueNoise2(globalConfig.worldSeed, x, z, 176, 211) * 3 +
    valueNoise2(globalConfig.worldSeed, x, z, 72, 212)
  ) / 4);
}

function canonicalDesertScoreAt(globalConfig: MinimalGlobalConfigForBlockVerification, x: number, z: number): number {
  return Math.trunc((
    valueNoise2(globalConfig.worldSeed, x, z, 224, 213) * 3 +
    (255 - canonicalMoistureAt(globalConfig, x, z))
  ) / 4);
}

function valueNoise2(seed: Buffer | Uint8Array, x: number, z: number, scale: number, salt: number): number {
  const cellX = divFloor(x, scale);
  const cellZ = divFloor(z, scale);
  const localX = positiveModulo(x, scale);
  const localZ = positiveModulo(z, scale);
  const tx = smoothFixed(localX, scale);
  const tz = smoothFixed(localZ, scale);
  const a = hashCoord3(seed, cellX, 0, cellZ, salt) & 255;
  const b = hashCoord3(seed, cellX + 1, 0, cellZ, salt) & 255;
  const c = hashCoord3(seed, cellX, 0, cellZ + 1, salt) & 255;
  const d = hashCoord3(seed, cellX + 1, 0, cellZ + 1, salt) & 255;
  return lerpFixed(lerpFixed(a, b, tx), lerpFixed(c, d, tx), tz);
}

function hashCoord3(seed: Buffer | Uint8Array, x: number, y: number, z: number, salt: number): number {
  let hash = (0x811c9dc5 ^ (salt >>> 0)) >>> 0;
  for (const byte of seed) hash = Math.imul((hash ^ byte) >>> 0, 0x01000193) >>> 0;
  hash = hashI32Bytes(hash, x);
  hash = hashI32Bytes(hash, y);
  hash = hashI32Bytes(hash, z);
  hash ^= hash >>> 16;
  hash = Math.imul(hash >>> 0, 0x7feb352d) >>> 0;
  hash ^= hash >>> 15;
  hash = Math.imul(hash >>> 0, 0x846ca68b) >>> 0;
  return (hash ^ (hash >>> 16)) >>> 0;
}

function hashI32Bytes(hash: number, value: number): number {
  const v = value | 0;
  hash = Math.imul((hash ^ (v & 255)) >>> 0, 0x01000193) >>> 0;
  hash = Math.imul((hash ^ ((v >>> 8) & 255)) >>> 0, 0x01000193) >>> 0;
  hash = Math.imul((hash ^ ((v >>> 16) & 255)) >>> 0, 0x01000193) >>> 0;
  return Math.imul((hash ^ ((v >>> 24) & 255)) >>> 0, 0x01000193) >>> 0;
}

function clampInt(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, Math.trunc(value)));
}

function divFloor(value: number, divisor: number): number {
  return Math.floor(value / divisor);
}

function positiveModulo(value: number, divisor: number): number {
  return ((value % divisor) + divisor) % divisor;
}

function smoothFixed(distance: number, scale: number): number {
  const fixed = Math.trunc((distance * 1024) / scale);
  return Math.trunc((fixed * fixed * (3072 - fixed * 2)) / (1024 * 1024));
}

function smoothRangeFixed(value: number, edge0: number, edge1: number): number {
  if (value <= edge0) return 0;
  if (value >= edge1) return 1024;
  return smoothFixed(value - edge0, edge1 - edge0);
}

function lerpFixed(a: number, b: number, t: number): number {
  return Math.trunc((a * (1024 - t) + b * t + 512) / 1024);
}

function lerpIntFixed(a: number, b: number, t: number): number {
  return Math.trunc((a * (1024 - t) + b * t + 512) / 1024);
}

function scaleByFixed(value: number, fixed: number): number {
  return Math.trunc((value * fixed) / 1024);
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

export function decodeFoundationState(dataValue: Buffer | Uint8Array): DecodedFoundationState {
  const data = Buffer.from(dataValue);
  if (data.length !== FOUNDATION_LEN) {
    throw new Error(`Invalid FoundationState length: ${data.length}`);
  }
  const magic = data.subarray(0, 8).toString("utf8");
  const version = data.readUInt8(8);
  if (magic !== FOUNDATION_MAGIC || version !== FOUNDATION_VERSION) {
    throw new Error(`Invalid FoundationState header: ${magic} v${version}`);
  }
  const width = data.readUInt8(94);
  const depth = data.readUInt8(95);
  validateFoundationSize(width, depth);
  return {
    magic,
    version,
    bump: data.readUInt8(9),
    status: data.readUInt8(10),
    chunkCount: data.readUInt8(11),
    owner: new PublicKey(data.subarray(12, 44)),
    globalConfig: new PublicKey(data.subarray(44, 76)),
    foundationId: data.readBigUInt64LE(76),
    minX: data.readInt32LE(84),
    minZ: data.readInt32LE(88),
    surfaceY: data.readInt16LE(92),
    width,
    depth,
    createdSlot: data.readBigUInt64LE(96),
  };
}

export function decodeBuildSite(dataValue: Buffer | Uint8Array): DecodedBuildSite {
  const data = Buffer.from(dataValue);
  if (data.length !== BUILD_SITE_LEN) throw new Error(`Invalid BuildSite length: ${data.length}`);
  const magic = data.subarray(0, 8).toString("utf8");
  const version = data.readUInt8(8);
  if (magic !== BUILD_SITE_MAGIC || version !== BUILD_SITE_VERSION || data.readUInt8(10) !== 1) {
    throw new Error("Invalid BuildSite account");
  }
  const width = data.readUInt32LE(100);
  const depth = data.readUInt32LE(104);
  const minX = data.readInt32LE(88);
  const minZ = data.readInt32LE(92);
  normalizeBuildSiteInput({ minX, minZ, surfaceY: data.readInt16LE(96), width, depth });
  return {
    magic,
    version,
    bump: data.readUInt8(9),
    status: data.readUInt8(10),
    owner: new PublicKey(data.subarray(16, 48)),
    globalConfig: new PublicKey(data.subarray(48, 80)),
    foundationId: data.readBigUInt64LE(80),
    minX,
    minZ,
    surfaceY: data.readInt16LE(96),
    width,
    depth,
    activeRevision: data.readUInt32LE(116),
    pendingRevision: data.readUInt32LE(120),
    createdSlot: data.readBigUInt64LE(108),
    updatedSlot: data.readBigUInt64LE(124),
  };
}

export function decodeFoundationChunkState(
  dataValue: Buffer | Uint8Array,
  expected: { globalConfig?: PublicKey; chunkX?: number; chunkZ?: number } = {},
): DecodedFoundationChunkState {
  const data = Buffer.from(dataValue);
  if (data.length !== FOUNDATION_CHUNK_LEN) {
    throw new Error(`Invalid FoundationChunkState length: ${data.length}`);
  }
  const magic = data.subarray(0, 8).toString("utf8");
  const version = data.readUInt8(8);
  if (magic !== FOUNDATION_CHUNK_MAGIC || version !== FOUNDATION_CHUNK_VERSION) {
    throw new Error(`Invalid FoundationChunkState header: ${magic} v${version}`);
  }
  const count = data.readUInt16LE(10);
  if (count > FOUNDATION_CHUNK_CAPACITY) {
    throw new Error(`Invalid FoundationChunkState count: ${count}`);
  }
  const globalConfig = new PublicKey(data.subarray(12, 44));
  const chunkX = data.readInt32LE(44);
  const chunkZ = data.readInt32LE(48);
  if (expected.globalConfig && !globalConfig.equals(expected.globalConfig)) {
    throw new Error("FoundationChunkState global config does not match the requested PDA");
  }
  if (expected.chunkX !== undefined && chunkX !== requireI32(expected.chunkX, "chunkX")) {
    throw new Error("FoundationChunkState X coordinate does not match the requested PDA");
  }
  if (expected.chunkZ !== undefined && chunkZ !== requireI32(expected.chunkZ, "chunkZ")) {
    throw new Error("FoundationChunkState Z coordinate does not match the requested PDA");
  }
  const records: DecodedFoundationRecord[] = [];
  for (let index = 0; index < count; index += 1) {
    const offset = FOUNDATION_CHUNK_HEADER_LEN + index * FOUNDATION_CHUNK_RECORD_LEN;
    const width = data.readUInt8(offset + 50);
    const depth = data.readUInt8(offset + 51);
    validateFoundationSize(width, depth);
    records.push({
      owner: new PublicKey(data.subarray(offset, offset + 32)),
      foundationId: data.readBigUInt64LE(offset + 32),
      minX: data.readInt32LE(offset + 40),
      minZ: data.readInt32LE(offset + 44),
      surfaceY: data.readInt16LE(offset + 48),
      width,
      depth,
    });
  }
  return {
    magic,
    version,
    bump: data.readUInt8(9),
    count,
    globalConfig,
    chunkX,
    chunkZ,
    records,
  };
}

function normalizeFoundationInput(input: FoundationInput): FoundationInput {
  const minX = requireI32(input?.minX, "minX");
  const minZ = requireI32(input?.minZ, "minZ");
  const surfaceY = requireI16(input?.surfaceY, "surfaceY");
  const width = requirePositiveInt(input?.width, "width");
  const depth = requirePositiveInt(input?.depth, "depth");
  validateFoundationSize(width, depth);
  requireI32(minX + width - 1, "maxX");
  requireI32(minZ + depth - 1, "maxZ");
  if (surfaceY <= CANONICAL_CHUNK_WORLD_CONFIG.minBuildY || surfaceY > CANONICAL_CHUNK_WORLD_CONFIG.maxBuildY) {
    throw new Error(`surfaceY must be in ${CANONICAL_CHUNK_WORLD_CONFIG.minBuildY + 1}..${CANONICAL_CHUNK_WORLD_CONFIG.maxBuildY}`);
  }
  return { minX, minZ, surfaceY, width, depth };
}

function normalizeBuildSiteInput(input: FoundationInput): FoundationInput {
  const minX = requireI32(input?.minX, "minX");
  const minZ = requireI32(input?.minZ, "minZ");
  const surfaceY = requireI16(input?.surfaceY, "surfaceY");
  const width = requireU32(input?.width, "width");
  const depth = requireU32(input?.depth, "depth");
  if (width < FOUNDATION_MIN_SIZE || depth < FOUNDATION_MIN_SIZE) {
    throw new Error(`Foundation dimensions must be at least ${FOUNDATION_MIN_SIZE}`);
  }
  requireI32(minX + width - 1, "maxX");
  requireI32(minZ + depth - 1, "maxZ");
  if (surfaceY <= CANONICAL_CHUNK_WORLD_CONFIG.minBuildY || surfaceY > CANONICAL_CHUNK_WORLD_CONFIG.maxBuildY) {
    throw new Error(`surfaceY must be in ${CANONICAL_CHUNK_WORLD_CONFIG.minBuildY + 1}..${CANONICAL_CHUNK_WORLD_CONFIG.maxBuildY}`);
  }
  return { minX, minZ, surfaceY, width, depth };
}

function foundationChunks(foundation: FoundationInput, chunkSize: number): Array<{ chunkX: number; chunkZ: number }> {
  const minChunkX = Math.floor(foundation.minX / chunkSize);
  const maxChunkX = Math.floor((foundation.minX + foundation.width - 1) / chunkSize);
  const minChunkZ = Math.floor(foundation.minZ / chunkSize);
  const maxChunkZ = Math.floor((foundation.minZ + foundation.depth - 1) / chunkSize);
  const chunks: Array<{ chunkX: number; chunkZ: number }> = [];
  for (let chunkZ = minChunkZ; chunkZ <= maxChunkZ; chunkZ += 1) {
    for (let chunkX = minChunkX; chunkX <= maxChunkX; chunkX += 1) {
      chunks.push({ chunkX, chunkZ });
    }
  }
  if (!chunks.length || chunks.length > FOUNDATION_MAX_CHUNKS) {
    throw new Error(`Foundation must span 1-${FOUNDATION_MAX_CHUNKS} chunks`);
  }
  return chunks;
}

function normalizeFoundationId(value: bigint | number | string): bigint {
  let id: bigint;
  try {
    id = BigInt(value);
  } catch {
    throw new Error("foundationId must be an unsigned 64-bit integer");
  }
  if (id < 0n || id > 0xffff_ffff_ffff_ffffn) {
    throw new Error("foundationId must be an unsigned 64-bit integer");
  }
  return id;
}

function validateFoundationSize(width: number, depth: number): void {
  if (width < FOUNDATION_MIN_SIZE || width > FOUNDATION_MAX_SIZE
    || depth < FOUNDATION_MIN_SIZE || depth > FOUNDATION_MAX_SIZE) {
    throw new Error(`Foundation dimensions must be in ${FOUNDATION_MIN_SIZE}..${FOUNDATION_MAX_SIZE}`);
  }
}

function requireI32(value: number, name: string): number {
  const normalized = Number(value);
  if (!Number.isInteger(normalized) || normalized < -0x8000_0000 || normalized > 0x7fff_ffff) {
    throw new Error(`${name} must be a signed 32-bit integer`);
  }
  return normalized;
}

function requireU32(value: number, name: string): number {
  const normalized = Number(value);
  if (!Number.isSafeInteger(normalized) || normalized < 0 || normalized > 0xffff_ffff) {
    throw new Error(`${name} must be an unsigned 32-bit integer`);
  }
  return normalized;
}

function requireI16(value: number, name: string): number {
  const normalized = Number(value);
  if (!Number.isInteger(normalized) || normalized < -0x8000 || normalized > 0x7fff) {
    throw new Error(`${name} must be a signed 16-bit integer`);
  }
  return normalized;
}

function requirePositiveInt(value: number, name: string): number {
  const normalized = Number(value);
  if (!Number.isSafeInteger(normalized) || normalized <= 0) {
    throw new Error(`${name} must be a positive integer`);
  }
  return normalized;
}
