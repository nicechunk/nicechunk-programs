import assert from "node:assert";
import { PublicKey } from "@solana/web3.js";
import {
  BACKPACK_FLAG_TOTAL_MASS_INITIALIZED,
  BACKPACK_ITEM_CATEGORY_MATERIAL,
  BACKPACK_ITEM_FLAG_MASS_VALID,
  BACKPACK_LEN,
  BACKPACK_SLOT_KIND_ITEM,
  MATERIAL_PHYSICS_HEADER_LEN,
  MATERIAL_PHYSICS_LEN,
  createInitializeMaterialPhysicsInstruction,
  createMigrateBackpackMassInstruction,
  createReplaceMaterialPhysicsInstruction,
  decodeBackpack,
  decodeBackpackSlotRecord,
  decodeMaterialPhysics,
  deriveMaterialPhysicsPda,
  encodeBackpackSlotRecord,
} from "../sdk/nicechunk-backpack.ts";

describe("authoritative backpack mass SDK", () => {
  const treasury = new PublicKey("CtPV2vmqNNwUSfMu5nz58ZtMPy6ZvxL4LyNdPHVW7WvF");
  const globalConfig = new PublicKey("3sPZxJpbcYSPus6tzSD2BHWXErLUngPhMQWiQXvowj9S");

  it("encodes material physics administration and migration instructions", () => {
    const [materialPhysics] = deriveMaterialPhysicsPda({ globalConfig });
    const initialize = createInitializeMaterialPhysicsInstruction({ authority: treasury, globalConfig });
    assert.equal(initialize.data.readUInt8(0), 1);
    assert.equal(initialize.data.readUInt8(1), 12);
    assert.ok(initialize.keys[1].pubkey.equals(materialPhysics));

    const replace = createReplaceMaterialPhysicsInstruction({
      authority: treasury,
      globalConfig,
      records: [
        { materialId: 1008, densityKgM3: 2_700 },
        { materialId: 1, densityKgM3: 1_000 },
      ],
    });
    assert.equal(replace.data.readUInt8(0), 1);
    assert.equal(replace.data.readUInt8(1), 13);
    assert.equal(replace.data.readUInt8(2), 2);
    assert.equal(replace.data.readUInt16LE(3), 1);
    assert.equal(replace.data.readUInt16LE(7), 1008);

    const backpack = PublicKey.unique();
    const migrate = createMigrateBackpackMassInstruction({
      owner: treasury,
      backpack,
      globalConfig,
    });
    assert.equal(migrate.data.readUInt8(0), 1);
    assert.equal(migrate.data.readUInt8(1), 14);
    assert.ok(migrate.keys[2].pubkey.equals(materialPhysics));
  });

  it("decodes material physics records and rejects unsorted data", () => {
    const data = Buffer.alloc(MATERIAL_PHYSICS_LEN);
    data.write("NCKPHY01", 0, "utf8");
    data.writeUInt16LE(1, 8);
    data.writeUInt8(250, 10);
    data.writeUInt8(1, 11);
    treasury.toBuffer().copy(data, 12);
    globalConfig.toBuffer().copy(data, 44);
    data.writeUInt32LE(3, 76);
    data.writeUInt8(2, 80);
    data.writeBigUInt64LE(10n, 84);
    data.writeBigUInt64LE(11n, 92);
    data.writeBigInt64LE(12n, 100);
    data.writeUInt16LE(1, MATERIAL_PHYSICS_HEADER_LEN);
    data.writeUInt16LE(1_000, MATERIAL_PHYSICS_HEADER_LEN + 2);
    data.writeUInt16LE(1008, MATERIAL_PHYSICS_HEADER_LEN + 4);
    data.writeUInt16LE(2_700, MATERIAL_PHYSICS_HEADER_LEN + 6);

    const decoded = decodeMaterialPhysics(data);
    assert.equal(decoded.revision, 3);
    assert.deepEqual(decoded.records, [
      { materialId: 1, densityKgM3: 1_000 },
      { materialId: 1008, densityKgM3: 2_700 },
    ]);

    data.writeUInt16LE(1, MATERIAL_PHYSICS_HEADER_LEN + 4);
    assert.throws(() => decodeMaterialPhysics(data), /Invalid MaterialPhysics record/);
  });

  it("round-trips unsigned item mass without changing durability", () => {
    const encoded = encodeBackpackSlotRecord({
      kind: BACKPACK_SLOT_KIND_ITEM,
      category: BACKPACK_ITEM_CATEGORY_MATERIAL,
      flags: 0,
      quantity: 1,
      resource: { worldX: 0, worldY: 0, worldZ: 0 },
      itemCode: 1008,
      itemId: 77n,
      itemPda: PublicKey.unique(),
      volumeMm3: 600_000,
      durabilityCurrent: 900,
      durabilityMax: 1_200,
      grade: 4,
      itemLevel: 17,
      qualityBps: 7_200,
      metadata: 42,
      massGrams: 4_000_000_000,
    });
    const decoded = decodeBackpackSlotRecord(encoded);
    assert.equal(decoded.flags & BACKPACK_ITEM_FLAG_MASS_VALID, BACKPACK_ITEM_FLAG_MASS_VALID);
    assert.equal(decoded.massGrams, 4_000_000_000);
    assert.equal(decoded.durabilityCurrent, 900);
  });

  it("decodes total mass and mining snapshot fields from the v3 header", () => {
    const data = Buffer.alloc(BACKPACK_LEN);
    data.write("NCKBPK01", 0, "utf8");
    data.writeUInt16LE(3, 8);
    data.writeUInt8(1, 11);
    treasury.toBuffer().copy(data, 20);
    data.writeUInt8(50, 52);
    data.writeUInt8(1, 54);
    data.writeUInt8(BACKPACK_FLAG_TOTAL_MASS_INITIALIZED, 55);
    data.writeBigUInt64LE(25_000n, 90);
    data.writeBigUInt64LE(24_000n, 98);
    data.writeBigUInt64LE(7n, 106);
    data.writeBigUInt64LE(3n, 114);

    const decoded = decodeBackpack(data);
    assert.equal(decoded.massInitialized, true);
    assert.equal(decoded.totalMassGrams, 25_000n);
    assert.equal(decoded.lastMinePreMassGrams, 24_000n);
    assert.equal(decoded.lastMineActionId, 7n);
    assert.equal(decoded.mineSequence, 3n);
  });
});
