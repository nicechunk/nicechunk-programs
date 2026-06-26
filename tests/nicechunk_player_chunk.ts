import { PublicKey, SystemProgram } from "@solana/web3.js";
import assert from "node:assert";
import {
  createOrRefreshPlayerSessionInstruction,
  createInitializePlayerInstruction,
  createSetEquippedBackpackInstruction,
  createSetEquipmentSlotInstruction,
  createUpdatePlayerPositionInstruction,
  decodePlayerProfile,
  derivePlayerSessionPda,
  derivePlayerProfilePda,
  EQUIPMENT_SLOT_COUNT,
  NICECHUNK_PLAYER_PROGRAM_ID,
  SESSION_ACTION_BREAK_BLOCK,
  SESSION_ACTION_PLACE_BLOCK,
  PLAYER_PROFILE_LEN,
} from "../sdk/nicechunk-player.ts";
import {
  BACKPACK_ITEM_CATEGORY_MATERIAL,
  BACKPACK_LEN,
  BACKPACK_SLOT_KIND_ITEM,
  BACKPACK_SLOT_RECORD_LEN,
  createInitializeBackpackInstruction,
  decodeBackpack,
  encodeBackpackSlotRecord,
  deriveBackpackPda,
  NICECHUNK_BACKPACK_PROGRAM_ID,
} from "../sdk/nicechunk-backpack.ts";
import {
  createExecuteSmeltingInstruction,
  createInitializeRecipeTableInstruction,
  createUpsertSmeltingRecipeInstruction,
  deriveRecipeTablePda,
  deriveSmeltingAuthorityPda,
  NICECHUNK_SMELTING_PROGRAM_ID,
  UPSERT_RECIPE_ARGS_LEN,
} from "../sdk/nicechunk-smelting.ts";
import {
  BLOCK_STONE,
  CHUNK_BROKEN_HEADER_LEN,
  CHUNK_BROKEN_MAGIC,
  CHUNK_BROKEN_RECORD_LEN,
  createMineBlockInstruction,
  decodeChunkBrokenState,
  deriveChunkBrokenPda,
  generatedBlockIdAt,
  NICECHUNK_CHUNK_PROGRAM_ID,
} from "../sdk/nicechunk-chunk.ts";
import {
  deriveGlobalConfigPda,
  NICECHUNK_CORE_PROGRAM_ID,
} from "../sdk/nicechunk-core.ts";

describe("nicechunk player and mining SDK", () => {
  const owner = new PublicKey("9XuoVVwqP2jipt3jpJVXCSS2N2jr9vDuV3d6K73FKVud");
  const sessionAuthority = new PublicKey("Z2WsAfHEgNiycsaKoSo83TzVzGnc6nLB1CkEKN9vymw");
  const [globalConfig] = deriveGlobalConfigPda(NICECHUNK_CORE_PROGRAM_ID);

  it("derives deterministic player and chunk-broken PDAs", () => {
    const [playerProfile] = derivePlayerProfilePda(owner, NICECHUNK_PLAYER_PROGRAM_ID);
    const [chunkBroken] = deriveChunkBrokenPda({
      globalConfig,
      chunkX: 0,
      chunkZ: 0,
      programId: NICECHUNK_CHUNK_PROGRAM_ID,
    });

    assert.equal(playerProfile.toBase58(), "3erZxS9JsMM8evKF84E3qPxZA6gWVTmGkAGTtRxzqHic");
    assert.equal(chunkBroken.toBase58(), "Fi5YQNm6JqC1dWPKzqQCJLYTq1B7ucz8K7f6pDhwfcBy");
  });

  it("builds initialize player account order", () => {
    const [playerProfile] = derivePlayerProfilePda(owner, NICECHUNK_PLAYER_PROGRAM_ID);
    const ix = createInitializePlayerInstruction({ payer: owner });

    assert.equal(ix.programId.toBase58(), NICECHUNK_PLAYER_PROGRAM_ID.toBase58());
    assert.deepEqual(ix.data, Buffer.from([0]));
    assert.equal(ix.keys[0].pubkey.toBase58(), owner.toBase58());
    assert.equal(ix.keys[0].isSigner, true);
    assert.equal(ix.keys[0].isWritable, true);
    assert.equal(ix.keys[1].pubkey.toBase58(), playerProfile.toBase58());
    assert.equal(ix.keys[1].isWritable, true);
    assert.equal(ix.keys[2].pubkey.toBase58(), globalConfig.toBase58());
    assert.equal(ix.keys[3].pubkey.toBase58(), SystemProgram.programId.toBase58());
  });

  it("builds initialize backpack with player profile guard", () => {
    const backpackId = 1n;
    const [playerProfile] = derivePlayerProfilePda(owner, NICECHUNK_PLAYER_PROGRAM_ID);
    const [backpack] = deriveBackpackPda({ creator: owner, backpackId });
    const ix = createInitializeBackpackInstruction({ payer: owner, backpackId });

    assert.equal(ix.programId.toBase58(), NICECHUNK_BACKPACK_PROGRAM_ID.toBase58());
    assert.equal(ix.data.readUInt8(0), 0);
    assert.equal(ix.data.readBigUInt64LE(1), backpackId);
    assert.equal(ix.keys[0].pubkey.toBase58(), owner.toBase58());
    assert.equal(ix.keys[0].isSigner, true);
    assert.equal(ix.keys[1].pubkey.toBase58(), playerProfile.toBase58());
    assert.equal(ix.keys[1].isWritable, false);
    assert.equal(ix.keys[2].pubkey.toBase58(), backpack.toBase58());
    assert.equal(ix.keys[2].isWritable, true);
    assert.equal(ix.keys[3].pubkey.toBase58(), SystemProgram.programId.toBase58());
  });

  it("decodes v2 backpack item reference slots", () => {
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
    };
    const data = Buffer.alloc(BACKPACK_LEN);
    data.write("NCKBPK01", 0, "utf8");
    data.writeUInt16LE(2, 8);
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
    assert.equal(decoded.version, 2);
    assert.equal(decoded.records.length, 0);
    assert.equal(decoded.slots.length, 1);
    assert.equal(decoded.slots[0].kind, BACKPACK_SLOT_KIND_ITEM);
    assert.equal(decoded.slots[0].quantity, 2);
    assert.equal(decoded.slots[0].itemPda.toBase58(), itemPda.toBase58());
  });

  it("builds update position and equipment instructions", () => {
    const updatePosition = createUpdatePlayerPositionInstruction({ authority: owner, x: 16, y: 2, z: -16 });
    assert.equal(updatePosition.data.readUInt8(0), 1);
    assert.equal(updatePosition.data.readInt32LE(1), 16);
    assert.equal(updatePosition.data.readInt32LE(5), 2);
    assert.equal(updatePosition.data.readInt32LE(9), -16);

    const item = PublicKey.default;
    const setEquipment = createSetEquipmentSlotInstruction({ authority: owner, slot: 8, item });
    assert.equal(setEquipment.data.readUInt8(0), 2);
    assert.equal(setEquipment.data.readUInt8(1), 8);
    assert.equal(new PublicKey(setEquipment.data.subarray(2, 34)).toBase58(), item.toBase58());

    const backpack = new PublicKey("CEzcpJe9UTq5FmVzpTfgPffMbqdG97YJeFMJYwUSFhNF");
    const bindBackpack = createSetEquippedBackpackInstruction({ authority: owner, backpack });
    assert.equal(bindBackpack.data.readUInt8(0), 5);
    assert.equal(bindBackpack.keys[0].pubkey.toBase58(), owner.toBase58());
    assert.equal(bindBackpack.keys[0].isSigner, true);
    assert.equal(bindBackpack.keys[0].isWritable, true);
    assert.equal(bindBackpack.keys[1].isWritable, true);
    assert.equal(bindBackpack.keys[2].pubkey.toBase58(), backpack.toBase58());
    assert.equal(bindBackpack.keys[3].pubkey.toBase58(), SystemProgram.programId.toBase58());
  });

  it("decodes a player profile buffer", () => {
    const [playerProfile, bump] = derivePlayerProfilePda(owner, NICECHUNK_PLAYER_PROGRAM_ID);
    assert.ok(playerProfile);
    const data = Buffer.alloc(PLAYER_PROFILE_LEN);
    let offset = 0;
    data.write("NCKPLY01", offset, "utf8"); offset += 8;
    data.writeUInt16LE(1, offset); offset += 2;
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

    const decoded = decodePlayerProfile(data);
    assert.equal(offset, PLAYER_PROFILE_LEN);
    assert.equal(decoded.owner.toBase58(), owner.toBase58());
    assert.equal(decoded.position.y, 2);
    assert.equal(decoded.equipment.length, EQUIPMENT_SLOT_COUNT);
    assert.equal(decoded.equippedBackpack.toBase58(), equippedBackpack.toBase58());
  });

  it("builds player session and canonical mine instructions", () => {
    const [playerProfile] = derivePlayerProfilePda(owner, NICECHUNK_PLAYER_PROGRAM_ID);
    const [playerSession] = derivePlayerSessionPda({ owner, sessionAuthority, programId: NICECHUNK_PLAYER_PROGRAM_ID });
    const [chunkBroken] = deriveChunkBrokenPda({ globalConfig, chunkX: 0, chunkZ: 0, programId: NICECHUNK_CHUNK_PROGRAM_ID });
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
    assert.equal(mineIx.keys[4].pubkey.toBase58(), globalConfig.toBase58());
    assert.equal(mineIx.keys[5].pubkey.toBase58(), SystemProgram.programId.toBase58());
  });

  it("builds smelting recipe table and execution instructions", () => {
    const tableId = 1n;
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
    assert.equal(init.data.readUInt8(0), 0);
    assert.equal(init.data.readBigUInt64LE(1), tableId);
    assert.equal(init.keys[1].pubkey.toBase58(), recipeTable.toBase58());

    const upsert = createUpsertSmeltingRecipeInstruction({
      authority: owner,
      recipeTable,
      recipe: { recipeId, inputs: [inputSlot], outputs: [outputSlot], minHeatTier: 2 },
    });
    assert.equal(upsert.data.readUInt8(0), 1);
    assert.equal(upsert.data.length, 1 + UPSERT_RECIPE_ARGS_LEN);
    assert.equal(upsert.data.readBigUInt64LE(1), recipeId);
    assert.equal(upsert.data.readUInt8(10), 2);

    const backpack = new PublicKey("6pCaR8qLHvGeU3BAzwzAHMPjDk1ewNtrbAcqAGeMSH2Q");
    const execute = createExecuteSmeltingInstruction({
      owner,
      recipeTable,
      backpack,
      recipeId,
      inputIndexes: [0],
      fuelIndexes: [1],
    });
    assert.equal(execute.data.readUInt8(0), 2);
    assert.equal(execute.data.readUInt8(10), 1);
    assert.equal(execute.keys[0].pubkey.toBase58(), owner.toBase58());
    assert.equal(execute.keys[1].pubkey.toBase58(), recipeTable.toBase58());
    assert.equal(execute.keys[2].pubkey.toBase58(), backpack.toBase58());
    assert.equal(execute.keys[3].pubkey.toBase58(), smeltingAuthority.toBase58());
    assert.equal(execute.keys[4].pubkey.toBase58(), NICECHUNK_BACKPACK_PROGRAM_ID.toBase58());
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
      worldSeed: Buffer.alloc(32, 7),
      chunkSize: 16,
      minBuildY: -32,
      maxBuildY: 256,
      maxTerrainHeight: 160,
      seaLevel: 2,
    };
    assert.equal(generatedBlockIdAt(config, { chunkX: 0, chunkZ: 0, localX: 1, y: 0, localZ: 2 }), 17);
    assert.equal(generatedBlockIdAt(config, { chunkX: 0, chunkZ: 0, localX: 1, y: -31, localZ: 2 }), 4);
  });
});
