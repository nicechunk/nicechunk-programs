import assert from "node:assert/strict";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import {
  createInitializePlayerInstruction,
  createSetPlayerNameInstruction,
  createUpsertPlayerAppearanceInstruction,
  derivePlayerProfilePda,
  deriveUsernameIndexPda,
  NICECHUNK_PLAYER_PROGRAM_ID,
} from "../sdk/nicechunk-player.ts";
import { deriveGlobalConfigPda } from "../sdk/nicechunk-core.ts";

describe("nicechunk player SDK protocol", () => {
  const owner = PublicKey.unique();
  const [globalConfig] = deriveGlobalConfigPda();

  it("derives the canonical case-insensitive username index", async () => {
    const [mixedCase] = await deriveUsernameIndexPda({ playerName: "Jerry_Miner" });
    const [lowerCase] = await deriveUsernameIndexPda({ playerName: "jerry_miner" });

    assert.equal(mixedCase.toBase58(), lowerCase.toBase58());
    await assert.rejects(
      deriveUsernameIndexPda({ playerName: "x".repeat(33) }),
      /max 32 characters/,
    );
  });

  it("includes the username index when initializing a named player", async () => {
    const [playerProfile] = derivePlayerProfilePda(owner, NICECHUNK_PLAYER_PROGRAM_ID);
    const [usernameIndex] = await deriveUsernameIndexPda({ playerName: "Jerry_Miner" });

    assert.throws(
      () => createInitializePlayerInstruction({ payer: owner, playerName: "Jerry_Miner" }),
      /Username index PDA is required/,
    );
    const instruction = createInitializePlayerInstruction({
      payer: owner,
      playerName: "Jerry_Miner",
      usernameIndex,
    });

    assert.equal(instruction.data.readUInt8(0), 0);
    assert.equal(instruction.keys.length, 5);
    assert.equal(instruction.keys[0].isSigner, true);
    assert.equal(instruction.keys[0].isWritable, true);
    assert.equal(instruction.keys[1].pubkey.toBase58(), playerProfile.toBase58());
    assert.equal(instruction.keys[2].pubkey.toBase58(), globalConfig.toBase58());
    assert.equal(instruction.keys[3].pubkey.toBase58(), SystemProgram.programId.toBase58());
    assert.equal(instruction.keys[4].pubkey.toBase58(), usernameIndex.toBase58());
    assert.equal(instruction.keys[4].isWritable, true);
  });

  it("includes the username index in name and appearance writes", async () => {
    const [usernameIndex] = await deriveUsernameIndexPda({ playerName: "Jerry_Miner" });
    const setName = createSetPlayerNameInstruction({
      authority: owner,
      playerName: "Jerry_Miner",
      usernameIndex,
    });
    const upsertAppearance = createUpsertPlayerAppearanceInstruction({
      authority: owner,
      displayName: "Jerry_Miner",
      modelCode: "NCM2:test-model",
      usernameIndex,
    });

    assert.equal(setName.keys.length, 5);
    assert.equal(setName.keys[0].isWritable, true);
    assert.equal(setName.keys[3].pubkey.toBase58(), SystemProgram.programId.toBase58());
    assert.equal(setName.keys[4].pubkey.toBase58(), usernameIndex.toBase58());
    assert.equal(upsertAppearance.keys.length, 6);
    assert.equal(upsertAppearance.keys[5].pubkey.toBase58(), usernameIndex.toBase58());
  });
});
