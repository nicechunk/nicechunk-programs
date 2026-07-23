import { strict as assert } from "node:assert";
import { PublicKey } from "@solana/web3.js";
import {
  createSetBurdenMiningRuleInstruction,
  createSetMiningTravelRuleInstruction,
  createSyncPlayerSkillsInstruction,
  decodePlayerSkills,
  decodeSkillRuleTable,
  derivePlayerSkillsPda,
  encodeSkillSourceRule,
  NICECHUNK_SKILLS_PROGRAM_ID,
  PLAYER_SKILL_IDS,
  PLAYER_SKILLS_LEN,
  SKILL_BURDEN_RULE_MAGIC,
  SKILL_RULE_TABLE_HEADER_LEN,
  SKILL_RULE_TABLE_LEN,
  SKILL_SOURCE_RULE_LEN,
  SOURCE_RULE_FLAG_BACKFILL_ON_FIRST_SYNC,
  SOURCE_SEED_GLOBAL_OWNER,
} from "../sdk/nicechunk-skills.ts";

describe("nicechunk skills SDK", () => {
  it("encodes a fixed source rule record", () => {
    const sourceProgram = new PublicKey("GnVKn442KDTDgCyjVG7SEtCQQLjaCiLvrEZDWSU13wbj");
    const encoded = encodeSkillSourceRule({
      ruleId: 1_001,
      sourceProgram,
      sourceMagic: "NCKPRG01",
      sourceSeed: "player-progress",
      seedLayout: SOURCE_SEED_GLOBAL_OWNER,
      metricWidth: 8,
      metricOffset: 76,
      ownerOffset: 12,
      globalConfigOffset: 44,
      maxDeltaPerSync: 25_000,
      flags: SOURCE_RULE_FLAG_BACKFILL_ON_FIRST_SYNC,
      xpPerUnit: { precisionGathering: 115, burden: 18 },
    });
    assert.equal(encoded.length, 136);
    assert.equal(encoded.readUInt32LE(4), 1_001);
    assert.equal(new PublicKey(encoded.subarray(8, 40)).toBase58(), sourceProgram.toBase58());
    assert.equal(encoded.subarray(40, 48).toString("utf8"), "NCKPRG01");
    assert.equal(encoded.readUInt32LE(92), 115);
    assert.equal(encoded.readUInt32LE(96), 18);
  });

  it("decodes all ten XP values and levels", () => {
    const owner = PublicKey.unique();
    const globalConfig = PublicKey.unique();
    const data = Buffer.alloc(PLAYER_SKILLS_LEN);
    data.write("NCKSKL01", 0, "utf8");
    data.writeUInt16LE(1, 8);
    data.writeUInt8(1, 11);
    owner.toBuffer().copy(data, 12);
    globalConfig.toBuffer().copy(data, 44);
    PLAYER_SKILL_IDS.forEach((_skillId, index) => {
      data.writeBigUInt64LE(BigInt((index + 1) * 1_000), 76 + index * 8);
      data.writeUInt8(index + 1, 156 + index);
    });
    const decoded = decodePlayerSkills(data);
    assert.equal(decoded.owner.toBase58(), owner.toBase58());
    assert.equal(decoded.xp.appraisal, 10_000n);
    assert.equal(decoded.levels.appraisal, 10);
    assert.equal(decoded.lastMiningCoordinate, null);
    assert.equal(decoded.miningTravelCount, 0n);
    assert.equal(decoded.burdenWorkGrams, 0n);
    assert.equal(decoded.lastBurdenMineSequence, 0n);
  });

  it("encodes a trusted mining coordinate and instructions sysvar", () => {
    const payer = PublicKey.unique();
    const owner = PublicKey.unique();
    const source = PublicKey.unique();
    const instruction = createSyncPlayerSkillsInstruction({
      payer,
      owner,
      sourceAccounts: [source],
      miningCoordinate: { x: -160, y: 94, z: 320 },
    });
    assert.equal(instruction.data.length, 13);
    assert.equal(instruction.data.readInt32LE(1), -160);
    assert.equal(instruction.data.readInt32LE(5), 94);
    assert.equal(instruction.data.readInt32LE(9), 320);
    assert.equal(instruction.keys.length, 8);
    assert.equal(instruction.keys[6].pubkey.toBase58(), "Sysvar1nstructions1111111111111111111111111");
    assert.equal(instruction.keys[7].pubkey.toBase58(), source.toBase58());
  });

  it("encodes the 160-block swiftness rule", () => {
    const instruction = createSetMiningTravelRuleInstruction({
      authority: PublicKey.unique(),
      rule: { minimumDistance: 160, skill: "swiftness", xpAward: 1 },
    });
    assert.equal(instruction.data.readUInt8(0), 5);
    assert.equal(instruction.data.readUInt8(1), 1);
    assert.equal(instruction.data.readUInt16LE(2), 160);
    assert.equal(instruction.data.readUInt8(4), PLAYER_SKILL_IDS.indexOf("swiftness"));
    assert.equal(instruction.data.readUInt16LE(5), 1);
  });

  it("encodes and decodes the authoritative burden mining rule", () => {
    const authority = PublicKey.unique();
    const instruction = createSetBurdenMiningRuleInstruction({
      authority,
      rule: {
        skill: "burden",
        maxEffectiveMassGrams: 100_000,
        workPerXp: 100_000,
      },
    });
    assert.equal(instruction.data.readUInt8(0), 6);
    assert.equal(instruction.data.readUInt8(1), 1);
    assert.equal(instruction.data.readUInt8(2), PLAYER_SKILL_IDS.indexOf("burden"));
    assert.equal(instruction.data.readBigUInt64LE(3), 100_000n);
    assert.equal(instruction.data.readBigUInt64LE(11), 100_000n);

    const globalConfig = PublicKey.unique();
    const table = Buffer.alloc(SKILL_RULE_TABLE_LEN);
    table.write("NCKXPR01", 0, "utf8");
    table.writeUInt16LE(1, 8);
    table.writeUInt8(1, 11);
    authority.toBuffer().copy(table, 12);
    globalConfig.toBuffer().copy(table, 44);
    table.writeUInt8(10, 77);
    table.writeUInt16LE(160, 78);
    table.writeUInt8(PLAYER_SKILL_IDS.indexOf("swiftness"), 910);
    table.writeUInt8(1, 911);
    table.writeUInt16LE(1, 908);
    const burdenOffset = SKILL_RULE_TABLE_HEADER_LEN + 31 * SKILL_SOURCE_RULE_LEN;
    table.write(SKILL_BURDEN_RULE_MAGIC, burdenOffset, "utf8");
    table.writeUInt16LE(1, burdenOffset + 8);
    table.writeUInt8(1, burdenOffset + 10);
    table.writeUInt8(PLAYER_SKILL_IDS.indexOf("burden"), burdenOffset + 11);
    table.writeBigUInt64LE(100_000n, burdenOffset + 12);
    table.writeBigUInt64LE(100_000n, burdenOffset + 20);

    const decoded = decodeSkillRuleTable(table);
    assert.equal(decoded.authority.toBase58(), authority.toBase58());
    assert.equal(decoded.miningTravelRule.skill, "swiftness");
    assert.equal(decoded.burdenMiningRule?.skill, "burden");
    assert.equal(decoded.burdenMiningRule?.maxEffectiveMassGrams, 100_000n);
    assert.equal(decoded.burdenMiningRule?.workPerXp, 100_000n);
  });

  it("reserves source rule slots 30 and 31 for burden accounting", async () => {
    const { createUpsertSkillSourceRuleInstruction } = await import("../sdk/nicechunk-skills.ts");
    assert.throws(() => createUpsertSkillSourceRuleInstruction({
      authority: PublicKey.unique(),
      ruleIndex: 30,
      rule: {
        ruleId: 9_999,
        sourceProgram: PublicKey.unique(),
        sourceMagic: "NCKTEST1",
        sourceSeed: "test-source",
        seedLayout: SOURCE_SEED_GLOBAL_OWNER,
        metricWidth: 8,
        ownerOffset: 12,
        globalConfigOffset: 44,
        metricOffset: 76,
        maxDeltaPerSync: 10,
        xpPerUnit: { burden: 1 },
      },
    }), /Invalid source rule index/);
  });

  it("derives wallet-scoped skill progress", () => {
    const ownerA = PublicKey.unique();
    const ownerB = PublicKey.unique();
    const globalConfig = PublicKey.unique();
    const first = derivePlayerSkillsPda({ owner: ownerA, globalConfig })[0];
    const second = derivePlayerSkillsPda({ owner: ownerB, globalConfig })[0];
    assert.notEqual(first.toBase58(), second.toBase58());
    assert.equal(NICECHUNK_SKILLS_PROGRAM_ID.toBase58(), "5gkdfmRJogdSdPrT8rvnEkPdn2N2fLBnQ6YDdegUcu3P");
  });
});
