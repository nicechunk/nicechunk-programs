import {
  PublicKey,
  sendAndConfirmTransaction,
  Transaction,
} from "@solana/web3.js";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  createConfigureMaterialPhysicsInstruction,
  decodeMaterialPhysicsTable,
  deriveMaterialPhysicsPda,
  NICECHUNK_BACKPACK_PROGRAM_ID,
  NICECHUNK_BOOTSTRAP_AUTHORITY,
  type MaterialPhysicsRule,
} from "../sdk/nicechunk-backpack.ts";
import {
  decodeGlobalConfig,
  deriveGlobalConfigPda,
} from "../sdk/nicechunk-core.ts";
import {
  clusterUrl,
  connection,
  coreProgramId,
  readPayerKeypair,
} from "./core-script-utils.ts";

interface MaterialPhysicsDocument {
  schemaVersion: number;
  revision: number;
  seed: string;
  ruleCount: number;
  rules: MaterialPhysicsRule[];
}

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const rulePath = process.env.MATERIAL_PHYSICS_RULES
  ? path.resolve(process.env.MATERIAL_PHYSICS_RULES)
  : path.resolve(scriptDirectory, "../config/material_physics_v2.json");
const document = JSON.parse(fs.readFileSync(rulePath, "utf8")) as MaterialPhysicsDocument;
validateDocument(document);

const payer = readPayerKeypair();
const rpc = connection();
const selectedCoreProgramId = coreProgramId();
const selectedBackpackProgramId = new PublicKey(
  process.env.NICECHUNK_BACKPACK_PROGRAM_ID
    ?? process.env.NICECHUNK_GAME_PROGRAM_ID
    ?? NICECHUNK_BACKPACK_PROGRAM_ID,
);
const [globalConfig] = deriveGlobalConfigPda(selectedCoreProgramId);
const [materialPhysics, bump] = deriveMaterialPhysicsPda({
  globalConfig,
  backpackProgramId: selectedBackpackProgramId,
  coreProgramId: selectedCoreProgramId,
});

const globalConfigInfo = await rpc.getAccountInfo(globalConfig, "confirmed");
if (!globalConfigInfo?.owner.equals(selectedCoreProgramId)) {
  throw new Error(`GlobalConfig is missing or has the wrong owner: ${globalConfig.toBase58()}`);
}
const globalConfigState = decodeGlobalConfig(Buffer.from(globalConfigInfo.data));
const existingInfo = await rpc.getAccountInfo(materialPhysics, "confirmed");
if (existingInfo && !existingInfo.owner.equals(selectedBackpackProgramId)) {
  throw new Error(`MaterialPhysics has the wrong owner: ${existingInfo.owner.toBase58()}`);
}
const treasuryAuthority = payer.publicKey.equals(globalConfigState.developmentWallet);
const bootstrapAuthority = !existingInfo && payer.publicKey.equals(NICECHUNK_BOOTSTRAP_AUTHORITY);
if (!treasuryAuthority && !bootstrapAuthority) {
  throw new Error(
    `MaterialPhysics requires treasury authority ${globalConfigState.developmentWallet.toBase58()}${existingInfo ? "" : ` or one-time bootstrap authority ${NICECHUNK_BOOTSTRAP_AUTHORITY.toBase58()}`}, got ${payer.publicKey.toBase58()}.`,
  );
}
const existing = existingInfo ? decodeMaterialPhysicsTable(existingInfo.data) : null;
if (existing && rulesEqual(existing.rules, document.rules)) {
  console.log(JSON.stringify({
    status: "current",
    cluster: clusterUrl(),
    authority: payer.publicKey.toBase58(),
    programId: selectedBackpackProgramId.toBase58(),
    globalConfig: globalConfig.toBase58(),
    materialPhysics: materialPhysics.toBase58(),
    bump,
    revision: existing.revision,
    ruleCount: existing.ruleCount,
  }, null, 2));
  process.exit(0);
}

const requestedRevision = process.env.MATERIAL_PHYSICS_REVISION
  ? Number(process.env.MATERIAL_PHYSICS_REVISION)
  : null;
const revision = requestedRevision ?? Math.max(document.revision, (existing?.revision ?? 0) + 1);
if (!Number.isInteger(revision) || revision < 1 || revision > 0xffffffff) {
  throw new Error(`Invalid MaterialPhysics revision: ${revision}`);
}
if (existing && revision <= existing.revision) {
  throw new Error(`MaterialPhysics revision must exceed ${existing.revision}.`);
}

const instruction = createConfigureMaterialPhysicsInstruction({
  authority: payer.publicKey,
  revision,
  rules: document.rules,
  globalConfig,
  backpackProgramId: selectedBackpackProgramId,
  coreProgramId: selectedCoreProgramId,
});
if (process.env.DRY_RUN === "1") {
  console.log(JSON.stringify({
    status: "dry-run",
    cluster: clusterUrl(),
    authority: payer.publicKey.toBase58(),
    programId: selectedBackpackProgramId.toBase58(),
    globalConfig: globalConfig.toBase58(),
    materialPhysics: materialPhysics.toBase58(),
    bump,
    revision,
    ruleCount: document.rules.length,
    instructionBytes: instruction.data.length,
  }, null, 2));
  process.exit(0);
}

const signature = await sendAndConfirmTransaction(
  rpc,
  new Transaction().add(instruction),
  [payer],
  { commitment: "confirmed", preflightCommitment: "confirmed" },
);
const confirmedInfo = await rpc.getAccountInfo(materialPhysics, "confirmed");
if (!confirmedInfo) throw new Error("MaterialPhysics was not found after confirmation.");
const confirmed = decodeMaterialPhysicsTable(confirmedInfo.data);
if (confirmed.revision !== revision || !rulesEqual(confirmed.rules, document.rules)) {
  throw new Error("MaterialPhysics verification failed after confirmation.");
}

console.log(JSON.stringify({
  status: existing ? "updated" : "initialized",
  signature,
  cluster: clusterUrl(),
  authority: payer.publicKey.toBase58(),
  programId: selectedBackpackProgramId.toBase58(),
  globalConfig: globalConfig.toBase58(),
  materialPhysics: materialPhysics.toBase58(),
  bump,
  revision: confirmed.revision,
  ruleCount: confirmed.ruleCount,
}, null, 2));

function validateDocument(value: MaterialPhysicsDocument): void {
  if (value.schemaVersion !== 2
    || value.seed !== "material-physics-v2"
    || value.ruleCount !== value.rules?.length
    || value.rules.length < 1
    || value.rules.length > 128) {
    throw new Error(`Invalid MaterialPhysics rules document: ${rulePath}`);
  }
}

function rulesEqual(left: MaterialPhysicsRule[], right: MaterialPhysicsRule[]): boolean {
  if (left.length !== right.length) return false;
  const normalized = (rule: MaterialPhysicsRule): string => [
    rule.kind,
    rule.id,
    rule.densityKgM3,
    rule.standardVolumeMm3,
  ].join(":");
  return left.every((rule, index) => normalized(rule) === normalized(right[index]));
}
