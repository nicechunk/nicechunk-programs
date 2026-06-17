import {
  sendAndConfirmTransaction,
  Transaction,
} from "@solana/web3.js";
import {
  createMineBlockInstruction,
  decodeChunkBrokenState,
  deriveChunkBrokenPda,
  VERIFY_GENERATED_BLOCK_INSPECT_ONLY,
} from "../sdk/nicechunk-chunk.ts";
import { deriveGlobalConfigPda } from "../sdk/nicechunk-core.ts";
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
const worldX = readNumber("WORLD_X", 0);
const worldY = readNumber("WORLD_Y", 2);
const worldZ = readNumber("WORLD_Z", 0);
const chunkSize = readNumber("CHUNK_SIZE", 16);
const chunkX = Math.floor(worldX / chunkSize);
const chunkZ = Math.floor(worldZ / chunkSize);
const expectedBlockId = readNumber("EXPECTED_BLOCK_ID", VERIFY_GENERATED_BLOCK_INSPECT_ONLY);
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
    ? decodeChunkBrokenState({
      data: account.data,
      chunkX,
      chunkZ,
      chunkSize,
    })
    : null,
}, null, 2));
