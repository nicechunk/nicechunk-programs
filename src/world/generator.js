import { landBaseHeight, seaLevel } from "./config.js";
import { BiomeType, WorldMapBlock, renderTypeForBlock } from "./blocks.js";
import { defaultWorldSeed } from "./seedStorage.js";

const profileCache = new Map();
const heightCache = new Map();
const rawHeightCache = new Map();
const baseHeightCache = new Map();
const mountainCellCache = new Map();
let worldSeed = hashSeed(defaultWorldSeed);
export const bedrockMaxY = 0;
export const deepStoneStartY = 45;
export const maxVisualWaterDepth = 4;

export function setWorldSeed(seed) {
  worldSeed = hashSeed(seed);
  profileCache.clear();
  heightCache.clear();
  rawHeightCache.clear();
  baseHeightCache.clear();
  mountainCellCache.clear();
}

export function currentWorldSeed() {
  return worldSeed;
}

export function terrainHeight(x, z) {
  return terrainProfile(x, z).height;
}

export function terrainProfile(x, z) {
  const key = `${x},${z}`;
  const cached = profileCache.get(key);
  if (cached) return cached;

  const groundHeight = rawTerrainHeight(x, z);
  const west = rawTerrainHeight(x - 1, z);
  const east = rawTerrainHeight(x + 1, z);
  const north = rawTerrainHeight(x, z - 1);
  const south = rawTerrainHeight(x, z + 1);
  const slope = Math.max(Math.abs(groundHeight - west), Math.abs(groundHeight - east), Math.abs(groundHeight - north), Math.abs(groundHeight - south));
  const sample = sampleWorldNoise(x, z, groundHeight);
  const temperature = sample.temperature;
  const moisture = sample.humidity;
  const desert = desertAt(x, z);
  const forestBands = ridgeNoise(x * 0.009 - 30, z * 0.009 + 60);
  const forestPatches = fbm(x * 0.014 + 520, z * 0.014 - 280, 3);
  const forest = smoothstep(-0.5, 0.3, moisture + forestBands * 0.72 + forestPatches * 0.35 - desert * 1.2);
  const snowLine = landBaseHeight + 42 + Math.floor(temperature * 10);
  const biome = chooseBiome(x, z, groundHeight, slope, sample, snowLine);
  const surfaceFluid = getSurfaceFluidBlock(biome, sample, groundHeight, x, z);
  const waterSurfaceHeight = waterBlockSurfaceHeight(groundHeight, biome, surfaceFluid);
  const height = waterSurfaceHeight ?? groundHeight;
  const waterFloorHeight = waterSurfaceHeight === null ? null : waterBlockFloorHeight(groundHeight, biome, sample, x, z);
  sample.groundHeight = groundHeight;
  sample.waterFloorHeight = waterFloorHeight;
  sample.waterSurfaceHeight = waterSurfaceHeight;
  sample.waterBlock = waterBlockForFluid(surfaceFluid);
  const terrain = waterSurfaceHeight === null ? getSurfaceBlock(biome, sample, groundHeight, slope, x, z) : sample.waterBlock;
  const subsurface = getSubsurfaceBlock(biome, sample, x, z);
  const fluid = waterSurfaceHeight === null ? surfaceFluid : null;
  const vegetation = getSurfaceVegetationBlock(biome, terrain, fluid, sample, height, slope, x, z);
  const waterLevel = getSurfaceWaterLevel(x, z, height, biome, fluid);
  const surfaceType = renderTypeForBlock(terrain) ?? "grass";
  const treeTone = random2(Math.floor(x / 3) + 903, Math.floor(z / 3) - 447);
  const evergreen =
    biome === BiomeType.Snowfield ||
    biome === BiomeType.Tundra ||
    biome === BiomeType.Mountain ||
    height > 15 ||
    temperature < -0.2 ||
    treeTone > 0.72;
  const profile = {
    height,
    slope,
    temperature,
    moisture,
    humidity: moisture,
    desert,
    forest,
    snowLine,
    biome,
    terrain,
    subsurface,
    fluid,
    waterLevel,
    vegetation,
    noise: sample,
    surfaceType,
    tree: {
      type: evergreen ? "pine" : "broadleaf",
      trunkType: chooseTrunkType(biome, treeTone),
      leafType: chooseLeafType(biome, height, snowLine, temperature, moisture, treeTone),
    },
  };

  profileCache.set(key, profile);
  return profile;
}

export function sampleWorldNoise(x, z, y = 0) {
  const continentalness = continentAt(x, z);
  const erosion = riverAt(x, z);
  const weirdness = fbm(x * 0.0056 + 830, z * 0.0056 - 170, 4);
  return {
    height: rawTerrainHeight(x, z),
    temperature: temperatureAt(x, z),
    humidity: moistureAt(x, z),
    continentalness,
    erosion,
    weirdness,
    volcanic: volcanicAt(x, z),
    corruption: corruptionAt(x, z, y),
  };
}

export function getGeneratedBlock(x, y, z) {
  const surface = terrainProfile(x, z);
  const waterLevel = surfaceWaterLevel(x, z, surface);
  if (y > surface.height && waterLevel !== null && y <= waterLevel) {
    return generatedBlock(surface.fluid, undefined, surface.fluid, surface.biome, surface);
  }
  if (y > seaLevel && y > surface.height) return null;

  if (y > surface.height && y <= seaLevel) {
    const fluid = getSurfaceFluidBlock(surface.biome, surface.noise, surface.height, x, z);
    if (!fluid) return null;
    return generatedBlock(fluid, undefined, fluid, surface.biome, surface);
  }

  if (isCaveCell(x, y, z, surface)) return null;
  const caveWall = getAdjacentCaveBlock(x, y, z, surface);
  if (caveWall) return caveWall;

  const terrain = y === surface.height ? surface.terrain : getBlockByDepth(y, surface.height, surface.biome, surface.noise, x, z);
  if (!terrain) return null;
  const vegetation = y === surface.height ? surface.vegetation : undefined;
  return generatedBlock(terrain, vegetation, surface.fluid, surface.biome, surface);
}

export function surfaceWaterLevel(x, z, profile = terrainProfile(x, z)) {
  return profile.waterLevel ?? getSurfaceWaterLevel(x, z, profile.height, profile.biome, profile.fluid);
}

export function getBlockByDepth(y, surfaceY, biome, sample, x = 0, z = 0) {
  if (y <= bedrockMaxY) return WorldMapBlock.Bedrock;
  if (sample.waterSurfaceHeight !== null && sample.waterSurfaceHeight !== undefined) {
    if (y > sample.waterFloorHeight && y <= sample.waterSurfaceHeight) return sample.waterBlock ?? WorldMapBlock.Water;
    if (y === sample.waterFloorHeight) return getSurfaceBlock(biome, sample, sample.groundHeight ?? surfaceY, 0, x, z);
  }
  if (y < deepStoneStartY) return coalSeamAt(x, y, z, surfaceY) ? WorldMapBlock.Coal : WorldMapBlock.DeepStone;
  if (y < surfaceY - 6) {
    if (biome === BiomeType.Volcano && sample.volcanic > 0.62 && random2(x + y * 13, z - y * 17) > 0.76) return WorldMapBlock.Basalt;
    return WorldMapBlock.Stone;
  }
  if (y < surfaceY) return getSubsurfaceBlock(biome, sample, x, z);
  if (y === surfaceY) return getSurfaceBlock(biome, sample, surfaceY, 0, x, z);
  return null;
}

function waterBlockSurfaceHeight(groundHeight, biome, fluid) {
  if (!isWaterFluidBlock(fluid)) return null;
  if (biome === BiomeType.Ocean || groundHeight < seaLevel) return seaLevel;
  return groundHeight;
}

function waterBlockFloorHeight(groundHeight, biome, sample, x, z) {
  const depth = waterVisualDepth(groundHeight, biome, sample, x, z);
  const surfaceY = biome === BiomeType.Ocean || groundHeight < seaLevel ? seaLevel : groundHeight;
  return Math.max(bedrockMaxY + 1, surfaceY - depth);
}

function waterVisualDepth(groundHeight, biome, sample, x, z) {
  if (biome === BiomeType.Ocean || groundHeight < seaLevel) {
    return clampInt(seaLevel - groundHeight, 1, maxVisualWaterDepth);
  }
  if (biome === BiomeType.River) {
    const center = smoothstep(0.951, 0.959, sample.erosion);
    return 1 + Math.floor(center * (maxVisualWaterDepth - 1));
  }
  if (biome === BiomeType.Lake) {
    const center = smoothstep(0.48, 0.92, lakeAt(x, z));
    return 1 + Math.floor(center * (maxVisualWaterDepth - 1));
  }
  if (biome === BiomeType.Swamp || biome === BiomeType.Wetland) return 1;
  return 1;
}

function waterBlockForFluid(fluid) {
  if (fluid === WorldMapBlock.Ice) return WorldMapBlock.Ice;
  if (isWaterFluidBlock(fluid)) return WorldMapBlock.Water;
  return fluid;
}

function isWaterFluidBlock(block) {
  return block === WorldMapBlock.Water || block === WorldMapBlock.SwampWater || block === WorldMapBlock.ToxicWater || block === WorldMapBlock.Ice;
}

function generatedBlock(terrain, vegetation, fluid, biome, profile) {
  return {
    terrain,
    vegetation,
    fluid,
    biome,
    height: profile.height,
    temperature: profile.temperature,
    humidity: profile.humidity,
  };
}

export function treeNoise(x, z) {
  return random2(Math.floor(x / 2), Math.floor(z / 2));
}

export function treeDensityAt(x, z) {
  const profile = terrainProfile(x, z);
  if (!canBiomeGrowTrees(profile.biome) || profile.slope > 3 || profile.height <= 4) return 0;
  const altitudePenalty = smoothstep(profile.snowLine - 7, profile.snowLine - 2, profile.height);
  const forestFloor = smoothstep(0.22, 0.76, profile.forest);
  const meadowTrees = smoothstep(0.52, 0.92, ridgeNoise(x * 0.035 + 12, z * 0.035 - 19)) * 0.28;
  const biomeBoost =
    profile.biome === BiomeType.Rainforest ? 0.34 : profile.biome === BiomeType.Forest ? 0.18 : profile.biome === BiomeType.Snowfield ? 0.08 : 0;
  return Math.max(0, Math.min(0.92, forestFloor * (1 - altitudePenalty) + meadowTrees + biomeBoost));
}

function chooseBiome(x, z, height, slope, sample, snowLine) {
  const nearSea = height <= landBaseHeight + 1;
  const coastal = nearSea && sample.continentalness < 0.02;
  const cold = sample.temperature < -0.28 || height >= snowLine;
  const hot = sample.temperature > 0.18;
  const wet = sample.humidity > 0.16;
  const dry = sample.humidity < -0.12;
  const river = sample.erosion > 0.95 && height >= seaLevel + 1;
  const lake = lakeAt(x, z) > 0.48 && sample.erosion < 0.9 && height >= landBaseHeight - 1 && height <= landBaseHeight + 16 && slope <= 3;

  if (height < seaLevel - 1) return BiomeType.Ocean;
  if (river) return BiomeType.River;
  if (lake) return BiomeType.Lake;
  if (height < seaLevel) return BiomeType.Ocean;
  if (coastal) return BiomeType.Beach;
  if (sample.volcanic > 0.965 && height > seaLevel + 2) return BiomeType.Volcano;
  if (height >= snowLine + 2 || (cold && height >= snowLine - 3)) return BiomeType.Snowfield;
  if (height >= landBaseHeight + 24 || slope >= 5) return BiomeType.Mountain;
  if (wet && height <= landBaseHeight + 4 && (sample.corruption > 0.18 || sample.humidity > 0.34)) return BiomeType.Swamp;
  if (wet && nearSea) return BiomeType.Wetland;
  if (cold && dry) return BiomeType.Tundra;
  if (hot && dry) return BiomeType.Desert;
  if (hot && sample.humidity > -0.16) return BiomeType.Rainforest;
  if (sample.humidity > 0.08 || forestAt(x, z, sample) > 0.58) return BiomeType.Forest;
  return BiomeType.Plains;
}

function getSurfaceBlock(biome, sample, height, slope, x, z) {
  const roll = patchRoll(x, z, 421);
  switch (biome) {
    case BiomeType.Ocean:
      if (rareSpeckle(x, z, 0.018, 1009)) return WorldMapBlock.ShellBed;
      if (roll < 0.24) return WorldMapBlock.Clay;
      return roll < 0.72 ? WorldMapBlock.Sand : WorldMapBlock.Gravel;
    case BiomeType.Beach:
      if (rareSpeckle(x, z, 0.025, 1031)) return WorldMapBlock.ShellBed;
      return roll < 0.18 ? WorldMapBlock.Gravel : WorldMapBlock.Sand;
    case BiomeType.River:
      if (rareSpeckle(x, z, 0.035, 1049)) return roll < 0.5 ? WorldMapBlock.Clay : WorldMapBlock.Sand;
      return roll < 0.58 ? WorldMapBlock.Gravel : roll < 0.82 ? WorldMapBlock.Clay : WorldMapBlock.Sand;
    case BiomeType.Lake:
      if (rareSpeckle(x, z, 0.025, 1061)) return WorldMapBlock.Gravel;
      return roll < 0.55 ? WorldMapBlock.Clay : roll < 0.88 ? WorldMapBlock.Sand : WorldMapBlock.Gravel;
    case BiomeType.Volcano:
      if (sample.volcanic > 0.96 && rareSpeckle(x, z, 0.035, 1091)) return WorldMapBlock.Lava;
      return roll < 0.68 ? WorldMapBlock.Basalt : roll < 0.9 ? WorldMapBlock.Ash : WorldMapBlock.Stone;
    case BiomeType.Mountain:
      if (height >= landBaseHeight + 38 || sample.temperature < -0.34) {
        return roll > 0.68 || rareSpeckle(x, z, 0.025, 1103) ? WorldMapBlock.Snow : WorldMapBlock.Stone;
      }
      if (slope >= 4) return roll > 0.74 || rareSpeckle(x, z, 0.02, 1117) ? WorldMapBlock.Gravel : WorldMapBlock.Stone;
      return roll > 0.72 ? WorldMapBlock.Grass : WorldMapBlock.Stone;
    case BiomeType.Snowfield:
      if (rareSpeckle(x, z, 0.018, 1129)) return WorldMapBlock.Ice;
      return roll > 0.78 ? WorldMapBlock.FrozenSoil : WorldMapBlock.Snow;
    case BiomeType.Tundra:
      return roll > 0.76 ? WorldMapBlock.Snow : WorldMapBlock.FrozenSoil;
    case BiomeType.Desert:
      if (sample.weirdness < -0.74 || roll > 0.88) return WorldMapBlock.Quicksand;
      if (roll > 0.68 && saltFlatPatch(x, z)) return WorldMapBlock.SaltFlat;
      if (roll < 0.16 || rareSpeckle(x, z, 0.018, 1151)) return WorldMapBlock.DryDirt;
      return WorldMapBlock.Sand;
    case BiomeType.Swamp:
      if (sample.corruption > 0.5 && roll > 0.78) return WorldMapBlock.ToxicWater;
      return roll > 0.2 ? WorldMapBlock.Mud : WorldMapBlock.Clay;
    case BiomeType.Wetland:
      if (rareSpeckle(x, z, 0.02, 1181)) return WorldMapBlock.Clay;
      return roll > 0.72 ? WorldMapBlock.Clay : roll > 0.22 ? WorldMapBlock.Mud : WorldMapBlock.Grass;
    case BiomeType.Rainforest:
      if (rareSpeckle(x, z, 0.025, 1193)) return WorldMapBlock.Moss;
      return roll > 0.66 ? WorldMapBlock.Mud : WorldMapBlock.Grass;
    case BiomeType.Forest:
      return sample.humidity > 0.22 && roll > 0.9 && mossPatch(x, z) ? WorldMapBlock.Moss : WorldMapBlock.Grass;
    case BiomeType.Plains:
    default:
      if (sample.humidity < -0.24 && roll > 0.82) return WorldMapBlock.DryDirt;
      if (sample.humidity > 0.34 && roll > 0.9 && clayPatch(x, z)) return WorldMapBlock.Clay;
      return WorldMapBlock.Grass;
  }
}

function getSubsurfaceBlock(biome, sample, x, z) {
  const roll = patchRoll(x, z, 461, 24);
  switch (biome) {
    case BiomeType.Desert:
    case BiomeType.Beach:
      return roll < 0.76 ? WorldMapBlock.Sand : roll < 0.9 ? WorldMapBlock.Gravel : WorldMapBlock.Stone;
    case BiomeType.Wetland:
    case BiomeType.Swamp:
    case BiomeType.Lake:
    case BiomeType.River:
      return roll < 0.5 ? WorldMapBlock.Clay : roll < 0.86 ? WorldMapBlock.Mud : WorldMapBlock.Stone;
    case BiomeType.Snowfield:
    case BiomeType.Tundra:
      return roll < 0.82 ? WorldMapBlock.FrozenSoil : WorldMapBlock.Stone;
    case BiomeType.Volcano:
      return roll < 0.72 ? WorldMapBlock.Basalt : sample.volcanic > 0.86 && rareSpeckle(x, z, 0.018, 1217) ? WorldMapBlock.Lava : WorldMapBlock.Stone;
    case BiomeType.Mountain:
      return roll < 0.18 || rareSpeckle(x, z, 0.018, 1229) ? WorldMapBlock.Gravel : WorldMapBlock.Stone;
    case BiomeType.Ocean:
      return roll < 0.48 ? WorldMapBlock.Clay : roll < 0.78 ? WorldMapBlock.Sand : WorldMapBlock.Gravel;
    default:
      if (rareSpeckle(x, z, 0.018, 1249)) return random2(x + 1259, z - 1277) > 0.5 ? WorldMapBlock.Gravel : WorldMapBlock.Clay;
      return roll < 0.86 ? WorldMapBlock.Dirt : roll < 0.94 ? WorldMapBlock.Gravel : WorldMapBlock.Clay;
  }
}

function patchRoll(x, z, salt = 0, patchSize = 18) {
  const broad = valueNoise(x / patchSize + salt * 0.013, z / patchSize - salt * 0.017);
  const detail = valueNoise(x / (patchSize * 0.45) - salt * 0.019, z / (patchSize * 0.45) + salt * 0.023);
  return clamp((broad * 0.82 + detail * 0.18 + 1) * 0.5, 0, 1);
}

function rareSpeckle(x, z, chance, salt = 0) {
  return random2(x + salt * 17, z - salt * 19) > 1 - chance;
}

function saltFlatPatch(x, z) {
  return patchRoll(x, z, 1163, 32) > 0.72;
}

function mossPatch(x, z) {
  return patchRoll(x, z, 1171, 14) > 0.76;
}

function clayPatch(x, z) {
  return patchRoll(x, z, 1177, 28) > 0.7;
}

function getSurfaceFluidBlock(biome, sample, height, x, z) {
  if (biome === BiomeType.Volcano && sample.volcanic > 0.96 && patchRoll(x, z, 1283, 12) > 0.9) return WorldMapBlock.Lava;
  if (biome === BiomeType.River || biome === BiomeType.Lake) return WorldMapBlock.Water;
  if (biome === BiomeType.Swamp) {
    if (sample.corruption > 0.58 && patchRoll(x, z, 1297, 20) > 0.68) return WorldMapBlock.ToxicWater;
    if (height <= seaLevel + 3 || patchRoll(x, z, 1301, 24) > 0.56) return WorldMapBlock.SwampWater;
    return null;
  }
  if (biome === BiomeType.Wetland && patchRoll(x, z, 1307, 22) > 0.78) return WorldMapBlock.Water;
  if (height >= seaLevel) return null;
  if (sample.temperature < -0.55) return WorldMapBlock.Ice;
  if (sample.corruption > 0.58 && (biome === BiomeType.Swamp || biome === BiomeType.Wetland)) return WorldMapBlock.ToxicWater;
  if (biome === BiomeType.Swamp || (biome === BiomeType.Wetland && sample.humidity > 0.36)) return WorldMapBlock.SwampWater;
  return WorldMapBlock.Water;
}

function getSurfaceWaterLevel(x, z, height, biome, fluid) {
  if (!fluid) return null;
  if (fluid === WorldMapBlock.Lava) return height;
  if (height < seaLevel) return seaLevel;
  const depthRoll = random2(Math.floor(x / 5) + 281, Math.floor(z / 5) - 293);
  if (biome === BiomeType.Lake) return height + 2 + Math.floor(depthRoll * 3);
  if (biome === BiomeType.River) return height + 1 + Math.floor(depthRoll * 2);
  if (biome === BiomeType.Swamp) return height + 1 + (depthRoll > 0.82 ? 1 : 0);
  if (biome === BiomeType.Wetland) return height + 1;
  return height + 1;
}

function getSurfaceVegetationBlock(biome, terrain, fluid, sample, height, slope, x, z) {
  if (fluid === WorldMapBlock.Lava || terrain === WorldMapBlock.Lava) return null;
  const roll = random2(x + 503, z - 509);
  if (biome === BiomeType.Swamp && terrain === WorldMapBlock.SwampWater) return null;
  if (fluid === WorldMapBlock.Water || fluid === WorldMapBlock.SwampWater || fluid === WorldMapBlock.Ice || terrain === WorldMapBlock.SwampWater) {
    if (biome === BiomeType.Ocean) {
      if (sample.temperature > 0.18 && height >= seaLevel - 3 && roll > 0.992) return WorldMapBlock.Coral;
      if ((sample.temperature < -0.18 || sample.corruption > 0.42) && roll > 0.982) return WorldMapBlock.DeadCoral;
      return null;
    }
    if (biome === BiomeType.Swamp) return roll > 0.996 ? WorldMapBlock.Reed : null;
    return roll > 0.996 ? WorldMapBlock.Reed : null;
  }
  if (slope > 4 && biome !== BiomeType.Mountain) return null;
  switch (biome) {
    case BiomeType.Desert:
      if (terrain !== WorldMapBlock.Sand && terrain !== WorldMapBlock.DryDirt && terrain !== WorldMapBlock.Ash) return null;
      if (roll > 0.978 && random2(Math.floor(x / 5) + 701, Math.floor(z / 5) - 709) > 0.65) return WorldMapBlock.Cactus;
      if (roll > 0.955) return WorldMapBlock.DeadBush;
      if (roll > 0.932) return WorldMapBlock.Thorn;
      return roll > 0.86 ? WorldMapBlock.DryGrass : null;
    case BiomeType.Swamp:
      if (roll > 0.9996) return WorldMapBlock.Mushroom;
      if (roll > 0.985) return WorldMapBlock.Vine;
      return roll > 0.975 ? WorldMapBlock.Moss : null;
    case BiomeType.Wetland:
      if (roll > 0.985) return WorldMapBlock.Reed;
      return roll > 0.975 ? WorldMapBlock.Moss : null;
    case BiomeType.Snowfield:
    case BiomeType.Tundra:
      if (terrain !== WorldMapBlock.Snow && terrain !== WorldMapBlock.FrozenSoil) return null;
      if (roll > 0.97) return WorldMapBlock.SnowBush;
      return roll > 0.38 ? WorldMapBlock.Lichen : null;
    case BiomeType.Mountain:
      if (roll > 0.985 && sample.temperature < -0.16) return WorldMapBlock.SnowBush;
      if (roll > 0.64) return WorldMapBlock.Lichen;
      return roll > 0.52 ? WorldMapBlock.Bush : null;
    case BiomeType.Volcano:
      if (roll > 0.92) return WorldMapBlock.DeadWood;
      if (roll > 0.82) return WorldMapBlock.Thorn;
      return roll > 0.72 ? WorldMapBlock.DeadBush : null;
    case BiomeType.Rainforest:
      if (roll > 0.98) return WorldMapBlock.GiantRoot;
      if (roll > 0.965) return WorldMapBlock.Moss;
      if (roll > 0.945) return WorldMapBlock.Vine;
      if (roll > 0.9996) return WorldMapBlock.Mushroom;
      return roll > 0.92 ? WorldMapBlock.Bush : null;
    case BiomeType.Forest:
      if (roll > 0.9984 && sample.humidity > 0.18) return WorldMapBlock.Mushroom;
      if (roll > 0.94 && sample.humidity > 0.16) return WorldMapBlock.Vine;
      if (roll > 0.9) return WorldMapBlock.Bush;
      return roll > 0.985 ? WorldMapBlock.GrassPlant : null;
    case BiomeType.Plains:
      if (roll > 0.965) return WorldMapBlock.Bush;
      if (roll > 0.94 && sample.humidity < -0.12) return WorldMapBlock.DryGrass;
      return roll > 0.985 ? WorldMapBlock.GrassPlant : null;
    default:
      return null;
  }
}

function canBiomeGrowTrees(biome) {
  return [BiomeType.Plains, BiomeType.Forest, BiomeType.Rainforest, BiomeType.Snowfield, BiomeType.Tundra, BiomeType.Mountain].includes(biome);
}

function rawTerrainHeight(x, z) {
  const key = `${x},${z}`;
  const cached = heightCache.get(key);
  if (cached !== undefined) return cached;

  const limitRadius = 2;
  const rawHeight = rawUnconstrainedHeight(x, z);
  const baseHeight = cachedBaseLandHeight(x, z);
  const mountainRise = Math.max(0, rawHeight - baseHeight);
  const mountainWeight = smoothstep(3, 18, mountainRise);
  const localRoughness = ridgeNoise(x * 0.031 + 4100, z * 0.031 - 4200);
  let constrainedHeight = rawHeight;
  for (let dz = -limitRadius; dz <= limitRadius; dz++) {
    for (let dx = -limitRadius; dx <= limitRadius; dx++) {
      if (dx === 0 && dz === 0) continue;
      const distance = Math.abs(dx) + Math.abs(dz);
      const neighborRaw = rawUnconstrainedHeight(x + dx, z + dz);
      const allowedStep = distance * lerp(1.05, 4.4 + localRoughness * 1.4, mountainWeight);
      constrainedHeight = Math.min(constrainedHeight, neighborRaw + allowedStep);
    }
  }

  const lowlandSmooth = smoothstep(0, 12, mountainRise);
  const finalHeight = Math.max(1, Math.floor(lerp(constrainedHeight, rawHeight, lowlandSmooth * 0.9)));
  const normalizedHeight =
    rawHeight < seaLevel
      ? finalHeight
      : rawHeight < landBaseHeight
        ? Math.max(seaLevel + 1, finalHeight)
        : Math.max(landBaseHeight, finalHeight);
  heightCache.set(key, normalizedHeight);
  return normalizedHeight;
}

function rawUnconstrainedHeight(x, z) {
  const key = `${x},${z}`;
  const cached = rawHeightCache.get(key);
  if (cached !== undefined) return cached;
  const height = Math.max(cachedBaseLandHeight(x, z), mountainElevation(x, z));
  rawHeightCache.set(key, height);
  return height;
}

function cachedBaseLandHeight(x, z) {
  const key = `${x},${z}`;
  const cached = baseHeightCache.get(key);
  if (cached !== undefined) return cached;
  const height = baseLandHeight(x, z);
  baseHeightCache.set(key, height);
  return height;
}

function baseLandHeight(x, z) {
  const continent = continentAt(x, z);
  const shelf = smoothstep(-0.36, 0.14, continent);
  const inland = smoothstep(-0.06, 0.56, continent);
  const plains = fbm(x * 0.0042 + 120, z * 0.0042 - 60, 4);
  const lowHills = fbm(x * 0.008 - 40, z * 0.008 + 90, 3);
  const rollingRelief = fbm(x * 0.012 + 1330, z * 0.012 - 1270, 3);
  const dryWash = ridgeNoise(x * 0.018 - 2190, z * 0.018 + 2230);
  const desert = desertAt(x, z);
  const dunes =
    inland *
    smoothstep(0.24, 0.72, desert) *
    (2.5 + ridgeNoise(x * 0.018 + 930, z * 0.018 - 730) * 5.5 + fbm(x * 0.006 - 870, z * 0.006 + 280, 3) * 2);
  const oceanFloor =
    seaLevel -
    10 +
    fbm(x * 0.0065 - 640, z * 0.0065 + 390, 3) * 1.8 +
    ridgeNoise(x * 0.012 + 190, z * 0.012 - 240) * 0.8;
  const coastHeight = seaLevel - 1 + shelf * (landBaseHeight - seaLevel + 1);
  const microRidges = smoothstep(0.38, 0.94, ridgeNoise(x * 0.045 + 3310, z * 0.045 - 3370));
  const lowlandRelief = inland * ((plains + 1) * 2.8 + (lowHills + 1) * 1.9 + (rollingRelief + 1) * 1.15 + smoothstep(0.7, 0.96, dryWash) * 1.8 + microRidges * 2.2);
  const landHeight = landBaseHeight + inland * 4 + lowlandRelief + dunes;
  const riverCut = smoothstep(0.9, 0.985, riverAt(x, z));
  const canyonCut = smoothstep(0.72, 0.94, canyonAt(x, z)) * inland;
  const riverCarve = riverCut * (4.6 + inland * 9.2);
  const canyonCarve = canyonCut * (8 + smoothstep(0.18, 0.8, desert) * 18);
  const lakeCarve = smoothstep(0.78, 0.96, lakeAt(x, z)) * (2.2 + inland * 1.8);
  const totalCarve = Math.max(riverCarve, canyonCarve) + lakeCarve;
  const blendedHeight = lerp(oceanFloor, Math.max(coastHeight, landHeight) - totalCarve, shelf);
  if (blendedHeight < seaLevel) return blendedHeight;
  if (canyonCut > 0.42) return Math.max(seaLevel + 1, Math.min(blendedHeight, landBaseHeight - 1 - canyonCut * 12));
  if (riverCut > 0.86 && inland > 0.35) return Math.max(seaLevel + 1, Math.min(blendedHeight, landBaseHeight - 1));
  if (totalCarve > 2.6) return Math.max(seaLevel + 1, blendedHeight);
  return Math.max(landBaseHeight, blendedHeight);
}

function mountainElevation(x, z) {
  const mountainCellSize = 192;
  const cellX = Math.floor(x / mountainCellSize);
  const cellZ = Math.floor(z / mountainCellSize);
  let elevation = 0;

  for (let dz = -3; dz <= 3; dz++) {
    for (let dx = -3; dx <= 3; dx++) {
      const candidate = cachedMountainFromCell(cellX + dx, cellZ + dz, mountainCellSize);
      if (!candidate) continue;

      const localX = x - candidate.x;
      const localZ = z - candidate.z;
      const along = localX * Math.cos(candidate.angle) + localZ * Math.sin(candidate.angle);
      const across = -localX * Math.sin(candidate.angle) + localZ * Math.cos(candidate.angle);
      const warpedAlong = along + valueNoise(x * 0.018 + candidate.warpSeed, z * 0.018 - candidate.warpSeed) * candidate.warp;
      const warpedAcross = across + valueNoise(x * 0.021 - candidate.warpSeed, z * 0.021 + candidate.warpSeed) * candidate.warp * 0.65;
      const distance = Math.hypot(warpedAlong / candidate.longRadius, warpedAcross / candidate.shortRadius);
      if (distance > 1.16) continue;

      const spine = 1 - Math.min(1, Math.abs(warpedAcross) / candidate.shortRadius);
      const ridge = ridgeNoise(x * candidate.ridgeFrequency + candidate.warpSeed, z * candidate.ridgeFrequency - candidate.warpSeed);
      const erosion = fbm(x * candidate.detailFrequency - candidate.warpSeed, z * candidate.detailFrequency + candidate.warpSeed, 3);
      const shoulder = Math.pow(Math.max(0, 1 - distance), candidate.falloffPower);
      const spineLift = Math.pow(Math.max(0, spine), 1.8) * candidate.spineStrength;
      const crag = smoothstep(0.46, 0.92, ridge) * (0.18 + candidate.cragStrength);
      const eroded = 1 - Math.max(0, erosion) * candidate.erosionStrength;
      const summitNoise = fbm(x * 0.045 + candidate.warpSeed * 0.37, z * 0.045 - candidate.warpSeed * 0.23, 2) * candidate.summitRoughness;
      const localHeight =
        candidate.base +
        candidate.height * Math.max(0, shoulder * eroded + spineLift * shoulder + crag * shoulder * (1 - distance * 0.55) + summitNoise * shoulder);
      elevation = Math.max(elevation, localHeight);
    }
  }

  return elevation;
}

function cachedMountainFromCell(cellX, cellZ, cellSize) {
  const key = `${cellX},${cellZ}`;
  if (!mountainCellCache.has(key)) mountainCellCache.set(key, mountainFromCell(cellX, cellZ, cellSize));
  return mountainCellCache.get(key);
}

function mountainFromCell(cellX, cellZ, cellSize) {
  const presence = random2(cellX + 811, cellZ - 433);
  if (presence < 0.36) return null;

  const centerX = cellX * cellSize + random2(cellX - 17, cellZ + 29) * cellSize;
  const centerZ = cellZ * cellSize + random2(cellX + 37, cellZ - 41) * cellSize;
  const landInfluence = smoothstep(-0.08, 0.36, continentAt(Math.round(centerX), Math.round(centerZ)));
  if (landInfluence <= 0) return null;

  const radius = mountainRadius(cellX, cellZ);
  const longRadius = radius * (0.78 + random2(cellX + 59, cellZ - 61) * 0.82);
  const shortRadius = radius * (0.38 + random2(cellX - 67, cellZ + 71) * 0.46);
  const maxSlope = 0.58 + random2(cellX + 73, cellZ - 79) * 0.58;
  const height = Math.min(mountainHeight(cellX, cellZ) * landInfluence, Math.min(longRadius, shortRadius) * maxSlope);
  const warpSeed = random2(cellX + 97, cellZ - 101) * 10000;
  return {
    x: centerX,
    z: centerZ,
    radius,
    longRadius,
    shortRadius,
    angle: random2(cellX - 83, cellZ + 89) * Math.PI,
    warp: radius * (0.035 + random2(cellX + 107, cellZ - 109) * 0.075),
    warpSeed,
    ridgeFrequency: 0.018 + random2(cellX - 113, cellZ + 127) * 0.018,
    detailFrequency: 0.009 + random2(cellX + 131, cellZ - 137) * 0.011,
    falloffPower: 1.08 + random2(cellX - 139, cellZ + 149) * 1.1,
    spineStrength: 0.08 + random2(cellX + 151, cellZ - 157) * 0.24,
    cragStrength: random2(cellX - 163, cellZ + 167) * 0.28,
    erosionStrength: 0.08 + random2(cellX + 173, cellZ - 179) * 0.18,
    summitRoughness: 0.04 + random2(cellX - 181, cellZ + 191) * 0.12,
    height,
    base: cachedBaseLandHeight(Math.round(centerX), Math.round(centerZ)),
  };
}

function mountainRadius(cellX, cellZ) {
  let radius = 72 + random2(cellX + 101, cellZ - 103) * 96;
  let chance = 0.42;
  for (let i = 0; i < 8; i++) {
    if (random2(cellX + 211 + i * 37, cellZ - 223 - i * 41) >= chance) break;
    radius += 48 + random2(cellX - 307 - i * 43, cellZ + 331 + i * 47) * 88;
    chance *= 0.48;
  }
  return radius;
}

function mountainHeight(cellX, cellZ) {
  let height = 12 + Math.floor(random2(cellX - 409, cellZ + 419) * 16);
  let chance = 0.9;
  for (let i = 0; i < 520; i++) {
    if (random2(cellX + 503 + i * 11, cellZ - 509 - i * 13) >= chance) break;
    height += 1;
    chance *= 0.985;
  }
  return height;
}

function temperatureAt(x, z) {
  const broad = Math.sin((x + (worldSeed % 9973)) * 0.0011) * 0.42 + Math.cos((z - (worldSeed % 7919)) * 0.0009) * 0.28;
  return clamp(fbm(x * 0.0032 + 210, z * 0.0032 - 80, 4) * 0.58 + broad, -1, 1);
}

function moistureAt(x, z) {
  const broad = Math.cos((x - (worldSeed % 6827)) * 0.001) * 0.32 + Math.sin((z + (worldSeed % 9209)) * 0.0013) * 0.34;
  return clamp(fbm(x * 0.0038 - 120, z * 0.0038 + 340, 4) * 0.62 + broad, -1, 1);
}

function continentAt(x, z) {
  const oceanBasins = fbm(x * 0.00072 - 940, z * 0.00072 + 670, 5);
  const continentDetail = fbm(x * 0.00155 + 320, z * 0.00155 - 880, 4);
  const coastalWarp = ridgeNoise(x * 0.0022 - 140, z * 0.0022 + 260) * 0.24 - 0.12;
  return oceanBasins * 0.78 + continentDetail * 0.34 + coastalWarp + 0.12;
}

function desertAt(x, z) {
  const temperature = temperatureAt(x, z);
  const moisture = moistureAt(x, z);
  const desertMass = fbm(x * 0.0014 - 760, z * 0.0014 + 510, 4);
  return smoothstep(-0.46, 0.18, desertMass * 1.25 + temperature * 0.44 - moisture * 0.28);
}

function forestAt(x, z, sample) {
  const forestBands = ridgeNoise(x * 0.009 - 30, z * 0.009 + 60);
  const forestPatches = fbm(x * 0.014 + 520, z * 0.014 - 280, 3);
  const desert = desertAt(x, z);
  return smoothstep(-0.5, 0.3, sample.humidity + forestBands * 0.72 + forestPatches * 0.35 - desert * 1.2);
}

function riverAt(x, z) {
  const broadWarpX = valueNoise(x * 0.0018 - 1010, z * 0.0018 + 1040) * 95;
  const broadWarpZ = valueNoise(x * 0.0018 + 1110, z * 0.0018 - 1160) * 95;
  const bendWarpX = fbm(x * 0.0042 + 170, z * 0.0042 - 220, 3) * 32;
  const bendWarpZ = fbm(x * 0.0042 - 510, z * 0.0042 + 370, 3) * 32;
  const rx = x + broadWarpX + bendWarpX;
  const rz = z + broadWarpZ + bendWarpZ;
  const main = 1 - Math.abs(valueNoise(rx * 0.0038 + 170, rz * 0.0038 - 220));
  const branch = 1 - Math.abs(valueNoise((rx - 260) * 0.0075 - 510, (rz + 180) * 0.0075 + 370));
  const tributary = 1 - Math.abs(valueNoise((rx + 620) * 0.012 - 950, (rz - 410) * 0.012 + 730));
  return Math.max(main * 0.96, branch * 0.78, tributary * 0.56);
}

function canyonAt(x, z) {
  const warpX = valueNoise(x * 0.0015 + 2710, z * 0.0015 - 2770) * 130;
  const warpZ = valueNoise(x * 0.0015 - 2830, z * 0.0015 + 2890) * 130;
  const cx = x + warpX;
  const cz = z + warpZ;
  const main = 1 - Math.abs(valueNoise(cx * 0.0027 + 3010, cz * 0.0027 - 3070));
  const side = 1 - Math.abs(valueNoise((cx + 140) * 0.0062 - 3130, (cz - 220) * 0.0062 + 3190));
  return Math.max(main * 0.92, side * 0.48);
}

function lakeAt(x, z) {
  const basin = fbm(x * 0.008 - 720, z * 0.008 + 114, 4);
  const closure = ridgeNoise(x * 0.004 + 470, z * 0.004 - 880);
  return smoothstep(0.72, 0.96, basin + closure * 0.34);
}

function volcanicAt(x, z) {
  const volcanicMass = fbm(x * 0.0009 + 1310, z * 0.0009 - 1460, 5);
  const hotspot = ridgeNoise(x * 0.0024 - 1700, z * 0.0024 + 1730);
  const vents = ridgeNoise(x * 0.018 - 1700, z * 0.018 + 1730);
  return smoothstep(0.38, 0.68, volcanicMass * 0.7 + hotspot * 0.26 + vents * 0.08);
}

function corruptionAt(x, z, y = 0) {
  const mass = fbm(x * 0.0024 - 1910, z * 0.0024 + 2010, 4);
  const seep = fbm(x * 0.01 + y * 0.03, z * 0.01 - y * 0.02, 3);
  return smoothstep(-0.32, 0.24, mass * 0.78 + seep * 0.24);
}

function chooseTrunkType(biome, roll) {
  if (biome === BiomeType.Snowfield || biome === BiomeType.Tundra || biome === BiomeType.Mountain) return "pineTrunk";
  if (biome === BiomeType.Rainforest && roll > 0.72) return "giantRoot";
  if (biome === BiomeType.Volcano || biome === BiomeType.Desert) return "deadWood";
  return "trunk";
}

function chooseLeafType(biome, height, snowLine, temperature, moisture, roll) {
  if (biome === BiomeType.Snowfield || biome === BiomeType.Tundra || height >= snowLine - 4) return "pineLeaves";
  if (biome === BiomeType.Rainforest || moisture > 0.34) return roll > 0.55 ? "leavesTeal" : "leavesDark";
  if (height >= snowLine - 4) return roll > 0.64 ? "snowLeaves" : "pineLeaves";
  if (temperature < -0.28 || height > 16) return roll > 0.5 ? "pineLeaves" : "leavesTeal";
  if (roll < 0.22) return "leavesLight";
  if (roll > 0.82) return "leavesWarm";
  return "leaves";
}

function isCaveCell(x, y, z, profile) {
  if (y >= profile.height - 3 || y <= bedrockMaxY) return false;
  const depth = profile.height - y;
  const caveNoise = ridgeNoise(x * 0.052 + y * 0.013 - 2400, z * 0.052 - y * 0.017 + 2600);
  const chamberNoise = fbm(x * 0.026 + y * 0.019 + 2810, z * 0.026 - y * 0.015 - 2830, 3);
  const threshold = depth > 18 ? 0.72 : 0.84;
  return caveNoise + chamberNoise * 0.22 > threshold;
}

function getAdjacentCaveBlock(x, y, z, profile) {
  if (y >= profile.height - 2 || y <= bedrockMaxY) return null;
  const adjacentCave =
    isCaveCell(x + 1, y, z, profile) ||
    isCaveCell(x - 1, y, z, profile) ||
    isCaveCell(x, y + 1, z, profile) ||
    isCaveCell(x, y - 1, z, profile) ||
    isCaveCell(x, y, z + 1, profile) ||
    isCaveCell(x, y, z - 1, profile);
  if (!adjacentCave) return null;

  const deep = y < deepStoneStartY;
  const biome = deep ? BiomeType.DeepCave : BiomeType.Cave;
  const floorNoise = random2(x + y * 19 + 2939, z - y * 23 - 2953);
  const sample = { ...profile.noise, corruption: corruptionAt(x, z, y) };

  if (floorNoise > 0.985 && (deep || profile.noise.volcanic > 0.68)) {
    return generatedBlock(WorldMapBlock.Lava, undefined, WorldMapBlock.Lava, biome, profile);
  }
  if (floorNoise > 0.96 && sample.corruption > 0.52) {
    return generatedBlock(WorldMapBlock.ToxicWater, undefined, WorldMapBlock.ToxicWater, biome, profile);
  }

  const wall = deep ? (coalSeamAt(x, y, z, profile.height) ? WorldMapBlock.Coal : WorldMapBlock.DeepStone) : WorldMapBlock.Stone;
  const vegetation =
    deep && floorNoise > 0.45
      ? WorldMapBlock.GlowMycelium
      : floorNoise > 0.9966 && sample.humidity > 0.1
        ? WorldMapBlock.Mushroom
        : floorNoise > 0.5
          ? WorldMapBlock.Lichen
          : floorNoise > 0.38 && sample.humidity > 0.12
            ? WorldMapBlock.Moss
            : undefined;

  return generatedBlock(floorNoise < 0.18 ? WorldMapBlock.Gravel : wall, vegetation, undefined, biome, profile);
}

function coalSeamAt(x, y, z, surfaceY) {
  if (y <= bedrockMaxY + 2 || y >= deepStoneStartY || y >= surfaceY - 10) return false;
  const seamMass = valueNoise(x * 0.031 + 3400, z * 0.031 - 3550);
  if (seamMass < 0.54) return false;
  const layer = ridgeNoise(x * 0.045 + y * 0.08 - 3720, z * 0.045 - y * 0.06 + 3810);
  if (layer < 0.38) return false;
  const pocket = valueNoise(x * 0.12 + y * 0.19 + 3910, z * 0.12 - y * 0.17 - 4020);
  return pocket > 0.18;
}

function fbm(x, z, octaves) {
  let value = 0;
  let amplitude = 0.5;
  let frequency = 1;
  let normalizer = 0;

  for (let i = 0; i < octaves; i++) {
    value += valueNoise(x * frequency, z * frequency) * amplitude;
    normalizer += amplitude;
    amplitude *= 0.5;
    frequency *= 2;
  }

  return value / normalizer;
}

function ridgeNoise(x, z) {
  return 1 - Math.abs(valueNoise(x, z));
}

function valueNoise(x, z) {
  const ix = Math.floor(x);
  const iz = Math.floor(z);
  const fx = smooth(x - ix);
  const fz = smooth(z - iz);
  const a = random2(ix, iz);
  const b = random2(ix + 1, iz);
  const c = random2(ix, iz + 1);
  const d = random2(ix + 1, iz + 1);
  return lerp(lerp(a, b, fx), lerp(c, d, fx), fz) * 2 - 1;
}

function smooth(t) {
  return t * t * (3 - 2 * t);
}

function smoothstep(edge0, edge1, value) {
  const t = Math.max(0, Math.min(1, (value - edge0) / (edge1 - edge0)));
  return t * t * (3 - 2 * t);
}

function random2(x, z) {
  let n = Math.imul(x | 0, 374761393) ^ Math.imul(z | 0, 668265263) ^ Math.imul(worldSeed | 0, 2246822519);
  n = Math.imul(n ^ (n >>> 13), 1274126177);
  return ((n ^ (n >>> 16)) >>> 0) / 4294967295;
}

function hashSeed(seed) {
  const text = String(seed ?? "");
  let hash = 2166136261;
  for (let i = 0; i < text.length; i++) {
    hash ^= text.charCodeAt(i);
    hash = Math.imul(hash, 16777619);
  }
  return hash >>> 0;
}

function lerp(a, b, t) {
  return a + (b - a) * t;
}

function clamp(value, min, max) {
  return Math.max(min, Math.min(max, value));
}

function clampInt(value, min, max) {
  return Math.max(min, Math.min(max, Math.floor(value)));
}
