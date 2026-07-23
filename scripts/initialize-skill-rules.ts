import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import { resolve } from "node:path";
import {
  Connection,
  Keypair,
  PublicKey,
  sendAndConfirmTransaction,
  Transaction,
  type TransactionInstruction,
} from "@solana/web3.js";
import { deriveGlobalConfigPda, NICECHUNK_CORE_PROGRAM_ID } from "../sdk/nicechunk-core.ts";
import {
  createInitializeSkillRuleTableInstruction,
  createSetBurdenMiningRuleInstruction,
  createSetMiningTravelRuleInstruction,
  createSetSkillThresholdsInstruction,
  createSetSkillRuleTableAuthorityInstruction,
  createUpsertSkillSourceRuleInstruction,
  deriveSkillRuleTablePda,
  NICECHUNK_SKILLS_PROGRAM_ID,
  PLAYER_SKILL_IDS,
  SOURCE_RULE_FLAG_BACKFILL_ON_FIRST_SYNC,
  SOURCE_SEED_GLOBAL_OWNER,
  SOURCE_SEED_OWNER,
  type PlayerSkillId,
  type SkillSourceRuleInput,
} from "../sdk/nicechunk-skills.ts";

const CHUNK_PROGRAM_ID = new PublicKey("GnVKn442KDTDgCyjVG7SEtCQQLjaCiLvrEZDWSU13wbj");
const GAME_PROGRAM_ID = new PublicKey("6CurnvneezBuHwPUnrCiFg1QMWeUF67ufQxYebyr2UP7");
const PLAYER_PROGRAM_ID = new PublicKey("CHZHsBCGn58ih2WrPfKSYhvCEjMPGhArTiYCH7AWWBkB");

const SKILL_THRESHOLDS: Record<PlayerSkillId, readonly number[]> = {
  precisionGathering: [900, 3_481, 8_261, 15_663, 26_054, 39_764, 57_094, 78_323, 103_715, 133_517],
  burden: [1_300, 5_187, 12_563, 24_183, 40_715, 62_766, 90_898, 125_638, 167_483, 216_908],
  smelting: [1_200, 4_738, 11_398, 21_831, 36_608, 56_246, 81_223, 111_984, 148_950, 192_519],
  forging: [1_400, 5_644, 13_763, 26_628, 45_014, 69_627, 101_125, 140_126, 187_215, 242_950],
  craftsmanship: [1_800, 7_488, 18_638, 36_614, 62_649, 97_886, 143_399, 200_206, 269_280, 351_556],
  swiftness: [1_100, 4_211, 9_927, 18_727, 31_025, 47_192, 67_564, 92_454, 122_154, 156_939],
  exploration: [1_250, 4_961, 11_975, 22_994, 38_636, 59_462, 85_991, 118_707, 158_068, 204_510],
  stamina: [1_050, 4_020, 9_476, 17_876, 29_615, 45_047, 64_493, 88_252, 116_602, 149_806],
  strength: [1_450, 5_815, 14_132, 27_273, 46_011, 71_051, 103_045, 142_607, 190_317, 246_729],
  appraisal: [1_600, 6_518, 16_003, 31_120, 52_820, 81_976, 119_402, 165_867, 222_100, 288_799],
};

const SOURCE_RULES: readonly SkillSourceRuleInput[] = [
  {
    ruleId: 1_001,
    sourceProgram: CHUNK_PROGRAM_ID,
    sourceMagic: "NCKPRG01",
    sourceSeed: "player-progress",
    seedLayout: SOURCE_SEED_GLOBAL_OWNER,
    metricWidth: 8,
    metricOffset: 76,
    ownerOffset: 12,
    globalConfigOffset: 44,
    maxDeltaPerSync: 25_000,
    flags: SOURCE_RULE_FLAG_BACKFILL_ON_FIRST_SYNC,
    xpPerUnit: {
      precisionGathering: 115,
      exploration: 44,
      stamina: 9,
      strength: 18,
      appraisal: 22,
    },
  },
  {
    ruleId: 1_002,
    sourceProgram: CHUNK_PROGRAM_ID,
    sourceMagic: "NCKPRG01",
    sourceSeed: "player-progress",
    seedLayout: SOURCE_SEED_GLOBAL_OWNER,
    metricWidth: 8,
    metricOffset: 116,
    ownerOffset: 12,
    globalConfigOffset: 44,
    maxDeltaPerSync: 5_000,
    flags: SOURCE_RULE_FLAG_BACKFILL_ON_FIRST_SYNC,
    xpPerUnit: {
      exploration: 100,
      stamina: 2,
      strength: 2,
      appraisal: 45,
    },
  },
  {
    ruleId: 1_003,
    sourceProgram: CHUNK_PROGRAM_ID,
    sourceMagic: "NCKPRG01",
    sourceSeed: "player-progress",
    seedLayout: SOURCE_SEED_GLOBAL_OWNER,
    metricWidth: 4,
    metricOffset: 124,
    ownerOffset: 12,
    globalConfigOffset: 44,
    maxDeltaPerSync: 5_000,
    flags: SOURCE_RULE_FLAG_BACKFILL_ON_FIRST_SYNC,
    xpPerUnit: {
      exploration: 75,
      stamina: 8,
      appraisal: 15,
    },
  },
  {
    ruleId: 1_004,
    sourceProgram: GAME_PROGRAM_ID,
    sourceMagic: "NCKPRG01",
    sourceSeed: "player-progress",
    seedLayout: SOURCE_SEED_GLOBAL_OWNER,
    metricWidth: 8,
    metricOffset: 108,
    ownerOffset: 12,
    globalConfigOffset: 44,
    maxDeltaPerSync: 25_000,
    flags: SOURCE_RULE_FLAG_BACKFILL_ON_FIRST_SYNC,
    xpPerUnit: {
      smelting: 250,
      forging: 38,
      craftsmanship: 28,
      stamina: 6,
      appraisal: 36,
    },
  },
  {
    ruleId: 1_005,
    sourceProgram: PLAYER_PROGRAM_ID,
    sourceMagic: "NCKPLY01",
    sourceSeed: "player-v7",
    seedLayout: SOURCE_SEED_OWNER,
    metricWidth: 8,
    metricOffset: 449,
    ownerOffset: 12,
    globalConfigOffset: 44,
    maxDeltaPerSync: 10_000_000,
    unitDivisor: 100,
    flags: SOURCE_RULE_FLAG_BACKFILL_ON_FIRST_SYNC,
    xpPerUnit: {
      forging: 100,
      craftsmanship: 20,
      stamina: 10,
      strength: 10,
      appraisal: 15,
    },
  },
  {
    ruleId: 1_006,
    sourceProgram: PLAYER_PROGRAM_ID,
    sourceMagic: "NCKPLY01",
    sourceSeed: "player-v7",
    seedLayout: SOURCE_SEED_OWNER,
    metricWidth: 4,
    metricOffset: 457,
    ownerOffset: 12,
    globalConfigOffset: 44,
    maxDeltaPerSync: 5_000,
    flags: SOURCE_RULE_FLAG_BACKFILL_ON_FIRST_SYNC,
    xpPerUnit: {
      craftsmanship: 120,
      stamina: 15,
      strength: 20,
      appraisal: 50,
    },
  },
];

async function main(): Promise<void> {
  const options = parseArgs(process.argv.slice(2));
  const rpcUrl = options.url ?? process.env.SOLANA_RPC_URL ?? "https://api.devnet.solana.com";
  const keypairPath = resolve(options.keypair ?? process.env.SOLANA_KEYPAIR ?? `${homedir()}/.config/solana/id.json`);
  const authority = Keypair.fromSecretKey(Uint8Array.from(JSON.parse(readFileSync(keypairPath, "utf8"))));
  const programId = new PublicKey(options.programId ?? process.env.NICECHUNK_SKILLS_PROGRAM_ID ?? NICECHUNK_SKILLS_PROGRAM_ID);
  const connection = new Connection(rpcUrl, "confirmed");
  const [globalConfig] = deriveGlobalConfigPda(NICECHUNK_CORE_PROGRAM_ID);
  const [ruleTable] = deriveSkillRuleTablePda({ globalConfig, programId });
  const globalConfigAccount = await connection.getAccountInfo(globalConfig, "confirmed");
  if (!globalConfigAccount?.data?.length || globalConfigAccount.data.length < 85) {
    throw new Error("GlobalConfig is unavailable.");
  }
  const treasury = new PublicKey(globalConfigAccount.data.subarray(53, 85));

  if (!(await connection.getAccountInfo(ruleTable, "confirmed"))) {
    await submit(connection, authority, [createInitializeSkillRuleTableInstruction({
      authority: authority.publicKey,
      globalConfig,
      programId,
    })], "initialize rule table");
  }

  for (const skillId of PLAYER_SKILL_IDS) {
    await submit(connection, authority, [createSetSkillThresholdsInstruction({
      authority: authority.publicKey,
      skill: skillId,
      thresholds: SKILL_THRESHOLDS[skillId],
      globalConfig,
      programId,
    })], `set ${skillId} thresholds`);
  }

  for (let ruleIndex = 0; ruleIndex < SOURCE_RULES.length; ruleIndex += 1) {
    await submit(connection, authority, [createUpsertSkillSourceRuleInstruction({
      authority: authority.publicKey,
      ruleIndex,
      rule: SOURCE_RULES[ruleIndex],
      globalConfig,
      programId,
    })], `upsert source rule ${SOURCE_RULES[ruleIndex].ruleId}`);
  }

  await submit(connection, authority, [createSetMiningTravelRuleInstruction({
    authority: authority.publicKey,
    rule: {
      minimumDistance: 160,
      skill: "swiftness",
      xpAward: 1,
    },
    globalConfig,
    programId,
  })], "set mining travel rule");

  await submit(connection, authority, [createSetBurdenMiningRuleInstruction({
    authority: authority.publicKey,
    rule: {
      skill: "burden",
      maxEffectiveMassGrams: 100_000,
      workPerXp: 100_000,
    },
    globalConfig,
    programId,
  })], "set burden mining rule");

  if (!authority.publicKey.equals(treasury)) {
    await submit(connection, authority, [createSetSkillRuleTableAuthorityInstruction({
      authority: authority.publicKey,
      newAuthority: treasury,
      globalConfig,
      programId,
    })], "transfer rule authority to treasury");
  }

  console.log(JSON.stringify({
    rpcConfigured: true,
    programId: programId.toBase58(),
    authority: authority.publicKey.toBase58(),
    treasury: treasury.toBase58(),
    globalConfig: globalConfig.toBase58(),
    ruleTable: ruleTable.toBase58(),
    skills: PLAYER_SKILL_IDS.length,
    sourceRules: SOURCE_RULES.length,
  }, null, 2));
}

async function submit(
  connection: Connection,
  authority: Keypair,
  instructions: readonly TransactionInstruction[],
  label: string,
): Promise<void> {
  const signature = await sendAndConfirmTransaction(
    connection,
    new Transaction().add(...instructions),
    [authority],
    { commitment: "confirmed" },
  );
  console.log(`${label}: ${signature}`);
}

function parseArgs(values: string[]): { url?: string; keypair?: string; programId?: string } {
  const options: { url?: string; keypair?: string; programId?: string } = {};
  for (let index = 0; index < values.length; index += 1) {
    const value = values[index];
    if (value === "--url") options.url = values[++index];
    else if (value === "--keypair") options.keypair = values[++index];
    else if (value === "--program-id") options.programId = values[++index];
    else throw new Error(`Unknown argument: ${value}`);
  }
  return options;
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
