import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

export const DEFAULT_DEVNET_URL = "https://solana-devnet.api.onfinality.io/public";
export const DEVNET_NCK_MINT = "HSnWF5kjkWVrceW2SaSskScuLveUZE4gpthZ2ZXRPQPo";
export const DEVNET_CORE_PROGRAM_ID = "9EhMCRYMJej1F21KzaA5Zao3khGGc5aJbDGbnxaogQHu";
export const DEVNET_PLAYER_PROGRAM_ID = "oeaRMVnPoV4iENnGCCtaEeRxU5be515s4YYe6aXQuKe";
export const DEVNET_CHUNK_PROGRAM_ID = "12rCvz9PZ64Uix1TCiHEGU4AN4ZS1h4jv5u7CkqTRdk5";
export const DEVNET_GUARDIAN_PROGRAM_ID = "6frJyJSirfEwsztsxijcJLe29LSaceET1wanXSFwPQyE";

export function clusterUrl(): string {
  return process.env.CLUSTER_URL ?? process.env.SOLANA_RPC_URL ?? DEFAULT_DEVNET_URL;
}

export function connection(): Connection {
  return new Connection(clusterUrl(), "confirmed");
}

export function programId(): PublicKey | undefined {
  return process.env.NICECHUNK_CORE_PROGRAM_ID
    ? new PublicKey(process.env.NICECHUNK_CORE_PROGRAM_ID)
    : undefined;
}

export function coreProgramId(): PublicKey {
  return new PublicKey(process.env.NICECHUNK_CORE_PROGRAM_ID ?? DEVNET_CORE_PROGRAM_ID);
}

export function playerProgramId(): PublicKey {
  return new PublicKey(process.env.NICECHUNK_PLAYER_PROGRAM_ID ?? DEVNET_PLAYER_PROGRAM_ID);
}

export function chunkProgramId(): PublicKey {
  return new PublicKey(process.env.NICECHUNK_CHUNK_PROGRAM_ID ?? DEVNET_CHUNK_PROGRAM_ID);
}

export function guardianProgramId(): PublicKey {
  return new PublicKey(process.env.NICECHUNK_GUARDIAN_PROGRAM_ID ?? DEVNET_GUARDIAN_PROGRAM_ID);
}

export function nckMint(): PublicKey {
  return new PublicKey(process.env.NCK_MINT ?? DEVNET_NCK_MINT);
}

export function readPayerKeypair(): Keypair {
  const keypairPath =
    process.env.PAYER_KEYPAIR ??
    process.env.ANCHOR_WALLET ??
    path.join(os.homedir(), ".config/solana/id.json");
  const resolved = keypairPath.startsWith("~")
    ? path.join(os.homedir(), keypairPath.slice(1))
    : keypairPath;
  if (!fs.existsSync(resolved)) {
    throw new Error(`Payer keypair not found: ${resolved}. Set PAYER_KEYPAIR to your devnet payer keypair path.`);
  }
  return Keypair.fromSecretKey(Uint8Array.from(JSON.parse(fs.readFileSync(resolved, "utf8"))));
}

export function requireEnvNckMintForSend(): void {
  if (!process.env.NCK_MINT) {
    throw new Error(
      `NCK_MINT is required before sending transactions. Current devnet NCK mint is ${DEVNET_NCK_MINT}. Export NCK_MINT=<mint> if you intentionally replace it.`,
    );
  }
}
