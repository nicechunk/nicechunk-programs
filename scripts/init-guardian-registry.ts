import {
  sendAndConfirmTransaction,
  Transaction,
} from "@solana/web3.js";
import {
  createAssociatedTokenAccountInstruction,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import {
  createInitializeGuardianRegistryInstruction,
  decodeGuardianRegistry,
  deriveGuardianRegistryPda,
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

const conn = connection();
const payer = readPayerKeypair();
const selectedCoreProgramId = coreProgramId();
const selectedGuardianProgramId = guardianProgramId();
const selectedNckMint = nckMint();
const [globalConfig] = deriveGlobalConfigPda(selectedCoreProgramId);
const [registry, registryBump] = deriveGuardianRegistryPda({
  globalConfig,
  programId: selectedGuardianProgramId,
});
const [treasuryAuthority, treasuryBump] = deriveGuardianTreasuryAuthorityPda({
  globalConfig,
  programId: selectedGuardianProgramId,
});
const treasuryNckToken = getAssociatedTokenAddressSync(selectedNckMint, treasuryAuthority, true);

if (process.env.DRY_RUN === "1") {
  console.log(JSON.stringify({
    status: "dry-run",
    payer: payer.publicKey.toBase58(),
    coreProgramId: selectedCoreProgramId.toBase58(),
    guardianProgramId: selectedGuardianProgramId.toBase58(),
    globalConfig: globalConfig.toBase58(),
    registry: registry.toBase58(),
    registryBump,
    treasuryAuthority: treasuryAuthority.toBase58(),
    treasuryBump,
    treasuryNckToken: treasuryNckToken.toBase58(),
    nckMint: selectedNckMint.toBase58(),
  }, null, 2));
  process.exit(0);
}

const existing = await conn.getAccountInfo(registry, "confirmed");
if (existing?.owner.equals(selectedGuardianProgramId)) {
  console.log(JSON.stringify({
    status: "exists",
    registry: registry.toBase58(),
    decoded: decodeGuardianRegistry(existing.data),
  }, (_key, value) => typeof value === "bigint" ? value.toString() : value, 2));
  process.exit(0);
}

const tx = new Transaction();
const treasuryTokenInfo = await conn.getAccountInfo(treasuryNckToken, "confirmed");
if (!treasuryTokenInfo) {
  tx.add(createAssociatedTokenAccountInstruction(
    payer.publicKey,
    treasuryNckToken,
    treasuryAuthority,
    selectedNckMint,
  ));
}
tx.add(createInitializeGuardianRegistryInstruction({
  payer: payer.publicKey,
  treasuryNckToken,
  guardianProgramId: selectedGuardianProgramId,
  coreProgramId: selectedCoreProgramId,
  nckMint: selectedNckMint,
}));

const signature = await sendAndConfirmTransaction(conn, tx, [payer], {
  commitment: "confirmed",
  preflightCommitment: "confirmed",
});
const account = await conn.getAccountInfo(registry, "confirmed");

console.log(JSON.stringify({
  status: "initialized",
  signature,
  registry: registry.toBase58(),
  treasuryAuthority: treasuryAuthority.toBase58(),
  treasuryNckToken: treasuryNckToken.toBase58(),
  decoded: account ? decodeGuardianRegistry(account.data) : null,
}, (_key, value) => typeof value === "bigint" ? value.toString() : value, 2));
