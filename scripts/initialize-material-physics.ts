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
import {
  createInitializeMaterialPhysicsInstruction,
  createReplaceMaterialPhysicsInstruction,
  decodeMaterialPhysics,
  deriveMaterialPhysicsPda,
  NICECHUNK_BACKPACK_PROGRAM_ID,
  NICECHUNK_BOOTSTRAP_AUTHORITY,
} from "../sdk/nicechunk-backpack.ts";
import { deriveGlobalConfigPda, NICECHUNK_CORE_PROGRAM_ID } from "../sdk/nicechunk-core.ts";
import { materialPhysicsRecords } from "./material-physics-rules.ts";

const GLOBAL_CONFIG_TREASURY_OFFSET = 53;

async function main(): Promise<void> {
  const options = parseArgs(process.argv.slice(2));
  const rpcUrl = options.url ?? process.env.SOLANA_RPC_URL ?? "https://api.devnet.solana.com";
  const backpackProgramId = new PublicKey(
    options.programId ?? process.env.NICECHUNK_BACKPACK_PROGRAM_ID ?? NICECHUNK_BACKPACK_PROGRAM_ID,
  );
  const connection = new Connection(rpcUrl, "confirmed");
  const [globalConfig] = deriveGlobalConfigPda(NICECHUNK_CORE_PROGRAM_ID);
  const [materialPhysics] = deriveMaterialPhysicsPda({ globalConfig, programId: backpackProgramId });
  const globalConfigAccount = await connection.getAccountInfo(globalConfig, "confirmed");
  if (!globalConfigAccount?.owner.equals(NICECHUNK_CORE_PROGRAM_ID)
    || !globalConfigAccount.data?.length
    || globalConfigAccount.data.length < GLOBAL_CONFIG_TREASURY_OFFSET + 32) {
    throw new Error("GlobalConfig is unavailable or invalid.");
  }
  const treasury = new PublicKey(
    globalConfigAccount.data.subarray(GLOBAL_CONFIG_TREASURY_OFFSET, GLOBAL_CONFIG_TREASURY_OFFSET + 32),
  );
  let account = await connection.getAccountInfo(materialPhysics, "confirmed");

  if (options.verifyOnly) {
    verifyMaterialPhysicsAccount(account, backpackProgramId, globalConfig, treasury);
    printResult({ backpackProgramId, globalConfig, materialPhysics, treasury, updated: false });
    return;
  }

  const authority = readAuthority(options.keypair);
  if (!authority.publicKey.equals(treasury)
    && !authority.publicKey.equals(NICECHUNK_BOOTSTRAP_AUTHORITY)) {
    throw new Error("The selected keypair is neither the GlobalConfig treasury nor the one-time bootstrap authority.");
  }
  if (options.dryRun) {
    printResult({ backpackProgramId, globalConfig, materialPhysics, treasury, updated: false, dryRun: true });
    return;
  }

  let updated = false;
  if (!account) {
    await submit(connection, authority, [
      createInitializeMaterialPhysicsInstruction({
        authority: authority.publicKey,
        globalConfig,
        backpackProgramId,
      }),
      createReplaceMaterialPhysicsInstruction({
        authority: authority.publicKey,
        globalConfig,
        records: [...materialPhysicsRecords],
        backpackProgramId,
      }),
    ], "initialize MaterialPhysics rules");
    updated = true;
    account = await connection.getAccountInfo(materialPhysics, "confirmed");
  } else {
    const current = verifyMaterialPhysicsAccount(account, backpackProgramId, globalConfig, treasury, false);
    if (!recordsEqual(current.records, materialPhysicsRecords)) {
      await submit(connection, authority, [createReplaceMaterialPhysicsInstruction({
        authority: authority.publicKey,
        globalConfig,
        records: [...materialPhysicsRecords],
        backpackProgramId,
      })], "replace MaterialPhysics records");
      updated = true;
      account = await connection.getAccountInfo(materialPhysics, "confirmed");
    }
  }

  verifyMaterialPhysicsAccount(account, backpackProgramId, globalConfig, treasury);
  printResult({ backpackProgramId, globalConfig, materialPhysics, treasury, updated });
}

function verifyMaterialPhysicsAccount(
  account: Awaited<ReturnType<Connection["getAccountInfo"]>>,
  backpackProgramId: PublicKey,
  globalConfig: PublicKey,
  treasury: PublicKey,
  requireRecords = true,
) {
  if (!account?.owner.equals(backpackProgramId)) {
    throw new Error("MaterialPhysics PDA is unavailable or owned by the wrong program.");
  }
  const decoded = decodeMaterialPhysics(Buffer.from(account.data));
  if (!decoded.initialized
    || !decoded.globalConfig.equals(globalConfig)
    || !decoded.authority.equals(treasury)
    || (requireRecords && !recordsEqual(decoded.records, materialPhysicsRecords))) {
    throw new Error("MaterialPhysics PDA does not match the canonical rules.");
  }
  return decoded;
}

function recordsEqual(
  left: readonly { materialId: number; densityKgM3: number }[],
  right: readonly { materialId: number; densityKgM3: number }[],
): boolean {
  return left.length === right.length && left.every((record, index) => (
    record.materialId === right[index].materialId
      && record.densityKgM3 === right[index].densityKgM3
  ));
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
    { commitment: "confirmed", preflightCommitment: "confirmed" },
  );
  console.log(`${label}: ${signature}`);
}

function readAuthority(pathValue?: string): Keypair {
  const keypairPath = resolve(
    pathValue
      ?? process.env.PAYER_KEYPAIR
      ?? process.env.ANCHOR_WALLET
      ?? process.env.SOLANA_KEYPAIR
      ?? `${homedir()}/.config/solana/id.json`,
  );
  return Keypair.fromSecretKey(Uint8Array.from(JSON.parse(readFileSync(keypairPath, "utf8"))));
}

function printResult({
  backpackProgramId,
  globalConfig,
  materialPhysics,
  treasury,
  updated,
  dryRun = false,
}: {
  backpackProgramId: PublicKey;
  globalConfig: PublicKey;
  materialPhysics: PublicKey;
  treasury: PublicKey;
  updated: boolean;
  dryRun?: boolean;
}): void {
  console.log(JSON.stringify({
    backpackProgramId: backpackProgramId.toBase58(),
    globalConfig: globalConfig.toBase58(),
    materialPhysics: materialPhysics.toBase58(),
    treasury: treasury.toBase58(),
    recordCount: materialPhysicsRecords.length,
    updated,
    dryRun,
  }, null, 2));
}

function parseArgs(values: string[]): {
  url?: string;
  keypair?: string;
  programId?: string;
  verifyOnly: boolean;
  dryRun: boolean;
} {
  const options: {
    url?: string;
    keypair?: string;
    programId?: string;
    verifyOnly: boolean;
    dryRun: boolean;
  } = { verifyOnly: false, dryRun: false };
  for (let index = 0; index < values.length; index += 1) {
    const value = values[index];
    if (value === "--url") options.url = values[++index];
    else if (value === "--keypair") options.keypair = values[++index];
    else if (value === "--program-id") options.programId = values[++index];
    else if (value === "--verify-only") options.verifyOnly = true;
    else if (value === "--dry-run") options.dryRun = true;
    else throw new Error(`Unknown argument: ${value}`);
  }
  if (options.verifyOnly && options.dryRun) {
    throw new Error("--verify-only and --dry-run cannot be combined.");
  }
  return options;
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
