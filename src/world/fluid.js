import { chunkSize, waterFlowBudget } from "./config.js";
import { blockKey, chunkKey } from "./keys.js";
import { getGeneratedBlock } from "./generator.js";
import { isSolidCell } from "./state.js";
import { hasSourceWaterAt } from "./water.js";

export function flowWaterFromBreak(state, block) {
  const queue = [];
  const seeds = [
    [block.x, block.y, block.z],
    [block.x + 1, block.y, block.z],
    [block.x - 1, block.y, block.z],
    [block.x, block.y, block.z + 1],
    [block.x, block.y, block.z - 1],
    [block.x, block.y + 1, block.z],
  ];

  for (const [x, y, z] of seeds) {
    if (hasWaterAt(state, x, y, z)) queue.push([x, y, z]);
    if (hasWaterAt(state, x, y + 1, z)) queue.push([x, y + 1, z]);
  }

  const changedChunks = new Set();
  const visited = new Set();
  let budget = waterFlowBudget;

  while (queue.length && budget-- > 0) {
    const [x, y, z] = queue.shift();
    const visitKey = blockKey(x, y, z);
    if (visited.has(visitKey)) continue;
    visited.add(visitKey);

    const down = [x, y - 1, z];
    if (canWaterOccupy(state, ...down)) {
      addDynamicWater(state, ...down, changedChunks);
      queue.push(down);
    }

    for (const next of [
      [x + 1, y, z],
      [x - 1, y, z],
      [x, y, z + 1],
      [x, y, z - 1],
    ]) {
      if (!canWaterOccupy(state, ...next)) continue;
      addDynamicWater(state, ...next, changedChunks);
      queue.push(next);
    }
  }

  return changedChunks;
}

export function hasWaterAt(state, x, y, z) {
  const key = blockKey(x, y, z);
  if (state.placedBlocks.has(key)) return false;
  return (!state.removedBlocks.has(key) && hasSourceWaterAt(x, y, z)) || (state.dynamicWater.has(key) && !isSolidCell(state, x, y, z));
}

function addDynamicWater(state, x, y, z, changedChunks) {
  const key = blockKey(x, y, z);
  if (state.dynamicWater.has(key) || hasSourceWaterAt(x, y, z)) return;
  state.dynamicWater.add(key);
  changedChunks.add(chunkKey(Math.floor(x / chunkSize), Math.floor(z / chunkSize)));
}

function canWaterOccupy(state, x, y, z) {
  if (y <= 0 || hasWaterAt(state, x, y, z)) return false;
  const key = blockKey(x, y, z);
  if (state.placedBlocks.has(key)) return false;
  if (isSolidCell(state, x, y, z)) return false;
  return state.removedBlocks.has(key) || getGeneratedBlock(x, y, z) === null;
}
