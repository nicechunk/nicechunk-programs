import { ComputeBudgetProgram, PublicKey, SystemProgram, Transaction } from "@solana/web3.js";
import assert from "node:assert";
import { readFileSync } from "node:fs";
import {
  createOrRefreshPlayerSessionInstruction,
  createInitializePlayerInstruction,
  createSetEquippedBackpackInstruction,
  createSetEquipmentSlotInstruction,
  createSetPlayerEquipmentSlotInstruction,
  createSwapPlayerEquipmentSlotsInstruction,
  createTransferPlayerEquipmentSlotInstruction,
  createSetPlayerNameInstruction,
  createUpsertPlayerAppearanceInstruction,
  createUpdatePlayerPositionInstruction,
  decodePlayerAppearance,
  decodePlayerEquipment,
  decodePlayerProfile,
  derivePlayerSessionPda,
  derivePlayerAppearancePda,
  derivePlayerEquipmentPda,
  deriveEquipmentTransferAuthorityPda,
  derivePlayerProfilePda,
  APPEARANCE_EQUIPMENT_SLOT_COUNT,
  APPEARANCE_EQUIPMENT_SLOT_LEN,
  APPEARANCE_MODEL_CODE_MAX_BYTES,
  APPEARANCE_TITLE_MAX_BYTES,
  CLEAR_EQUIPMENT_BACKPACK_INDEX,
  EQUIPMENT_SLOT_COUNT,
  NICECHUNK_PLAYER_PROGRAM_ID,
  PLAYER_APPEARANCE_LEN,
  PLAYER_EQUIPMENT_HEADER_LEN,
  PLAYER_EQUIPMENT_LEN,
  PLAYER_EQUIPMENT_FLAG_CUSTODY,
  PLAYER_EQUIPMENT_SLOT_LEN,
  SESSION_ACTION_BREAK_BLOCK,
  SESSION_ACTION_PLACE_BLOCK,
  PLAYER_PROFILE_LEN,
} from "../sdk/nicechunk-player.ts";
import {
  BACKPACK_ITEM_CATEGORY_MATERIAL,
  BACKPACK_LEN,
  BACKPACK_SLOT_KIND_ITEM,
  BACKPACK_SLOT_RECORD_LEN,
  createForgeEquipmentInstruction,
  createInitializeBackpackInstruction,
  decodeBackpack,
  decodeBackpackDecorationMetadata,
  encodeBackpackSlotRecord,
  encodeBackpackDecorationMetadata,
  deriveBackpackPda,
  deriveMaterialPhysicsPda,
  NICECHUNK_BACKPACK_PROGRAM_ID,
} from "../sdk/nicechunk-backpack.ts";
import {
  createApplyCivilizationSmeltingRecipeInstruction,
  createExecuteSmeltingInstruction,
  createInitializeRecipeTableInstruction,
  createUpsertSmeltingRecipeInstruction,
  deriveRecipeTablePda,
  deriveSmeltingAuthorityPda,
  encodeCivilizationSmeltingRecipePatch,
  NICECHUNK_SMELTING_PROGRAM_ID,
  UPSERT_RECIPE_ARGS_LEN,
} from "../sdk/nicechunk-smelting.ts";
import {
  deriveCivilizationAdapterAuthorityPda,
  NICECHUNK_CIVILIZATION_PROGRAM_ID,
} from "../sdk/nicechunk-civilization.ts";
import {
  BLOCK_STONE,
  BATCH_MINE_MODE_DEBUG,
  RANGE_MINE_MAX_BLOCKS,
  RANGE_MINE_MODE_DEBUG,
  CHUNK_BROKEN_HEADER_LEN,
  CHUNK_BROKEN_MAGIC,
  CHUNK_BROKEN_RECORD_LEN,
  FOUNDATION_CHUNK_HEADER_LEN,
  FOUNDATION_CHUNK_LEN,
  FOUNDATION_CHUNK_MAGIC,
  FOUNDATION_CHUNK_RECORD_LEN,
  FOUNDATION_CHUNK_VERSION,
  FOUNDATION_LEN,
  FOUNDATION_MAGIC,
  FOUNDATION_VERSION,
  RESOURCE_DROP_RULE_LEN,
  SURFACE_DECORATION_RULE_LEN,
  SURFACE_DECORATION_TABLE_HEADER_LEN,
  SURFACE_DECORATION_TABLE_LEN,
  SURFACE_DECORATION_TABLE_MAGIC,
  SURFACE_DECORATION_TABLE_VERSION,
  createApplyCivilizationSurfaceDecorationRulesInstruction,
  createApplyCivilizationResourceDropRulesInstruction,
  createInitializeSurfaceDecorationTableInstruction,
  createInitializeResourceDropTableInstruction,
  createFoundationInstruction,
  createMineBlockInstruction,
  createMineBlockWithRewardsInstruction,
  createBatchMineWithRewardsInstruction,
  createRangeMineWithRewardsInstruction,
  decodeChunkBrokenState,
  decodeFoundationChunkState,
  decodeFoundationState,
  decodeSurfaceDecorationTable,
  deriveChunkBrokenPda,
  deriveFoundationChunkPda,
  deriveFoundationPda,
  derivePlayerProgressPda,
  deriveResourceDropTablePda,
  deriveSurfaceDecorationTablePda,
  encodeSurfaceDecorationRules,
  encodeCivilizationResourceDropRulesPatch,
  generatedBlockIdAt,
  NICECHUNK_CHUNK_PROGRAM_ID,
  resolveSurfaceDecorationAt,
} from "../sdk/nicechunk-chunk.ts";
import {
  deriveGlobalConfigPda,
  NICECHUNK_CORE_PROGRAM_ID,
} from "../sdk/nicechunk-core.ts";
import { resourceDropRules } from "../src/data/resourceDropRules.js";
// @ts-ignore Runtime rules are shared with chunk.js.
import { DEFAULT_SURFACE_DECORATION_RULES } from "../chunk.js/world/surface-decoration-rules.js";

describe("nicechunk player and mining SDK", () => {
  const owner = new PublicKey("9XuoVVwqP2jipt3jpJVXCSS2N2jr9vDuV3d6K73FKVud");
  const sessionAuthority = new PublicKey("Z2WsAfHEgNiycsaKoSo83TzVzGnc6nLB1CkEKN9vymw");
  const [globalConfig] = deriveGlobalConfigPda(NICECHUNK_CORE_PROGRAM_ID);

  it("derives deterministic player and chunk-broken PDAs", () => {
    const [playerProfile] = derivePlayerProfilePda(owner, NICECHUNK_PLAYER_PROGRAM_ID);
    const [playerAppearance] = derivePlayerAppearancePda(owner, NICECHUNK_PLAYER_PROGRAM_ID);
    const [chunkBroken] = deriveChunkBrokenPda({
      globalConfig,
      chunkX: 0,
      chunkZ: 0,
      programId: NICECHUNK_CHUNK_PROGRAM_ID,
    });
    const [playerProgress] = derivePlayerProgressPda({
      globalConfig,
      owner,
      programId: NICECHUNK_CHUNK_PROGRAM_ID,
    });
    const [foundation] = deriveFoundationPda({
      globalConfig,
      owner,
      foundationId: 7n,
      programId: NICECHUNK_CHUNK_PROGRAM_ID,
    });
    const [foundationChunk] = deriveFoundationChunkPda({
      globalConfig,
      chunkX: -1,
      chunkZ: 2,
      programId: NICECHUNK_CHUNK_PROGRAM_ID,
    });

    assert.equal(playerProfile.toBase58(), "EoF1AWvkQSU3c1hysJFGNLbDZHE8VNX9UUcR96GjCdRJ");
    assert.equal(playerAppearance.toBase58(), "85qpt8zVqoT7nHY9RxWfdQz12wdzoovsBvLqQ5N42a87");
    assert.equal(chunkBroken.toBase58(), "JCaDriShc3cnJGn4weuVsU266BLBjfFNzsw2z6GykKMY");
    assert.equal(playerProgress.toBase58(), "4aJHz4oKaydDYiVUu16y8u3HF3qFAjwnPT3y5TvzkH2t");
    assert.equal(foundation.toBase58(), deriveFoundationPda({
      globalConfig,
      owner,
      foundationId: "7",
      programId: NICECHUNK_CHUNK_PROGRAM_ID,
    })[0].toBase58());
    assert.equal(foundationChunk.toBase58(), deriveFoundationChunkPda({
      globalConfig,
      chunkX: -1,
      chunkZ: 2,
      programId: NICECHUNK_CHUNK_PROGRAM_ID,
    })[0].toBase58());
  });

  it("builds initialize player account order", () => {
    const [playerProfile] = derivePlayerProfilePda(owner, NICECHUNK_PLAYER_PROGRAM_ID);
    const ix = createInitializePlayerInstruction({ payer: owner, playerName: "Jerry_Miner" });

    assert.equal(ix.programId.toBase58(), NICECHUNK_PLAYER_PROGRAM_ID.toBase58());
    assert.equal(ix.data.readUInt8(0), 0);
    assert.equal(ix.data.subarray(1).toString("utf8"), "Jerry_Miner");
    assert.equal(ix.keys[0].pubkey.toBase58(), owner.toBase58());
    assert.equal(ix.keys[0].isSigner, true);
    assert.equal(ix.keys[0].isWritable, true);
    assert.equal(ix.keys[1].pubkey.toBase58(), playerProfile.toBase58());
    assert.equal(ix.keys[1].isWritable, true);
    assert.equal(ix.keys[2].pubkey.toBase58(), globalConfig.toBase58());
    assert.equal(ix.keys[3].pubkey.toBase58(), SystemProgram.programId.toBase58());
  });

  it("builds upsert player appearance instruction", () => {
    const [playerProfile] = derivePlayerProfilePda(owner, NICECHUNK_PLAYER_PROGRAM_ID);
    const [appearance] = derivePlayerAppearancePda(owner, NICECHUNK_PLAYER_PROGRAM_ID);
    const ix = createUpsertPlayerAppearanceInstruction({
      authority: owner,
      displayName: "Jerry_Miner",
      title: "Genesis Miner",
      modelKind: 1,
      modelCode: "NCM2:test-model",
    });

    assert.equal(ix.programId.toBase58(), NICECHUNK_PLAYER_PROGRAM_ID.toBase58());
    assert.equal(ix.data.readUInt8(0), 8);
    assert.equal(ix.data.readUInt8(1), 1);
    assert.equal(ix.data.readUInt16LE(2), Buffer.from("Jerry_Miner").length);
    assert.equal(ix.data.readUInt16LE(4), Buffer.from("Genesis Miner").length);
    assert.equal(ix.data.readUInt16LE(6), Buffer.from("NCM2:test-model").length);
    assert.equal(ix.keys[0].pubkey.toBase58(), owner.toBase58());
    assert.equal(ix.keys[0].isSigner, true);
    assert.equal(ix.keys[0].isWritable, true);
    assert.equal(ix.keys[1].pubkey.toBase58(), playerProfile.toBase58());
    assert.equal(ix.keys[1].isWritable, true);
    assert.equal(ix.keys[2].pubkey.toBase58(), appearance.toBase58());
    assert.equal(ix.keys[2].isWritable, true);
    assert.equal(ix.keys[3].pubkey.toBase58(), globalConfig.toBase58());
    assert.equal(ix.keys[4].pubkey.toBase58(), SystemProgram.programId.toBase58());
  });

  it("builds initialize backpack with player profile guard", () => {
    const backpackId = 1n;
    const [playerProfile] = derivePlayerProfilePda(owner, NICECHUNK_PLAYER_PROGRAM_ID);
    const [backpack] = deriveBackpackPda({ creator: owner, backpackId });
    const ix = createInitializeBackpackInstruction({ payer: owner, backpackId });

    assert.equal(ix.programId.toBase58(), NICECHUNK_BACKPACK_PROGRAM_ID.toBase58());
    assert.equal(ix.data.readUInt8(0), 1);
    assert.equal(ix.data.readUInt8(1), 0);
    assert.equal(ix.data.readBigUInt64LE(2), backpackId);
    assert.equal(ix.keys[0].pubkey.toBase58(), owner.toBase58());
    assert.equal(ix.keys[0].isSigner, true);
    assert.equal(ix.keys[1].pubkey.toBase58(), playerProfile.toBase58());
    assert.equal(ix.keys[1].isWritable, false);
    assert.equal(ix.keys[2].pubkey.toBase58(), backpack.toBase58());
    assert.equal(ix.keys[2].isWritable, true);
    assert.equal(ix.keys[3].pubkey.toBase58(), SystemProgram.programId.toBase58());
  });

  it("decodes v3 backpack item reference slots", () => {
    const [backpack, bump] = deriveBackpackPda({ creator: owner, backpackId: 9n });
    assert.ok(backpack);
    const itemPda = new PublicKey("CEzcpJe9UTq5FmVzpTfgPffMbqdG97YJeFMJYwUSFhNF");
    const slot = {
      kind: BACKPACK_SLOT_KIND_ITEM,
      category: BACKPACK_ITEM_CATEGORY_MATERIAL,
      flags: 0,
      quantity: 2,
      resource: { worldX: 0, worldY: 0, worldZ: 0 },
      itemCode: 101,
      itemId: 77n,
      itemPda,
      durabilityCurrent: 900,
      durabilityMax: 1200,
      grade: 4,
      itemLevel: 17,
      qualityBps: 7200,
      metadata: 42,
    };
    const data = Buffer.alloc(BACKPACK_LEN);
    data.write("NCKBPK01", 0, "utf8");
    data.writeUInt16LE(3, 8);
    data.writeUInt8(bump, 10);
    data.writeUInt8(1, 11);
    data.writeBigUInt64LE(9n, 12);
    owner.toBuffer().copy(data, 20);
    data.writeUInt8(50, 52);
    data.writeUInt8(1, 53);
    data.writeUInt8(1, 54);
    data.writeBigUInt64LE(10n, 66);
    data.writeBigUInt64LE(11n, 74);
    data.writeBigInt64LE(12n, 82);
    encodeBackpackSlotRecord(slot).copy(data, 128);

    const decoded = decodeBackpack(data);
    assert.equal(decoded.version, 3);
    assert.equal(decoded.records.length, 0);
    assert.equal(decoded.slots.length, 1);
    assert.equal(decoded.slots[0].kind, BACKPACK_SLOT_KIND_ITEM);
    assert.equal(decoded.slots[0].quantity, 2);
    assert.equal(decoded.slots[0].itemPda.toBase58(), itemPda.toBase58());
    assert.equal(decoded.slots[0].durabilityCurrent, 900);
    assert.equal(decoded.slots[0].durabilityMax, 1200);
    assert.equal(decoded.slots[0].grade, 4);
    assert.equal(decoded.slots[0].itemLevel, 17);
    assert.equal(decoded.slots[0].qualityBps, 7200);
    assert.equal(decoded.slots[0].metadata, 42);
  });

  it("round-trips generic surface decoration identity through block metadata", () => {
    const metadata = encodeBackpackDecorationMetadata({ ruleId: 1, decorationId: 2 });
    assert.equal(metadata, 0x00010002);
    assert.deepEqual(decodeBackpackDecorationMetadata(metadata), {
      ruleId: 1,
      decorationId: 2,
    });
    assert.equal(decodeBackpackDecorationMetadata(0), null);
  });

  it("builds update position and equipment instructions", () => {
    const updatePosition = createUpdatePlayerPositionInstruction({ authority: owner, x: 16, y: 2, z: -16 });
    assert.equal(updatePosition.data.readUInt8(0), 1);
    assert.equal(updatePosition.data.readInt32LE(1), 16);
    assert.equal(updatePosition.data.readInt32LE(5), 2);
    assert.equal(updatePosition.data.readInt32LE(9), -16);

    const equipmentBackpack = new PublicKey("6pCaR8qLHvGeU3BAzwzAHMPjDk1ewNtrbAcqAGeMSH2Q");
    const setEquipment = createSetEquipmentSlotInstruction({
      authority: owner,
      slot: 8,
      backpack: equipmentBackpack,
      backpackSlotIndex: 3,
    });
    assert.equal(setEquipment.data.readUInt8(0), 2);
    assert.equal(setEquipment.data.readUInt8(1), 8);
    assert.equal(setEquipment.data.readUInt8(2), 3);
    assert.equal(setEquipment.keys[3].pubkey.toBase58(), equipmentBackpack.toBase58());

    const clearEquipment = createSetEquipmentSlotInstruction({ authority: owner, slot: 8 });
    assert.equal(clearEquipment.data.readUInt8(2), CLEAR_EQUIPMENT_BACKPACK_INDEX);
    assert.equal(clearEquipment.keys.length, 3);

    const modelCode = Buffer.from([0xe0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13]);
    const setEquipmentV2 = createSetPlayerEquipmentSlotInstruction({
      authority: owner,
      slot: 7,
      backpack: equipmentBackpack,
      backpackSlotIndex: 3,
      modelCode,
    });
    assert.equal(setEquipmentV2.data.readUInt8(0), 12);
    assert.equal(setEquipmentV2.data.readUInt8(1), 7);
    assert.equal(setEquipmentV2.data.readUInt8(2), 3);
    assert.equal(setEquipmentV2.data.readUInt16LE(3), modelCode.length);
    assert.deepEqual(setEquipmentV2.data.subarray(5), modelCode);
    assert.equal(setEquipmentV2.keys[2].pubkey.toBase58(), derivePlayerEquipmentPda(owner)[0].toBase58());
    assert.equal(setEquipmentV2.keys[5].pubkey.toBase58(), equipmentBackpack.toBase58());

    const transferEquipment = createTransferPlayerEquipmentSlotInstruction({
      authority: owner,
      slot: 7,
      backpack: equipmentBackpack,
      backpackSlotIndex: 3,
      modelCode,
    });
    assert.equal(transferEquipment.data.readUInt8(0), 13);
    assert.equal(transferEquipment.keys.length, 9);
    assert.equal(transferEquipment.keys[4].pubkey.toBase58(), deriveMaterialPhysicsPda({ globalConfig })[0].toBase58());
    assert.equal(transferEquipment.keys[6].pubkey.toBase58(), equipmentBackpack.toBase58());
    assert.equal(
      transferEquipment.keys[8].pubkey.toBase58(),
      deriveEquipmentTransferAuthorityPda()[0].toBase58(),
    );
    const swapEquipment = createSwapPlayerEquipmentSlotsInstruction({
      authority: owner,
      fromSlot: 2,
      toSlot: 7,
    });
    assert.deepEqual(Array.from(swapEquipment.data), [14, 2, 7]);

    const backpack = new PublicKey("CEzcpJe9UTq5FmVzpTfgPffMbqdG97YJeFMJYwUSFhNF");
    const bindBackpack = createSetEquippedBackpackInstruction({ authority: owner, backpack });
    assert.equal(bindBackpack.data.readUInt8(0), 5);
    assert.equal(bindBackpack.keys[0].pubkey.toBase58(), owner.toBase58());
    assert.equal(bindBackpack.keys[0].isSigner, true);
    assert.equal(bindBackpack.keys[0].isWritable, true);
    assert.equal(bindBackpack.keys[1].isWritable, true);
    assert.equal(bindBackpack.keys[2].pubkey.toBase58(), backpack.toBase58());
    assert.equal(bindBackpack.keys[3].pubkey.toBase58(), SystemProgram.programId.toBase58());

    const setName = createSetPlayerNameInstruction({ authority: owner, playerName: "New_Name_100" });
    assert.equal(setName.data.readUInt8(0), 7);
    assert.equal(setName.data.subarray(1).toString("utf8"), "New_Name_100");
    assert.equal(setName.keys[0].pubkey.toBase58(), owner.toBase58());
    assert.equal(setName.keys[1].pubkey.toBase58(), derivePlayerProfilePda(owner, NICECHUNK_PLAYER_PROGRAM_ID)[0].toBase58());
    assert.equal(setName.keys[2].pubkey.toBase58(), globalConfig.toBase58());
  });

  it("decodes a player profile buffer", () => {
    const [playerProfile, bump] = derivePlayerProfilePda(owner, NICECHUNK_PLAYER_PROGRAM_ID);
    assert.ok(playerProfile);
    const data = Buffer.alloc(PLAYER_PROFILE_LEN);
    let offset = 0;
    data.write("NCKPLY01", offset, "utf8"); offset += 8;
    data.writeUInt16LE(7, offset); offset += 2;
    data.writeUInt8(bump, offset++);
    data.writeUInt8(1, offset++);
    owner.toBuffer().copy(data, offset); offset += 32;
    globalConfig.toBuffer().copy(data, offset); offset += 32;
    data.writeUInt16LE(1, offset); offset += 2;
    data.writeInt32LE(1, offset); offset += 4;
    data.writeInt32LE(2, offset); offset += 4;
    data.writeInt32LE(3, offset); offset += 4;
    for (const value of [100, 100, 100, 1, 1, 0]) {
      data.writeUInt16LE(value, offset); offset += 2;
    }
    data.writeUInt8(EQUIPMENT_SLOT_COUNT, offset++);
    offset += EQUIPMENT_SLOT_COUNT * 32;
    data.writeUInt8(0, offset++);
    data.writeUInt8(0, offset++);
    const equippedBackpack = new PublicKey("CEzcpJe9UTq5FmVzpTfgPffMbqdG97YJeFMJYwUSFhNF");
    equippedBackpack.toBuffer().copy(data, offset); offset += 32;
    data.writeBigUInt64LE(10n, offset); offset += 8;
    data.writeBigUInt64LE(11n, offset); offset += 8;
    data.writeBigInt64LE(12n, offset); offset += 8;
    data.writeBigUInt64LE(1234n, offset); offset += 8;
    data.writeUInt32LE(3, offset); offset += 4;
    data.writeUInt8(5, offset++);
    data.writeUInt8(21, offset++);
    const playerName = Buffer.from("OnChainJerry_100", "utf8");
    data.writeUInt16LE(playerName.length, offset); offset += 2;
    playerName.copy(data, offset); offset += playerName.length;
    offset += 300 - playerName.length;
    offset += 8;

    const decoded = decodePlayerProfile(data);
    assert.equal(offset, PLAYER_PROFILE_LEN);
    assert.equal(decoded.owner.toBase58(), owner.toBase58());
    assert.equal(decoded.position.y, 2);
    assert.equal(decoded.equipment.length, EQUIPMENT_SLOT_COUNT);
    assert.equal(decoded.equippedBackpack.toBase58(), equippedBackpack.toBase58());
    assert.equal(decoded.forgingXp, 1234n);
    assert.equal(decoded.forgedItemCount, 3);
    assert.equal(decoded.bestForgedGrade, 5);
    assert.equal(decoded.bestForgedItemLevel, 21);
    assert.equal(decoded.playerName, "OnChainJerry_100");
  });

  it("decodes the authoritative player equipment PDA", () => {
    const [playerProfile] = derivePlayerProfilePda(owner, NICECHUNK_PLAYER_PROGRAM_ID);
    const [playerEquipment, bump] = derivePlayerEquipmentPda(owner, NICECHUNK_PLAYER_PROGRAM_ID);
    const backpack = new PublicKey("6pCaR8qLHvGeU3BAzwzAHMPjDk1ewNtrbAcqAGeMSH2Q");
    const itemPda = new PublicKey("CEzcpJe9UTq5FmVzpTfgPffMbqdG97YJeFMJYwUSFhNF");
    const data = Buffer.alloc(PLAYER_EQUIPMENT_LEN);
    data.write("NCKEQP01", 0, "utf8");
    data.writeUInt16LE(1, 8);
    data.writeUInt8(bump, 10);
    data.writeUInt8(1, 11);
    owner.toBuffer().copy(data, 12);
    playerProfile.toBuffer().copy(data, 44);
    globalConfig.toBuffer().copy(data, 76);
    data.writeUInt8(EQUIPMENT_SLOT_COUNT, 108);
    data.writeBigUInt64LE(10n, 112);
    data.writeBigUInt64LE(11n, 120);
    for (let index = 0; index < EQUIPMENT_SLOT_COUNT; index += 1) {
      const offset = PLAYER_EQUIPMENT_HEADER_LEN + index * PLAYER_EQUIPMENT_SLOT_LEN;
      data.writeUInt8(index, offset + 1);
      data.writeUInt8(CLEAR_EQUIPMENT_BACKPACK_INDEX, offset + 2);
    }
    const offset = PLAYER_EQUIPMENT_HEADER_LEN + 7 * PLAYER_EQUIPMENT_SLOT_LEN;
    const model = Buffer.from([0xe0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13]);
    data.writeUInt8(1, offset);
    data.writeUInt8(7, offset + 1);
    data.writeUInt8(4, offset + 2);
    data.writeUInt8(1 | PLAYER_EQUIPMENT_FLAG_CUSTODY, offset + 3);
    data.writeUInt16LE(model.length, offset + 4);
    backpack.toBuffer().copy(data, offset + 8);
    data.writeUInt8(2, offset + 40);
    data.writeUInt8(2, offset + 41);
    data.writeUInt32LE(1, offset + 44);
    data.writeUInt16LE(8, offset + 58);
    data.writeBigUInt64LE(77n, offset + 60);
    itemPda.toBuffer().copy(data, offset + 68);
    data.writeUInt32LE(0x1234abcd, offset + 116);
    model.copy(data, offset + 120);

    const decoded = decodePlayerEquipment(data);
    assert.equal(playerEquipment.toBase58(), derivePlayerEquipmentPda(owner)[0].toBase58());
    assert.equal(decoded.owner.toBase58(), owner.toBase58());
    assert.equal(decoded.slots[7].equipped, true);
    assert.equal(decoded.slots[7].custodied, true);
    assert.equal(decoded.slots[7].backpackIndex, 4);
    assert.equal(decoded.slots[7].itemId, 77n);
    assert.equal(decoded.slots[7].itemPda.toBase58(), itemPda.toBase58());
    assert.deepEqual(decoded.slots[7].modelCode, model);
  });

  it("decodes a public player appearance buffer", () => {
    const [playerProfile] = derivePlayerProfilePda(owner, NICECHUNK_PLAYER_PROGRAM_ID);
    const [appearance, bump] = derivePlayerAppearancePda(owner, NICECHUNK_PLAYER_PROGRAM_ID);
    assert.ok(appearance);
    const treasury = new PublicKey("CtPV2vmqNNwUSfMu5nz58ZtMPy6ZvxL4LyNdPHVW7WvF");
    const data = Buffer.alloc(PLAYER_APPEARANCE_LEN);
    let offset = 0;
    data.write("NCKAPP01", offset, "utf8"); offset += 8;
    data.writeUInt16LE(1, offset); offset += 2;
    data.writeUInt8(bump, offset++);
    data.writeUInt8(1, offset++);
    owner.toBuffer().copy(data, offset); offset += 32;
    playerProfile.toBuffer().copy(data, offset); offset += 32;
    globalConfig.toBuffer().copy(data, offset); offset += 32;
    treasury.toBuffer().copy(data, offset); offset += 32;
    data.writeUInt8(1, offset++);
    data.writeUInt16LE(0, offset); offset += 2;
    const displayName = Buffer.from("Jerry_Miner", "utf8");
    const title = Buffer.from("Genesis Miner", "utf8");
    const modelCode = Buffer.from("NCM2:test-model", "utf8");
    data.writeUInt16LE(displayName.length, offset); offset += 2;
    data.writeUInt16LE(title.length, offset); offset += 2;
    data.writeUInt16LE(modelCode.length, offset); offset += 2;
    data.writeUInt8(APPEARANCE_EQUIPMENT_SLOT_COUNT, offset++);
    data.writeBigUInt64LE(10n, offset); offset += 8;
    data.writeBigUInt64LE(11n, offset); offset += 8;
    data.writeBigInt64LE(12n, offset); offset += 8;
    data.writeBigInt64LE(13n, offset); offset += 8;
    offset = 256;
    displayName.copy(data, offset); offset += 300;
    title.copy(data, offset); offset += APPEARANCE_TITLE_MAX_BYTES;
    modelCode.copy(data, offset); offset += APPEARANCE_MODEL_CODE_MAX_BYTES;
    const equipmentOffset = offset + 7 * APPEARANCE_EQUIPMENT_SLOT_LEN;
    const itemPda = new PublicKey("CEzcpJe9UTq5FmVzpTfgPffMbqdG97YJeFMJYwUSFhNF");
    const itemCode = Buffer.from("NCM2:item", "utf8");
    data.writeUInt8(1, equipmentOffset);
    data.writeUInt8(7, equipmentOffset + 1);
    data.writeUInt16LE(3, equipmentOffset + 2);
    itemPda.toBuffer().copy(data, equipmentOffset + 4);
    data.writeUInt16LE(itemCode.length, equipmentOffset + 36);
    data.writeUInt32LE(450, equipmentOffset + 38);
    data.writeInt16LE(10, equipmentOffset + 42);
    data.writeInt16LE(20, equipmentOffset + 44);
    data.writeInt16LE(30, equipmentOffset + 46);
    data.writeInt16LE(90, equipmentOffset + 48);
    data.writeInt16LE(0, equipmentOffset + 50);
    data.writeInt16LE(-45, equipmentOffset + 52);
    itemCode.copy(data, equipmentOffset + 64);

    const decoded = decodePlayerAppearance(data);
    assert.equal(decoded.owner.toBase58(), owner.toBase58());
    assert.equal(decoded.playerProfile.toBase58(), playerProfile.toBase58());
    assert.equal(decoded.treasuryAuthority.toBase58(), treasury.toBase58());
    assert.equal(decoded.displayName, "Jerry_Miner");
    assert.equal(decoded.title, "Genesis Miner");
    assert.equal(decoded.modelCode, "NCM2:test-model");
    assert.equal(decoded.equipment.length, APPEARANCE_EQUIPMENT_SLOT_COUNT);
    assert.equal(decoded.equipment[7].equipped, true);
    assert.equal(decoded.equipment[7].itemPda.toBase58(), itemPda.toBase58());
    assert.equal(decoded.equipment[7].massGrams, 450);
    assert.equal(decoded.equipment[7].gripPoint.y, 20);
    assert.equal(decoded.equipment[7].gripRotation.z, -45);
    assert.equal(decoded.equipment[7].modelCode, "NCM2:item");
  });

  it("builds forge equipment instructions", () => {
    const backpack = new PublicKey("6pCaR8qLHvGeU3BAzwzAHMPjDk1ewNtrbAcqAGeMSH2Q");
    const [playerProfile] = derivePlayerProfilePda(owner, NICECHUNK_PLAYER_PROGRAM_ID);
    const codeBytes = Uint8Array.from([0xe0, 1, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);
    const forge = createForgeEquipmentInstruction({
      owner,
      backpack,
      itemId: 91n,
      codeBytes,
      inputIndexes: [2, 0, 2],
    });
    assert.equal(forge.programId.toBase58(), NICECHUNK_BACKPACK_PROGRAM_ID.toBase58());
    assert.equal(forge.data.readUInt8(0), 1);
    assert.equal(forge.data.readUInt8(1), 8);
    assert.equal(forge.data.readBigUInt64LE(2), 91n);
    assert.equal(forge.data.readUInt16LE(10), codeBytes.length);
    assert.equal(forge.data.readUInt8(12), 2);
    assert.deepEqual([...forge.data.subarray(13, 13 + codeBytes.length)], [...codeBytes]);
    assert.deepEqual([...forge.data.subarray(13 + codeBytes.length)], [2, 0]);
    assert.equal(forge.keys[0].pubkey.toBase58(), owner.toBase58());
    assert.equal(forge.keys[0].isSigner, true);
    assert.equal(forge.keys[1].pubkey.toBase58(), playerProfile.toBase58());
    assert.equal(forge.keys[1].isWritable, true);
    assert.equal(forge.keys[2].pubkey.toBase58(), backpack.toBase58());
    assert.equal(forge.keys[3].pubkey.toBase58(), NICECHUNK_PLAYER_PROGRAM_ID.toBase58());
    assert.equal(forge.keys[4].pubkey.toBase58(), SystemProgram.programId.toBase58());
  });

  it("builds player session and canonical mine instructions", () => {
    const [playerProfile] = derivePlayerProfilePda(owner, NICECHUNK_PLAYER_PROGRAM_ID);
    const [playerSession] = derivePlayerSessionPda({ owner, sessionAuthority, programId: NICECHUNK_PLAYER_PROGRAM_ID });
    const [chunkBroken] = deriveChunkBrokenPda({ globalConfig, chunkX: 0, chunkZ: 0, programId: NICECHUNK_CHUNK_PROGRAM_ID });
    const [foundationChunk] = deriveFoundationChunkPda({ globalConfig, chunkX: 0, chunkZ: 0, programId: NICECHUNK_CHUNK_PROGRAM_ID });
    const sessionIx = createOrRefreshPlayerSessionInstruction({ owner, sessionAuthority, expiresAt: 1_800_000_000n });

    assert.equal(sessionIx.programId.toBase58(), NICECHUNK_PLAYER_PROGRAM_ID.toBase58());
    assert.equal(sessionIx.data.readUInt8(0), 4);
    assert.equal(sessionIx.data.readBigInt64LE(1), 1_800_000_000n);
    assert.equal(sessionIx.data.readUInt16LE(9), SESSION_ACTION_BREAK_BLOCK | SESSION_ACTION_PLACE_BLOCK);
    assert.equal(sessionIx.keys[0].pubkey.toBase58(), owner.toBase58());
    assert.equal(sessionIx.keys[1].pubkey.toBase58(), sessionAuthority.toBase58());
    assert.equal(sessionIx.keys[2].pubkey.toBase58(), playerProfile.toBase58());
    assert.equal(sessionIx.keys[3].pubkey.toBase58(), playerSession.toBase58());

    const mineIx = createMineBlockInstruction({
      payer: sessionAuthority,
      owner,
      sessionAuthority,
      block: { worldX: 1, worldY: 0, worldZ: 2, expectedBlockId: BLOCK_STONE },
    });
    assert.equal(mineIx.programId.toBase58(), NICECHUNK_CHUNK_PROGRAM_ID.toBase58());
    assert.equal(mineIx.data.readUInt8(0), 5);
    assert.equal(mineIx.data.readInt32LE(1), 1);
    assert.equal(mineIx.data.readInt16LE(5), 0);
    assert.equal(mineIx.data.readInt32LE(7), 2);
    assert.equal(mineIx.data.readUInt16LE(11), BLOCK_STONE);
    assert.equal(mineIx.keys[0].pubkey.toBase58(), sessionAuthority.toBase58());
    assert.equal(mineIx.keys[1].pubkey.toBase58(), playerProfile.toBase58());
    assert.equal(mineIx.keys[2].pubkey.toBase58(), playerSession.toBase58());
    assert.equal(mineIx.keys[3].pubkey.toBase58(), chunkBroken.toBase58());
    assert.equal(mineIx.keys[4].pubkey.toBase58(), foundationChunk.toBase58());
    assert.equal(mineIx.keys[4].isWritable, false);
    assert.equal(mineIx.keys[5].pubkey.toBase58(), globalConfig.toBase58());
    assert.equal(mineIx.keys[6].pubkey.toBase58(), SystemProgram.programId.toBase58());

    const backpack = new PublicKey("6pCaR8qLHvGeU3BAzwzAHMPjDk1ewNtrbAcqAGeMSH2Q");
    const [playerProgress] = derivePlayerProgressPda({ globalConfig, owner, programId: NICECHUNK_CHUNK_PROGRAM_ID });
    const rewardMineIx = createMineBlockWithRewardsInstruction({
      payer: sessionAuthority,
      owner,
      sessionAuthority,
      backpack,
      block: { worldX: 1, worldY: 0, worldZ: 2, expectedBlockId: BLOCK_STONE },
    });
    assert.equal(rewardMineIx.data.readUInt8(0), 8);
    assert.equal(rewardMineIx.keys.length, 13);
    assert.equal(rewardMineIx.keys[0].pubkey.toBase58(), sessionAuthority.toBase58());
    assert.equal(rewardMineIx.keys[1].pubkey.toBase58(), playerProfile.toBase58());
    assert.equal(rewardMineIx.keys[2].pubkey.toBase58(), playerSession.toBase58());
    assert.equal(rewardMineIx.keys[3].pubkey.toBase58(), playerProgress.toBase58());
    assert.equal(rewardMineIx.keys[3].isWritable, true);
    assert.equal(rewardMineIx.keys[4].pubkey.toBase58(), chunkBroken.toBase58());
    assert.equal(rewardMineIx.keys[5].pubkey.toBase58(), foundationChunk.toBase58());
    assert.equal(rewardMineIx.keys[5].isWritable, false);
    assert.equal(rewardMineIx.keys[8].pubkey.toBase58(), deriveSurfaceDecorationTablePda({
      globalConfig,
      programId: NICECHUNK_CHUNK_PROGRAM_ID,
    })[0].toBase58());
    assert.equal(rewardMineIx.keys[10].pubkey.toBase58(), backpack.toBase58());
    assert.equal(rewardMineIx.keys[11].pubkey.toBase58(), deriveMaterialPhysicsPda({ globalConfig })[0].toBase58());
    assert.equal(rewardMineIx.keys[12].pubkey.toBase58(), SystemProgram.programId.toBase58());

    const batchMineIx = createBatchMineWithRewardsInstruction({
      payer: sessionAuthority,
      owner,
      sessionAuthority,
      backpack,
      blocks: [
        { worldX: 1, worldY: 0, worldZ: 2, expectedBlockId: BLOCK_STONE },
        { worldX: 2, worldY: 0, worldZ: 2, expectedBlockId: BLOCK_STONE },
      ],
    });
    assert.equal(batchMineIx.data.readUInt8(0), 20);
    assert.equal(batchMineIx.data.readUInt8(1), BATCH_MINE_MODE_DEBUG);
    assert.equal(batchMineIx.data.readUInt8(2), 2);
    assert.equal(batchMineIx.data.readInt32LE(3), 1);
    assert.equal(batchMineIx.data.readInt32LE(15), 2);
    assert.equal(batchMineIx.keys.length, 13);
    assert.equal(batchMineIx.keys[4].pubkey.toBase58(), chunkBroken.toBase58());
    assert.equal(batchMineIx.keys[10].pubkey.toBase58(), backpack.toBase58());
    assert.equal(batchMineIx.keys[11].pubkey.toBase58(), deriveMaterialPhysicsPda({ globalConfig })[0].toBase58());

    const rangeBlocks = Array.from({ length: RANGE_MINE_MAX_BLOCKS }, (_unused, index) => ({
      worldX: index % 16,
      worldY: 20 + Math.floor(index / 128),
      worldZ: Math.floor(index / 16) % 8,
      expectedBlockId: BLOCK_STONE,
    }));
    const rangeMineIx = createRangeMineWithRewardsInstruction({
      payer: sessionAuthority,
      owner,
      sessionAuthority,
      backpack,
      blocks: rangeBlocks,
    });
    assert.equal(rangeMineIx.data.readUInt8(0), 21);
    assert.equal(rangeMineIx.data.readUInt8(1), RANGE_MINE_MODE_DEBUG);
    assert.equal(rangeMineIx.data.readUInt8(12), 16);
    assert.equal(rangeMineIx.data.readUInt16LE(13), 5);
    assert.equal(rangeMineIx.data.readUInt8(15), 8);
    assert.equal(rangeMineIx.data.length, 1 + 15 + 80 + 480);
    assert.equal(rangeMineIx.keys.length, 13);
    assert.equal(rangeMineIx.keys[4].pubkey.toBase58(), chunkBroken.toBase58());
    assert.equal(rangeMineIx.keys[11].pubkey.toBase58(), deriveMaterialPhysicsPda({ globalConfig })[0].toBase58());
    const maxRangeTransaction = new Transaction({
      feePayer: sessionAuthority,
      recentBlockhash: owner.toBase58(),
    }).add(
      ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
      rangeMineIx,
    );
    assert.ok(
      maxRangeTransaction.serialize({ requireAllSignatures: false, verifySignatures: false }).length <= 1232,
      "a 640-block compressed range must fit the Solana packet limit",
    );
  });

  it("builds and decodes foundation instructions and PDA state", () => {
    const foundationId = 42n;
    const foundation = { minX: -1, minZ: 15, surfaceY: 101, width: 16, depth: 2 };
    const instruction = createFoundationInstruction({
      payer: sessionAuthority,
      owner,
      sessionAuthority,
      foundationId,
      foundation,
    });
    const [foundationPda] = deriveFoundationPda({
      globalConfig,
      owner,
      foundationId,
      programId: NICECHUNK_CHUNK_PROGRAM_ID,
    });
    assert.equal(instruction.data.length, 21);
    assert.equal(instruction.data.readUInt8(0), 14);
    assert.equal(instruction.data.readBigUInt64LE(1), foundationId);
    assert.equal(instruction.data.readInt32LE(9), foundation.minX);
    assert.equal(instruction.data.readInt16LE(13), foundation.surfaceY);
    assert.equal(instruction.data.readInt32LE(15), foundation.minZ);
    assert.equal(instruction.data.readUInt8(19), foundation.width);
    assert.equal(instruction.data.readUInt8(20), foundation.depth);
    assert.equal(instruction.keys.length, 10);
    assert.equal(instruction.keys[3].pubkey.toBase58(), foundationPda.toBase58());
    assert.equal(instruction.keys[6].pubkey.toBase58(), deriveFoundationChunkPda({
      globalConfig,
      chunkX: -1,
      chunkZ: 0,
      programId: NICECHUNK_CHUNK_PROGRAM_ID,
    })[0].toBase58());
    assert.equal(instruction.keys[9].pubkey.toBase58(), deriveFoundationChunkPda({
      globalConfig,
      chunkX: 0,
      chunkZ: 1,
      programId: NICECHUNK_CHUNK_PROGRAM_ID,
    })[0].toBase58());

    const stateData = Buffer.alloc(FOUNDATION_LEN);
    stateData.write(FOUNDATION_MAGIC, 0, "utf8");
    stateData.writeUInt8(FOUNDATION_VERSION, 8);
    stateData.writeUInt8(221, 9);
    stateData.writeUInt8(1, 10);
    stateData.writeUInt8(4, 11);
    owner.toBuffer().copy(stateData, 12);
    globalConfig.toBuffer().copy(stateData, 44);
    stateData.writeBigUInt64LE(foundationId, 76);
    stateData.writeInt32LE(foundation.minX, 84);
    stateData.writeInt32LE(foundation.minZ, 88);
    stateData.writeInt16LE(foundation.surfaceY, 92);
    stateData.writeUInt8(foundation.width, 94);
    stateData.writeUInt8(foundation.depth, 95);
    stateData.writeBigUInt64LE(999n, 96);
    const decodedState = decodeFoundationState(stateData);
    assert.equal(decodedState.owner.toBase58(), owner.toBase58());
    assert.equal(decodedState.globalConfig.toBase58(), globalConfig.toBase58());
    assert.equal(decodedState.foundationId, foundationId);
    assert.equal(decodedState.createdSlot, 999n);

    const chunkData = Buffer.alloc(FOUNDATION_CHUNK_LEN);
    chunkData.write(FOUNDATION_CHUNK_MAGIC, 0, "utf8");
    chunkData.writeUInt8(FOUNDATION_CHUNK_VERSION, 8);
    chunkData.writeUInt8(17, 9);
    chunkData.writeUInt16LE(1, 10);
    globalConfig.toBuffer().copy(chunkData, 12);
    chunkData.writeInt32LE(-1, 44);
    chunkData.writeInt32LE(0, 48);
    const recordOffset = FOUNDATION_CHUNK_HEADER_LEN;
    owner.toBuffer().copy(chunkData, recordOffset);
    chunkData.writeBigUInt64LE(foundationId, recordOffset + 32);
    chunkData.writeInt32LE(foundation.minX, recordOffset + 40);
    chunkData.writeInt32LE(foundation.minZ, recordOffset + 44);
    chunkData.writeInt16LE(foundation.surfaceY, recordOffset + 48);
    chunkData.writeUInt8(foundation.width, recordOffset + 50);
    chunkData.writeUInt8(foundation.depth, recordOffset + 51);
    assert.equal(chunkData.length, FOUNDATION_CHUNK_HEADER_LEN + 32 * FOUNDATION_CHUNK_RECORD_LEN);
    const decodedChunk = decodeFoundationChunkState(chunkData, { globalConfig, chunkX: -1, chunkZ: 0 });
    assert.equal(decodedChunk.count, 1);
    assert.equal(decodedChunk.records[0].foundationId, foundationId);
    assert.equal(decodedChunk.records[0].minX, -1);
  });

  it("builds resource drop table and civilization adapter instructions", () => {
    const [resourceDropTable] = deriveResourceDropTablePda({
      globalConfig,
      programId: NICECHUNK_CHUNK_PROGRAM_ID,
    });
    const init = createInitializeResourceDropTableInstruction({
      payer: owner,
      rules: resourceDropRules,
    });
    assert.equal(init.programId.toBase58(), NICECHUNK_CHUNK_PROGRAM_ID.toBase58());
    assert.equal(init.data.readUInt8(0), 7);
    assert.equal(init.data.readUInt8(1), resourceDropRules.length);
    assert.equal(init.data.length, 2 + resourceDropRules.length * RESOURCE_DROP_RULE_LEN);
    assert.equal(init.keys[0].pubkey.toBase58(), owner.toBase58());
    assert.equal(init.keys[1].pubkey.toBase58(), resourceDropTable.toBase58());
    assert.equal(init.keys[2].pubkey.toBase58(), globalConfig.toBase58());
    assert.equal(init.keys[3].pubkey.toBase58(), SystemProgram.programId.toBase58());

    const ruleBook = new PublicKey("8FiAnDzZ6zHPNDMW7pd77FjTzWnCPH8S6pB1LsayZegF");
    const tally = new PublicKey("GWqbDeSLQeTUzc5UpVaRT1KpAx7URoL3Q3f2Vm1XqgCd");
    const receipt = new PublicKey("9NZ1HCiRkwHgtwc9B5U7RrdTsh19LDGK1B4GosR8Dip7");
    const [adapterAuthority] = deriveCivilizationAdapterAuthorityPda({
      ruleBook,
      targetProgram: NICECHUNK_CHUNK_PROGRAM_ID,
    });
    const patchRules = resourceDropRules.map((rule, index) => ({
      ...rule,
      chanceBps: index === 0 ? 36 : rule.chanceBps,
    }));
    const patchBytes = encodeCivilizationResourceDropRulesPatch(patchRules);
    const apply = createApplyCivilizationResourceDropRulesInstruction({
      executor: owner,
      resourceDropTable,
      globalConfig,
      ruleBook,
      tally,
      receipt,
      rules: patchRules,
    });
    assert.equal(resourceDropRules.length, 37);
    assert.equal(patchBytes.length, 1 + resourceDropRules.length * RESOURCE_DROP_RULE_LEN);
    assert.equal(apply.data.readUInt8(0), 10);
    assert.equal(apply.data.length, 1 + patchBytes.length);
    assert.equal(apply.data.subarray(1).toString("hex"), patchBytes.toString("hex"));
    assert.equal(apply.keys[0].pubkey.toBase58(), owner.toBase58());
    assert.equal(apply.keys[1].pubkey.toBase58(), resourceDropTable.toBase58());
    assert.equal(apply.keys[2].pubkey.toBase58(), globalConfig.toBase58());
    assert.equal(apply.keys[3].pubkey.toBase58(), ruleBook.toBase58());
    assert.equal(apply.keys[4].pubkey.toBase58(), tally.toBase58());
    assert.equal(apply.keys[5].pubkey.toBase58(), receipt.toBase58());
    assert.equal(apply.keys[6].pubkey.toBase58(), SystemProgram.programId.toBase58());
    assert.equal(apply.keys[7].pubkey.toBase58(), NICECHUNK_CIVILIZATION_PROGRAM_ID.toBase58());
    assert.equal(apply.keys[8].pubkey.toBase58(), adapterAuthority.toBase58());
    assert.equal(apply.keys[8].isSigner, false);
    assert.equal(apply.keys[8].isWritable, false);
  });

  it("encodes, decodes, resolves, and builds surface decoration PDA instructions", () => {
    const rules = DEFAULT_SURFACE_DECORATION_RULES;
    const [surfaceDecorationTable] = deriveSurfaceDecorationTablePda({
      globalConfig,
      programId: NICECHUNK_CHUNK_PROGRAM_ID,
    });
    const payload = encodeSurfaceDecorationRules(rules);
    assert.equal(payload.length, 1 + rules.length * SURFACE_DECORATION_RULE_LEN);

    const account = Buffer.alloc(SURFACE_DECORATION_TABLE_LEN);
    account.write(SURFACE_DECORATION_TABLE_MAGIC, 0, "utf8");
    account.writeUInt8(SURFACE_DECORATION_TABLE_VERSION, 8);
    account.writeUInt8(254, 9);
    account.writeUInt8(rules.length, 10);
    account.writeUInt32LE(7, 12);
    payload.copy(account, SURFACE_DECORATION_TABLE_HEADER_LEN, 1);
    const decoded = decodeSurfaceDecorationTable(account);
    assert.equal(decoded.revision, 7);
    assert.deepEqual(decoded.rules, rules);

    const config = {
      worldSeed: Buffer.from("6e6963656368756e6b2d6d61696e6e65742d3030310000000000000000000000", "hex"),
      chunkSize: 16,
      minBuildY: -32,
      maxBuildY: 320,
      maxTerrainHeight: 240,
      seaLevel: 96,
    };
    const match = resolveSurfaceDecorationAt(config, decoded.rules, -131, -131);
    assert.equal(match?.surfaceBlockId, 5);
    assert.equal(match?.decorationId, 103);
    assert.equal(match?.ruleId, 21);
    assert.equal(match?.roll, 38);
    const cactusDecoration = resolveSurfaceDecorationAt(config, decoded.rules, 826, -1997);
    assert.equal(cactusDecoration?.decorationId, 8);
    assert.equal(cactusDecoration?.dropBlockId, 32);
    assert.equal(cactusDecoration?.ruleId, 20);
    assert.equal(cactusDecoration?.roll, 2);
    assert.equal(resolveSurfaceDecorationAt(config, decoded.rules, 799, -999), null, "tree-occupied faces cannot also contain PDA decorations");

    const init = createInitializeSurfaceDecorationTableInstruction({
      payer: owner,
      rules,
    });
    assert.equal(init.data.readUInt8(0), 11);
    assert.equal(init.data.readUInt8(1), rules.length);
    assert.equal(init.keys[1].pubkey.toBase58(), surfaceDecorationTable.toBase58());

    const ruleBook = new PublicKey("8FiAnDzZ6zHPNDMW7pd77FjTzWnCPH8S6pB1LsayZegF");
    const tally = new PublicKey("GWqbDeSLQeTUzc5UpVaRT1KpAx7URoL3Q3f2Vm1XqgCd");
    const receipt = new PublicKey("9NZ1HCiRkwHgtwc9B5U7RrdTsh19LDGK1B4GosR8Dip7");
    const apply = createApplyCivilizationSurfaceDecorationRulesInstruction({
      executor: owner,
      surfaceDecorationTable,
      globalConfig,
      ruleBook,
      tally,
      receipt,
      rules,
    });
    assert.equal(apply.data.readUInt8(0), 13);
    assert.equal(apply.data.subarray(1).toString("hex"), payload.toString("hex"));
    assert.equal(apply.keys[8].pubkey.toBase58(), deriveCivilizationAdapterAuthorityPda({
      ruleBook,
      targetProgram: NICECHUNK_CHUNK_PROGRAM_ID,
    })[0].toBase58());
  });

  it("keeps target adapters pinned to the canonical civilization program", () => {
    const chunkLib = readFileSync("programs/nicechunk_chunk/src/lib.rs", "utf8");
    const chunkConfig = readFileSync("programs/nicechunk_chunk/src/cluster_config.rs", "utf8");
    const smeltingLib = readFileSync("programs/nicechunk_smelting/src/lib.rs", "utf8");
    const smeltingConfig = readFileSync("programs/nicechunk_smelting/src/cluster_config.rs", "utf8");

    for (const source of [chunkConfig, smeltingConfig]) {
      assert.match(source, /NICECHUNK_CIVILIZATION_PROGRAM_ID/);
      assert.match(source, /3MRG4UjxTK1rMq7TGM4bX1GrD8C36bQtt1RdTmJD7Jah/);
    }
    assert.match(chunkLib, /civilization_program\.key,\s*&NICECHUNK_CIVILIZATION_PROGRAM_ID/s);
    assert.match(smeltingLib, /civilization_program\.key,\s*&NICECHUNK_CIVILIZATION_PROGRAM_ID/s);
  });

  it("builds smelting recipe table and execution instructions", () => {
    const tableId = 2n;
    const recipeId = 1001n;
    const [recipeTable] = deriveRecipeTablePda({ tableId });
    const [smeltingAuthority] = deriveSmeltingAuthorityPda();
    const materialPda = new PublicKey("CEzcpJe9UTq5FmVzpTfgPffMbqdG97YJeFMJYwUSFhNF");
    const inputSlot = {
      kind: 1,
      category: 0,
      flags: 0,
      quantity: 1,
      resource: { worldX: 1, worldY: 0, worldZ: 2 },
      itemCode: 0,
      itemId: 0n,
      itemPda: PublicKey.default,
    };
    const outputSlot = {
      kind: BACKPACK_SLOT_KIND_ITEM,
      category: BACKPACK_ITEM_CATEGORY_MATERIAL,
      flags: 0,
      quantity: 1,
      resource: { worldX: 0, worldY: 0, worldZ: 0 },
      itemCode: 101,
      itemId: 501n,
      itemPda: materialPda,
    };

    const init = createInitializeRecipeTableInstruction({ payer: owner, tableId });
    assert.equal(init.programId.toBase58(), NICECHUNK_SMELTING_PROGRAM_ID.toBase58());
    assert.equal(init.data.readUInt8(0), 3);
    assert.equal(init.data.readUInt8(1), 0);
    assert.equal(init.data.readBigUInt64LE(2), tableId);
    assert.equal(init.keys[1].pubkey.toBase58(), recipeTable.toBase58());

    const upsert = createUpsertSmeltingRecipeInstruction({
      authority: owner,
      recipeTable,
      recipe: { recipeId, inputs: [inputSlot], outputs: [outputSlot], minHeatTier: 2, yieldBps: 6200 },
    });
    assert.equal(upsert.data.readUInt8(0), 3);
    assert.equal(upsert.data.readUInt8(1), 1);
    assert.equal(upsert.data.length, 2 + UPSERT_RECIPE_ARGS_LEN);
    assert.equal(upsert.data.readBigUInt64LE(2), recipeId);
    assert.equal(upsert.data.readUInt8(11), 2);
    assert.equal(upsert.data.readUInt16LE(14), 6200);

    const ruleBook = new PublicKey("8FiAnDzZ6zHPNDMW7pd77FjTzWnCPH8S6pB1LsayZegF");
    const tally = new PublicKey("GWqbDeSLQeTUzc5UpVaRT1KpAx7URoL3Q3f2Vm1XqgCd");
    const receipt = new PublicKey("9NZ1HCiRkwHgtwc9B5U7RrdTsh19LDGK1B4GosR8Dip7");
    const [adapterAuthority] = deriveCivilizationAdapterAuthorityPda({
      ruleBook,
      targetProgram: NICECHUNK_SMELTING_PROGRAM_ID,
    });
    const recipe = { recipeId, inputs: [inputSlot], outputs: [outputSlot], minHeatTier: 2, yieldBps: 6200 };
    const civilizationApply = createApplyCivilizationSmeltingRecipeInstruction({
      executor: owner,
      recipeTable,
      ruleBook,
      tally,
      receipt,
      recipe,
    });
    const patchBytes = encodeCivilizationSmeltingRecipePatch(recipe);
    assert.equal(civilizationApply.data.readUInt8(0), 3);
    assert.equal(civilizationApply.data.readUInt8(1), 4);
    assert.equal(patchBytes.length, 16 + 2 * BACKPACK_SLOT_RECORD_LEN);
    assert.equal(civilizationApply.data.length, 2 + patchBytes.length);
    assert.equal(civilizationApply.data.subarray(2).toString("hex"), patchBytes.toString("hex"));
    assert.equal(civilizationApply.keys[0].pubkey.toBase58(), owner.toBase58());
    assert.equal(civilizationApply.keys[1].pubkey.toBase58(), recipeTable.toBase58());
    assert.equal(civilizationApply.keys[2].pubkey.toBase58(), NICECHUNK_CIVILIZATION_PROGRAM_ID.toBase58());
    assert.equal(civilizationApply.keys[3].pubkey.toBase58(), ruleBook.toBase58());
    assert.equal(civilizationApply.keys[4].pubkey.toBase58(), tally.toBase58());
    assert.equal(civilizationApply.keys[5].pubkey.toBase58(), receipt.toBase58());
    assert.equal(civilizationApply.keys[6].pubkey.toBase58(), SystemProgram.programId.toBase58());
    assert.equal(civilizationApply.keys[7].pubkey.toBase58(), adapterAuthority.toBase58());
    assert.equal(civilizationApply.keys[7].isSigner, false);
    assert.equal(civilizationApply.keys[7].isWritable, false);

    const backpack = new PublicKey("6pCaR8qLHvGeU3BAzwzAHMPjDk1ewNtrbAcqAGeMSH2Q");
    const [playerProgress] = derivePlayerProgressPda({
      globalConfig,
      owner,
      programId: NICECHUNK_SMELTING_PROGRAM_ID,
    });
    const execute = createExecuteSmeltingInstruction({
      owner,
      recipeTable,
      backpack,
      recipeId,
      inputIndexes: [0],
      fuelIndexes: [1],
    });
    assert.equal(execute.data.readUInt8(0), 3);
    assert.equal(execute.data.readUInt8(1), 2);
    assert.equal(execute.data.readUInt8(11), 1);
    assert.equal(execute.keys[0].pubkey.toBase58(), owner.toBase58());
    assert.equal(execute.keys[1].pubkey.toBase58(), recipeTable.toBase58());
    assert.equal(execute.keys[2].pubkey.toBase58(), backpack.toBase58());
    assert.equal(execute.keys[3].pubkey.toBase58(), playerProgress.toBase58());
    assert.equal(execute.keys[4].pubkey.toBase58(), globalConfig.toBase58());
    assert.equal(execute.keys[5].pubkey.toBase58(), deriveMaterialPhysicsPda({ globalConfig })[0].toBase58());
    assert.equal(execute.keys[6].pubkey.toBase58(), smeltingAuthority.toBase58());
    assert.equal(execute.keys[7].pubkey.toBase58(), NICECHUNK_BACKPACK_PROGRAM_ID.toBase58());
    assert.equal(execute.keys[8].pubkey.toBase58(), SystemProgram.programId.toBase58());
  });

  it("decodes compact chunk-broken state", () => {
    const capacity = 2;
    const data = Buffer.alloc(CHUNK_BROKEN_HEADER_LEN + capacity * CHUNK_BROKEN_RECORD_LEN);
    data.write(CHUNK_BROKEN_MAGIC, 0, "utf8");
    data.writeUInt8(1, 4);
    data.writeUInt8(252, 5);
    data.writeUInt16LE(1, 6);
    data.writeUInt16LE(capacity, 8);
    data.writeInt16LE(-32, 10);
    data.writeUIntLE(0x017f, CHUNK_BROKEN_HEADER_LEN, CHUNK_BROKEN_RECORD_LEN);

    const decoded = decodeChunkBrokenState({ data, chunkX: -1, chunkZ: 2, chunkSize: 16 });
    assert.equal(decoded.count, 1);
    assert.equal(decoded.capacity, 2);
    assert.equal(decoded.brokenBlocks[0].x, -1);
    assert.equal(decoded.brokenBlocks[0].y, -31);
    assert.equal(decoded.brokenBlocks[0].z, 39);
    assert.equal(decoded.brokenBlocks[0].packed, "7f0100");
  });

  it("computes canonical block ids in SDK", () => {
    const config = {
      worldSeed: Buffer.from("6e6963656368756e6b2d6d61696e6e65742d3030310000000000000000000000", "hex"),
      chunkSize: 16,
      minBuildY: -32,
      maxBuildY: 320,
      maxTerrainHeight: 240,
      seaLevel: 96,
    };
    assert.equal(generatedBlockIdAt(config, { chunkX: 0, chunkZ: 0, localX: 0, y: 85, localZ: 0 }), 5);
    assert.equal(generatedBlockIdAt(config, { chunkX: 0, chunkZ: 0, localX: 0, y: 96, localZ: 0 }), 17);
    assert.equal(generatedBlockIdAt(config, { chunkX: 16, chunkZ: 56, localX: 0, y: 114, localZ: 0 }), 1);
    assert.equal(generatedBlockIdAt(config, { chunkX: 38, chunkZ: 62, localX: 9, y: 115, localZ: 10 }), 0);
    assert.equal(generatedBlockIdAt(config, { chunkX: 39, chunkZ: 62, localX: 14, y: 116, localZ: 11 }), 0);
  });
});
