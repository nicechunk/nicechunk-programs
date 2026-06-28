export const defaultWorldSeed = "ba1a9d446157c537af58fc5ff53a28422cdc6ab3dd88daa24095db3bd9c0f041";

const playWorldSeedStorageKey = "nicechunk.play.worldSeed.v1";
const worldConfigStorageKey = "nicechunk.worldConfig.v1";

export function persistPlayWorldSeed(seed) {
  const normalized = normalizeSeed(seed);
  try {
    localStorage.setItem(playWorldSeedStorageKey, normalized);
  } catch {
    // The game can still run if browser storage is unavailable.
  }
}

export function readPlayWorldSeed() {
  return readStoredSeed(playWorldSeedStorageKey) ?? readCachedWorldConfigSeed();
}

export function normalizeSeed(seed) {
  const normalized = String(seed ?? "").trim();
  return normalized || defaultWorldSeed;
}

function readStoredSeed(key) {
  try {
    const seed = localStorage.getItem(key);
    return seed ? normalizeSeed(seed) : null;
  } catch {
    return null;
  }
}

function readCachedWorldConfigSeed() {
  try {
    const parsed = JSON.parse(localStorage.getItem(worldConfigStorageKey) || "null");
    return parsed?.worldSeedHex ? normalizeSeed(parsed.worldSeedHex) : null;
  } catch {
    return null;
  }
}
