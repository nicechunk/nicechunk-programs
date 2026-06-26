import {
  sendAndConfirmTransaction,
  Transaction,
} from "@solana/web3.js";
import {
  createMineBlockInstruction,
  decodeChunkBrokenState,
  deriveChunkBrokenPda,
  generatedBlockIdAt,
} from "../sdk/nicechunk-chunk.ts";
import { decodeGlobalConfig, deriveGlobalConfigPda } from "../sdk/nicechunk-core.ts";
import {
  chunkProgramId,
  connection,
  coreProgramId,
  readPayerKeypair,
} from "./core-script-utils.ts";

function readNumber(name: string, fallback: number): number {
  const raw = process.env[name];
  if (raw === undefined || raw === "") return fallback;
  const value = Number(raw);
  if (!Number.isFinite(value)) {
    throw new Error(`${name} must be a finite number`);
  }
  return value;
}

const conn = connection();
const payer = readPayerKeypair();
const selectedCoreProgramId = coreProgramId();
const selectedChunkProgramId = chunkProgramId();
const [globalConfig] = deriveGlobalConfigPda(selectedCoreProgramId);
const globalConfigAccount = await conn.getAccountInfo(globalConfig, "confirmed");
if (!globalConfigAccount) throw new Error(`GlobalConfig account not found: ${globalConfig.toBase58()}`);
const decodedGlobalConfig = decodeGlobalConfig(globalConfigAccount.data);
const worldX = readNumber("WORLD_X", 0);
const worldY = readNumber("WORLD_Y", decodedGlobalConfig.seaLevel);
const worldZ = readNumber("WORLD_Z", 0);
const chunkSize = decodedGlobalConfig.chunkSize;
const chunkX = Math.floor(worldX / chunkSize);
const chunkZ = Math.floor(worldZ / chunkSize);
const expectedBlockId = generatedBlockIdAt(decodedGlobalConfig, {
  chunkX,
  chunkZ,
  localX: ((worldX % chunkSize) + chunkSize) % chunkSize,
  y: worldY,
  localZ: ((worldZ % chunkSize) + chunkSize) % chunkSize,
});
const [chunkBroken, bump] = deriveChunkBrokenPda({
  globalConfig,
  chunkX,
  chunkZ,
  programId: selectedChunkProgramId,
});

if (process.env.DRY_RUN === "1") {
  console.log(JSON.stringify({
    status: "dry-run",
    payer: payer.publicKey.toBase58(),
    owner: payer.publicKey.toBase58(),
    sessionAuthority: payer.publicKey.toBase58(),
    coreProgramId: selectedCoreProgramId.toBase58(),
    chunkProgramId: selectedChunkProgramId.toBase58(),
    globalConfig: globalConfig.toBase58(),
    chunkBroken: chunkBroken.toBase58(),
    bump,
    worldX,
    worldY,
    worldZ,
    chunkX,
    chunkZ,
    expectedBlockId,
  }, null, 2));
  process.exit(0);
}

const ix = createMineBlockInstruction({
  payer: payer.publicKey,
  owner: payer.publicKey,
  sessionAuthority: payer.publicKey,
  block: { worldX, worldY, worldZ, expectedBlockId },
  chunkProgramId: selectedChunkProgramId,
  coreProgramId: selectedCoreProgramId,
  chunkSize,
});
const signature = await sendAndConfirmTransaction(conn, new Transaction().add(ix), [payer], {
  commitment: "confirmed",
});
const account = await conn.getAccountInfo(chunkBroken, "confirmed");

console.log(JSON.stringify({
  status: "mined",
  signature,
  payer: payer.publicKey.toBase58(),
  chunkBroken: chunkBroken.toBase58(),
  bump,
  decoded: account
    ? decodeChunkBrokenState({ data: account.data, chunkX, chunkZ, chunkSize })
    : null,
}, null, 2));
