import { blockKey, parseCellKey } from "./keys.js";
import { canonicalBlockIdAt, canonicalSurfaceHeightAt } from "./canonicalResource.js";
import { EMPTY_BLOCK, WorldMapBlock } from "./blocks.js";
import { minBuildY } from "./config.js";

export function createWorldState() {
  const state = {
    solidBlocks: new Set(),
    generatedSolidRefs: new Map(),
    removedBlocks: new Set(),
    placedBlocks: null,
    placedColumnTops: new Map(),
    dynamicWater: new Set(),
    blockDamage: new Map(),
  };
  state.placedBlocks = createIndexedPlacedBlocks(state);
  return state;
}

export function isSolidCell(state, x, y, z) {
  const key = blockKey(x, y, z);
  if (state.removedBlocks.has(key)) return false;
  if (state.placedBlocks.has(key)) return true;
  if (state.solidBlocks.has(key)) return true;
  const block = canonicalBlockIdAt({ x, y, z });
  return isSolidTerrain(block);
}

export function surfaceHeight(state, x, z) {
  const blockX = Math.round(x);
  const blockZ = Math.round(z);
  const columnTop = state.placedColumnTops?.get(columnKey(blockX, blockZ));
  let top = Math.max(canonicalSurfaceHeightAt({ x: blockX, z: blockZ }), Number.isFinite(columnTop) ? columnTop : -Infinity);
  for (let y = top; y >= minBuildY; y--) {
    const key = blockKey(blockX, y, blockZ);
    if (state.placedBlocks.has(key) && !state.removedBlocks.has(key)) return y;
    if (state.removedBlocks.has(key)) continue;
    const block = canonicalBlockIdAt({ x: blockX, y, z: blockZ });
    if (block && !isWaterTerrain(block)) return y;
  }
  return minBuildY;
}

function createIndexedPlacedBlocks(state) {
  const map = new Map();
  const baseSet = map.set.bind(map);
  const baseDelete = map.delete.bind(map);
  const baseClear = map.clear.bind(map);

  map.set = (key, value) => {
    const result = baseSet(key, value);
    indexPlacedBlockKey(state, key);
    return result;
  };
  map.delete = (key) => {
    const deleted = baseDelete(key);
    if (deleted) reindexPlacedColumn(state, key);
    return deleted;
  };
  map.clear = () => {
    baseClear();
    state.placedColumnTops.clear();
  };

  return map;
}

function indexPlacedBlockKey(state, key) {
  const [x, y, z] = parseCellKey(key);
  if (!Number.isFinite(x) || !Number.isFinite(y) || !Number.isFinite(z)) return;
  const column = columnKey(x, z);
  const current = state.placedColumnTops.get(column);
  if (!Number.isFinite(current) || y > current) state.placedColumnTops.set(column, y);
}

function reindexPlacedColumn(state, key) {
  const [x, , z] = parseCellKey(key);
  if (!Number.isFinite(x) || !Number.isFinite(z)) return;
  const column = columnKey(x, z);
  let top = -Infinity;
  for (const placedKey of state.placedBlocks.keys()) {
    const [px, py, pz] = parseCellKey(placedKey);
    if (px === x && pz === z && py > top) top = py;
  }
  if (Number.isFinite(top)) state.placedColumnTops.set(column, top);
  else state.placedColumnTops.delete(column);
}

function columnKey(x, z) {
  return `${x},${z}`;
}

function isWaterTerrain(block) {
  return block === WorldMapBlock.Water || block === WorldMapBlock.SwampWater || block === WorldMapBlock.ToxicWater || block === WorldMapBlock.Ice;
}

function isSolidTerrain(block) {
  return block !== EMPTY_BLOCK &&
    block !== WorldMapBlock.Water &&
    block !== WorldMapBlock.SwampWater &&
    block !== WorldMapBlock.ToxicWater;
}
