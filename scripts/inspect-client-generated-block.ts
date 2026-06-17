import { getGeneratedBlock, setWorldSeed, terrainProfile, surfaceWaterLevel } from "../src/world/generator.js";
import { WorldMapBlock } from "../src/world/blocks.js";
import { chunkSize } from "../src/world/config.js";

const blockNames = new Map(Object.entries(WorldMapBlock).map(([name, id]) => [id, name]));

function readNumber(name: string, fallback: number): number {
  const raw = process.env[name];
  if (raw === undefined || raw === "") return fallback;
  const value = Number(raw);
  if (!Number.isFinite(value)) throw new Error(`${name} must be a finite number`);
  return value;
}

const seed = process.env.WORLD_SEED_TEXT ?? "nicechunk-mainnet-001";
setWorldSeed(seed);

const chunkX = readNumber("CHUNK_X", 0);
const chunkZ = readNumber("CHUNK_Z", 0);
const localX = readNumber("LOCAL_X", 0);
const localZ = readNumber("LOCAL_Z", 0);
const worldX = chunkX * chunkSize + localX;
const worldZ = chunkZ * chunkSize + localZ;
const profile = terrainProfile(worldX, worldZ);
const y = readNumber("BLOCK_Y", profile.height);
const generated = getGeneratedBlock(worldX, y, worldZ);
const waterLevel = surfaceWaterLevel(worldX, worldZ, profile);

console.log(JSON.stringify({
  seed,
  chunk: { x: chunkX, z: chunkZ },
  local: { x: localX, y, z: localZ },
  world: { x: worldX, y, z: worldZ },
  profile: {
    height: profile.height,
    slope: profile.slope,
    biome: profile.biome,
    terrain: profile.terrain,
    terrainName: blockNames.get(profile.terrain) ?? null,
    subsurface: profile.subsurface,
    subsurfaceName: blockNames.get(profile.subsurface) ?? null,
    fluid: profile.fluid ?? null,
    fluidName: blockNames.get(profile.fluid) ?? null,
    vegetation: profile.vegetation ?? null,
    vegetationName: blockNames.get(profile.vegetation) ?? null,
    waterLevel,
    temperature: profile.temperature,
    humidity: profile.humidity,
    surfaceType: profile.surfaceType,
  },
  generated: generated
    ? {
        terrain: generated.terrain,
        terrainName: blockNames.get(generated.terrain) ?? null,
        vegetation: generated.vegetation ?? null,
        vegetationName: blockNames.get(generated.vegetation) ?? null,
        fluid: generated.fluid ?? null,
        fluidName: blockNames.get(generated.fluid) ?? null,
        biome: generated.biome,
        height: generated.height,
      }
    : null,
  effectiveBlockId: generated?.vegetation ?? generated?.terrain ?? 0,
  effectiveBlockName: blockNames.get(generated?.vegetation ?? generated?.terrain ?? 0) ?? "Air",
}, null, 2));
