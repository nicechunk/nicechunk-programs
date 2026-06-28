import { currentWorldSeed, getGeneratedBlock } from "./generator.js";
import { WorldMapBlock } from "./blocks.js";
import { blockKey } from "./keys.js";

const sourceWaterCache = new Map();
let cachedWorldSeed = currentWorldSeed();

export function hasSourceWaterAt(x, y, z) {
  if (cachedWorldSeed !== currentWorldSeed()) {
    sourceWaterCache.clear();
    cachedWorldSeed = currentWorldSeed();
  }

  const key = blockKey(x, y, z);
  if (!sourceWaterCache.has(key)) {
    const block = getGeneratedBlock(x, y, z);
    const hasNaturalFluid = [WorldMapBlock.Water, WorldMapBlock.SwampWater, WorldMapBlock.ToxicWater, WorldMapBlock.Ice].includes(block?.terrain);
    sourceWaterCache.set(key, hasNaturalFluid);
  }
  return sourceWaterCache.get(key);
}
