import {
  sendAndConfirmTransaction,
  Transaction,
} from "@solana/web3.js";
import {
  createInitializePlayerInstruction,
  decodePlayerProfile,
  derivePlayerProfilePda,
} from "../sdk/nicechunk-player.ts";
import {
  connection,
  coreProgramId,
  playerProgramId,
  readPayerKeypair,
} from "./core-script-utils.ts";

const conn = connection();
const payer = readPayerKeypair();
const selectedPlayerProgramId = playerProgramId();
const selectedCoreProgramId = coreProgramId();
const [playerProfile, bump] = derivePlayerProfilePda(payer.publicKey, selectedPlayerProgramId);

if (process.env.DRY_RUN === "1") {
  console.log(JSON.stringify({
    status: "dry-run",
    owner: payer.publicKey.toBase58(),
    playerProgramId: selectedPlayerProgramId.toBase58(),
    coreProgramId: selectedCoreProgramId.toBase58(),
    playerProfile: playerProfile.toBase58(),
    bump,
  }, null, 2));
  process.exit(0);
}

const existing = await conn.getAccountInfo(playerProfile, "confirmed");
if (existing) {
  console.log(JSON.stringify({
    status: "exists",
    owner: payer.publicKey.toBase58(),
    playerProgramId: selectedPlayerProgramId.toBase58(),
    coreProgramId: selectedCoreProgramId.toBase58(),
    playerProfile: playerProfile.toBase58(),
    bump,
    decoded: decodePlayerProfile(existing.data).owner.toBase58(),
  }, null, 2));
  process.exit(0);
}

const ix = createInitializePlayerInstruction({
  payer: payer.publicKey,
  playerProgramId: selectedPlayerProgramId,
  coreProgramId: selectedCoreProgramId,
});
const tx = new Transaction().add(ix);
const signature = await sendAndConfirmTransaction(conn, tx, [payer], {
  commitment: "confirmed",
});

console.log(JSON.stringify({
  status: "initialized",
  signature,
  owner: payer.publicKey.toBase58(),
  playerProfile: playerProfile.toBase58(),
  bump,
}, null, 2));
