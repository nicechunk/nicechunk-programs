import {
  sendAndConfirmTransaction,
  Transaction,
} from "@solana/web3.js";
import {
  createVerifyGeneratedBlockInstruction,
  generatedBlockIdAt,
  generatedSurfaceHeight,
  VERIFY_GENERATED_BLOCK_INSPECT_ONLY,
} from "../sdk/nicechunk-chunk.ts";
import {
  decodeGlobalConfig,
  deriveGlobalConfigPda,
} from "../sdk/nicechunk-core.ts";
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
  if (!Number.isFinite(value)) throw new Error(`${name} must be a finite number`);
  return value;
}

const conn = connection();
const shouldSend = process.env.SEND === "1";
const payer = shouldSend ? readPayerKeypair() : null;
const selectedCoreProgramId = coreProgramId();
const selectedChunkProgramId = chunkProgramId();
const [globalConfig] = deriveGlobalConfigPda(selectedCoreProgramId);
const globalConfigAccount = await conn.getAccountInfo(globalConfig, "confirmed");

if (!globalConfigAccount) {
  throw new Error(`GlobalConfig account not found: ${globalConfig.toBase58()}`);
}

const decodedGlobalConfig = decodeGlobalConfig(globalConfigAccount.data);
const block = {
  chunkX: readNumber("CHUNK_X", 0),
  chunkZ: readNumber("CHUNK_Z", 0),
  localX: readNumber("LOCAL_X", 0),
  y: readNumber("BLOCK_Y", decodedGlobalConfig.seaLevel),
  localZ: readNumber("LOCAL_Z", 0),
  expectedBlockId: readNumber("EXPECTED_BLOCK_ID", VERIFY_GENERATED_BLOCK_INSPECT_ONLY),
};
const worldX = block.chunkX * decodedGlobalConfig.chunkSize + block.localX;
const worldZ = block.chunkZ * decodedGlobalConfig.chunkSize + block.localZ;
const localExpected = generatedBlockIdAt(decodedGlobalConfig, block);
const localSurfaceY = generatedSurfaceHeight(decodedGlobalConfig, worldX, worldZ);
const ix = createVerifyGeneratedBlockInstruction({
  block,
  chunkProgramId: selectedChunkProgramId,
  coreProgramId: selectedCoreProgramId,
});
const tx = new Transaction().add(ix);
const { blockhash } = await conn.getLatestBlockhash("confirmed");
tx.recentBlockhash = blockhash;
tx.feePayer = payer?.publicKey ?? decodedGlobalConfig.developmentWallet;
if (payer) tx.sign(payer);

const simulation = shouldSend
  ? await conn.simulateTransaction(tx, [payer!])
  : await conn.simulateTransaction(tx);
const result = {
  status: simulation.value.err ? "simulation-failed" : "simulation-ok",
  mode: shouldSend ? "send" : "simulate",
  feePayer: tx.feePayer.toBase58(),
  coreProgramId: selectedCoreProgramId.toBase58(),
  chunkProgramId: selectedChunkProgramId.toBase58(),
  globalConfig: globalConfig.toBase58(),
  block,
  world: { x: worldX, y: block.y, z: worldZ },
  localSurfaceY,
  localExpectedBlockId: localExpected,
  expectedMatchesLocal:
    block.expectedBlockId === VERIFY_GENERATED_BLOCK_INSPECT_ONLY ||
    block.expectedBlockId === localExpected,
  simulationError: simulation.value.err,
  logs: simulation.value.logs,
};

if (shouldSend) {
  if (simulation.value.err) {
    console.log(JSON.stringify(result, null, 2));
    process.exit(1);
  }
  const signature = await sendAndConfirmTransaction(conn, tx, [payer!], {
    commitment: "confirmed",
  });
  console.log(JSON.stringify({ ...result, status: "sent", signature }, null, 2));
} else {
  console.log(JSON.stringify(result, null, 2));
  if (simulation.value.err) process.exit(1);
}
