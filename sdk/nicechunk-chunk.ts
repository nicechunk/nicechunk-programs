import {
  PublicKey,
  SystemProgram,
  TransactionInstruction,
} from "@solana/web3.js";
import {
  deriveGlobalConfigPda,
  NICECHUNK_CORE_PROGRAM_ID,
} from "./nicechunk-core.ts";
import {
  derivePlayerProfilePda,
  derivePlayerSessionPda,
  NICECHUNK_PLAYER_PROGRAM_ID,
} from "./nicechunk-player.ts";
import { NICECHUNK_BACKPACK_PROGRAM_ID } from "./nicechunk-backpack.ts";

const env = typeof process !== "undefined" ? process.env : {};

export const NICECHUNK_CHUNK_PROGRAM_ID = new PublicKey(
  env.NICECHUNK_CHUNK_PROGRAM_ID ?? "7JD6kASAfQeiVLUi51mrfWSbeh96ntRJnRiFQKCqUVhn",
);
export const CHUNK_BROKEN_SEED = "chunk-broken";
export const RESOURCE_DROP_TABLE_SEED = "resource-drops";
export const CHUNK_BROKEN_MAGIC = "NCBK";
export const CHUNK_BROKEN_HEADER_LEN = 16;
export const CHUNK_BROKEN_RECORD_LEN = 3;
export const CHUNK_BROKEN_INITIAL_CAPACITY = 64;
export const CHUNK_BROKEN_MAX_CAPACITY = 2048;
export const RESOURCE_DROP_RULE_LEN = 15;
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
export const BLOCK_MOSS = 37;
export const BLOCK_SHELL_BED = 46;
export const BLOCK_COAL = 47;
const TREE_MAX_LEAF_RADIUS = 2;
const TREE_MAX_BLOCK_HEIGHT_ABOVE_SURFACE = 9;
const MAX_WATER_LEVEL_ABOVE_SEA = 6;

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

export interface ResourceDropRuleInput {
  sourceBlockId: number;
  dropBlockId: number;
  chanceBps: number;
  minAltitude: number;
  maxAltitude: number;
  minDepth: number;
  maxDepth: number;
  salt: number;
}

export interface MinimalGlobalConfigForBlockVerification {
  worldSeed: Buffer | Uint8Array;
  chunkSize: number;
  minBuildY: number;
  maxBuildY: number;
  maxTerrainHeight: number;
  seaLevel: number;
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
      { pubkey: globalConfig, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data,
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
  const [resourceDropTable] = deriveResourceDropTablePda({ globalConfig, programId: chunkProgramId });
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
      { pubkey: chunkBroken, isSigner: false, isWritable: true },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
      { pubkey: resourceDropTable, isSigner: false, isWritable: false },
      { pubkey: NICECHUNK_BACKPACK_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: backpack, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data,
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
  });
  return new TransactionInstruction({
    programId: chunkProgramId,
    keys: [
      { pubkey: payer, isSigner: true, isWritable: true },
      { pubkey: resourceDropTable, isSigner: false, isWritable: true },
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
  return canonicalBlockIdAt(globalConfig, worldX, block.y, worldZ);
}

export function generatedSurfaceHeight(
  globalConfig: MinimalGlobalConfigForBlockVerification,
  worldX: number,
  worldZ: number,
): number {
  return canonicalSurfaceHeight(globalConfig, worldX, worldZ);
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
    return treeBlock !== BLOCK_AIR ? treeBlock : BLOCK_AIR;
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
  const minSurface = Math.max(globalConfig.minBuildY + 8, globalConfig.seaLevel - 28);
  const maxSurface = Math.max(minSurface, Math.min(globalConfig.maxTerrainHeight, globalConfig.maxBuildY - 1));
  const terrain = canonicalTerrainFactors(globalConfig, x, z);
  const { wx, wz, shelf, inland, waterMask } = terrain;

  const ocean =
    globalConfig.seaLevel - 16 +
    Math.trunc((valueNoise2(globalConfig.worldSeed, wx, wz, 96, 24) - 128) * 5 / 128) +
    Math.trunc((valueNoise2(globalConfig.worldSeed, wx, wz, 36, 25) - 128) * 2 / 128);
  const coast = globalConfig.seaLevel - 3 + Math.trunc(shelf * 8 / 1024);
  const plains = Math.trunc((valueNoise2(globalConfig.worldSeed, wx, wz, 120, 26) - 128) * 4 / 128);
  const hills = Math.trunc((valueNoise2(globalConfig.worldSeed, wx, wz, 56, 27) - 128) * 7 / 128);
  const rolling = Math.trunc((valueNoise2(globalConfig.worldSeed, wx, wz, 28, 28) - 128) * 2 / 128);
  const roughness = smoothRangeFixed(Math.abs(valueNoise2(globalConfig.worldSeed, wx, wz, 180, 40) - 128), 54, 122);

  const mountainRidge = Math.abs(valueNoise2(globalConfig.worldSeed, wx, wz, 96, 29) - 128);
  const ridgeLift = smoothRangeFixed(mountainRidge, 70, 124);
  const mountainMass = scaleByFixed(smoothRangeFixed(valueNoise2(globalConfig.worldSeed, wx, wz, 300, 30), 194, 244), inland);
  const mountain = scaleByFixed(6 + scaleByFixed(20, ridgeLift), mountainMass);

  let land = globalConfig.seaLevel + 7 + Math.trunc(inland * 8 / 1024) + scaleByFixed(plains + scaleByFixed(hills + rolling, roughness), inland) + mountain;
  if (waterMask > 0) {
    const waterLevel = canonicalInlandWaterLevel(globalConfig, wx, wz);
    const waterBed = waterLevel - 3 - Math.trunc(waterMask * 2 / 1024) + Math.trunc((valueNoise2(globalConfig.worldSeed, wx, wz, 32, 39) - 128) / 128);
    land = lerpIntFixed(land, waterBed, waterMask);
  }

  return clampInt(lerpIntFixed(ocean, Math.max(coast, land), shelf), minSurface, maxSurface);
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
    if (moisture > 224 && valueNoise2(globalConfig.worldSeed, x, z, 72, 109) > 168) return BLOCK_MOSS;
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
  if (y <= globalConfig.minBuildY + 3 || y >= surface - 7) return false;
  const seam = hashCoord3(globalConfig.worldSeed, divFloor(x, 8), divFloor(y, 4), divFloor(z, 8), 301) % 100;
  if (seam < 84) return false;
  return hashCoord3(globalConfig.worldSeed, x + y * 3, y, z - y * 5, 302) % 100 >= 38;
}

function canonicalTreeBlockIdAt(
  globalConfig: MinimalGlobalConfigForBlockVerification,
  x: number,
  y: number,
  z: number,
): number {
  let best: { z: number; x: number; block: number } | null = null;
  for (const cellSize of [7, 9]) {
    const minCellX = treeCandidateMinCell(x, TREE_MAX_LEAF_RADIUS, cellSize);
    const maxCellX = treeCandidateMaxCell(x, TREE_MAX_LEAF_RADIUS, cellSize);
    const minCellZ = treeCandidateMinCell(z, TREE_MAX_LEAF_RADIUS, cellSize);
    const maxCellZ = treeCandidateMaxCell(z, TREE_MAX_LEAF_RADIUS, cellSize);
    const inner = Math.max(1, cellSize - 2);

    for (let cellZ = minCellZ; cellZ <= maxCellZ; cellZ += 1) {
      for (let cellX = minCellX; cellX <= maxCellX; cellX += 1) {
        const treeX = cellX * cellSize + 1 + (hashCoord3(globalConfig.worldSeed, cellX, 0, cellZ, 401) % inner);
        const treeZ = cellZ * cellSize + 1 + (hashCoord3(globalConfig.worldSeed, cellX, 0, cellZ, 402) % inner);
        if (Math.abs(treeX - x) > TREE_MAX_LEAF_RADIUS || Math.abs(treeZ - z) > TREE_MAX_LEAF_RADIUS) continue;

        const wet = canonicalWetAt(globalConfig, treeX, treeZ);
        if ((wet ? 7 : 9) !== cellSize) continue;
        const density = wet ? 180 : 218;
        if ((hashCoord3(globalConfig.worldSeed, cellX, 0, cellZ, 403) & 255) <= density) continue;

        const surface = canonicalSurfaceHeight(globalConfig, treeX, treeZ);
        if (!treeVerticalBoundsCanContain(surface, y)) continue;
        if (!canonicalCanGrowTree(globalConfig, treeX, treeZ, surface)) continue;
        const tree = canonicalTreeFromCandidate(globalConfig, treeX, treeZ, surface);
        const block = canonicalTreeVolumeBlock(globalConfig, tree, x, y, z);
        if (block !== BLOCK_AIR && (!best || tree.z < best.z || (tree.z === best.z && tree.x < best.x))) {
          best = { z: tree.z, x: tree.x, block };
        }
      }
    }
  }
  return best?.block ?? BLOCK_AIR;
}

function treeVerticalBoundsCanContain(surface: number, y: number): boolean {
  return y >= surface + 1 && y <= surface + TREE_MAX_BLOCK_HEIGHT_ABOVE_SURFACE;
}

function treeCandidateMinCell(worldCoord: number, radius: number, cellSize: number): number {
  return divFloor(worldCoord - radius - (cellSize - 2), cellSize);
}

function treeCandidateMaxCell(worldCoord: number, radius: number, cellSize: number): number {
  return divFloor(worldCoord + radius - 1, cellSize);
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
  const wet = canonicalWetAt(globalConfig, x, z);
  const density = wet ? 180 : 218;
  const cellSize = wet ? 7 : 9;
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
  const pine = canonicalColdAt(globalConfig, x, z, surface)
    || surface >= globalConfig.seaLevel + 32
    || (hashCoord3(globalConfig.worldSeed, x, surface, z, 404) & 255) > 206;
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
  return surface >= globalConfig.seaLevel + 30
    || (surface >= globalConfig.seaLevel + 18 && valueNoise2(globalConfig.worldSeed, x, z, 160, 201) < 42);
}

function canonicalDesertAt(globalConfig: MinimalGlobalConfigForBlockVerification, x: number, z: number): boolean {
  return canonicalDesertScoreAt(globalConfig, x, z) > 178;
}

function canonicalWetAt(globalConfig: MinimalGlobalConfigForBlockVerification, x: number, z: number): boolean {
  return canonicalMoistureAt(globalConfig, x, z) > 188;
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
  const river = scaleByFixed(smoothRangeFixed(riverLine, 118, 128), inland);
  const lake = scaleByFixed(smoothRangeFixed(valueNoise2(globalConfig.worldSeed, wx, wz, 220, 37), 210, 242), inland);
  return { wx, wz, shelf, inland, waterMask: Math.max(river, lake) };
}

function canonicalWaterLevel(
  globalConfig: MinimalGlobalConfigForBlockVerification,
  x: number,
  z: number,
  surface: number,
): number | null {
  if (surface < globalConfig.seaLevel) return globalConfig.seaLevel;
  const { waterMask, wx, wz } = canonicalTerrainFactors(globalConfig, x, z);
  if (waterMask <= 96) return null;
  return canonicalInlandWaterLevel(globalConfig, wx, wz);
}

function canonicalInlandWaterLevel(
  globalConfig: MinimalGlobalConfigForBlockVerification,
  wx: number,
  wz: number,
): number {
  return globalConfig.seaLevel + 6 + Math.trunc((valueNoise2(globalConfig.worldSeed, wx, wz, 180, 41) - 128) / 128);
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
