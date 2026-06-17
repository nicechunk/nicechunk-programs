import { PublicKey, SystemProgram } from "@solana/web3.js";
import assert from "node:assert";
import {
  createOrRefreshPlayerSessionInstruction,
  createInitializePlayerInstruction,
  createSetEquippedBackpackInstruction,
  createSetEquipmentSlotInstruction,
  createUpdatePlayerPositionInstruction,
  decodePlayerProfile,
  derivePlayerSessionPda,
  derivePlayerProfilePda,
  EQUIPMENT_SLOT_COUNT,
  NICECHUNK_PLAYER_PROGRAM_ID,
  SESSION_ACTION_BREAK_BLOCK,
  SESSION_ACTION_PLACE_BLOCK,
  PLAYER_PROFILE_LEN,
} from "../sdk/nicechunk-player.ts";
import {
  createDelegateChunkInstruction,
  createRecordBlockChangeInstruction,
  createRecordBlockChangeWithSessionInstruction,
  deriveChunkDelegationPdas,
  deriveChunkPda,
  MAGICBLOCK_DELEGATION_PROGRAM_ID,
  NICECHUNK_CHUNK_PROGRAM_ID,
} from "../sdk/nicechunk-chunk.ts";
import {
  deriveGlobalConfigPda,
  NICECHUNK_CORE_PROGRAM_ID,
} from "../sdk/nicechunk-core.ts";

describe("nicechunk player and chunk SDK", () => {
  const owner = new PublicKey("9XuoVVwqP2jipt3jpJVXCSS2N2jr9vDuV3d6K73FKVud");
  const [globalConfig] = deriveGlobalConfigPda(NICECHUNK_CORE_PROGRAM_ID);

  it("derives deterministic player and chunk PDAs", () => {
    const [playerProfile] = derivePlayerProfilePda(owner, NICECHUNK_PLAYER_PROGRAM_ID);
    const [chunk] = deriveChunkPda({
      globalConfig,
      chunkX: 0,
      chunkZ: 0,
      programId: NICECHUNK_CHUNK_PROGRAM_ID,
    });

    assert.equal(playerProfile.toBase58(), "3erZxS9JsMM8evKF84E3qPxZA6gWVTmGkAGTtRxzqHic");
    assert.equal(chunk.toBase58(), "AQQf3xk9B8uA9CUJFSvvMUpSuvnwWEJcTgA3i8FAha77");
  });

  it("builds initialize player account order", () => {
    const [playerProfile] = derivePlayerProfilePda(owner, NICECHUNK_PLAYER_PROGRAM_ID);
    const ix = createInitializePlayerInstruction({ payer: owner });

    assert.equal(ix.programId.toBase58(), NICECHUNK_PLAYER_PROGRAM_ID.toBase58());
    assert.deepEqual(ix.data, Buffer.from([0]));
    assert.equal(ix.keys[0].pubkey.toBase58(), owner.toBase58());
    assert.equal(ix.keys[0].isSigner, true);
    assert.equal(ix.keys[0].isWritable, true);
    assert.equal(ix.keys[1].pubkey.toBase58(), playerProfile.toBase58());
    assert.equal(ix.keys[1].isWritable, true);
    assert.equal(ix.keys[2].pubkey.toBase58(), globalConfig.toBase58());
    assert.equal(ix.keys[3].pubkey.toBase58(), SystemProgram.programId.toBase58());
  });

  it("builds update position and equipment instructions", () => {
    const updatePosition = createUpdatePlayerPositionInstruction({
      authority: owner,
      x: 16,
      y: 2,
      z: -16,
    });
    assert.equal(updatePosition.data.readUInt8(0), 1);
    assert.equal(updatePosition.data.readInt32LE(1), 16);
    assert.equal(updatePosition.data.readInt32LE(5), 2);
    assert.equal(updatePosition.data.readInt32LE(9), -16);

    const item = PublicKey.default;
    const setEquipment = createSetEquipmentSlotInstruction({ authority: owner, slot: 8, item });
    assert.equal(setEquipment.data.readUInt8(0), 2);
    assert.equal(setEquipment.data.readUInt8(1), 8);
    assert.equal(new PublicKey(setEquipment.data.subarray(2, 34)).toBase58(), item.toBase58());

    const backpack = new PublicKey("CEzcpJe9UTq5FmVzpTfgPffMbqdG97YJeFMJYwUSFhNF");
    const bindBackpack = createSetEquippedBackpackInstruction({ authority: owner, backpack });
    assert.equal(bindBackpack.data.readUInt8(0), 5);
    assert.equal(bindBackpack.keys[0].pubkey.toBase58(), owner.toBase58());
    assert.equal(bindBackpack.keys[0].isSigner, true);
    assert.equal(bindBackpack.keys[0].isWritable, true);
    assert.equal(bindBackpack.keys[1].isWritable, true);
    assert.equal(bindBackpack.keys[2].pubkey.toBase58(), backpack.toBase58());
    assert.equal(bindBackpack.keys[3].pubkey.toBase58(), SystemProgram.programId.toBase58());
  });

  it("decodes a player profile buffer", () => {
    const [playerProfile, bump] = derivePlayerProfilePda(owner, NICECHUNK_PLAYER_PROGRAM_ID);
    assert.ok(playerProfile);
    const data = Buffer.alloc(PLAYER_PROFILE_LEN);
    let offset = 0;
    data.write("NCKPLY01", offset, "utf8"); offset += 8;
    data.writeUInt16LE(1, offset); offset += 2;
    data.writeUInt8(bump, offset++);
    data.writeUInt8(1, offset++);
    owner.toBuffer().copy(data, offset); offset += 32;
    globalConfig.toBuffer().copy(data, offset); offset += 32;
    data.writeUInt16LE(1, offset); offset += 2;
    data.writeInt32LE(1, offset); offset += 4;
    data.writeInt32LE(2, offset); offset += 4;
    data.writeInt32LE(3, offset); offset += 4;
    for (const value of [100, 100, 100, 1, 1, 0]) {
      data.writeUInt16LE(value, offset); offset += 2;
    }
    data.writeUInt8(EQUIPMENT_SLOT_COUNT, offset++);
    offset += EQUIPMENT_SLOT_COUNT * 32;
    data.writeUInt8(0, offset++);
    data.writeUInt8(0, offset++);
    const equippedBackpack = new PublicKey("CEzcpJe9UTq5FmVzpTfgPffMbqdG97YJeFMJYwUSFhNF");
    equippedBackpack.toBuffer().copy(data, offset); offset += 32;
    data.writeBigUInt64LE(10n, offset); offset += 8;
    data.writeBigUInt64LE(11n, offset); offset += 8;
    data.writeBigInt64LE(12n, offset); offset += 8;

    const decoded = decodePlayerProfile(data);
    assert.equal(offset, PLAYER_PROFILE_LEN);
    assert.equal(decoded.owner.toBase58(), owner.toBase58());
    assert.equal(decoded.position.y, 2);
    assert.equal(decoded.equipment.length, EQUIPMENT_SLOT_COUNT);
    assert.equal(decoded.equippedBackpack.toBase58(), equippedBackpack.toBase58());
  });

  it("builds record block change instruction", () => {
    const [playerProfile] = derivePlayerProfilePda(owner, NICECHUNK_PLAYER_PROGRAM_ID);
    const [chunk] = deriveChunkPda({
      globalConfig,
      chunkX: 0,
      chunkZ: 0,
      programId: NICECHUNK_CHUNK_PROGRAM_ID,
    });
    const ix = createRecordBlockChangeInstruction({
      authority: owner,
      change: {
        chunkX: 0,
        chunkZ: 0,
        localX: 1,
        y: 2,
        localZ: 3,
        previousBlockId: 1,
        newBlockId: 0,
        action: 1,
        toolSlot: 0,
      },
    });

    assert.equal(ix.programId.toBase58(), NICECHUNK_CHUNK_PROGRAM_ID.toBase58());
    assert.equal(ix.data.readUInt8(0), 1);
    assert.equal(ix.data.readUInt8(9), 1);
    assert.equal(ix.data.readInt16LE(10), 2);
    assert.equal(ix.data.readUInt8(12), 3);
    assert.equal(ix.keys[1].pubkey.toBase58(), playerProfile.toBase58());
    assert.equal(ix.keys[2].pubkey.toBase58(), chunk.toBase58());
  });

  it("builds player session and session block change instructions", () => {
    const sessionAuthority = new PublicKey("Z2WsAfHEgNiycsaKoSo83TzVzGnc6nLB1CkEKN9vymw");
    const [playerProfile] = derivePlayerProfilePda(owner, NICECHUNK_PLAYER_PROGRAM_ID);
    const [playerSession] = derivePlayerSessionPda({
      owner,
      sessionAuthority,
      programId: NICECHUNK_PLAYER_PROGRAM_ID,
    });
    const sessionIx = createOrRefreshPlayerSessionInstruction({
      owner,
      sessionAuthority,
      expiresAt: 1_800_000_000n,
    });

    assert.equal(sessionIx.programId.toBase58(), NICECHUNK_PLAYER_PROGRAM_ID.toBase58());
    assert.equal(sessionIx.data.readUInt8(0), 4);
    assert.equal(sessionIx.data.readBigInt64LE(1), 1_800_000_000n);
    assert.equal(sessionIx.data.readUInt16LE(9), SESSION_ACTION_BREAK_BLOCK | SESSION_ACTION_PLACE_BLOCK);
    assert.equal(sessionIx.keys[0].pubkey.toBase58(), owner.toBase58());
    assert.equal(sessionIx.keys[0].isSigner, true);
    assert.equal(sessionIx.keys[1].pubkey.toBase58(), sessionAuthority.toBase58());
    assert.equal(sessionIx.keys[1].isSigner, true);
    assert.equal(sessionIx.keys[2].pubkey.toBase58(), playerProfile.toBase58());
    assert.equal(sessionIx.keys[3].pubkey.toBase58(), playerSession.toBase58());

    const chunkIx = createRecordBlockChangeWithSessionInstruction({
      owner,
      sessionAuthority,
      change: {
        chunkX: 0,
        chunkZ: 0,
        localX: 4,
        y: 2,
        localZ: 4,
        previousBlockId: 1,
        newBlockId: 0,
        action: 1,
        toolSlot: 0,
      },
    });
    assert.equal(chunkIx.data.readUInt8(0), 3);
    assert.equal(chunkIx.keys[0].pubkey.toBase58(), sessionAuthority.toBase58());
    assert.equal(chunkIx.keys[1].pubkey.toBase58(), playerProfile.toBase58());
    assert.equal(chunkIx.keys[2].pubkey.toBase58(), playerSession.toBase58());
  });

  it("builds delegate chunk ER instruction", () => {
    const [chunk] = deriveChunkPda({
      globalConfig,
      chunkX: 0,
      chunkZ: 0,
      programId: NICECHUNK_CHUNK_PROGRAM_ID,
    });
    const delegation = deriveChunkDelegationPdas({ chunk });
    const ix = createDelegateChunkInstruction({
      payer: owner,
      chunkX: 0,
      chunkZ: 0,
      commitFrequencyMs: 250,
    });

    assert.equal(ix.programId.toBase58(), NICECHUNK_CHUNK_PROGRAM_ID.toBase58());
    assert.equal(ix.data.readUInt8(0), 2);
    assert.equal(ix.data.readUInt32LE(9), 250);
    assert.equal(ix.keys[0].pubkey.toBase58(), owner.toBase58());
    assert.equal(ix.keys[0].isSigner, true);
    assert.equal(ix.keys[1].pubkey.toBase58(), chunk.toBase58());
    assert.equal(ix.keys[3].pubkey.toBase58(), NICECHUNK_CHUNK_PROGRAM_ID.toBase58());
    assert.equal(ix.keys[4].pubkey.toBase58(), delegation.delegateBuffer.toBase58());
    assert.equal(ix.keys[5].pubkey.toBase58(), delegation.delegationRecord.toBase58());
    assert.equal(ix.keys[6].pubkey.toBase58(), delegation.delegationMetadata.toBase58());
    assert.equal(ix.keys[7].pubkey.toBase58(), MAGICBLOCK_DELEGATION_PROGRAM_ID.toBase58());
    assert.equal(ix.keys[8].pubkey.toBase58(), SystemProgram.programId.toBase58());
  });
});
