import { Transaction, sendAndConfirmTransaction } from "@solana/web3.js";
import {
  createInitializeGlobalConfigInstruction,
  deriveGlobalConfigPda,
  NICECHUNK_CORE_PROGRAM_ID,
} from "../sdk/nicechunk-core.ts";
import {
  clusterUrl,
  connection,
  nckMint,
  programId,
  readPayerKeypair,
  requireEnvNckMintForSend,
} from "./core-script-utils.ts";

requireEnvNckMintForSend();

const selectedProgramId = programId() ?? NICECHUNK_CORE_PROGRAM_ID;
const selectedNckMint = nckMint();
const payer = readPayerKeypair();
const rpc = connection();
const [globalConfig] = deriveGlobalConfigPda(selectedProgramId);
const dryRun = process.env.DRY_RUN === "1";

console.log(`Cluster URL: ${clusterUrl()}`);
console.log(`Payer: ${payer.publicKey.toBase58()}`);
console.log(`Program: ${selectedProgramId.toBase58()}`);
console.log(`NCK Mint: ${selectedNckMint.toBase58()}`);
console.log(`GlobalConfig PDA: ${globalConfig.toBase58()}`);

const ix = createInitializeGlobalConfigInstruction({
  payer: payer.publicKey,
  nckMint: selectedNckMint,
  programId: selectedProgramId,
});

if (dryRun) {
  console.log(`Dry run only. Instruction keys: ${ix.keys.length}`);
  process.exit(0);
}

const existing = await rpc.getAccountInfo(globalConfig, "confirmed");
if (existing?.owner.equals(selectedProgramId)) {
  console.log("GlobalConfig already initialized.");
  process.exit(0);
}

const signature = await sendAndConfirmTransaction(rpc, new Transaction().add(ix), [payer], {
  commitment: "confirmed",
  preflightCommitment: "confirmed",
});

console.log(`Initialized GlobalConfig. Signature: ${signature}`);
