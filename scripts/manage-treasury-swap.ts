import { lstat, readFile } from "node:fs/promises";
import { resolve } from "node:path";

import { getAssociatedTokenAddressSync, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import {
  Connection,
  Keypair,
  PublicKey,
  Transaction,
  type TransactionInstruction,
} from "@solana/web3.js";
import {
  NICECHUNK_DEVNET_NCK_MINT,
  NICECHUNK_GAME_PROGRAM_ID,
  NICECHUNK_MARKET_TREASURY,
  createConfigureTreasurySwapInstruction,
  createInitializeTreasurySwapInstruction,
  createTreasurySwapNckLiquidityInstruction,
  createTreasurySwapSolLiquidityInstruction,
  decodeTreasurySwapState,
  deriveTreasurySwapPdas,
  quoteTreasurySwap,
  type TreasurySwapConfig,
} from "../sdk/nicechunk-market.ts";

const DEVNET_GENESIS_HASH = "EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG";
const EXECUTION_CONFIRMATION = NICECHUNK_MARKET_TREASURY.toBase58();
const ACTIVATION_CONFIRMATION = "ENABLE_FIXED_RATE_SWAP";
const U64_MAX = 0xffff_ffff_ffff_ffffn;
const BPS_DENOMINATOR = 10_000n;

interface ParsedOptions {
  rpcUrl: string;
  keypairPath: string;
  config: TreasurySwapConfig & {
    lamportsPerNck: bigint;
    minimumNckUnits: bigint;
    maximumNckUnits: bigint;
  };
  depositSolLamports: bigint;
  depositNckUnits: bigint;
  execute: boolean;
  activate: boolean;
  confirmAdmin: string;
  confirmActivation: string;
}

interface TransactionPhase {
  name: string;
  instructions: TransactionInstruction[];
}

const options = parseArguments(process.argv.slice(2));
const connection = new Connection(options.rpcUrl, "confirmed");
const genesisHash = await connection.getGenesisHash();
if (genesisHash !== DEVNET_GENESIS_HASH) {
  throw new Error(`Treasury Swap administration is Devnet-only; received genesis ${genesisHash}.`);
}

const programAccount = await connection.getAccountInfo(NICECHUNK_GAME_PROGRAM_ID, "confirmed");
if (!programAccount?.executable) {
  throw new Error(`Unified Game program ${NICECHUNK_GAME_PROGRAM_ID.toBase58()} is not executable on Devnet.`);
}

const pdas = deriveTreasurySwapPdas();
const treasuryNckToken = getAssociatedTokenAddressSync(
  NICECHUNK_DEVNET_NCK_MINT,
  NICECHUNK_MARKET_TREASURY,
);
const [stateAccount, solVaultAccount, nckVaultAccount, solVaultRentLamports] = await Promise.all([
  connection.getAccountInfo(pdas.state[0], "confirmed"),
  connection.getAccountInfo(pdas.solVault[0], "confirmed"),
  connection.getAccountInfo(pdas.nckVault[0], "confirmed"),
  connection.getMinimumBalanceForRentExemption(0, "confirmed"),
]);

const existingState = stateAccount?.data?.length
  ? decodeTreasurySwapState(stateAccount.data)
  : null;
if (stateAccount && !existingState) {
  throw new Error("Treasury Swap state exists but is not decodable.");
}
if (existingState && !stateAccount?.owner.equals(NICECHUNK_GAME_PROGRAM_ID)) {
  throw new Error("Treasury Swap state has an unexpected owner.");
}
if ((!existingState && (solVaultAccount || nckVaultAccount))
  || (existingState && (!solVaultAccount || !nckVaultAccount))) {
  throw new Error("Treasury Swap initialization is incomplete; refusing to repair it implicitly.");
}

const currentSolLiquidity = solVaultAccount
  ? BigInt(solVaultAccount.lamports) - BigInt(solVaultRentLamports)
  : 0n;
const currentNckLiquidity = nckVaultAccount
  ? decodeNckVaultAmount(nckVaultAccount, pdas.authority[0])
  : 0n;
const projectedSolLiquidity = nonnegative(currentSolLiquidity) + options.depositSolLamports;
const projectedNckLiquidity = currentNckLiquidity + options.depositNckUnits;
const requiredSolLiquidity = quoteTreasurySwap({
  direction: "NCK_TO_SOL",
  amountIn: options.config.maximumNckUnits,
  state: options.config,
}).amountOut;
const requiredNckLiquidity = options.config.maximumNckUnits
  * (BPS_DENOMINATOR - BigInt(options.config.feeBps))
  / BPS_DENOMINATOR;

if (options.activate
  && (projectedSolLiquidity < requiredSolLiquidity || projectedNckLiquidity < requiredNckLiquidity)) {
  throw new Error(
    `Activation requires at least ${requiredSolLiquidity} liquid lamports and ${requiredNckLiquidity} NCK base units; `
    + `projected reserves are ${projectedSolLiquidity} and ${projectedNckLiquidity}.`,
  );
}

const phases: TransactionPhase[] = [];
if (!existingState) {
  phases.push({
    name: "initialize-paused",
    instructions: [createInitializeTreasurySwapInstruction({
      admin: NICECHUNK_MARKET_TREASURY,
      config: options.config,
    })],
  });
} else if (!existingState.paused || !sameConfig(existingState, options.config)) {
  phases.push({
    name: "pause-and-configure",
    instructions: [createConfigureTreasurySwapInstruction({
      admin: NICECHUNK_MARKET_TREASURY,
      config: options.config,
      paused: true,
    })],
  });
}

const fundingInstructions: TransactionInstruction[] = [];
if (options.depositSolLamports > 0n) {
  fundingInstructions.push(createTreasurySwapSolLiquidityInstruction({
    admin: NICECHUNK_MARKET_TREASURY,
    amountLamports: options.depositSolLamports,
  }));
}
if (options.depositNckUnits > 0n) {
  const treasuryTokenAccount = await connection.getAccountInfo(treasuryNckToken, "confirmed");
  if (!treasuryTokenAccount) {
    throw new Error(`Treasury NCK token account ${treasuryNckToken.toBase58()} does not exist.`);
  }
  fundingInstructions.push(createTreasurySwapNckLiquidityInstruction({
    admin: NICECHUNK_MARKET_TREASURY,
    adminNckToken: treasuryNckToken,
    amountNckUnits: options.depositNckUnits,
  }));
}
if (fundingInstructions.length) phases.push({ name: "fund-reserves", instructions: fundingInstructions });
if (options.activate) {
  phases.push({
    name: "activate",
    instructions: [createConfigureTreasurySwapInstruction({
      admin: NICECHUNK_MARKET_TREASURY,
      config: options.config,
      paused: false,
    })],
  });
}

const plan = {
  mode: options.execute ? "execute" : "dry-run",
  network: "devnet",
  genesisHash,
  programId: NICECHUNK_GAME_PROGRAM_ID.toBase58(),
  admin: NICECHUNK_MARKET_TREASURY.toBase58(),
  nckMint: NICECHUNK_DEVNET_NCK_MINT.toBase58(),
  pdas: {
    state: pdas.state[0].toBase58(),
    authority: pdas.authority[0].toBase58(),
    solVault: pdas.solVault[0].toBase58(),
    nckVault: pdas.nckVault[0].toBase58(),
  },
  state: existingState ? { exists: true, paused: existingState.paused, revision: existingState.revision.toString() } : { exists: false },
  config: stringifyBigInts(options.config),
  reserves: {
    currentSolLamports: nonnegative(currentSolLiquidity).toString(),
    currentNckUnits: currentNckLiquidity.toString(),
    depositSolLamports: options.depositSolLamports.toString(),
    depositNckUnits: options.depositNckUnits.toString(),
    projectedSolLamports: projectedSolLiquidity.toString(),
    projectedNckUnits: projectedNckLiquidity.toString(),
    activationRequiredSolLamports: requiredSolLiquidity.toString(),
    activationRequiredNckUnits: requiredNckLiquidity.toString(),
  },
  activate: options.activate,
  phases: phases.map((phase) => ({ name: phase.name, instructionCount: phase.instructions.length })),
};
console.log(JSON.stringify(plan, null, 2));

if (!options.execute) {
  console.log("Dry-run only. No transaction was signed or sent.");
  process.exit(0);
}
if (options.confirmAdmin !== EXECUTION_CONFIRMATION) {
  throw new Error(`Execution requires --confirm-admin ${EXECUTION_CONFIRMATION}.`);
}
if (options.activate && options.confirmActivation !== ACTIVATION_CONFIRMATION) {
  throw new Error(`Activation requires --confirm-activation ${ACTIVATION_CONFIRMATION}.`);
}
const admin = await loadSecureKeypair(options.keypairPath);
if (!admin.publicKey.equals(NICECHUNK_MARKET_TREASURY)) {
  throw new Error(`Keypair must match Treasury Swap admin ${EXECUTION_CONFIRMATION}.`);
}

const signatures: Array<{ phase: string; signature: string }> = [];
for (const phase of phases) {
  const transaction = new Transaction().add(...phase.instructions);
  transaction.feePayer = admin.publicKey;
  const latest = await connection.getLatestBlockhash("confirmed");
  transaction.recentBlockhash = latest.blockhash;
  transaction.lastValidBlockHeight = latest.lastValidBlockHeight;
  transaction.sign(admin);
  const simulation = await connection.simulateTransaction(transaction, {
    commitment: "confirmed",
    sigVerify: true,
  });
  if (simulation.value.err) {
    throw new Error(`${phase.name} simulation failed: ${JSON.stringify(simulation.value.err)}\n${(simulation.value.logs || []).join("\n")}`);
  }
  const signature = await connection.sendRawTransaction(transaction.serialize(), {
    preflightCommitment: "confirmed",
    skipPreflight: false,
    maxRetries: 3,
  });
  const confirmation = await connection.confirmTransaction({ signature, ...latest }, "confirmed");
  if (confirmation.value.err) {
    throw new Error(`${phase.name} confirmation failed: ${JSON.stringify(confirmation.value.err)}`);
  }
  signatures.push({ phase: phase.name, signature });
}
console.log(JSON.stringify({ status: phases.length ? "confirmed" : "no-op", signatures }, null, 2));

function parseArguments(args: string[]): ParsedOptions {
  if (args.includes("--help")) {
    console.log(usage());
    process.exit(0);
  }
  const values = new Map<string, string>();
  const flags = new Set<string>();
  for (let index = 0; index < args.length; index += 1) {
    const argument = args[index];
    if (argument === "--execute" || argument === "--activate") {
      flags.add(argument);
      continue;
    }
    if (!argument.startsWith("--") || index + 1 >= args.length || args[index + 1].startsWith("--")) {
      throw new Error(`Invalid argument ${argument}.\n${usage()}`);
    }
    values.set(argument, args[index + 1]);
    index += 1;
  }
  const rpcUrl = values.get("--rpc-url") || process.env.SOLANA_RPC_URL || "";
  if (!rpcUrl) throw new Error(`--rpc-url is required.\n${usage()}`);
  const config = {
    lamportsPerNck: requiredU64(values, "--lamports-per-nck", false),
    minimumNckUnits: requiredU64(values, "--minimum-nck-units", false),
    maximumNckUnits: requiredU64(values, "--maximum-nck-units", false),
    feeBps: Number(requiredU64(values, "--fee-bps", true)),
  };
  if (config.maximumNckUnits < config.minimumNckUnits || config.feeBps > 1_000) {
    throw new Error("Invalid Treasury Swap limits or fee; fee must be 0..1000 bps and maximum must cover minimum.");
  }
  return {
    rpcUrl,
    keypairPath: values.get("--keypair") || "",
    config,
    depositSolLamports: optionalU64(values.get("--deposit-sol-lamports")),
    depositNckUnits: optionalU64(values.get("--deposit-nck-units")),
    execute: flags.has("--execute"),
    activate: flags.has("--activate"),
    confirmAdmin: values.get("--confirm-admin") || "",
    confirmActivation: values.get("--confirm-activation") || "",
  };
}

function requiredU64(values: Map<string, string>, name: string, allowZero: boolean): bigint {
  const raw = values.get(name);
  if (raw == null) throw new Error(`${name} is required.\n${usage()}`);
  return parseU64(raw, name, allowZero);
}

function optionalU64(raw: string | undefined): bigint {
  return raw == null ? 0n : parseU64(raw, "reserve amount", true);
}

function parseU64(raw: string, label: string, allowZero: boolean): bigint {
  if (!/^[0-9]+$/.test(raw)) throw new Error(`${label} must be an integer in base units.`);
  const value = BigInt(raw);
  if ((!allowZero && value === 0n) || value > U64_MAX) throw new Error(`${label} is outside u64 range.`);
  return value;
}

function sameConfig(
  state: ReturnType<typeof decodeTreasurySwapState>,
  config: ParsedOptions["config"],
): boolean {
  return state.lamportsPerNck === config.lamportsPerNck
    && state.minimumNckUnits === config.minimumNckUnits
    && state.maximumNckUnits === config.maximumNckUnits
    && state.feeBps === config.feeBps;
}

function decodeNckVaultAmount(
  account: Awaited<ReturnType<Connection["getAccountInfo"]>>,
  authority: PublicKey,
): bigint {
  if (!account
    || !account.owner.equals(TOKEN_PROGRAM_ID)
    || account.data.length < 165
    || !account.data.subarray(0, 32).equals(NICECHUNK_DEVNET_NCK_MINT.toBuffer())
    || !account.data.subarray(32, 64).equals(authority.toBuffer())) {
    throw new Error("Treasury Swap NCK vault is invalid.");
  }
  let amount = 0n;
  for (let index = 71; index >= 64; index -= 1) {
    amount = (amount << 8n) | BigInt(account.data[index]);
  }
  return amount;
}

async function loadSecureKeypair(path: string): Promise<Keypair> {
  if (!path) throw new Error("--keypair is required with --execute.");
  const absolutePath = resolve(path);
  const metadata = await lstat(absolutePath);
  if (!metadata.isFile() || metadata.isSymbolicLink() || (metadata.mode & 0o077) !== 0) {
    throw new Error("Treasury keypair must be a regular, non-symlink file with no group/world permissions.");
  }
  const bytes = JSON.parse(await readFile(absolutePath, "utf8"));
  if (!Array.isArray(bytes) || bytes.length !== 64 || bytes.some((value) => !Number.isInteger(value) || value < 0 || value > 255)) {
    throw new Error("Treasury keypair file is invalid.");
  }
  return Keypair.fromSecretKey(Uint8Array.from(bytes));
}

function stringifyBigInts(value: object): object {
  return Object.fromEntries(Object.entries(value).map(([key, entry]) => [
    key,
    typeof entry === "bigint" ? entry.toString() : entry,
  ]));
}

function nonnegative(value: bigint): bigint {
  return value > 0n ? value : 0n;
}

function usage(): string {
  return [
    "Usage: node scripts/manage-treasury-swap.ts [options]",
    "",
    "Required economic inputs (integer base units):",
    "  --rpc-url URL",
    "  --lamports-per-nck N       1 NCK price in lamports",
    "  --minimum-nck-units N      minimum NCK-side size (1 NCK = 1000000)",
    "  --maximum-nck-units N      maximum NCK-side size",
    "  --fee-bps N                0..1000",
    "",
    "Optional funding and activation:",
    "  --deposit-sol-lamports N",
    "  --deposit-nck-units N",
    "  --activate                 enable only after projected reserves cover max trade",
    "",
    "Dry-run is the default. Execution additionally requires:",
    "  --execute --keypair PATH",
    `  --confirm-admin ${EXECUTION_CONFIRMATION}`,
    `  --confirm-activation ${ACTIVATION_CONFIRMATION}  (only with --activate)`,
  ].join("\n");
}
