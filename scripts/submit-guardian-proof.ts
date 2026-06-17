import {
  sendAndConfirmTransaction,
  Transaction,
} from "@solana/web3.js";
import {
  createSubmitGuardianProofInstruction,
  decodeGuardianRegion,
  deriveGuardianRegionPda,
} from "../sdk/nicechunk-guardian.ts";
import { deriveGlobalConfigPda } from "../sdk/nicechunk-core.ts";
import {
  connection,
  coreProgramId,
  guardianProgramId,
  readPayerKeypair,
} from "./core-script-utils.ts";

function readNumber(name: string, fallback: number): number {
  const raw = process.env[name];
  if (raw === undefined || raw === "") return fallback;
  const value = Number(raw);
  if (!Number.isFinite(value)) throw new Error(`${name} must be a finite number`);
  return value;
}

const conn = connection();
const operator = readPayerKeypair();
const selectedCoreProgramId = coreProgramId();
const selectedGuardianProgramId = guardianProgramId();
const regionX = readNumber("REGION_X", 0);
const regionY = readNumber("REGION_Y", 0);
const [globalConfig] = deriveGlobalConfigPda(selectedCoreProgramId);
const [region] = deriveGuardianRegionPda({
  globalConfig,
  regionX,
  regionY,
  programId: selectedGuardianProgramId,
});

if (process.env.DRY_RUN === "1") {
  console.log(JSON.stringify({
    status: "dry-run",
    operator: operator.publicKey.toBase58(),
    region: region.toBase58(),
    regionX,
    regionY,
  }, null, 2));
  process.exit(0);
}

const ix = createSubmitGuardianProofInstruction({
  operator: operator.publicKey,
  regionX,
  regionY,
  guardianProgramId: selectedGuardianProgramId,
  coreProgramId: selectedCoreProgramId,
});
const signature = await sendAndConfirmTransaction(conn, new Transaction().add(ix), [operator], {
  commitment: "confirmed",
  preflightCommitment: "confirmed",
});
const account = await conn.getAccountInfo(region, "confirmed");

console.log(JSON.stringify({
  status: "proof-submitted",
  signature,
  region: region.toBase58(),
  decoded: account ? decodeGuardianRegion(account.data, region) : null,
}, (_key, value) => typeof value === "bigint" ? value.toString() : value, 2));
