import { PublicKey } from "@solana/web3.js";
import assert from "node:assert/strict";
import { describe, it } from "mocha";

import {
  BUILD_SITE_LEN,
  BUILD_SITE_MAGIC,
  BUILD_SITE_STATUS_ACTIVE,
  BUILD_SITE_STATUS_CANCELING,
  BUILD_SITE_STATUS_INDEXING,
  BUILD_SITE_VERSION,
  BUILDING_MANIFEST_LEN,
  BUILDING_MANIFEST_MAGIC,
  BUILDING_MANIFEST_VERSION,
  BUILDING_SHARD_HEADER_LEN,
  BUILDING_SHARD_MAGIC,
  BUILDING_SHARD_VERSION,
  BUILDING_STATUS_ACTIVE,
  createBeginBuildingInstruction,
  createBuildSiteInstruction,
  createCancelBuildSiteIndexingInstruction,
  createCancelBuildingUploadInstruction,
  createFinalizeBuildingInstruction,
  createPublishGuardianBlueprintInstruction,
  createRegisterBuildSiteChunksInstruction,
  createWriteBuildingShardInstruction,
  decodeBuildingManifest,
  decodeBuildingShard,
  decodeBuildSite,
  deriveBuildingManifestPda,
  deriveBuildingShardPda,
  deriveBuildSitePda,
  deriveGuardianBlueprintAuthorityPda,
  deriveLandContractAuthorityPda,
  deriveMarketUserPda,
  foundationIndexBatch,
  foundationRollbackBatch,
  GUARDIAN_BLUEPRINT_PUBLISHER_WALLET,
  GUARDIAN_TREASURY_WALLET,
  MAX_LAND_CONTRACTS_PER_SITE,
  NICECHUNK_BUILDING_PROGRAM_ID,
} from "../sdk/nicechunk-building.ts";
import {
  deriveFoundationChunkPda,
  FOUNDATION_CHUNK_MAGIC,
  FOUNDATION_CHUNK_SEED,
  FOUNDATION_CHUNK_VERSION,
} from "../sdk/nicechunk-chunk.ts";
import { deriveGlobalConfigPda } from "../sdk/nicechunk-core.ts";

describe("nicechunk building SDK", () => {
  it("derives Building Program PDAs independently from Chunk", () => {
    const globalConfig = new PublicKey("4uQeVj5tqViQh7yWWGStvkEG1Zmhx6uasJtWCJziofM");
    const [site] = deriveBuildSitePda({ globalConfig, foundationId: 42n });
    const [manifest] = deriveBuildingManifestPda({ globalConfig, foundationId: 42n, revision: 3 });

    assert.notEqual(site.toBase58(), manifest.toBase58());
    assert.equal(NICECHUNK_BUILDING_PROGRAM_ID.toBase58(), "39UMTUWXQkuomkFNbDPF5NGZnJmG6pDkJHVSkZyqVwWx");
  });

  it("derives only v3 FoundationChunk PDAs for immutable land", () => {
    const globalConfig = deriveGlobalConfigPda()[0];
    const chunkX = -17;
    const chunkZ = 29;
    const chunkXBytes = Buffer.alloc(4);
    const chunkZBytes = Buffer.alloc(4);
    chunkXBytes.writeInt32LE(chunkX);
    chunkZBytes.writeInt32LE(chunkZ);
    const expected = PublicKey.findProgramAddressSync(
      [Buffer.from("foundation-chunk-v3"), globalConfig.toBuffer(), chunkXBytes, chunkZBytes],
      new PublicKey("GnVKn442KDTDgCyjVG7SEtCQQLjaCiLvrEZDWSU13wbj"),
    )[0];

    assert.equal(FOUNDATION_CHUNK_SEED, "foundation-chunk-v3");
    assert.equal(FOUNDATION_CHUNK_MAGIC, "NCKFCI03");
    assert.equal(FOUNDATION_CHUNK_VERSION, 3);
    assert.equal(
      deriveFoundationChunkPda({ globalConfig, chunkX, chunkZ })[0].toBase58(),
      expected.toBase58(),
    );
  });

  it("builds every final Building instruction with the canonical ABI", () => {
    const authority = PublicKey.unique();
    const owner = PublicKey.unique();
    const foundationId = 42n;
    const create = createBuildSiteInstruction({
      authority,
      owner,
      foundationId,
      foundation: { minX: 0, minZ: 0, surfaceY: 10, width: 16, depth: 16 },
    });
    assert.equal(create.data.readUInt8(0), 0);
    assert.equal(create.data.length, 27);
    assert.equal(create.keys.length, 10);
    assert.equal(create.keys[6].pubkey.toBase58(), owner.toBase58());
    assert.equal(create.keys[7].pubkey.toBase58(), deriveMarketUserPda({ owner })[0].toBase58());
    assert.equal(
      create.keys[8].pubkey.toBase58(),
      deriveLandContractAuthorityPda({ globalConfig: create.keys[4].pubkey })[0].toBase58(),
    );

    const indexing = decodeBuildSite(buildSiteBytes({ status: BUILD_SITE_STATUS_INDEXING, registered: 0n, total: 1n }));
    const register = createRegisterBuildSiteChunksInstruction({ authority, owner, foundation: indexing });
    assert.equal(register.data.readUInt8(0), 1);
    assert.equal(register.keys.length, 13);
    assert.equal(register.keys[8].pubkey.toBase58(), owner.toBase58());
    assert.equal(register.keys[9].pubkey.toBase58(), deriveMarketUserPda({ owner })[0].toBase58());

    const rollback = createCancelBuildSiteIndexingInstruction({ authority, owner, foundation: indexing });
    assert.equal(rollback.data.readUInt8(0), 6);
    assert.equal(rollback.keys.length, 12);

    const expectedHash = Buffer.alloc(32, 7);
    const begin = createBeginBuildingInstruction({
      authority,
      owner,
      foundationId,
      revision: 1,
      quarterTurns: 3,
      payloadLen: 9_000,
      expectedHash,
      offsetX: -2,
      offsetZ: 3,
    });
    assert.equal(begin.data.readUInt8(0), 2);
    assert.equal(begin.data.length, 58);
    assert.equal(begin.data.readInt32LE(50), -2);
    assert.equal(begin.data.readInt32LE(54), 3);

    const write = createWriteBuildingShardInstruction({
      authority,
      owner,
      foundationId,
      revision: 1,
      shardIndex: 1,
      offset: 700,
      bytes: Buffer.from([1, 2, 3]),
    });
    assert.equal(write.data.readUInt8(0), 3);
    assert.equal(write.data.length, 19);
    assert.equal(write.keys[5].pubkey.toBase58(), deriveBuildingShardPda({
      globalConfig: write.keys[6].pubkey,
      foundationId,
      revision: 1,
      shardIndex: 1,
    })[0].toBase58());

    const finalize = createFinalizeBuildingInstruction({ authority, owner, foundationId, revision: 1, shardCount: 2 });
    const cancel = createCancelBuildingUploadInstruction({ authority, owner, foundationId, revision: 1, shardCount: 2 });
    assert.equal(finalize.data.readUInt8(0), 4);
    assert.equal(cancel.data.readUInt8(0), 5);
    assert.equal(finalize.keys.length, 9);
    assert.equal(finalize.keys.at(-1)?.isWritable, false);
    assert.equal(cancel.keys.at(-1)?.isWritable, true);
  });

  it("routes Guardian blueprint writes through the Building Program PDA", () => {
    const instruction = createPublishGuardianBlueprintInstruction({
      publisher: GUARDIAN_BLUEPRINT_PUBLISHER_WALLET,
      regionX: -2,
      regionY: 3,
      blueprintHash: "25232284e49cf2cb4201bb072e27626c",
      blueprintRevision: 7n,
      blueprintRecordCount: 4,
    });
    const globalConfig = instruction.keys[3].pubkey;
    const [authority] = deriveGuardianBlueprintAuthorityPda({ globalConfig });

    assert.equal(instruction.programId.toBase58(), NICECHUNK_BUILDING_PROGRAM_ID.toBase58());
    assert.equal(instruction.data.readUInt8(0), 8);
    assert.equal(instruction.data.readInt32LE(1), -2);
    assert.equal(instruction.data.readInt32LE(5), 3);
    assert.equal(instruction.keys.length, 5);
    assert.equal(instruction.keys[0].pubkey.toBase58(), GUARDIAN_BLUEPRINT_PUBLISHER_WALLET.toBase58());
    assert.equal(instruction.keys[0].isSigner, true);
    assert.equal(instruction.keys[1].pubkey.toBase58(), authority.toBase58());
    assert.equal(instruction.keys[1].isSigner, false);
    assert.equal(GUARDIAN_TREASURY_WALLET.toBase58(), "9XuoVVwqP2jipt3jpJVXCSS2N2jr9vDuV3d6K73FKVud");
  });

  it("decodes immutable chunk-aligned BuildSite V3 land state", () => {
    const active = buildSiteBytes({ status: BUILD_SITE_STATUS_ACTIVE, registered: 1n, total: 1n });
    const activeSite = decodeBuildSite(active);
    assert.equal(activeSite.foundationId, 42n);
    assert.equal(activeSite.width, 16);
    assert.equal(activeSite.depth, 16);
    assert.equal(activeSite.contractType, 1);
    assert.equal(activeSite.landContractCount, 1);
    assert.equal(activeSite.updatedSlot, 12n);

    const wrongConfig = Buffer.from(active);
    PublicKey.unique().toBuffer().copy(wrongConfig, 48);
    assert.throws(() => decodeBuildSite(wrongConfig), /BuildSite state/);

    const partialChunk = buildSiteBytes({ status: BUILD_SITE_STATUS_ACTIVE, registered: 1n, total: 1n });
    partialChunk.writeUInt32LE(17, 100);
    assert.throws(() => decodeBuildSite(partialChunk), /complete 16 x 16 chunks/);

    const oversized = buildSiteBytes({
      status: BUILD_SITE_STATUS_ACTIVE,
      registered: BigInt(MAX_LAND_CONTRACTS_PER_SITE + 1),
      total: BigInt(MAX_LAND_CONTRACTS_PER_SITE + 1),
      width: 16 * (MAX_LAND_CONTRACTS_PER_SITE + 1),
      depth: 16,
    });
    assert.throws(() => decodeBuildSite(oversized), /at most 4096 contracts/);
  });

  it("rejects retired BuildSite layouts and decodes explicit building placement", () => {
    assert.throws(() => decodeBuildSite(Buffer.alloc(136)), /Invalid Building Program BuildSite/);
    const data = buildingManifestBytes();
    const centered = decodeBuildingManifest(data);
    assert.equal(centered.offsetX, 0);
    assert.equal(centered.offsetZ, 0);

    data.writeInt32LE(-2, 152);
    data.writeInt32LE(3, 156);
    const shifted = decodeBuildingManifest(data);
    assert.equal(shifted.offsetX, -2);
    assert.equal(shifted.offsetZ, 3);

    PublicKey.unique().toBuffer().copy(data, 48);
    assert.throws(() => decodeBuildingManifest(data), /BuildingManifest state/);
  });

  it("matches on-chain immutable land indexing order", () => {
    const indexing = decodeBuildSite(buildSiteBytes({
      status: BUILD_SITE_STATUS_INDEXING,
      registered: 2n,
      total: 4n,
      width: 32,
      depth: 32,
    }));
    assert.deepEqual(foundationIndexBatch(indexing), [
      { chunkX: 0, chunkZ: 1 },
      { chunkX: 1, chunkZ: 1 },
    ]);

    const canceling = decodeBuildSite(buildSiteBytes({
      status: BUILD_SITE_STATUS_CANCELING,
      registered: 3n,
      total: 4n,
      width: 32,
      depth: 32,
    }));
    assert.deepEqual(foundationRollbackBatch(canceling), [
      { chunkX: 0, chunkZ: 1 },
      { chunkX: 1, chunkZ: 0 },
      { chunkX: 0, chunkZ: 0 },
    ]);
    assert.throws(() => foundationIndexBatch(canceling), /Unsupported BuildSite indexing status/);
  });

  it("decodes complete BuildingShard payload bytes", () => {
    const payload = Buffer.from([4, 5, 6]);
    const data = Buffer.alloc(BUILDING_SHARD_HEADER_LEN + payload.length);
    data.write(BUILDING_SHARD_MAGIC, 0, "utf8");
    data.writeUInt8(BUILDING_SHARD_VERSION, 8);
    data.writeUInt8(2, 10);
    data.writeUInt16LE(payload.length, 12);
    data.writeUInt16LE(payload.length, 14);
    deriveGlobalConfigPda()[0].toBuffer().copy(data, 16);
    data.writeBigUInt64LE(42n, 48);
    data.writeUInt32LE(3, 56);
    payload.copy(data, BUILDING_SHARD_HEADER_LEN);
    assert.deepEqual([...decodeBuildingShard(data).payload], [...payload]);
    PublicKey.unique().toBuffer().copy(data, 16);
    assert.throws(() => decodeBuildingShard(data), /BuildingShard state/);
  });
});

function buildingManifestBytes(): Buffer {
  const data = Buffer.alloc(BUILDING_MANIFEST_LEN);
  data.write(BUILDING_MANIFEST_MAGIC, 0, "utf8");
  data.writeUInt8(BUILDING_MANIFEST_VERSION, 8);
  data.writeUInt8(1, 9);
  data.writeUInt8(BUILDING_STATUS_ACTIVE, 10);
  data.writeUInt8(1, 11);
  data.writeUInt8(1, 12);
  data.writeUInt16LE(1, 14);
  PublicKey.unique().toBuffer().copy(data, 16);
  deriveGlobalConfigPda()[0].toBuffer().copy(data, 48);
  data.writeBigUInt64LE(42n, 80);
  data.writeUInt32LE(3, 88);
  data.writeUInt32LE(13, 92);
  data.fill(5, 96, 128);
  data.writeUInt16LE(4, 128);
  data.writeUInt16LE(5, 130);
  data.writeUInt16LE(6, 132);
  data.writeBigUInt64LE(11n, 136);
  data.writeBigUInt64LE(12n, 144);
  return data;
}

function buildSiteBytes({
  status,
  registered,
  total,
  width = 16,
  depth = 16,
}: {
  status: number;
  registered: bigint;
  total: bigint;
  width?: number;
  depth?: number;
}): Buffer {
  const data = Buffer.alloc(BUILD_SITE_LEN);
  data.write(BUILD_SITE_MAGIC, 0, "utf8");
  data.writeUInt8(BUILD_SITE_VERSION, 8);
  data.writeUInt8(1, 9);
  data.writeUInt8(status, 10);
  data.writeUInt8(1, 11);
  data.writeUInt32LE((width / 16) * (depth / 16), 12);
  PublicKey.unique().toBuffer().copy(data, 16);
  deriveGlobalConfigPda()[0].toBuffer().copy(data, 48);
  data.writeBigUInt64LE(42n, 80);
  data.writeInt32LE(0, 88);
  data.writeInt32LE(0, 92);
  data.writeInt16LE(10, 96);
  data.writeUInt32LE(width, 100);
  data.writeUInt32LE(depth, 104);
  data.writeBigUInt64LE(11n, 108);
  data.writeUInt32LE(0, 116);
  data.writeUInt32LE(0, 120);
  data.writeBigUInt64LE(12n, 124);
  data.writeBigUInt64LE(registered, 132);
  data.writeBigUInt64LE(total, 140);
  return data;
}
