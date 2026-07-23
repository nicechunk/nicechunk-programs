import { Buffer } from "buffer";
import {
  PublicKey,
  SYSVAR_INSTRUCTIONS_PUBKEY,
  SystemProgram,
  TransactionInstruction,
} from "@solana/web3.js";
import { deriveGlobalConfigPda, NICECHUNK_CORE_PROGRAM_ID } from "./nicechunk-core.ts";

const env = typeof process !== "undefined" ? process.env : {};

export const NICECHUNK_SKILLS_PROGRAM_ID = new PublicKey(
  env.NICECHUNK_SKILLS_PROGRAM_ID ?? "5gkdfmRJogdSdPrT8rvnEkPdn2N2fLBnQ6YDdegUcu3P",
);
export const PLAYER_SKILLS_SEED = "player-skills-v1";
export const SKILL_RULE_TABLE_SEED = "skill-rules-v1";
export const PLAYER_SKILLS_MAGIC = "NCKSKL01";
export const SKILL_RULE_TABLE_MAGIC = "NCKXPR01";
export const PLAYER_SKILLS_VERSION = 1;
export const SKILL_RULE_TABLE_VERSION = 1;
export const PLAYER_SKILLS_LEN = 480;
export const SKILL_RULE_TABLE_HEADER_LEN = 912;
export const SKILL_SOURCE_RULE_LEN = 136;
export const SKILL_SOURCE_RULE_MAX_COUNT = 32;
export const SKILL_GENERIC_SOURCE_RULE_MAX_COUNT = 30;
export const SKILL_BURDEN_WORK_CURSOR_INDEX = 30;
export const SKILL_BURDEN_SEQUENCE_CURSOR_INDEX = 31;
export const SKILL_BURDEN_RULE_RECORD_INDEX = 31;
export const SKILL_BURDEN_RULE_MAGIC = "NCKBRD01";
export const SKILL_BURDEN_RULE_VERSION = 1;
export const SKILL_RULE_TABLE_LEN =
  SKILL_RULE_TABLE_HEADER_LEN + SKILL_SOURCE_RULE_MAX_COUNT * SKILL_SOURCE_RULE_LEN;
export const SKILL_COUNT = 10;
export const SKILL_MAX_LEVEL = 10;
export const SOURCE_RULE_FLAG_BACKFILL_ON_FIRST_SYNC = 1 << 0;
export const SOURCE_SEED_GLOBAL_OWNER = 0;
export const SOURCE_SEED_OWNER = 1;
export const SOURCE_SEED_MAX_BYTES = 24;

export const PLAYER_SKILL_IDS = Object.freeze([
  "precisionGathering",
  "burden",
  "smelting",
  "forging",
  "craftsmanship",
  "swiftness",
  "exploration",
  "stamina",
  "strength",
  "appraisal",
] as const);

export type PlayerSkillId = (typeof PLAYER_SKILL_IDS)[number];

export interface SkillSourceRuleInput {
  enabled?: boolean;
  metricWidth: 4 | 8;
  flags?: number;
  seedLayout: 0 | 1;
  ruleId: number;
  sourceProgram: PublicKey;
  sourceMagic: string | Uint8Array;
  sourceSeed: string | Uint8Array;
  ownerOffset: number;
  globalConfigOffset: number;
  metricOffset: number;
  maxDeltaPerSync: bigint | number;
  unitDivisor?: number;
  xpPerUnit: Partial<Record<PlayerSkillId, number>> | readonly number[];
}

export interface MiningCoordinateInput {
  x: number;
  y: number;
  z: number;
}

export interface MiningTravelRuleInput {
  enabled?: boolean;
  minimumDistance: number;
  skill: PlayerSkillId | number;
  xpAward: number;
}

export interface BurdenMiningRuleInput {
  enabled?: boolean;
  skill: PlayerSkillId | number;
  maxEffectiveMassGrams: bigint | number;
  workPerXp: bigint | number;
}

export interface DecodedBurdenMiningRule {
  enabled: boolean;
  skill: PlayerSkillId;
  skillIndex: number;
  maxEffectiveMassGrams: bigint;
  workPerXp: bigint;
}

export interface DecodedSkillRuleTable {
  version: number;
  authority: PublicKey;
  globalConfig: PublicKey;
  ruleCount: number;
  revision: number;
  createdSlot: bigint;
  updatedSlot: bigint;
  createdAt: bigint;
  miningTravelRule: Readonly<{
    enabled: boolean;
    minimumDistance: number;
    skill: PlayerSkillId;
    skillIndex: number;
    xpAward: number;
  }>;
  burdenMiningRule: Readonly<DecodedBurdenMiningRule> | null;
}

export interface DecodedPlayerSkills {
  version: number;
  owner: PublicKey;
  globalConfig: PublicKey;
  xp: Record<PlayerSkillId, bigint>;
  levels: Record<PlayerSkillId, number>;
  cursorMask: number;
  ruleRevision: number;
  cursors: readonly bigint[];
  createdSlot: bigint;
  updatedSlot: bigint;
  createdAt: bigint;
  lastMiningCoordinate: Readonly<MiningCoordinateInput> | null;
  miningTravelCount: bigint;
  burdenWorkGrams: bigint;
  lastBurdenMineSequence: bigint;
}

export function derivePlayerSkillsPda({
  owner,
  globalConfig = deriveGlobalConfigPda(NICECHUNK_CORE_PROGRAM_ID)[0],
  programId = NICECHUNK_SKILLS_PROGRAM_ID,
}: {
  owner: PublicKey;
  globalConfig?: PublicKey;
  programId?: PublicKey;
}): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(PLAYER_SKILLS_SEED), globalConfig.toBuffer(), owner.toBuffer()],
    programId,
  );
}

export function deriveSkillRuleTablePda({
  globalConfig = deriveGlobalConfigPda(NICECHUNK_CORE_PROGRAM_ID)[0],
  programId = NICECHUNK_SKILLS_PROGRAM_ID,
}: {
  globalConfig?: PublicKey;
  programId?: PublicKey;
} = {}): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(SKILL_RULE_TABLE_SEED), globalConfig.toBuffer()],
    programId,
  );
}

export function deriveSkillSourcePda({
  rule,
  owner,
  globalConfig = deriveGlobalConfigPda(NICECHUNK_CORE_PROGRAM_ID)[0],
}: {
  rule: SkillSourceRuleInput;
  owner: PublicKey;
  globalConfig?: PublicKey;
}): [PublicKey, number] {
  const sourceSeed = normalizeFixedBytes(rule.sourceSeed, SOURCE_SEED_MAX_BYTES, "sourceSeed", false);
  const seed = sourceSeed.subarray(0, sourceSeed.findIndex((value) => value === 0) < 0
    ? sourceSeed.length
    : sourceSeed.findIndex((value) => value === 0));
  const seeds = rule.seedLayout === SOURCE_SEED_GLOBAL_OWNER
    ? [seed, globalConfig.toBuffer(), owner.toBuffer()]
    : [seed, owner.toBuffer()];
  return PublicKey.findProgramAddressSync(seeds, rule.sourceProgram);
}

export function createInitializeSkillRuleTableInstruction({
  authority,
  globalConfig = deriveGlobalConfigPda(NICECHUNK_CORE_PROGRAM_ID)[0],
  programId = NICECHUNK_SKILLS_PROGRAM_ID,
}: {
  authority: PublicKey;
  globalConfig?: PublicKey;
  programId?: PublicKey;
}): TransactionInstruction {
  const [ruleTable] = deriveSkillRuleTablePda({ globalConfig, programId });
  return new TransactionInstruction({
    programId,
    keys: [
      { pubkey: authority, isSigner: true, isWritable: true },
      { pubkey: ruleTable, isSigner: false, isWritable: true },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: Buffer.from([0]),
  });
}

export function createSetSkillThresholdsInstruction({
  authority,
  skill,
  thresholds,
  globalConfig = deriveGlobalConfigPda(NICECHUNK_CORE_PROGRAM_ID)[0],
  programId = NICECHUNK_SKILLS_PROGRAM_ID,
}: {
  authority: PublicKey;
  skill: PlayerSkillId | number;
  thresholds: readonly (bigint | number)[];
  globalConfig?: PublicKey;
  programId?: PublicKey;
}): TransactionInstruction {
  const skillIndex = typeof skill === "number" ? skill : PLAYER_SKILL_IDS.indexOf(skill);
  if (!Number.isInteger(skillIndex) || skillIndex < 0 || skillIndex >= SKILL_COUNT) {
    throw new Error("Invalid skill index.");
  }
  validateThresholds(thresholds);
  const data = Buffer.alloc(2 + SKILL_MAX_LEVEL * 8);
  data.writeUInt8(1, 0);
  data.writeUInt8(skillIndex, 1);
  thresholds.forEach((threshold, index) => data.writeBigUInt64LE(BigInt(threshold), 2 + index * 8));
  const [ruleTable] = deriveSkillRuleTablePda({ globalConfig, programId });
  return new TransactionInstruction({
    programId,
    keys: [
      { pubkey: authority, isSigner: true, isWritable: false },
      { pubkey: ruleTable, isSigner: false, isWritable: true },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
    ],
    data,
  });
}

export function createUpsertSkillSourceRuleInstruction({
  authority,
  ruleIndex,
  rule,
  globalConfig = deriveGlobalConfigPda(NICECHUNK_CORE_PROGRAM_ID)[0],
  programId = NICECHUNK_SKILLS_PROGRAM_ID,
}: {
  authority: PublicKey;
  ruleIndex: number;
  rule: SkillSourceRuleInput;
  globalConfig?: PublicKey;
  programId?: PublicKey;
}): TransactionInstruction {
  if (!Number.isInteger(ruleIndex) || ruleIndex < 0 || ruleIndex >= SKILL_GENERIC_SOURCE_RULE_MAX_COUNT) {
    throw new Error("Invalid source rule index.");
  }
  const [ruleTable] = deriveSkillRuleTablePda({ globalConfig, programId });
  return new TransactionInstruction({
    programId,
    keys: [
      { pubkey: authority, isSigner: true, isWritable: false },
      { pubkey: ruleTable, isSigner: false, isWritable: true },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
    ],
    data: Buffer.concat([Buffer.from([2, ruleIndex]), encodeSkillSourceRule(rule)]),
  });
}

export function createSyncPlayerSkillsInstruction({
  payer,
  owner,
  sourceAccounts = [],
  miningCoordinate = null,
  globalConfig = deriveGlobalConfigPda(NICECHUNK_CORE_PROGRAM_ID)[0],
  programId = NICECHUNK_SKILLS_PROGRAM_ID,
}: {
  payer: PublicKey;
  owner: PublicKey;
  sourceAccounts?: readonly PublicKey[];
  miningCoordinate?: MiningCoordinateInput | null;
  globalConfig?: PublicKey;
  programId?: PublicKey;
}): TransactionInstruction {
  const [playerSkills] = derivePlayerSkillsPda({ owner, globalConfig, programId });
  const [ruleTable] = deriveSkillRuleTablePda({ globalConfig, programId });
  const uniqueSources = [...new Map(sourceAccounts.map((source) => [source.toBase58(), source])).values()];
  const coordinate = miningCoordinate ? normalizeMiningCoordinate(miningCoordinate) : null;
  const data = Buffer.alloc(coordinate ? 13 : 1);
  data.writeUInt8(3, 0);
  if (coordinate) {
    data.writeInt32LE(coordinate.x, 1);
    data.writeInt32LE(coordinate.y, 5);
    data.writeInt32LE(coordinate.z, 9);
  }
  return new TransactionInstruction({
    programId,
    keys: [
      { pubkey: payer, isSigner: true, isWritable: true },
      { pubkey: owner, isSigner: false, isWritable: false },
      { pubkey: playerSkills, isSigner: false, isWritable: true },
      { pubkey: ruleTable, isSigner: false, isWritable: false },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ...(coordinate ? [{ pubkey: SYSVAR_INSTRUCTIONS_PUBKEY, isSigner: false, isWritable: false }] : []),
      ...uniqueSources.map((pubkey) => ({ pubkey, isSigner: false, isWritable: false })),
    ],
    data,
  });
}

export function createSetSkillRuleTableAuthorityInstruction({
  authority,
  newAuthority,
  globalConfig = deriveGlobalConfigPda(NICECHUNK_CORE_PROGRAM_ID)[0],
  programId = NICECHUNK_SKILLS_PROGRAM_ID,
}: {
  authority: PublicKey;
  newAuthority: PublicKey;
  globalConfig?: PublicKey;
  programId?: PublicKey;
}): TransactionInstruction {
  const [ruleTable] = deriveSkillRuleTablePda({ globalConfig, programId });
  return new TransactionInstruction({
    programId,
    keys: [
      { pubkey: authority, isSigner: true, isWritable: false },
      { pubkey: ruleTable, isSigner: false, isWritable: true },
      { pubkey: newAuthority, isSigner: false, isWritable: false },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
    ],
    data: Buffer.from([4]),
  });
}

export function createSetMiningTravelRuleInstruction({
  authority,
  rule,
  globalConfig = deriveGlobalConfigPda(NICECHUNK_CORE_PROGRAM_ID)[0],
  programId = NICECHUNK_SKILLS_PROGRAM_ID,
}: {
  authority: PublicKey;
  rule: MiningTravelRuleInput;
  globalConfig?: PublicKey;
  programId?: PublicKey;
}): TransactionInstruction {
  const skillIndex = typeof rule.skill === "number" ? rule.skill : PLAYER_SKILL_IDS.indexOf(rule.skill);
  if (!Number.isInteger(skillIndex) || skillIndex < 0 || skillIndex >= SKILL_COUNT) {
    throw new Error("Invalid mining travel skill index.");
  }
  const minimumDistance = requireUnsigned(rule.minimumDistance, 0xffff, "minimumDistance", 1);
  const xpAward = requireUnsigned(rule.xpAward, 0xffff, "xpAward", 1);
  const data = Buffer.alloc(7);
  data.writeUInt8(5, 0);
  data.writeUInt8(rule.enabled === false ? 0 : 1, 1);
  data.writeUInt16LE(minimumDistance, 2);
  data.writeUInt8(skillIndex, 4);
  data.writeUInt16LE(xpAward, 5);
  const [ruleTable] = deriveSkillRuleTablePda({ globalConfig, programId });
  return new TransactionInstruction({
    programId,
    keys: [
      { pubkey: authority, isSigner: true, isWritable: false },
      { pubkey: ruleTable, isSigner: false, isWritable: true },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
    ],
    data,
  });
}

export function createSetBurdenMiningRuleInstruction({
  authority,
  rule,
  globalConfig = deriveGlobalConfigPda(NICECHUNK_CORE_PROGRAM_ID)[0],
  programId = NICECHUNK_SKILLS_PROGRAM_ID,
}: {
  authority: PublicKey;
  rule: BurdenMiningRuleInput;
  globalConfig?: PublicKey;
  programId?: PublicKey;
}): TransactionInstruction {
  const skillIndex = typeof rule.skill === "number" ? rule.skill : PLAYER_SKILL_IDS.indexOf(rule.skill);
  if (!Number.isInteger(skillIndex) || skillIndex < 0 || skillIndex >= SKILL_COUNT) {
    throw new Error("Invalid burden mining skill index.");
  }
  const maxEffectiveMassGrams = requireUnsignedBigInt(
    rule.maxEffectiveMassGrams,
    "maxEffectiveMassGrams",
    1n,
  );
  const workPerXp = requireUnsignedBigInt(rule.workPerXp, "workPerXp", 1n);
  const data = Buffer.alloc(19);
  data.writeUInt8(6, 0);
  data.writeUInt8(rule.enabled === false ? 0 : 1, 1);
  data.writeUInt8(skillIndex, 2);
  data.writeBigUInt64LE(maxEffectiveMassGrams, 3);
  data.writeBigUInt64LE(workPerXp, 11);
  const [ruleTable] = deriveSkillRuleTablePda({ globalConfig, programId });
  return new TransactionInstruction({
    programId,
    keys: [
      { pubkey: authority, isSigner: true, isWritable: false },
      { pubkey: ruleTable, isSigner: false, isWritable: true },
      { pubkey: globalConfig, isSigner: false, isWritable: false },
    ],
    data,
  });
}

export function encodeSkillSourceRule(rule: SkillSourceRuleInput): Buffer {
  const data = Buffer.alloc(SKILL_SOURCE_RULE_LEN);
  const sourceMagic = normalizeFixedBytes(rule.sourceMagic, 8, "sourceMagic", true);
  const sourceSeedValue = bytesFrom(rule.sourceSeed);
  if (!sourceSeedValue.length || sourceSeedValue.length > SOURCE_SEED_MAX_BYTES) {
    throw new Error(`sourceSeed must contain 1-${SOURCE_SEED_MAX_BYTES} bytes.`);
  }
  const xpPerUnit = normalizeXpRates(rule.xpPerUnit);
  const metricWidth = Number(rule.metricWidth);
  const flags = Number(rule.flags ?? 0);
  const seedLayout = Number(rule.seedLayout);
  const ruleId = requireUnsigned(rule.ruleId, 0xffff_ffff, "ruleId", 1);
  const ownerOffset = requireUnsigned(rule.ownerOffset, 0xffff, "ownerOffset");
  const globalConfigOffset = requireUnsigned(rule.globalConfigOffset, 0xffff, "globalConfigOffset");
  const metricOffset = requireUnsigned(rule.metricOffset, 0xffff, "metricOffset");
  const maxDelta = BigInt(rule.maxDeltaPerSync);
  const unitDivisor = requireUnsigned(rule.unitDivisor ?? 1, 0xffff_ffff, "unitDivisor", 1);
  if (![4, 8].includes(metricWidth)) throw new Error("metricWidth must be 4 or 8.");
  if (![SOURCE_SEED_GLOBAL_OWNER, SOURCE_SEED_OWNER].includes(seedLayout)) {
    throw new Error("Invalid source seed layout.");
  }
  if (flags < 0 || flags > SOURCE_RULE_FLAG_BACKFILL_ON_FIRST_SYNC) throw new Error("Invalid source rule flags.");
  if (maxDelta <= 0n || maxDelta > 0xffff_ffff_ffff_ffffn) throw new Error("Invalid maxDeltaPerSync.");
  if (!xpPerUnit.some((value) => value > 0)) throw new Error("At least one XP rate is required.");

  data.writeUInt8(rule.enabled === false ? 0 : 1, 0);
  data.writeUInt8(metricWidth, 1);
  data.writeUInt8(flags, 2);
  data.writeUInt8(seedLayout, 3);
  data.writeUInt32LE(ruleId, 4);
  rule.sourceProgram.toBuffer().copy(data, 8);
  sourceMagic.copy(data, 40);
  data.writeUInt8(sourceSeedValue.length, 48);
  data.writeUInt16LE(ownerOffset, 50);
  data.writeUInt16LE(globalConfigOffset, 52);
  data.writeUInt16LE(metricOffset, 54);
  data.writeBigUInt64LE(maxDelta, 56);
  data.writeUInt32LE(unitDivisor, 64);
  sourceSeedValue.copy(data, 68);
  xpPerUnit.forEach((value, index) => data.writeUInt32LE(value, 92 + index * 4));
  return data;
}

export function decodePlayerSkills(data: Buffer | Uint8Array): DecodedPlayerSkills {
  const bytes = Buffer.from(data);
  if (bytes.length !== PLAYER_SKILLS_LEN || bytes.subarray(0, 8).toString("utf8") !== PLAYER_SKILLS_MAGIC) {
    throw new Error("Invalid PlayerSkills account.");
  }
  const version = bytes.readUInt16LE(8);
  if (version !== PLAYER_SKILLS_VERSION || bytes.readUInt8(11) !== 1) {
    throw new Error("Unsupported PlayerSkills account version.");
  }
  const xp = {} as Record<PlayerSkillId, bigint>;
  const levels = {} as Record<PlayerSkillId, number>;
  PLAYER_SKILL_IDS.forEach((skillId, index) => {
    xp[skillId] = bytes.readBigUInt64LE(76 + index * 8);
    levels[skillId] = bytes.readUInt8(156 + index);
  });
  const hasLastMiningCoordinate = (bytes.readUInt8(468) & 1) !== 0;
  return {
    version,
    owner: new PublicKey(bytes.subarray(12, 44)),
    globalConfig: new PublicKey(bytes.subarray(44, 76)),
    xp,
    levels,
    cursorMask: bytes.readUInt32LE(166),
    ruleRevision: bytes.readUInt32LE(172),
    cursors: Object.freeze(Array.from({ length: SKILL_SOURCE_RULE_MAX_COUNT }, (_, index) => (
      bytes.readBigUInt64LE(176 + index * 8)
    ))),
    createdSlot: bytes.readBigUInt64LE(432),
    updatedSlot: bytes.readBigUInt64LE(440),
    createdAt: bytes.readBigInt64LE(448),
    lastMiningCoordinate: hasLastMiningCoordinate
      ? Object.freeze({
          x: bytes.readInt32LE(456),
          y: bytes.readInt32LE(460),
          z: bytes.readInt32LE(464),
        })
      : null,
    miningTravelCount: bytes.readBigUInt64LE(472),
    burdenWorkGrams: bytes.readBigUInt64LE(
      176 + SKILL_BURDEN_WORK_CURSOR_INDEX * 8,
    ),
    lastBurdenMineSequence: bytes.readBigUInt64LE(
      176 + SKILL_BURDEN_SEQUENCE_CURSOR_INDEX * 8,
    ),
  };
}

export function decodeSkillRuleTable(data: Buffer | Uint8Array): DecodedSkillRuleTable {
  const bytes = Buffer.from(data);
  if (bytes.length !== SKILL_RULE_TABLE_LEN
    || bytes.subarray(0, 8).toString("utf8") !== SKILL_RULE_TABLE_MAGIC) {
    throw new Error("Invalid SkillRuleTable account.");
  }
  const version = bytes.readUInt16LE(8);
  const ruleCount = bytes.readUInt8(76);
  if (version !== SKILL_RULE_TABLE_VERSION
    || bytes.readUInt8(11) !== 1
    || bytes.readUInt8(77) !== SKILL_COUNT
    || ruleCount > SKILL_GENERIC_SOURCE_RULE_MAX_COUNT) {
    throw new Error("Unsupported SkillRuleTable account version.");
  }
  const miningSkillIndex = bytes.readUInt8(910);
  if (miningSkillIndex >= SKILL_COUNT) throw new Error("Invalid mining travel skill index.");
  const burdenOffset = SKILL_RULE_TABLE_HEADER_LEN
    + SKILL_BURDEN_RULE_RECORD_INDEX * SKILL_SOURCE_RULE_LEN;
  const burdenRecord = bytes.subarray(burdenOffset, burdenOffset + SKILL_SOURCE_RULE_LEN);
  const burdenMiningRule = burdenRecord.every((value) => value === 0)
    ? null
    : decodeBurdenMiningRule(burdenRecord);
  return {
    version,
    authority: new PublicKey(bytes.subarray(12, 44)),
    globalConfig: new PublicKey(bytes.subarray(44, 76)),
    ruleCount,
    revision: bytes.readUInt32LE(80),
    createdSlot: bytes.readBigUInt64LE(84),
    updatedSlot: bytes.readBigUInt64LE(92),
    createdAt: bytes.readBigInt64LE(100),
    miningTravelRule: Object.freeze({
      enabled: bytes.readUInt8(911) !== 0,
      minimumDistance: bytes.readUInt16LE(78),
      skill: PLAYER_SKILL_IDS[miningSkillIndex],
      skillIndex: miningSkillIndex,
      xpAward: bytes.readUInt16LE(908),
    }),
    burdenMiningRule: burdenMiningRule ? Object.freeze(burdenMiningRule) : null,
  };
}

function decodeBurdenMiningRule(record: Buffer): DecodedBurdenMiningRule {
  if (record.length !== SKILL_SOURCE_RULE_LEN
    || record.subarray(0, 8).toString("utf8") !== SKILL_BURDEN_RULE_MAGIC
    || record.readUInt16LE(8) !== SKILL_BURDEN_RULE_VERSION) {
    throw new Error("Invalid burden mining rule.");
  }
  const enabled = record.readUInt8(10) !== 0;
  const skillIndex = record.readUInt8(11);
  const maxEffectiveMassGrams = record.readBigUInt64LE(12);
  const workPerXp = record.readBigUInt64LE(20);
  if (skillIndex >= SKILL_COUNT || (enabled && (!maxEffectiveMassGrams || !workPerXp))) {
    throw new Error("Invalid burden mining rule configuration.");
  }
  return {
    enabled,
    skill: PLAYER_SKILL_IDS[skillIndex],
    skillIndex,
    maxEffectiveMassGrams,
    workPerXp,
  };
}

function normalizeMiningCoordinate(coordinate: MiningCoordinateInput): MiningCoordinateInput {
  const x = requireSigned(coordinate.x, -0x8000_0000, 0x7fff_ffff, "miningCoordinate.x");
  const y = requireSigned(coordinate.y, -0x8000, 0x7fff, "miningCoordinate.y");
  const z = requireSigned(coordinate.z, -0x8000_0000, 0x7fff_ffff, "miningCoordinate.z");
  return { x, y, z };
}

function validateThresholds(thresholds: readonly (bigint | number)[]): void {
  if (thresholds.length !== SKILL_MAX_LEVEL) throw new Error(`Exactly ${SKILL_MAX_LEVEL} thresholds are required.`);
  let previous = 0n;
  for (const threshold of thresholds) {
    const value = BigInt(threshold);
    if (value <= previous || value > 0xffff_ffff_ffff_ffffn) {
      throw new Error("Skill thresholds must be strictly increasing unsigned u64 values.");
    }
    previous = value;
  }
}

function normalizeXpRates(
  rates: Partial<Record<PlayerSkillId, number>> | readonly number[],
): number[] {
  const values = Array.isArray(rates)
    ? [...rates]
    : PLAYER_SKILL_IDS.map((skillId) => Number((rates as Partial<Record<PlayerSkillId, number>>)[skillId] ?? 0));
  if (values.length !== SKILL_COUNT) throw new Error(`Exactly ${SKILL_COUNT} XP rates are required.`);
  return values.map((value, index) => requireUnsigned(value, 0xffff_ffff, `xpPerUnit[${index}]`));
}

function normalizeFixedBytes(
  value: string | Uint8Array,
  length: number,
  label: string,
  exact: boolean,
): Buffer {
  const bytes = bytesFrom(value);
  if ((exact && bytes.length !== length) || (!exact && bytes.length > length)) {
    throw new Error(`${label} must contain ${exact ? length : `at most ${length}`} bytes.`);
  }
  const result = Buffer.alloc(length);
  bytes.copy(result);
  return result;
}

function bytesFrom(value: string | Uint8Array): Buffer {
  return typeof value === "string" ? Buffer.from(value, "utf8") : Buffer.from(value);
}

function requireUnsigned(
  value: number,
  maximum: number,
  label: string,
  minimum = 0,
): number {
  const normalized = Number(value);
  if (!Number.isSafeInteger(normalized) || normalized < minimum || normalized > maximum) {
    throw new Error(`Invalid ${label}.`);
  }
  return normalized;
}

function requireSigned(value: number, minimum: number, maximum: number, label: string): number {
  const normalized = Number(value);
  if (!Number.isSafeInteger(normalized) || normalized < minimum || normalized > maximum) {
    throw new Error(`Invalid ${label}.`);
  }
  return normalized;
}

function requireUnsignedBigInt(
  value: bigint | number,
  label: string,
  minimum = 0n,
): bigint {
  const normalized = BigInt(value);
  if (normalized < minimum || normalized > 0xffff_ffff_ffff_ffffn) {
    throw new Error(`Invalid ${label}.`);
  }
  return normalized;
}
