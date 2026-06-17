import {
  PublicKey,
  sendAndConfirmTransaction,
  Transaction,
} from "@solana/web3.js";
import {
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import {
  chunkToGuardianRegion,
  createRegisterGuardianInstruction,
  decodeGuardianRegion,
  deriveGuardianRegionPda,
  deriveGuardianTreasuryAuthorityPda,
} from "../sdk/nicechunk-guardian.ts";
import { deriveGlobalConfigPda } from "../sdk/nicechunk-core.ts";
import {
  connection,
  coreProgramId,
  guardianProgramId,
  nckMint,
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
const payer = readPayerKeypair();
const selectedCoreProgramId = coreProgramId();
const selectedGuardianProgramId = guardianProgramId();
const selectedNckMint = nckMint();
const [globalConfig] = deriveGlobalConfigPda(selectedCoreProgramId);
const [treasuryAuthority] = deriveGuardianTreasuryAuthorityPda({
  globalConfig,
  programId: selectedGuardianProgramId,
});
const treasuryNckToken = getAssociatedTokenAddressSync(selectedNckMint, treasuryAuthority, true);
const chunkX = readNumber("CHUNK_X", 0);
const chunkY = readNumber("CHUNK_Y", 0);
const regionX = process.env.REGION_X ? readNumber("REGION_X", 0) : chunkToGuardianRegion(chunkX);
const regionY = process.env.REGION_Y ? readNumber("REGION_Y", 0) : chunkToGuardianRegion(chunkY);
const host = process.env.GUARDIAN_HOST ?? "127.0.0.1";
const port = readNumber("GUARDIAN_PORT", 8899);
const useTls = process.env.GUARDIAN_USE_TLS === "1";
const operator = process.env.GUARDIAN_OPERATOR
  ? new PublicKey(process.env.GUARDIAN_OPERATOR)
  : payer.publicKey;
const isGenesis = process.env.GUARDIAN_GENESIS === "1";
const ownerNckToken = getAssociatedTokenAddressSync(selectedNckMint, payer.publicKey);
const [region, bump] = deriveGuardianRegionPda({
  globalConfig,
  regionX,
  regionY,
  programId: selectedGuardianProgramId,
});

if (process.env.DRY_RUN === "1") {
  console.log(JSON.stringify({
    status: "dry-run",
    payer: payer.publicKey.toBase58(),
    owner: payer.publicKey.toBase58(),
    operator: operator.toBase58(),
    guardianProgramId: selectedGuardianProgramId.toBase58(),
    globalConfig: globalConfig.toBase58(),
    region: region.toBase58(),
    bump,
    chunkX,
    chunkY,
    regionX,
    regionY,
    host,
    port,
    useTls,
    isGenesis,
    ownerNckToken: ownerNckToken.toBase58(),
    treasuryNckToken: treasuryNckToken.toBase58(),
  }, null, 2));
  process.exit(0);
}

const ix = createRegisterGuardianInstruction({
  payer: payer.publicKey,
  owner: payer.publicKey,
  ownerNckToken,
  treasuryNckToken,
  regionX,
  regionY,
  host,
  port,
  useTls,
  operator,
  isGenesis,
  guardianProgramId: selectedGuardianProgramId,
  coreProgramId: selectedCoreProgramId,
  nckMint: selectedNckMint,
});
const signature = await sendAndConfirmTransaction(conn, new Transaction().add(ix), [payer], {
  commitment: "confirmed",
  preflightCommitment: "confirmed",
});
const account = await conn.getAccountInfo(region, "confirmed");

console.log(JSON.stringify({
  status: "registered",
  signature,
  region: region.toBase58(),
  bump,
  decoded: account ? decodeGuardianRegion(account.data, region) : null,
}, (_key, value) => typeof value === "bigint" ? value.toString() : value, 2));
