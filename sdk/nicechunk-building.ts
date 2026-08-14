import {
  PublicKey,
  SystemProgram,
  TransactionInstruction,
  type PublicKeyInitData,
} from "@solana/web3.js";
import { Buffer } from "node:buffer";
import {
  deriveFoundationChunkPda,
  NICECHUNK_CHUNK_PROGRAM_ID,
} from "./nicechunk-chunk.ts";
import {
  deriveGlobalConfigPda,
  NICECHUNK_CORE_PROGRAM_ID,
} from "./nicechunk-core.ts";
import {
  derivePlayerProfilePda,
  derivePlayerSessionPda,
  NICECHUNK_PLAYER_PROGRAM_ID,
} from "./nicechunk-player.ts";

export const NICECHUNK_BUILDING_PROGRAM_ID = new PublicKey(
  "39UMTUWXQkuomkFNbDPF5NGZnJmG6pDkJHVSkZyqVwWx",
);
export const NICECHUNK_MARKET_PROGRAM_ID = new PublicKey(
  "6CurnvneezBuHwPUnrCiFg1QMWeUF67ufQxYebyr2UP7",
);
export const MARKET_USER_SEED = "market-user-v1";
export const LAND_CONTRACT_AUTHORITY_SEED = "land-contract-authority-v1";
export const LAND_CONTRACT_TYPE_BLANK = 1;
export const MAX_LAND_CONTRACTS_PER_SITE = 4_096;
export const BUILD_SITE_SEED = "build-site-v3";
export const BUILD_SITE_MAGIC = "NCKSITE3";
export const BUILD_SITE_VERSION = 3;
export const BUILD_SITE_LEN = 160;
export const BUILD_SITE_STATUS_INDEXING = 0;
export const BUILD_SITE_STATUS_ACTIVE = 1;
export const BUILD_SITE_STATUS_CANCELING = 2;
export const BUILDING_MANIFEST_SEED = "building-v3";
export const BUILDING_MANIFEST_MAGIC = "NCKBLD03";
export const BUILDING_MANIFEST_VERSION = 3;
export const BUILDING_MANIFEST_LEN = 160;
export const BUILDING_STATUS_UPLOADING = 0;
export const BUILDING_STATUS_ACTIVE = 1;
export const BUILDING_SHARD_SEED = "building-data-v2";
export const BUILDING_SHARD_MAGIC = "NCKBDT02";
export const BUILDING_SHARD_VERSION = 2;
export const BUILDING_SHARD_HEADER_LEN = 64;
export const BUILDING_SHARD_PAYLOAD_LEN = 8_192;
export const BUILDING_MAX_PAYLOAD_LEN = 65_535;
export const BUILDING_MAX_WRITE_LEN = 700;
export const BUILDING_MAX_SHARDS = 8;
export const BUILDING_CHUNK_AUTHORITY_SEED = "chunk-authority-v2";
export const FOUNDATION_CHUNK_SIZE = 16;

const VALID_BUILD_SITE_STATUSES = new Set([
  BUILD_SITE_STATUS_INDEXING,
  BUILD_SITE_STATUS_ACTIVE,
  BUILD_SITE_STATUS_CANCELING,
]);

export interface DeriveBuildSitePdaInput {
  globalConfig: PublicKeyInitData;
  foundationId: bigint | number | string;
  programId?: PublicKeyInitData;
}

export interface DeriveBuildingManifestPdaInput extends DeriveBuildSitePdaInput {
  revision: number;
}

export interface FoundationInput {
  minX: number;
  minZ: number;
  surfaceY: number;
  width: number;
  depth: number;
}

export interface DecodedBuildSite {
  version: number;
  status: number;
  bump: number;
  contractType: number;
  landContractCount: number;
  owner: PublicKey;
  globalConfig: PublicKey;
  foundationId: bigint;
  minX: number;
  minZ: number;
  surfaceY: number;
  width: number;
  depth: number;
  activeRevision: number;
  pendingRevision: number;
  createdSlot: bigint;
  updatedSlot: bigint;
  registeredChunks: bigint;
  totalChunks: bigint;
}

export interface DecodedBuildingManifest {
  version: number;
  status: number;
  bump: number;
  quarterTurns: number;
  shardCount: number;
  uploadedBitmap: number;
  owner: PublicKey;
  globalConfig: PublicKey;
  foundationId: bigint;
  revision: number;
  payloadLen: number;
  expectedHash: Uint8Array;
  sizeX: number;
  sizeY: number;
  sizeZ: number;
  createdSlot: bigint;
  updatedSlot: bigint;
  offsetX: number;
  offsetZ: number;
}

export interface DecodedBuildingShard {
  version: number;
  bump: number;
  shardIndex: number;
  payloadLen: number;
  uploadedLen: number;
  globalConfig: PublicKey;
  foundationId: bigint;
  revision: number;
  payload: Uint8Array;
}

export function deriveBuildSitePda({
  globalConfig,
  foundationId,
  programId = NICECHUNK_BUILDING_PROGRAM_ID,
}: DeriveBuildSitePdaInput): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [
      Buffer.from(BUILD_SITE_SEED),
      new PublicKey(globalConfig).toBuffer(),
      u64Buffer(foundationId, "foundationId"),
    ],
    new PublicKey(programId),
  );
}

export function deriveBuildingManifestPda({
  globalConfig,
  foundationId,
  revision,
  programId = NICECHUNK_BUILDING_PROGRAM_ID,
}: DeriveBuildingManifestPdaInput): [PublicKey, number] {
  if (!Number.isInteger(revision) || revision <= 0 || revision > 0xffff_ffff) {
    throw new Error("revision must be a positive u32");
  }
  const revisionBytes = Buffer.alloc(4);
  revisionBytes.writeUInt32LE(revision);
  return PublicKey.findProgramAddressSync(
    [
      Buffer.from(BUILDING_MANIFEST_SEED),
      new PublicKey(globalConfig).toBuffer(),
      u64Buffer(foundationId, "foundationId"),
      revisionBytes,
    ],
    new PublicKey(programId),
  );
}

export function deriveBuildingShardPda({
  globalConfig,
  foundationId,
  revision,
  shardIndex,
  programId = NICECHUNK_BUILDING_PROGRAM_ID,
}: DeriveBuildingManifestPdaInput & {
  shardIndex: number;
}): [PublicKey, number] {
  const normalizedRevision = positiveU32(revision, "revision");
  const normalizedShardIndex = boundedInteger(shardIndex, 0, BUILDING_MAX_SHARDS - 1, "shardIndex");
  const revisionBytes = Buffer.alloc(4);
  revisionBytes.writeUInt32LE(normalizedRevision);
  return PublicKey.findProgramAddressSync(
    [
      Buffer.from(BUILDING_SHARD_SEED),
      new PublicKey(globalConfig).toBuffer(),
      u64Buffer(foundationId, "foundationId"),
      revisionBytes,
      Buffer.from([normalizedShardIndex]),
    ],
    new PublicKey(programId),
  );
}

export function deriveBuildingChunkAuthorityPda({
  globalConfig,
  programId = NICECHUNK_BUILDING_PROGRAM_ID,
}: {
  globalConfig: PublicKeyInitData;
  programId?: PublicKeyInitData;
}): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(BUILDING_CHUNK_AUTHORITY_SEED), new PublicKey(globalConfig).toBuffer()],
    new PublicKey(programId),
  );
}

export function deriveMarketUserPda({
  owner,
  programId = NICECHUNK_MARKET_PROGRAM_ID,
}: {
  owner: PublicKeyInitData;
  programId?: PublicKeyInitData;
}): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(MARKET_USER_SEED), new PublicKey(owner).toBuffer()],
    new PublicKey(programId),
  );
}

export function deriveLandContractAuthorityPda({
  globalConfig,
  programId = NICECHUNK_BUILDING_PROGRAM_ID,
}: {
  globalConfig: PublicKeyInitData;
  programId?: PublicKeyInitData;
}): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(LAND_CONTRACT_AUTHORITY_SEED), new PublicKey(globalConfig).toBuffer()],
    new PublicKey(programId),
  );
}

export function createBuildSiteInstruction({
  authority,
  owner,
  foundationId,
  foundation,
  buildingProgramId = NICECHUNK_BUILDING_PROGRAM_ID,
  playerProgramId = NICECHUNK_PLAYER_PROGRAM_ID,
  marketProgramId = NICECHUNK_MARKET_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
}: {
  authority: PublicKey;
  owner: PublicKey;
  foundationId: bigint | number | string;
  foundation: FoundationInput;
  buildingProgramId?: PublicKey;
  playerProgramId?: PublicKey;
  marketProgramId?: PublicKey;
  coreProgramId?: PublicKey;
}): TransactionInstruction {
  const id = positiveU64(foundationId, "foundationId");
  const normalized = normalizeFoundation(foundation);
  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  const [playerProfile] = derivePlayerProfilePda(owner, playerProgramId);
  const [playerSession] = derivePlayerSessionPda({
    owner,
    sessionAuthority: authority,
    programId: playerProgramId,
  });
  const [buildSite] = deriveBuildSitePda({ globalConfig, foundationId: id, programId: buildingProgramId });
  const [marketUser] = deriveMarketUserPda({ owner, programId: marketProgramId });
  const [contractAuthority] = deriveLandContractAuthorityPda({
    globalConfig,
    programId: buildingProgramId,
  });
  const data = Buffer.alloc(27);
  data.writeUInt8(0, 0);
  data.writeBigUInt64LE(id, 1);
  data.writeInt32LE(normalized.minX, 9);
  data.writeInt16LE(normalized.surfaceY, 13);
  data.writeInt32LE(normalized.minZ, 15);
  data.writeUInt32LE(normalized.width, 19);
  data.writeUInt32LE(normalized.depth, 23);
  return new TransactionInstruction({
    programId: buildingProgramId,
    keys: [
      { pubkey: authority, isSigner: true, isWritable: true },
      { pubkey: playerProfile, isSigner: false, isWritable: false },
      { pubkey: playerSession, isSigner: false, isWritable: false },
      { pubkey: buildSite, isSigner: false, isWritable: true },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      { pubkey: owner, isSigner: false, isWritable: false },
      { pubkey: marketUser, isSigner: false, isWritable: true },
      { pubkey: contractAuthority, isSigner: false, isWritable: false },
      { pubkey: marketProgramId, isSigner: false, isWritable: false },
    ],
    data,
  });
}

export function createRegisterBuildSiteChunksInstruction({
  authority,
  owner,
  foundation,
  limit = 4,
  buildingProgramId = NICECHUNK_BUILDING_PROGRAM_ID,
  chunkProgramId = NICECHUNK_CHUNK_PROGRAM_ID,
  playerProgramId = NICECHUNK_PLAYER_PROGRAM_ID,
  marketProgramId = NICECHUNK_MARKET_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
}: {
  authority: PublicKey;
  owner: PublicKey;
  foundation: DecodedBuildSite;
  limit?: number;
  buildingProgramId?: PublicKey;
  chunkProgramId?: PublicKey;
  playerProgramId?: PublicKey;
  marketProgramId?: PublicKey;
  coreProgramId?: PublicKey;
}): TransactionInstruction {
  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  const batch = foundationIndexBatch(foundation, limit);
  if (!batch.length) throw new Error("BuildSite has no remaining Chunk index work");
  const [playerProfile] = derivePlayerProfilePda(owner, playerProgramId);
  const [playerSession] = derivePlayerSessionPda({ owner, sessionAuthority: authority, programId: playerProgramId });
  const [buildSite] = deriveBuildSitePda({
    globalConfig,
    foundationId: foundation.foundationId,
    programId: buildingProgramId,
  });
  const [chunkAuthority] = deriveBuildingChunkAuthorityPda({ globalConfig, programId: buildingProgramId });
  const data = Buffer.alloc(9);
  data.writeUInt8(1, 0);
  data.writeBigUInt64LE(foundation.foundationId, 1);
  return new TransactionInstruction({
    programId: buildingProgramId,
    keys: [
      { pubkey: authority, isSigner: true, isWritable: true },
      { pubkey: playerProfile, isSigner: false, isWritable: false },
      { pubkey: playerSession, isSigner: false, isWritable: false },
      { pubkey: buildSite, isSigner: false, isWritable: true },
      { pubkey: chunkAuthority, isSigner: false, isWritable: false },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
      { pubkey: chunkProgramId, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      { pubkey: owner, isSigner: false, isWritable: false },
      { pubkey: deriveMarketUserPda({ owner, programId: marketProgramId })[0], isSigner: false, isWritable: true },
      { pubkey: deriveLandContractAuthorityPda({ globalConfig, programId: buildingProgramId })[0], isSigner: false, isWritable: false },
      { pubkey: marketProgramId, isSigner: false, isWritable: false },
      ...batch.map(({ chunkX, chunkZ }) => ({
        pubkey: deriveFoundationChunkPda({ globalConfig, chunkX, chunkZ, programId: chunkProgramId })[0],
        isSigner: false,
        isWritable: true,
      })),
    ],
    data,
  });
}

export function createCancelBuildSiteIndexingInstruction({
  authority,
  owner,
  foundation,
  limit = 4,
  buildingProgramId = NICECHUNK_BUILDING_PROGRAM_ID,
  chunkProgramId = NICECHUNK_CHUNK_PROGRAM_ID,
  playerProgramId = NICECHUNK_PLAYER_PROGRAM_ID,
  marketProgramId = NICECHUNK_MARKET_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
}: {
  authority: PublicKey;
  owner: PublicKey;
  foundation: DecodedBuildSite;
  limit?: number;
  buildingProgramId?: PublicKey;
  chunkProgramId?: PublicKey;
  playerProgramId?: PublicKey;
  marketProgramId?: PublicKey;
  coreProgramId?: PublicKey;
}): TransactionInstruction {
  if (foundation.status === BUILD_SITE_STATUS_ACTIVE) {
    throw new Error("Active land cannot be canceled");
  }
  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  const batch = foundationRollbackBatch(foundation, limit);
  const [playerProfile] = derivePlayerProfilePda(owner, playerProgramId);
  const [playerSession] = derivePlayerSessionPda({ owner, sessionAuthority: authority, programId: playerProgramId });
  const [buildSite] = deriveBuildSitePda({
    globalConfig,
    foundationId: foundation.foundationId,
    programId: buildingProgramId,
  });
  const [chunkAuthority] = deriveBuildingChunkAuthorityPda({ globalConfig, programId: buildingProgramId });
  const data = Buffer.alloc(9);
  data.writeUInt8(6, 0);
  data.writeBigUInt64LE(foundation.foundationId, 1);
  return new TransactionInstruction({
    programId: buildingProgramId,
    keys: [
      { pubkey: authority, isSigner: true, isWritable: true },
      { pubkey: playerProfile, isSigner: false, isWritable: false },
      { pubkey: playerSession, isSigner: false, isWritable: false },
      { pubkey: buildSite, isSigner: false, isWritable: true },
      { pubkey: chunkAuthority, isSigner: false, isWritable: false },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
      { pubkey: chunkProgramId, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      { pubkey: owner, isSigner: false, isWritable: false },
      { pubkey: deriveMarketUserPda({ owner, programId: marketProgramId })[0], isSigner: false, isWritable: true },
      { pubkey: deriveLandContractAuthorityPda({ globalConfig, programId: buildingProgramId })[0], isSigner: false, isWritable: false },
      { pubkey: marketProgramId, isSigner: false, isWritable: false },
      ...batch.map(({ chunkX, chunkZ }) => ({
        pubkey: deriveFoundationChunkPda({ globalConfig, chunkX, chunkZ, programId: chunkProgramId })[0],
        isSigner: false,
        isWritable: true,
      })),
    ],
    data,
  });
}

export function createBeginBuildingInstruction({
  authority,
  owner,
  foundationId,
  revision,
  quarterTurns,
  payloadLen,
  expectedHash,
  offsetX,
  offsetZ,
  buildingProgramId = NICECHUNK_BUILDING_PROGRAM_ID,
  playerProgramId = NICECHUNK_PLAYER_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
}: {
  authority: PublicKey;
  owner: PublicKey;
  foundationId: bigint | number | string;
  revision: number;
  quarterTurns: number;
  payloadLen: number;
  expectedHash: Buffer | Uint8Array;
  offsetX: number;
  offsetZ: number;
  buildingProgramId?: PublicKey;
  playerProgramId?: PublicKey;
  coreProgramId?: PublicKey;
}): TransactionInstruction {
  const id = positiveU64(foundationId, "foundationId");
  const normalizedRevision = positiveU32(revision, "revision");
  const turns = boundedInteger(quarterTurns, 0, 3, "quarterTurns");
  const length = boundedInteger(payloadLen, 1, BUILDING_MAX_PAYLOAD_LEN, "payloadLen");
  const hash = Buffer.from(expectedHash);
  if (hash.length !== 32) throw new Error("expectedHash must contain 32 bytes");
  assertI32(offsetX, "offsetX");
  assertI32(offsetZ, "offsetZ");
  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  const [playerProfile] = derivePlayerProfilePda(owner, playerProgramId);
  const [playerSession] = derivePlayerSessionPda({ owner, sessionAuthority: authority, programId: playerProgramId });
  const [buildSite] = deriveBuildSitePda({ globalConfig, foundationId: id, programId: buildingProgramId });
  const [manifest] = deriveBuildingManifestPda({
    globalConfig,
    foundationId: id,
    revision: normalizedRevision,
    programId: buildingProgramId,
  });
  const data = Buffer.alloc(58);
  data.writeUInt8(2, 0);
  data.writeBigUInt64LE(id, 1);
  data.writeUInt32LE(normalizedRevision, 9);
  data.writeUInt8(turns, 13);
  data.writeUInt32LE(length, 14);
  hash.copy(data, 18);
  data.writeInt32LE(offsetX, 50);
  data.writeInt32LE(offsetZ, 54);
  return new TransactionInstruction({
    programId: buildingProgramId,
    keys: [
      { pubkey: authority, isSigner: true, isWritable: true },
      { pubkey: playerProfile, isSigner: false, isWritable: false },
      { pubkey: playerSession, isSigner: false, isWritable: false },
      { pubkey: buildSite, isSigner: false, isWritable: true },
      { pubkey: manifest, isSigner: false, isWritable: true },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data,
  });
}

export function createWriteBuildingShardInstruction({
  authority,
  owner,
  foundationId,
  revision,
  shardIndex,
  offset,
  bytes,
  buildingProgramId = NICECHUNK_BUILDING_PROGRAM_ID,
  playerProgramId = NICECHUNK_PLAYER_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
}: {
  authority: PublicKey;
  owner: PublicKey;
  foundationId: bigint | number | string;
  revision: number;
  shardIndex: number;
  offset: number;
  bytes: Buffer | Uint8Array;
  buildingProgramId?: PublicKey;
  playerProgramId?: PublicKey;
  coreProgramId?: PublicKey;
}): TransactionInstruction {
  const id = positiveU64(foundationId, "foundationId");
  const normalizedRevision = positiveU32(revision, "revision");
  const normalizedShardIndex = boundedInteger(shardIndex, 0, BUILDING_MAX_SHARDS - 1, "shardIndex");
  const normalizedOffset = boundedInteger(offset, 0, 0xffff, "offset");
  const payload = Buffer.from(bytes);
  if (!payload.length || payload.length > BUILDING_MAX_WRITE_LEN) throw new Error("Invalid building write length");
  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  const [playerProfile] = derivePlayerProfilePda(owner, playerProgramId);
  const [playerSession] = derivePlayerSessionPda({ owner, sessionAuthority: authority, programId: playerProgramId });
  const [buildSite] = deriveBuildSitePda({ globalConfig, foundationId: id, programId: buildingProgramId });
  const [manifest] = deriveBuildingManifestPda({ globalConfig, foundationId: id, revision: normalizedRevision, programId: buildingProgramId });
  const [shard] = deriveBuildingShardPda({
    globalConfig,
    foundationId: id,
    revision: normalizedRevision,
    shardIndex: normalizedShardIndex,
    programId: buildingProgramId,
  });
  const data = Buffer.alloc(16 + payload.length);
  data.writeUInt8(3, 0);
  data.writeBigUInt64LE(id, 1);
  data.writeUInt32LE(normalizedRevision, 9);
  data.writeUInt8(normalizedShardIndex, 13);
  data.writeUInt16LE(normalizedOffset, 14);
  payload.copy(data, 16);
  return new TransactionInstruction({
    programId: buildingProgramId,
    keys: [
      { pubkey: authority, isSigner: true, isWritable: true },
      { pubkey: playerProfile, isSigner: false, isWritable: false },
      { pubkey: playerSession, isSigner: false, isWritable: false },
      { pubkey: buildSite, isSigner: false, isWritable: false },
      { pubkey: manifest, isSigner: false, isWritable: true },
      { pubkey: shard, isSigner: false, isWritable: true },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data,
  });
}

export function createFinalizeBuildingInstruction(input: {
  authority: PublicKey;
  owner: PublicKey;
  foundationId: bigint | number | string;
  revision: number;
  shardCount: number;
  buildingProgramId?: PublicKey;
  playerProgramId?: PublicKey;
  coreProgramId?: PublicKey;
}): TransactionInstruction {
  return createCompleteBuildingInstruction(input, 4, false);
}

export function createCancelBuildingUploadInstruction(input: {
  authority: PublicKey;
  owner: PublicKey;
  foundationId: bigint | number | string;
  revision: number;
  shardCount: number;
  buildingProgramId?: PublicKey;
  playerProgramId?: PublicKey;
  coreProgramId?: PublicKey;
}): TransactionInstruction {
  return createCompleteBuildingInstruction(input, 5, true);
}

function createCompleteBuildingInstruction({
  authority,
  owner,
  foundationId,
  revision,
  shardCount,
  buildingProgramId = NICECHUNK_BUILDING_PROGRAM_ID,
  playerProgramId = NICECHUNK_PLAYER_PROGRAM_ID,
  coreProgramId = NICECHUNK_CORE_PROGRAM_ID,
}: {
  authority: PublicKey;
  owner: PublicKey;
  foundationId: bigint | number | string;
  revision: number;
  shardCount: number;
  buildingProgramId?: PublicKey;
  playerProgramId?: PublicKey;
  coreProgramId?: PublicKey;
}, tag: 4 | 5, writableShards: boolean): TransactionInstruction {
  const id = positiveU64(foundationId, "foundationId");
  const normalizedRevision = positiveU32(revision, "revision");
  const normalizedShardCount = boundedInteger(shardCount, 1, BUILDING_MAX_SHARDS, "shardCount");
  const [globalConfig] = deriveGlobalConfigPda(coreProgramId);
  const [playerProfile] = derivePlayerProfilePda(owner, playerProgramId);
  const [playerSession] = derivePlayerSessionPda({ owner, sessionAuthority: authority, programId: playerProgramId });
  const [buildSite] = deriveBuildSitePda({ globalConfig, foundationId: id, programId: buildingProgramId });
  const [manifest] = deriveBuildingManifestPda({ globalConfig, foundationId: id, revision: normalizedRevision, programId: buildingProgramId });
  const data = Buffer.alloc(13);
  data.writeUInt8(tag, 0);
  data.writeBigUInt64LE(id, 1);
  data.writeUInt32LE(normalizedRevision, 9);
  return new TransactionInstruction({
    programId: buildingProgramId,
    keys: [
      { pubkey: authority, isSigner: true, isWritable: true },
      { pubkey: playerProfile, isSigner: false, isWritable: false },
      { pubkey: playerSession, isSigner: false, isWritable: false },
      { pubkey: buildSite, isSigner: false, isWritable: true },
      { pubkey: manifest, isSigner: false, isWritable: true },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ...Array.from({ length: normalizedShardCount }, (_unused, shardIndex) => ({
        pubkey: deriveBuildingShardPda({
          globalConfig,
          foundationId: id,
          revision: normalizedRevision,
          shardIndex,
          programId: buildingProgramId,
        })[0],
        isSigner: false,
        isWritable: writableShards,
      })),
    ],
    data,
  });
}

export function decodeBuildSite(dataValue: Buffer | Uint8Array): DecodedBuildSite {
  const data = Buffer.from(dataValue);
  if (data.length !== BUILD_SITE_LEN
    || data.subarray(0, 8).toString("utf8") !== BUILD_SITE_MAGIC
    || data.readUInt8(8) !== BUILD_SITE_VERSION
    || !VALID_BUILD_SITE_STATUSES.has(data.readUInt8(10))) {
    throw new Error("Invalid Building Program BuildSite account");
  }
  const status = data.readUInt8(10);
  const contractType = data.readUInt8(11);
  const landContractCount = data.readUInt32LE(12);
  const foundationId = data.readBigUInt64LE(80);
  const minX = data.readInt32LE(88);
  const minZ = data.readInt32LE(92);
  const surfaceY = data.readInt16LE(96);
  const width = data.readUInt32LE(100);
  const depth = data.readUInt32LE(104);
  const activeRevision = data.readUInt32LE(116);
  const pendingRevision = data.readUInt32LE(120);
  const registeredChunks = data.readBigUInt64LE(132);
  const totalChunks = data.readBigUInt64LE(140);
  const globalConfig = new PublicKey(data.subarray(48, 80));
  const maxX = minX + width - 1;
  const maxZ = minZ + depth - 1;
  const active = { minX, minZ, surfaceY, width, depth };
  const activeChunks = foundationChunkCount(active);
  let invalidStatus = false;
  if (status === BUILD_SITE_STATUS_INDEXING) {
    invalidStatus = activeRevision !== 0
      || pendingRevision !== 0
      || totalChunks !== activeChunks
      || registeredChunks === totalChunks;
  } else if (status === BUILD_SITE_STATUS_ACTIVE) {
    invalidStatus = totalChunks !== activeChunks
      || registeredChunks !== totalChunks;
  } else if (status === BUILD_SITE_STATUS_CANCELING) {
    invalidStatus = activeRevision !== 0
      || pendingRevision !== 0
      || totalChunks !== activeChunks;
  }
  if (foundationId === 0n
    || contractType !== LAND_CONTRACT_TYPE_BLANK
    || BigInt(landContractCount) !== activeChunks
    || activeChunks > BigInt(MAX_LAND_CONTRACTS_PER_SITE)
    || data.subarray(98, 100).some((byte) => byte !== 0)
    || data.subarray(148, 160).some((byte) => byte !== 0)
    || maxX < -0x8000_0000
    || maxX > 0x7fff_ffff
    || maxZ < -0x8000_0000
    || maxZ > 0x7fff_ffff
    || !globalConfig.equals(deriveGlobalConfigPda()[0])
    || registeredChunks > totalChunks
    || (pendingRevision !== 0 && pendingRevision !== activeRevision + 1)
    || invalidStatus) {
    throw new Error("Invalid Building Program BuildSite state");
  }
  return {
    version: BUILD_SITE_VERSION,
    status,
    bump: data.readUInt8(9),
    contractType,
    landContractCount,
    owner: new PublicKey(data.subarray(16, 48)),
    globalConfig,
    foundationId,
    minX,
    minZ,
    surfaceY,
    width,
    depth,
    activeRevision,
    pendingRevision,
    createdSlot: data.readBigUInt64LE(108),
    updatedSlot: data.readBigUInt64LE(124),
    registeredChunks,
    totalChunks,
  };
}

export function decodeBuildingManifest(dataValue: Buffer | Uint8Array): DecodedBuildingManifest {
  const data = Buffer.from(dataValue);
  if (data.length !== BUILDING_MANIFEST_LEN
    || data.subarray(0, 8).toString("utf8") !== BUILDING_MANIFEST_MAGIC
    || data.readUInt8(8) !== BUILDING_MANIFEST_VERSION) {
    throw new Error("Invalid Building Program BuildingManifest account");
  }
  const status = data.readUInt8(10);
  const quarterTurns = data.readUInt8(11);
  const shardCount = data.readUInt8(12);
  const uploadedBitmap = data.readUInt16LE(14);
  const foundationId = data.readBigUInt64LE(80);
  const revision = data.readUInt32LE(88);
  const payloadLen = data.readUInt32LE(92);
  const globalConfig = new PublicKey(data.subarray(48, 80));
  const sizeX = data.readUInt16LE(128);
  const sizeY = data.readUInt16LE(130);
  const sizeZ = data.readUInt16LE(132);
  const expectedShardCount = Math.ceil(payloadLen / 8_192);
  const completeBitmap = (1 << expectedShardCount) - 1;
  const hasDimensions = sizeX > 0 && sizeY > 0 && sizeZ > 0;
  if (status > BUILDING_STATUS_ACTIVE
    || quarterTurns > 3
    || payloadLen < 1
    || payloadLen > 65_535
    || shardCount !== expectedShardCount
    || (uploadedBitmap & ~completeBitmap) !== 0
    || foundationId === 0n
    || revision === 0
    || !globalConfig.equals(deriveGlobalConfigPda()[0])
    || sizeX > 256
    || sizeY > 256
    || sizeZ > 256
    || (status === BUILDING_STATUS_UPLOADING && hasDimensions)
    || (status === BUILDING_STATUS_ACTIVE && !hasDimensions)) {
    throw new Error("Invalid Building Program BuildingManifest state");
  }
  return {
    version: BUILDING_MANIFEST_VERSION,
    status,
    bump: data.readUInt8(9),
    quarterTurns,
    shardCount,
    uploadedBitmap,
    owner: new PublicKey(data.subarray(16, 48)),
    globalConfig,
    foundationId,
    revision,
    payloadLen,
    expectedHash: data.subarray(96, 128),
    sizeX,
    sizeY,
    sizeZ,
    createdSlot: data.readBigUInt64LE(136),
    updatedSlot: data.readBigUInt64LE(144),
    offsetX: data.readInt32LE(152),
    offsetZ: data.readInt32LE(156),
  };
}

export function decodeBuildingShard(
  dataValue: Buffer | Uint8Array,
  { allowIncomplete = false }: { allowIncomplete?: boolean } = {},
): DecodedBuildingShard {
  const data = Buffer.from(dataValue);
  if (data.length < BUILDING_SHARD_HEADER_LEN
    || data.subarray(0, 8).toString("utf8") !== BUILDING_SHARD_MAGIC
    || data.readUInt8(8) !== BUILDING_SHARD_VERSION) {
    throw new Error("Invalid BuildingShard account");
  }
  const payloadLen = data.readUInt16LE(12);
  const uploadedLen = data.readUInt16LE(14);
  const globalConfig = new PublicKey(data.subarray(16, 48));
  if (payloadLen < 1
    || payloadLen > BUILDING_SHARD_PAYLOAD_LEN
    || uploadedLen > payloadLen
    || data.length !== BUILDING_SHARD_HEADER_LEN + payloadLen
    || !globalConfig.equals(deriveGlobalConfigPda()[0])
    || (!allowIncomplete && uploadedLen !== payloadLen)) {
    throw new Error("Invalid BuildingShard state");
  }
  return {
    version: BUILDING_SHARD_VERSION,
    bump: data.readUInt8(9),
    shardIndex: data.readUInt8(10),
    payloadLen,
    uploadedLen,
    globalConfig,
    foundationId: data.readBigUInt64LE(48),
    revision: data.readUInt32LE(56),
    payload: data.subarray(BUILDING_SHARD_HEADER_LEN, BUILDING_SHARD_HEADER_LEN + uploadedLen),
  };
}

export function foundationIndexBatch(
  foundation: DecodedBuildSite,
  limit = 4,
): Array<{ chunkX: number; chunkZ: number }> {
  const normalizedLimit = boundedInteger(limit, 1, 4, "limit");
  if (foundation.status === BUILD_SITE_STATUS_ACTIVE) return [];
  if (foundation.status !== BUILD_SITE_STATUS_INDEXING) {
    throw new Error("Unsupported BuildSite indexing status");
  }
  const registered = foundation.registeredChunks;
  const remaining = foundation.totalChunks - registered;
  if (registered < 0n || remaining <= 0n) return [];
  const count = Number(remaining < BigInt(normalizedLimit) ? remaining : BigInt(normalizedLimit));
  const active = normalizeFoundation(foundation);
  return Array.from(
    { length: count },
    (_unused, offset) => foundationChunkAt(active, registered + BigInt(offset)),
  );
}

export function foundationRollbackBatch(
  foundation: DecodedBuildSite,
  limit = 4,
): Array<{ chunkX: number; chunkZ: number }> {
  const normalizedLimit = boundedInteger(limit, 1, 4, "limit");
  if (foundation.status === BUILD_SITE_STATUS_ACTIVE) {
    throw new Error("Active land cannot be canceled");
  }
  if (foundation.status !== BUILD_SITE_STATUS_INDEXING
    && foundation.status !== BUILD_SITE_STATUS_CANCELING) {
    throw new Error("Unsupported BuildSite cancellation status");
  }
  const registered = foundation.registeredChunks;
  if (registered <= 0n) return [];
  const count = Number(registered < BigInt(normalizedLimit) ? registered : BigInt(normalizedLimit));
  const active = normalizeFoundation(foundation);
  return Array.from(
    { length: count },
    (_unused, offset) => foundationChunkAt(active, registered - BigInt(offset) - 1n),
  );
}

function normalizeFoundation(input: FoundationInput): FoundationInput {
  const minX = signedInteger(input.minX, 32, "minX");
  const minZ = signedInteger(input.minZ, 32, "minZ");
  const surfaceY = signedInteger(input.surfaceY, 16, "surfaceY");
  const width = positiveU32(input.width, "width");
  const depth = positiveU32(input.depth, "depth");
  if (minX % FOUNDATION_CHUNK_SIZE !== 0
    || minZ % FOUNDATION_CHUNK_SIZE !== 0
    || width < FOUNDATION_CHUNK_SIZE
    || depth < FOUNDATION_CHUNK_SIZE
    || width % FOUNDATION_CHUNK_SIZE !== 0
    || depth % FOUNDATION_CHUNK_SIZE !== 0) {
    throw new Error("Land foundations must use complete 16 x 16 chunks");
  }
  if (surfaceY <= -32 || surfaceY > 320) {
    throw new Error("Foundation surfaceY must be within the canonical build range");
  }
  const contractCount = BigInt(width / FOUNDATION_CHUNK_SIZE) * BigInt(depth / FOUNDATION_CHUNK_SIZE);
  if (contractCount > BigInt(MAX_LAND_CONTRACTS_PER_SITE)) {
    throw new Error(`A land parcel may use at most ${MAX_LAND_CONTRACTS_PER_SITE} contracts`);
  }
  const maxX = BigInt(minX) + BigInt(width) - 1n;
  const maxZ = BigInt(minZ) + BigInt(depth) - 1n;
  if (maxX > 0x7fff_ffffn || maxZ > 0x7fff_ffffn) {
    throw new Error("Foundation rectangle exceeds signed 32-bit world coordinates");
  }
  return { minX, minZ, surfaceY, width, depth };
}

function foundationChunkAt(
  foundation: FoundationInput,
  index: bigint,
): { chunkX: number; chunkZ: number } {
  const span = foundationChunkSpan(foundation);
  const total = span.spanX * span.spanZ;
  if (index < 0n || index >= total) throw new Error("Invalid BuildSite Chunk index");
  return {
    chunkX: span.minChunkX + Number(index % span.spanX),
    chunkZ: span.minChunkZ + Number(index / span.spanX),
  };
}

function foundationChunkCount(foundation: FoundationInput): bigint {
  const span = foundationChunkSpan(foundation);
  return span.spanX * span.spanZ;
}

function foundationChunkSpan(foundation: FoundationInput): {
  minChunkX: number;
  minChunkZ: number;
  spanX: bigint;
  spanZ: bigint;
} {
  const normalized = normalizeFoundation(foundation);
  const minChunkX = Math.floor(normalized.minX / FOUNDATION_CHUNK_SIZE);
  const minChunkZ = Math.floor(normalized.minZ / FOUNDATION_CHUNK_SIZE);
  return {
    minChunkX,
    minChunkZ,
    spanX: BigInt(normalized.width / FOUNDATION_CHUNK_SIZE),
    spanZ: BigInt(normalized.depth / FOUNDATION_CHUNK_SIZE),
  };
}

function u64Buffer(value: bigint | number | string, name: string): Buffer {
  const normalized = positiveU64(value, name);
  const bytes = Buffer.alloc(8);
  bytes.writeBigUInt64LE(normalized);
  return bytes;
}

function positiveU64(value: bigint | number | string, name: string): bigint {
  const normalized = BigInt(value);
  if (normalized <= 0n || normalized > 0xffff_ffff_ffff_ffffn) {
    throw new Error(`${name} must be a positive u64`);
  }
  return normalized;
}

function positiveU32(value: number, name: string): number {
  return boundedInteger(value, 1, 0xffff_ffff, name);
}

function boundedInteger(value: number, min: number, max: number, name: string): number {
  if (!Number.isInteger(value) || value < min || value > max) {
    throw new Error(`${name} must be an integer in ${min}..${max}`);
  }
  return value;
}

function signedInteger(value: number, bits: 16 | 32, name: string): number {
  const min = bits === 16 ? -0x8000 : -0x8000_0000;
  const max = bits === 16 ? 0x7fff : 0x7fff_ffff;
  return boundedInteger(value, min, max, name);
}

function assertI32(value: number, name: string): void {
  if (!Number.isInteger(value) || value < -0x8000_0000 || value > 0x7fff_ffff) {
    throw new Error(`${name} must fit in a signed int32`);
  }
}
