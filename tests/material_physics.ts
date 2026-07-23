import { Keypair, SystemProgram, Transaction } from "@solana/web3.js";
import assert from "node:assert/strict";
import fs from "node:fs";
import { describe, it } from "mocha";

import {
  BACKPACK_FLAG_MASS_STATE_VALID,
  BACKPACK_ITEM_CATEGORY_MATERIAL,
  BACKPACK_ITEM_FLAG_MASS_VALID,
  BACKPACK_LEN,
  BACKPACK_SLOT_KIND_BLOCK,
  BACKPACK_SLOT_KIND_ITEM,
  BACKPACK_VERSION,
  MATERIAL_PHYSICS_HEADER_LEN,
  MATERIAL_PHYSICS_ITEM_KEY_MASK,
  MATERIAL_PHYSICS_LEN,
  MATERIAL_PHYSICS_MAGIC,
  MATERIAL_PHYSICS_RULE_LEN,
  MATERIAL_PHYSICS_VERSION,
  createAppendSmeltingItemInstruction,
  createConfigureMaterialPhysicsInstruction,
  decodeBackpack,
  decodeMaterialPhysicsTable,
  deriveMaterialPhysicsPda,
  materialPhysicsMassGrams,
  type MaterialPhysicsRule,
} from "../sdk/nicechunk-backpack.ts";
import { deriveGlobalConfigPda } from "../sdk/nicechunk-core.ts";

const document = JSON.parse(fs.readFileSync(
  new URL("../config/material_physics_v2.json", import.meta.url),
  "utf8",
)) as { revision: number; ruleCount: number; rules: MaterialPhysicsRule[] };

describe("MaterialPhysics PDA", () => {
  it("contains the complete canonical natural and manufactured rule set", () => {
    assert.equal(document.ruleCount, 111);
    assert.equal(document.rules.length, 111);
    assert.equal(new Set(document.rules.map(ruleKey)).size, document.rules.length);
    assert.deepEqual(findRule("block", 3), {
      kind: "block",
      id: 3,
      name: "stone",
      densityKgM3: 2600,
      standardVolumeMm3: 1_000_000,
    });
    assert.deepEqual(findRule("block", 23), {
      kind: "block",
      id: 23,
      name: "leaves",
      densityKgM3: 250,
      standardVolumeMm3: 1_000_000,
    });
    assert.equal(findRule("block", 19).name, "toxicWater");
    assert.equal(findRule("block", 20).name, "lava");
    assert.deepEqual(findRule("item", 1010), {
      kind: "item",
      id: 1010,
      name: "glass_ingot",
      densityKgM3: 2500,
      standardVolumeMm3: 250_000,
    });
  });

  it("uses the same rounded integer mass formula as the on-chain program", () => {
    assert.equal(materialPhysicsMassGrams(findRule("block", 3), 1_000_000), 2600);
    assert.equal(materialPhysicsMassGrams(findRule("block", 49), 100_000), 14);
    assert.equal(materialPhysicsMassGrams(findRule("item", 1010), 250_000), 625);
  });

  it("encodes the treasury instruction and decodes the resulting table layout", () => {
    const authority = Keypair.generate().publicKey;
    const globalConfig = deriveGlobalConfigPda()[0];
    const materialPhysics = deriveMaterialPhysicsPda({ globalConfig })[0];
    const instruction = createConfigureMaterialPhysicsInstruction({
      authority,
      revision: document.revision,
      rules: document.rules,
      globalConfig,
    });

    assert.equal(instruction.keys.length, 4);
    assert.equal(instruction.keys[0].pubkey.toBase58(), authority.toBase58());
    assert.equal(instruction.keys[1].pubkey.toBase58(), globalConfig.toBase58());
    assert.equal(instruction.keys[2].pubkey.toBase58(), materialPhysics.toBase58());
    assert.equal(instruction.keys[3].pubkey.toBase58(), SystemProgram.programId.toBase58());
    assert.deepEqual([...instruction.data.subarray(0, 2)], [1, 12]);
    assert.equal(instruction.data.readUInt32LE(2), document.revision);
    assert.equal(instruction.data.readUInt8(6), document.ruleCount);

    const account = Buffer.alloc(MATERIAL_PHYSICS_LEN);
    account.write(MATERIAL_PHYSICS_MAGIC, 0, "utf8");
    account.writeUInt8(MATERIAL_PHYSICS_VERSION, 8);
    account.writeUInt8(254, 9);
    account.writeUInt8(document.ruleCount, 10);
    account.writeUInt32LE(document.revision, 12);
    instruction.data.subarray(7).copy(account, MATERIAL_PHYSICS_HEADER_LEN);
    const decoded = decodeMaterialPhysicsTable(account);
    assert.equal(decoded.revision, document.revision);
    assert.equal(decoded.ruleCount, document.ruleCount);
    assert.equal(decoded.rules[0].id, 1);
    assert.equal(decoded.rules.at(-1)?.id, 1060);
    assert.equal(decoded.rules.at(-1)?.kind, "item");
  });

  it("fits the complete rule table in one Solana transaction", () => {
    const authority = Keypair.generate();
    const instruction = createConfigureMaterialPhysicsInstruction({
      authority: authority.publicKey,
      revision: document.revision,
      rules: document.rules,
    });
    const transaction = new Transaction({
      feePayer: authority.publicKey,
      recentBlockhash: Keypair.generate().publicKey.toBase58(),
    }).add(instruction);
    transaction.sign(authority);
    assert.ok(transaction.serialize().length <= 1232);
  });

  it("requires authoritative mass on every decoded Backpack slot", () => {
    const owner = Keypair.generate().publicKey;
    const fixture = Buffer.alloc(BACKPACK_LEN);
    fixture.write("NCKBPK01", 0, "utf8");
    fixture.writeUInt16LE(BACKPACK_VERSION, 8);
    fixture.writeUInt8(1, 11);
    owner.toBuffer().copy(fixture, 20);
    fixture.writeUInt8(50, 52);
    fixture.writeUInt8(1, 53);
    fixture.writeUInt8(BACKPACK_FLAG_MASS_STATE_VALID, 55);
    fixture.writeBigUInt64LE(2600n, 90);
    fixture.writeUInt8(BACKPACK_SLOT_KIND_BLOCK, 128);
    fixture.writeUInt16LE(BACKPACK_ITEM_FLAG_MASS_VALID, 130);
    fixture.writeUInt32LE(1, 132);
    fixture.writeInt16LE(3 << 9, 140);
    fixture.writeUInt32LE(1_000_000, 188);
    fixture.writeUInt32LE(2600, 192);
    const decoded = decodeBackpack(fixture);
    assert.equal(decoded.totalMassGrams, 2600n);
    assert.equal(decoded.slots[0].massGrams, 2600);

    fixture.writeUInt16LE(0, 130);
    assert.throws(() => decodeBackpack(fixture), /authoritative mass/);
    fixture.writeUInt8(0, 55);
    assert.throws(() => decodeBackpack(fixture), /mass state/);
  });

  it("passes MaterialPhysics when the smelting program appends an output", () => {
    const smeltingAuthority = Keypair.generate().publicKey;
    const owner = Keypair.generate().publicKey;
    const backpack = Keypair.generate().publicKey;
    const itemPda = Keypair.generate().publicKey;
    const instruction = createAppendSmeltingItemInstruction({
      smeltingAuthority,
      owner,
      backpack,
      slot: {
        kind: BACKPACK_SLOT_KIND_ITEM,
        category: BACKPACK_ITEM_CATEGORY_MATERIAL,
        flags: 0,
        quantity: 1,
        resource: { worldX: 0, worldY: 0, worldZ: 0 },
        itemCode: 1010,
        itemId: 1n,
        itemPda,
        volumeMm3: 250_000,
        durabilityCurrent: 1,
        durabilityMax: 1,
        grade: 1,
        itemLevel: 1,
        qualityBps: 10_000,
      },
    });
    assert.equal(instruction.keys.length, 4);
    assert.equal(instruction.keys[3].pubkey.toBase58(), deriveMaterialPhysicsPda()[0].toBase58());
  });
});

function findRule(kind: MaterialPhysicsRule["kind"], id: number): MaterialPhysicsRule {
  const rule = document.rules.find((candidate) => candidate.kind === kind && candidate.id === id);
  assert.ok(rule, `Missing ${kind} rule ${id}`);
  return rule;
}

function ruleKey(rule: MaterialPhysicsRule): number {
  return rule.kind === "item" ? MATERIAL_PHYSICS_ITEM_KEY_MASK | rule.id : rule.id;
}

assert.equal(
  MATERIAL_PHYSICS_LEN,
  MATERIAL_PHYSICS_HEADER_LEN + 128 * MATERIAL_PHYSICS_RULE_LEN,
);
