import {
  sendAndConfirmTransaction,
  Transaction,
} from "@solana/web3.js";
import {
  createRecordBlockChangeInstruction,
  decodeChunkState,
  deriveChunkPda,
} from "../sdk/nicechunk-chunk.ts";
import { deriveGlobalConfigPda } from "../sdk/nicechunk-core.ts";
import {
  chunkProgramId,
  connection,
  coreProgramId,
  playerProgramId,
  readPayerKeypair,
} from "./core-script-utils.ts";

function readNumber(name: string, fallback: number): number {
  const raw = process.env[name];
  if (raw === undefined || raw === "") {
    return fallback;
  }
  const value = Number(raw);
  if (!Number.isFinite(value)) {
    throw new Error(`${name} must be a finite number`);
  }
  return value;
}

const conn = connection();
const payer = readPayerKeypair();
const selectedCoreProgramId = coreProgramId();
const selectedPlayerProgramId = playerProgramId();
const selectedChunkProgramId = chunkProgramId();
const [globalConfig] = deriveGlobalConfigPda(selectedCoreProgramId);

const change = {
  chunkX: readNumber("CHUNK_X", 0),
  chunkZ: readNumber("CHUNK_Z", 0),
  localX: readNumber("LOCAL_X", 0),
  y: readNumber("BLOCK_Y", 2),
  localZ: readNumber("LOCAL_Z", 0),
  previousBlockId: readNumber("PREVIOUS_BLOCK_ID", 1),
  newBlockId: readNumber("NEW_BLOCK_ID", 0),
  action: readNumber("ACTION", 1),
  toolSlot: readNumber("TOOL_SLOT", 0),
};
const [chunk, bump] = deriveChunkPda({
  globalConfig,
  chunkX: change.chunkX,
  chunkZ: change.chunkZ,
  programId: selectedChunkProgramId,
});

if (process.env.DRY_RUN === "1") {
  console.log(JSON.stringify({
    status: "dry-run",
    authority: payer.publicKey.toBase58(),
    coreProgramId: selectedCoreProgramId.toBase58(),
    playerProgramId: selectedPlayerProgramId.toBase58(),
    chunkProgramId: selectedChunkProgramId.toBase58(),
    globalConfig: globalConfig.toBase58(),
    chunk: chunk.toBase58(),
    bump,
    change,
  }, null, 2));
  process.exit(0);
}

const ix = createRecordBlockChangeInstruction({
  authority: payer.publicKey,
  change,
  chunkProgramId: selectedChunkProgramId,
  playerProgramId: selectedPlayerProgramId,
  coreProgramId: selectedCoreProgramId,
});
const tx = new Transaction().add(ix);
const signature = await sendAndConfirmTransaction(conn, tx, [payer], {
  commitment: "confirmed",
});
const account = await conn.getAccountInfo(chunk, "confirmed");

console.log(JSON.stringify({
  status: "recorded",
  signature,
  authority: payer.publicKey.toBase58(),
  chunk: chunk.toBase58(),
  bump,
  decoded: account ? decodeChunkState(account.data) : null,
}, (_key, value) => typeof value === "bigint" ? value.toString() : value, 2));
