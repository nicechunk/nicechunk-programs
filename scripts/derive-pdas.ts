import { PublicKey } from "@solana/web3.js";
import { deriveGlobalConfigPda, NICECHUNK_CORE_PROGRAM_ID } from "../sdk/nicechunk-core.ts";
import { deriveChunkBrokenPda, NICECHUNK_CHUNK_PROGRAM_ID } from "../sdk/nicechunk-chunk.ts";
import {
  deriveGuardianRegistryPda,
  deriveGuardianRegionPda,
  deriveGuardianTreasuryAuthorityPda,
  NICECHUNK_GUARDIAN_PROGRAM_ID,
} from "../sdk/nicechunk-guardian.ts";
import { derivePlayerProfilePda, NICECHUNK_PLAYER_PROGRAM_ID } from "../sdk/nicechunk-player.ts";
import { chunkProgramId, guardianProgramId, playerProgramId, programId } from "./core-script-utils.ts";

const selectedProgramId = programId() ?? NICECHUNK_CORE_PROGRAM_ID;
const [globalConfig, bump] = deriveGlobalConfigPda(selectedProgramId);
const selectedPlayerProgramId = playerProgramId() ?? NICECHUNK_PLAYER_PROGRAM_ID;
const selectedChunkProgramId = chunkProgramId() ?? NICECHUNK_CHUNK_PROGRAM_ID;
const selectedGuardianProgramId = guardianProgramId() ?? NICECHUNK_GUARDIAN_PROGRAM_ID;
const owner = process.env.PLAYER_WALLET ? new PublicKey(process.env.PLAYER_WALLET) : undefined;
const chunkX = Number(process.env.CHUNK_X ?? 0);
const chunkZ = Number(process.env.CHUNK_Z ?? 0);
const regionX = Number(process.env.REGION_X ?? 0);
const regionY = Number(process.env.REGION_Y ?? 0);
const playerProfile = owner ? derivePlayerProfilePda(owner, selectedPlayerProgramId) : undefined;
const chunkBroken = deriveChunkBrokenPda({ globalConfig, chunkX, chunkZ, programId: selectedChunkProgramId });
const guardianRegistry = deriveGuardianRegistryPda({ globalConfig, programId: selectedGuardianProgramId });
const guardianTreasuryAuthority = deriveGuardianTreasuryAuthorityPda({ globalConfig, programId: selectedGuardianProgramId });
const guardianRegion = deriveGuardianRegionPda({ globalConfig, regionX, regionY, programId: selectedGuardianProgramId });

console.log(JSON.stringify({
  clusterUrl: process.env.CLUSTER_URL ?? "https://api.devnet.solana.com",
  programId: selectedProgramId.toBase58(),
  globalConfig: globalConfig.toBase58(),
  bump,
  playerProgramId: selectedPlayerProgramId.toBase58(),
  playerProfile: playerProfile?.[0].toBase58() ?? null,
  playerProfileBump: playerProfile?.[1] ?? null,
  chunkProgramId: selectedChunkProgramId.toBase58(),
  chunkX,
  chunkZ,
  chunkBroken: chunkBroken[0].toBase58(),
  chunkBrokenBump: chunkBroken[1],
  guardianProgramId: selectedGuardianProgramId.toBase58(),
  guardianRegistry: guardianRegistry[0].toBase58(),
  guardianRegistryBump: guardianRegistry[1],
  guardianTreasuryAuthority: guardianTreasuryAuthority[0].toBase58(),
  guardianTreasuryAuthorityBump: guardianTreasuryAuthority[1],
  regionX,
  regionY,
  guardianRegion: guardianRegion[0].toBase58(),
  guardianRegionBump: guardianRegion[1],
}, null, 2));
