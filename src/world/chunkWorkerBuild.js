import { chunkSize, minBuildY } from "./config.js";
import { currentWorldSeed, setWorldSeed, terrainProfile } from "./generator.js";
import { canonicalAboveSurfaceBlocksInArea, canonicalRenderTypeAt, canonicalSurfaceHeightAt, canonicalWaterLevelAt, setCanonicalWorldConfig } from "./canonicalResource.js";
import { WorldMapBlock, renderTypeForBlock } from "./blocks.js";
import { blockKey, parseCellKey } from "./keys.js";

const cubeHalfSize = 0.5;
const waterVisualHeightScale = 2 / 3;
const waterVisualCenterOffset = -1 / 6;
const cavityNeighborOffsets = [
  [1, 0, 0],
  [-1, 0, 0],
  [0, 1, 0],
  [0, -1, 0],
  [0, 0, 1],
  [0, 0, -1],
];

export function buildChunkRenderData({ chunkX, chunkZ, detailMode = "surface", worldConfig = null, removedKeys = [], placedEntries = [], dynamicWaterKeys = [] } = {}) {
  if (worldConfig) {
    setCanonicalWorldConfig(worldConfig);
    const worldSeed = worldConfig.worldSeedHex ?? (Array.isArray(worldConfig.worldSeed) ? worldConfig.worldSeed.join(",") : "");
    if (worldSeed) setWorldSeed(worldSeed);
  }

  const removed = new Set(removedKeys);
  const placed = new Map(placedEntries);
  const dynamicWater = new Set(dynamicWaterKeys);
  const surfaceAt = createSurfaceReader();
  const fullDetail = detailMode === "full";
  const decorationDetail = detailMode !== "surface";
  const treeDetail = detailMode !== "surface";
  const retainCollisionKeys = detailMode === "full" || detailMode === "decorated";
  const terrainColumnDepth = fullDetail ? 2 : 0;
  const byType = new Map();
  const waterInstances = [];
  const solidKeys = new Set();
  const minX = chunkX * chunkSize;
  const maxX = minX + chunkSize - 1;
  const minZ = chunkZ * chunkSize;
  const maxZ = minZ + chunkSize - 1;
  const aboveSurfaceByColumn = treeDetail ? createAboveSurfaceColumnMap(canonicalAboveSurfaceBlocksInArea({ minX, maxX, minZ, maxZ })) : null;
  const decorationInstances = [];
  const decorationBudget = {
    grassTuft: 4,
    dryGrassTuft: 6,
    pebble: 18,
    shoreDamp: 30,
    shoreFoam: 22,
    reed: 10,
    mushroom: 1,
    fallenLog: 2,
  };
  const vegetationBudget = {
    bush: 2,
    cactus: 3,
    reed: 4,
    vine: 2,
    groundPatch: 4,
    coral: 2,
    tuft: 6,
    mushroom: 1,
    large: 1,
    thorn: 2,
  };

  for (let localZ = 0; localZ < chunkSize; localZ += 1) {
    for (let localX = 0; localX < chunkSize; localX += 1) {
      const x = minX + localX;
      const z = minZ + localZ;
      const height = surfaceAt(x, z);
      const columnStart = exposedColumnStart(x, z, height, terrainColumnDepth, surfaceAt);

      for (let y = Math.max(minBuildY, columnStart); y <= height; y += 1) {
        const key = blockKey(x, y, z);
        if (placed.has(key) && !removed.has(key)) continue;
        const type = canonicalRenderTypeAt({ x, y, z });
        if (!type || removed.has(key)) continue;
        addVoxel(byType, solidKeys, x, y, z, type, key);
      }

      const waterLevel = canonicalWaterLevelAt({ x, z, surface: height });
      const underwater = waterLevel !== null && waterLevel > height;
      let canonicalAboveSurface = false;
      if (treeDetail) {
        if (!underwater && !removed.has(blockKey(x, height, z))) {
          canonicalAboveSurface = addAboveSurfaceColumnVoxels(aboveSurfaceByColumn, byType, solidKeys, removed, placed, x, z);
        }
      }

      if (decorationDetail && !underwater && !removed.has(blockKey(x, height, z))) {
        const profile = terrainProfile(x, z);
        if (
          !canonicalAboveSurface &&
          profile.vegetation &&
          height > 3 &&
          shouldPlaceVegetationDecoration(vegetationBudget, profile.vegetation, x, z)
        ) {
          addVegetationDecoration(decorationInstances, x, height, z, profile.vegetation);
        }
        addSurfaceDetail(decorationInstances, decorationBudget, x, height, z, profile, canonicalAboveSurface);
      }

      if (waterLevel !== null && waterLevel > height) {
        waterInstances.push({ x, y: waterLevel, z, type: "water" });
        if (fullDetail) addWaterEdgeDetail(decorationInstances, decorationBudget, x, z, waterLevel);
      }
    }
  }

  for (const [key, type] of placed) {
    if (removed.has(key)) continue;
    const [x, y, z] = parseCellKey(key);
    if (Math.floor(x / chunkSize) !== chunkX || Math.floor(z / chunkSize) !== chunkZ) continue;
    if (isNonSolidVisualType(type)) {
      waterInstances.push({ x, y, z, type });
    } else {
      addVoxel(byType, solidKeys, x, y, z, type, key);
    }
  }

  for (const key of dynamicWater) {
    if (removed.has(key)) continue;
    const [x, y, z] = parseCellKey(key);
    if (Math.floor(x / chunkSize) !== chunkX || Math.floor(z / chunkSize) !== chunkZ) continue;
    waterInstances.push({ x, y, z, type: "water" });
  }

  addRemovedBlockCavityShell({ byType, solidKeys, chunkX, chunkZ, removed, placed });

  const chunkMeshSolidKeys = new Set();
  for (const entries of byType.values()) {
    for (const entry of entries) chunkMeshSolidKeys.add(entry.key);
  }

  const meshes = [];
  const transfer = [];
  for (const [type, entries] of byType) {
    const mesh = createVisibleVoxelMesh(entries, chunkMeshSolidKeys, removed, placed, surfaceAt);
    if (!mesh) continue;
    mesh.type = type;
    meshes.push(mesh);
    transfer.push(mesh.positions.buffer, mesh.normals.buffer, mesh.indices.buffer);
  }

  const instances = buildInstances([...waterInstances, ...decorationInstances]);
  for (const item of instances) transfer.push(item.matrices.buffer);

  return {
    chunkX,
    chunkZ,
    detailMode,
    meshes,
    instances,
    solidKeys: retainCollisionKeys ? Array.from(solidKeys) : [],
    transfer,
  };
}

function createAboveSurfaceColumnMap(blocks) {
  const byColumn = new Map();
  for (const block of blocks) {
    let byZ = byColumn.get(block.x);
    if (!byZ) {
      byZ = new Map();
      byColumn.set(block.x, byZ);
    }
    if (!byZ.has(block.z)) byZ.set(block.z, []);
    byZ.get(block.z).push(block);
  }
  return byColumn;
}

function addAboveSurfaceColumnVoxels(aboveSurfaceByColumn, byType, solidKeys, removed, placed, x, z) {
  const blocks = aboveSurfaceByColumn?.get(x)?.get(z);
  if (!blocks?.length) return false;
  let added = false;
  for (const block of blocks) {
    const key = blockKey(block.x, block.y, block.z);
    if (removed.has(key) || (placed.has(key) && !removed.has(key))) continue;
    addVoxel(byType, solidKeys, block.x, block.y, block.z, block.type, key);
    added = true;
  }
  return added;
}

function addVoxel(byType, solidKeys, x, y, z, type, key) {
  if (!type || isNonSolidVisualType(type)) return;
  if (!byType.has(type)) byType.set(type, []);
  byType.get(type).push({ x, y, z, type, key });
  solidKeys.add(key);
}

function addRemovedBlockCavityShell({ byType, solidKeys, chunkX, chunkZ, removed, placed }) {
  if (!removed.size) return;
  const minX = chunkX * chunkSize;
  const maxX = minX + chunkSize - 1;
  const minZ = chunkZ * chunkSize;
  const maxZ = minZ + chunkSize - 1;

  for (const removedKey of removed) {
    const [rx, ry, rz] = parseCellKey(removedKey);
    if (!Number.isFinite(rx) || !Number.isFinite(ry) || !Number.isFinite(rz)) continue;

    for (const [dx, dy, dz] of cavityNeighborOffsets) {
      const x = rx + dx;
      const y = ry + dy;
      const z = rz + dz;
      if (x < minX || x > maxX || z < minZ || z > maxZ) continue;

      const key = blockKey(x, y, z);
      if (removed.has(key) || placed.has(key) || solidKeys.has(key)) continue;

      const type = canonicalRenderTypeAt({ x, y, z });
      if (!type || isNonSolidVisualType(type)) continue;
      addVoxel(byType, solidKeys, x, y, z, type, key);
    }
  }
}

function addVegetationDecoration(out, x, height, z, vegetation) {
  const type = renderTypeForBlock(vegetation);
  if (!type) return;

  if (vegetation === WorldMapBlock.Cactus) {
    const cactusHeight = 1.4 + random2(x + 601, z - 607) * 1.2;
    addInstance(out, type, x, height + 0.5 + cactusHeight * 0.5, z, 0.42, cactusHeight, 0.42);
    return;
  }

  if (vegetation === WorldMapBlock.Bush || vegetation === WorldMapBlock.DeadBush || vegetation === WorldMapBlock.SnowBush) {
    addBushCluster(out, x, height, z, type, vegetation);
    return;
  }

  if (vegetation === WorldMapBlock.GiantRoot || vegetation === WorldMapBlock.DeadWood) {
    addInstance(out, type, x, height + 0.64, z, 0.32, 0.26, 1.45, random2(x + 617, z - 619) * Math.PI);
    return;
  }

  if (vegetation === WorldMapBlock.Reed) {
    addReedCluster(out, x, z, height + 0.5);
    return;
  }

  if (vegetation === WorldMapBlock.Mushroom) {
    addMushroomCluster(out, x, height, z);
    return;
  }

  if (vegetation === WorldMapBlock.Coral || vegetation === WorldMapBlock.DeadCoral) {
    const count = 2 + Math.floor(random2(x - 631, z + 641) * 3);
    for (let i = 0; i < count; i += 1) {
      const px = x + randomOffset(x, z, 61 + i * 2) * 0.72;
      const pz = z + randomOffset(x, z, 62 + i * 2) * 0.72;
      const sy = 0.34 + random2(x + i * 643, z - i * 647) * 0.34;
      addInstance(out, type, px, height + 0.52 + sy * 0.5, pz, 0.18, sy, 0.18);
    }
    return;
  }

  if (vegetation === WorldMapBlock.Vine) {
    addInstance(out, "vine", x + randomOffset(x, z, 63), height + 0.54, z + randomOffset(x, z, 64), 0.42, 1, 0.9, random2(x + 653, z - 659) * Math.PI);
    return;
  }

  if (vegetation === WorldMapBlock.Seaweed || vegetation === WorldMapBlock.AquaticPlant || vegetation === WorldMapBlock.SwampGrass) {
    addTuft(out, type, x, height, z, 0.52, 0.42);
    return;
  }

  if (vegetation === WorldMapBlock.ShellBed || vegetation === WorldMapBlock.Moss || vegetation === WorldMapBlock.Lichen || vegetation === WorldMapBlock.GlowMycelium) {
    addGroundPatch(out, type, x, height, z, 0.58, 0.32, 67);
    return;
  }

  if (vegetation === WorldMapBlock.Thorn) {
    addInstance(out, "thorn", x, height + 0.78, z, 0.1, 0.72, 0.48, random2(x + 661, z - 673) * Math.PI);
    return;
  }

  addTuft(out, type, x, height, z, 0.46, 0.34);
}

function shouldPlaceVegetationDecoration(budget, vegetation, x, z) {
  const category = vegetationDecorationCategory(vegetation);
  if ((budget[category] ?? 0) <= 0) return false;
  if (random2(x + 2309, z - 2311) >= vegetationDecorationChance(category)) return false;
  budget[category] -= 1;
  return true;
}

function vegetationDecorationCategory(vegetation) {
  if (vegetation === WorldMapBlock.Bush || vegetation === WorldMapBlock.DeadBush || vegetation === WorldMapBlock.SnowBush) return "bush";
  if (vegetation === WorldMapBlock.Cactus) return "cactus";
  if (vegetation === WorldMapBlock.Reed || vegetation === WorldMapBlock.Seaweed || vegetation === WorldMapBlock.AquaticPlant || vegetation === WorldMapBlock.SwampGrass) return "reed";
  if (vegetation === WorldMapBlock.Vine) return "vine";
  if (vegetation === WorldMapBlock.ShellBed || vegetation === WorldMapBlock.Moss || vegetation === WorldMapBlock.Lichen || vegetation === WorldMapBlock.GlowMycelium) return "groundPatch";
  if (vegetation === WorldMapBlock.Coral || vegetation === WorldMapBlock.DeadCoral) return "coral";
  if (vegetation === WorldMapBlock.Mushroom) return "mushroom";
  if (vegetation === WorldMapBlock.GiantRoot || vegetation === WorldMapBlock.DeadWood) return "large";
  if (vegetation === WorldMapBlock.Thorn) return "thorn";
  return "tuft";
}

function vegetationDecorationChance(category) {
  switch (category) {
    case "bush": return 0.1;
    case "vine": return 0.08;
    case "cactus": return 0.16;
    case "reed": return 0.18;
    case "groundPatch": return 0.16;
    case "coral": return 0.12;
    case "mushroom": return 0.34;
    case "large": return 0.08;
    case "thorn": return 0.12;
    default: return 0.14;
  }
}

function addSurfaceDetail(out, budget, x, height, z, profile, treeCell) {
  if ((profile.terrain === WorldMapBlock.Grass || profile.terrain === WorldMapBlock.Mud) && profile.slope <= 2 && height > 3 && !treeCell) {
    const tuftRoll = random2(x + 149, z - 173);
    const forestBoost = profile.forest > 0.55 ? 0.08 : 0;
    if (tuftRoll > 0.985 - forestBoost * 0.25 && budget.grassTuft > 0) {
      addTuft(out, "grassPlant", x, height, z, 0.46, 0.34);
      budget.grassTuft -= 1;
    } else if (tuftRoll < 0.025 && budget.dryGrassTuft > 0) {
      addTuft(out, "dryGrass", x, height, z, 0.34, 0.26);
      budget.dryGrassTuft -= 1;
    }

    if (profile.forest > 0.62 && budget.dryGrassTuft > 0 && random2(x - 1487, z + 1511) > 0.985) {
      addGroundPatch(out, "dryGrass", x, height, z, 0.5, 0.22, 17);
      budget.dryGrassTuft -= 1;
    }

    if (budget.mushroom > 0 && profile.forest > 0.5 && random2(x + 2221, z - 2237) > 0.99 && isMushroomClusterCell(x, z, profile)) {
      addMushroomCluster(out, x, height, z);
      budget.mushroom -= 1;
    }

    if (budget.fallenLog > 0 && profile.forest > 0.74 && profile.slope <= 1 && random2(x + 1931, z - 1949) > 0.47) {
      addFallenLog(out, x, height, z);
      budget.fallenLog -= 1;
    }
  }

  const stonySurface = [WorldMapBlock.Stone, WorldMapBlock.DeepStone, WorldMapBlock.Gravel, WorldMapBlock.Basalt, WorldMapBlock.Ash].includes(profile.terrain);
  const sandySurface = [WorldMapBlock.Sand, WorldMapBlock.DryDirt, WorldMapBlock.SaltFlat, WorldMapBlock.Quicksand].includes(profile.terrain);
  if ((sandySurface || stonySurface) && budget.pebble > 0) {
    const shardChance = stonySurface ? 0.43 - Math.min(profile.slope, 5) * 0.018 : 1;
    const pebbleChance = stonySurface ? 0.45 : 0.495;
    if (!sandySurface && random2(x + 2099, z - 2111) > shardChance) {
      addRockShard(out, x, height, z, profile);
      budget.pebble -= 1;
    } else if (random2(x - 509, z + 419) > pebbleChance) {
      addPebble(out, x, height, z);
      budget.pebble -= 1;
    }
  }

  if (stonySurface && profile.slope <= 1 && budget.grassTuft > 0 && random2(x + 1559, z - 1571) > 0.49) {
    addGroundPatch(out, "lichen", x, height, z, 0.28, 0.2, 19);
    budget.grassTuft -= 1;
  }

  if ((profile.terrain === WorldMapBlock.Snow || profile.terrain === WorldMapBlock.FrozenSoil) && profile.slope <= 3 && budget.shoreFoam > 0 && random2(x - 1607, z + 1613) > 0.9) {
    addGroundPatch(out, "shoreFoam", x, height, z, 0.62, 0.16, 23);
    budget.shoreFoam -= 1;
  }

  if (profile.terrain === WorldMapBlock.Sand && budget.shoreDamp > 0 && isNearWater(x, z)) {
    addGroundPatch(out, "shoreDamp", x, height, z, 0.58, 0.42, 11);
    budget.shoreDamp -= 1;
  }

  if ((profile.terrain === WorldMapBlock.Sand || profile.terrain === WorldMapBlock.Mud || profile.terrain === WorldMapBlock.Clay) && budget.reed > 0 && isNearWater(x, z) && random2(x + 1811, z - 1823) > 0.42) {
    addReedCluster(out, x, z, height + 0.5);
    budget.reed -= 1;
  }
}

function addWaterEdgeDetail(out, budget, x, z, waterY) {
  if (!isNearLand(x, z)) return;
  if (budget.shoreFoam > 0 && random2(x + 1291, z - 1301) >= 0.22) {
    const sx = 0.44 + random2(x - 1319, z + 1321) * 0.34;
    const sz = 0.16 + random2(x + 1327, z - 1361) * 0.16;
    addInstance(out, "shoreFoam", x + randomOffset(x, z, 13) * 0.5, waterY + 0.572, z + randomOffset(x, z, 14) * 0.5, sx, 1, sz, random2(x + 1367, z - 1373) * Math.PI);
    budget.shoreFoam -= 1;
  }
  if (budget.reed > 0 && random2(x - 1709, z + 1721) > 0.42) {
    addReedCluster(out, x, z, waterY + 0.6);
    budget.reed -= 1;
  }
}

function addBushCluster(out, x, height, z, leafType, vegetation) {
  const centerX = x + randomOffset(x, z, 51) * 0.8;
  const centerZ = z + randomOffset(x, z, 52) * 0.8;
  const branchType = vegetation === WorldMapBlock.SnowBush ? "pineTrunk" : vegetation === WorldMapBlock.DeadBush ? "deadWood" : "trunkDark";
  const branchCount = 2 + Math.floor(random2(x + 1649, z - 1657) * 2);
  for (let i = 0; i < branchCount; i += 1) {
    const yaw = random2(x + i * 1663, z - i * 1667) * Math.PI * 2;
    addInstance(out, branchType, centerX, height + 0.62 + random2(x + i * 1741, z - i * 1747) * 0.24, centerZ, 0.075, 0.085, 0.48, yaw);
  }
  const leafCount = vegetation === WorldMapBlock.DeadBush ? 1 : 2 + Math.floor(random2(x - 1753, z + 1759) * 2);
  for (let i = 0; i < leafCount; i += 1) {
    const yaw = (i / leafCount) * Math.PI * 2 + random2(x + i * 1787, z - i * 1789) * 0.42;
    const width = (vegetation === WorldMapBlock.SnowBush ? 0.46 : 0.52) + random2(x + i * 1817, z - i * 1823) * 0.18;
    addInstance(out, leafType, centerX, height + 0.78 + random2(x + i * 1861, z - i * 1867) * 0.18, centerZ, width, vegetation === WorldMapBlock.DeadBush ? 0.34 : 0.46, 0.08, yaw);
  }
}

function addReedCluster(out, x, z, baseY) {
  const count = 1 + Math.floor(random2(x + 1733, z - 1741) * 3);
  for (let i = 0; i < count; i += 1) {
    const px = x + randomOffset(x, z, 25 + i * 2) * 0.72;
    const pz = z + randomOffset(x, z, 26 + i * 2) * 0.72;
    const height = 0.72 + random2(x + i * 1753, z - i * 1759) * 0.5;
    addInstance(out, "reedStem", px, baseY + height * 0.5, pz, 0.08, height, 0.08);
    if (random2(x - i * 1777, z + i * 1783) > 0.38) addInstance(out, "reedTip", px, baseY + 0.06 + height, pz, 0.12, 0.18, 0.12);
  }
}

function addMushroomCluster(out, x, height, z) {
  const count = 1 + Math.floor(random2(x + 1871, z - 1877) * 3);
  for (let i = 0; i < count; i += 1) {
    const px = x + randomOffset(x, z, 31 + i * 2) * 0.76;
    const pz = z + randomOffset(x, z, 32 + i * 2) * 0.76;
    const stemHeight = 0.16 + random2(x + i * 1889, z - i * 1901) * 0.12;
    const capWidth = 0.22 + random2(x - i * 1907, z + i * 1913) * 0.12;
    addInstance(out, "mushroomStem", px, height + 0.52 + stemHeight * 0.5, pz, 0.08, stemHeight, 0.08);
    addInstance(out, "mushroomCap", px, height + 0.56 + stemHeight, pz, capWidth, 0.1, capWidth);
  }
}

function addFallenLog(out, x, height, z) {
  const yaw = Math.round(random2(x - 1951, z + 1973) * 7) * (Math.PI / 4);
  const length = 1.35 + random2(x + 1979, z - 1987) * 0.85;
  const px = x + randomOffset(x, z, 37) * 0.42;
  const pz = z + randomOffset(x, z, 38) * 0.42;
  const trunkType = random2(x - 1993, z + 1997) > 0.78 ? "trunkDark" : "trunk";
  addInstance(out, trunkType, px, height + 0.62, pz, 0.28, 0.28, length, yaw);
}

function isMushroomClusterCell(x, z, profile) {
  const cellSize = profile.forest > 0.74 ? 10 : 14;
  const cellX = Math.floor(x / cellSize);
  const cellZ = Math.floor(z / cellSize);
  if (random2(cellX + 2027, cellZ - 2029) < (profile.forest > 0.74 ? 0.9958 : 0.9964)) return false;
  const inner = Math.max(1, cellSize - 4);
  const clusterX = cellX * cellSize + 2 + Math.floor(random2(cellX - 2039, cellZ + 2053) * inner);
  const clusterZ = cellZ * cellSize + 2 + Math.floor(random2(cellX + 2063, cellZ - 2069) * inner);
  return Math.abs(x - clusterX) <= 1 && Math.abs(z - clusterZ) <= 1 && random2(x - 2081, z + 2083) > 0.24;
}

function addTuft(out, type, x, height, z, width, depth) {
  addInstance(out, type, x + randomOffset(x, z, 7) * 0.72, height + 0.516, z + randomOffset(x, z, 8) * 0.72, width, 1, depth, random2(x + 701, z - 733) * Math.PI);
}

function addGroundPatch(out, type, x, height, z, width, depth, salt) {
  addInstance(out, type, x + randomOffset(x, z, salt) * 0.78, height + 0.516, z + randomOffset(x, z, salt + 1) * 0.78, width, 1, depth, random2(x + salt * 79, z - salt * 83) * Math.PI);
}

function addPebble(out, x, height, z) {
  const sx = 0.12 + random2(x + 941, z - 967) * 0.13;
  const sy = 0.06 + random2(x - 983, z + 997) * 0.06;
  const sz = 0.1 + random2(x + 1013, z - 1031) * 0.12;
  addInstance(out, "pebble", x + randomOffset(x, z, 9) * 0.88, height + 0.5 + sy * 0.5, z + randomOffset(x, z, 10) * 0.88, sx, sy, sz);
}

function addRockShard(out, x, height, z, profile) {
  const longSide = 0.22 + random2(x + 2141, z - 2143) * 0.22;
  const shortSide = 0.08 + random2(x - 2153, z + 2161) * 0.08;
  const heightScale = 0.08 + Math.min(profile.slope, 5) * 0.012 + random2(x + 2179, z - 2203) * 0.04;
  addInstance(out, "pebble", x + randomOffset(x, z, 41) * 0.82, height + 0.5 + heightScale * 0.5, z + randomOffset(x, z, 42) * 0.82, shortSide, heightScale, longSide, random2(x - 2131, z + 2137) * Math.PI);
}

function isNearWater(x, z) {
  return hasSurfaceWater(x + 1, z) || hasSurfaceWater(x - 1, z) || hasSurfaceWater(x, z + 1) || hasSurfaceWater(x, z - 1);
}

function isNearLand(x, z) {
  return !hasSurfaceWater(x + 1, z) || !hasSurfaceWater(x - 1, z) || !hasSurfaceWater(x, z + 1) || !hasSurfaceWater(x, z - 1);
}

function hasSurfaceWater(x, z) {
  const surface = canonicalSurfaceHeightAt({ x, z });
  const waterLevel = canonicalWaterLevelAt({ x, z, surface });
  return waterLevel !== null && surface < waterLevel;
}

function randomOffset(x, z, salt) {
  return (random2(x + salt * 101, z - salt * 131) - 0.5) * 0.52;
}

function addInstance(out, type, x, y, z, sx = 1, sy = 1, sz = 1, yaw = 0) {
  out.push({ type, x, y, z, sx, sy, sz, yaw });
}

function buildInstances(cells) {
  if (!cells.length) return [];
  const byType = new Map();
  for (const cell of cells) {
    if (!byType.has(cell.type)) byType.set(cell.type, []);
    byType.get(cell.type).push(cell);
  }
  const out = [];
  for (const [type, cells] of byType) {
    const matrices = new Float32Array(cells.length * 16);
    cells.forEach((cell, index) => {
      const shortWater = isNonSolidVisualType(cell.type);
      writeMatrix(
        matrices,
        index * 16,
        cell.x,
        shortWater ? cell.y + waterVisualCenterOffset : cell.y,
        cell.z,
        cell.sx ?? 1,
        shortWater ? waterVisualHeightScale : cell.sy ?? 1,
        cell.sz ?? 1,
        cell.yaw ?? 0,
      );
    });
    out.push({ type, matrices, count: cells.length, interactive: false });
  }
  return out;
}

function writeMatrix(out, offset, x, y, z, sx = 1, sy = 1, sz = 1, yaw = 0) {
  const c = Math.cos(yaw);
  const s = Math.sin(yaw);
  out[offset + 0] = c * sx;
  out[offset + 1] = 0;
  out[offset + 2] = -s * sx;
  out[offset + 3] = 0;
  out[offset + 4] = 0;
  out[offset + 5] = sy;
  out[offset + 6] = 0;
  out[offset + 7] = 0;
  out[offset + 8] = s * sz;
  out[offset + 9] = 0;
  out[offset + 10] = c * sz;
  out[offset + 11] = 0;
  out[offset + 12] = x;
  out[offset + 13] = y;
  out[offset + 14] = z;
  out[offset + 15] = 1;
}

function random2(x, z) {
  let n = Math.imul(x | 0, 374761393) ^ Math.imul(z | 0, 668265263) ^ Math.imul(currentWorldSeed() | 0, 2246822519);
  n = Math.imul(n ^ (n >>> 13), 1274126177);
  return ((n ^ (n >>> 16)) >>> 0) / 4294967295;
}

function createVisibleVoxelMesh(entries, chunkMeshSolidKeys, removed, placed, surfaceAt) {
  const positions = [];
  const normals = [];
  const indices = [];
  const occlusionMemo = new Map();
  const faceGroups = new Map();

  for (const entry of entries) {
    if (removed.has(entry.key)) continue;
    for (let faceIndex = 0; faceIndex < voxelFaces.length; faceIndex += 1) {
      const face = voxelFaces[faceIndex];
      if (isVoxelFaceOccluded(chunkMeshSolidKeys, removed, placed, occlusionMemo, surfaceAt, entry.x + face.dx, entry.y + face.dy, entry.z + face.dz)) continue;
      addGreedyFaceCell(faceGroups, faceIndex, entry.x, entry.y, entry.z);
    }
  }

  for (const group of faceGroups.values()) appendGreedyFaceGroup(group, positions, normals, indices);
  if (!indices.length) return null;
  return {
    positions: new Float32Array(positions),
    normals: new Float32Array(normals),
    indices: indices.length > 65535 ? new Uint32Array(indices) : new Uint16Array(indices),
  };
}

function isVoxelFaceOccluded(chunkMeshSolidKeys, removed, placed, occlusionMemo, surfaceAt, x, y, z) {
  const key = blockKey(x, y, z);
  if (removed.has(key)) return false;
  if (chunkMeshSolidKeys.has(key)) return true;
  const placedType = placed.get(key);
  if (placedType && !isNonSolidVisualType(placedType)) return true;
  if (occlusionMemo.has(key)) return occlusionMemo.get(key);
  const occluded = y <= surfaceAt(x, z);
  occlusionMemo.set(key, occluded);
  return occluded;
}

function createSurfaceReader() {
  const cache = new Map();
  return (x, z) => {
    const cachedByZ = cache.get(x);
    const cached = cachedByZ?.get(z);
    if (cached !== undefined) return cached;
    const height = canonicalSurfaceHeightAt({ x, z });
    if (cachedByZ) {
      cachedByZ.set(z, height);
    } else {
      cache.set(x, new Map([[z, height]]));
    }
    return height;
  };
}

function exposedColumnStart(x, z, height, maxDepth, surfaceAt) {
  const neighborFloor = Math.min(
    surfaceAt(x + 1, z),
    surfaceAt(x - 1, z),
    surfaceAt(x, z + 1),
    surfaceAt(x, z - 1),
    surfaceAt(x + 1, z + 1),
    surfaceAt(x - 1, z - 1),
    surfaceAt(x + 1, z - 1),
    surfaceAt(x - 1, z + 1),
  );
  return Math.max(minBuildY, Math.min(height - maxDepth, neighborFloor + 1));
}

function addGreedyFaceCell(faceGroups, faceIndex, x, y, z) {
  const cell = greedyFaceCell(faceIndex, x, y, z);
  const groupKey = `${faceIndex}:${cell.plane}`;
  let group = faceGroups.get(groupKey);
  if (!group) {
    group = { faceIndex, plane: cell.plane, cellsByV: new Map(), minU: cell.u, maxU: cell.u, minV: cell.v, maxV: cell.v };
    faceGroups.set(groupKey, group);
  }
  addGreedyCell(group, cell.u, cell.v);
  group.minU = Math.min(group.minU, cell.u);
  group.maxU = Math.max(group.maxU, cell.u);
  group.minV = Math.min(group.minV, cell.v);
  group.maxV = Math.max(group.maxV, cell.v);
}

function addGreedyCell(group, u, v) {
  let row = group.cellsByV.get(v);
  if (!row) {
    row = new Set();
    group.cellsByV.set(v, row);
  }
  row.add(u);
}

function hasGreedyCell(group, u, v) {
  return group.cellsByV.get(v)?.has(u) ?? false;
}

function greedyFaceCell(faceIndex, x, y, z) {
  switch (faceIndex) {
    case 0: return { plane: x + cubeHalfSize, u: z, v: y };
    case 1: return { plane: x - cubeHalfSize, u: z, v: y };
    case 2: return { plane: y + cubeHalfSize, u: x, v: z };
    case 3: return { plane: y - cubeHalfSize, u: x, v: z };
    case 4: return { plane: z + cubeHalfSize, u: x, v: y };
    default: return { plane: z - cubeHalfSize, u: x, v: y };
  }
}

function appendGreedyFaceGroup(group, positions, normals, indices) {
  const visitedByV = new Map();
  for (let v = group.minV; v <= group.maxV; v += 1) {
    for (let u = group.minU; u <= group.maxU; u += 1) {
      if (hasVisitedGreedyCell(visitedByV, u, v) || !hasGreedyCell(group, u, v)) continue;
      let width = 1;
      while (hasGreedyCell(group, u + width, v) && !hasVisitedGreedyCell(visitedByV, u + width, v)) width += 1;
      let height = 1;
      growHeight: while (v + height <= group.maxV) {
        for (let dx = 0; dx < width; dx += 1) {
          if (!hasGreedyCell(group, u + dx, v + height) || hasVisitedGreedyCell(visitedByV, u + dx, v + height)) break growHeight;
        }
        height += 1;
      }
      for (let dy = 0; dy < height; dy += 1) for (let dx = 0; dx < width; dx += 1) addVisitedGreedyCell(visitedByV, u + dx, v + dy);
      appendGreedyQuad(group.faceIndex, group.plane, u - cubeHalfSize, u + width - cubeHalfSize, v - cubeHalfSize, v + height - cubeHalfSize, positions, normals, indices);
    }
  }
}

function hasVisitedGreedyCell(visitedByV, u, v) {
  return visitedByV.get(v)?.has(u) ?? false;
}

function addVisitedGreedyCell(visitedByV, u, v) {
  let row = visitedByV.get(v);
  if (!row) {
    row = new Set();
    visitedByV.set(v, row);
  }
  row.add(u);
}

function appendGreedyQuad(faceIndex, plane, u0, u1, v0, v1, positions, normals, indices) {
  const face = voxelFaces[faceIndex];
  const corners = greedyQuadCorners(faceIndex, plane, u0, u1, v0, v1);
  const vertexOffset = positions.length / 3;
  for (const corner of corners) {
    positions.push(corner[0], corner[1], corner[2]);
    normals.push(face.normal[0], face.normal[1], face.normal[2]);
  }
  indices.push(vertexOffset, vertexOffset + 1, vertexOffset + 2, vertexOffset, vertexOffset + 2, vertexOffset + 3);
}

function greedyQuadCorners(faceIndex, plane, u0, u1, v0, v1) {
  switch (faceIndex) {
    case 0: return [[plane, v1, u0], [plane, v1, u1], [plane, v0, u1], [plane, v0, u0]];
    case 1: return [[plane, v1, u1], [plane, v1, u0], [plane, v0, u0], [plane, v0, u1]];
    case 2: return [[u0, plane, v1], [u1, plane, v1], [u1, plane, v0], [u0, plane, v0]];
    case 3: return [[u0, plane, v0], [u1, plane, v0], [u1, plane, v1], [u0, plane, v1]];
    case 4: return [[u1, v1, plane], [u0, v1, plane], [u0, v0, plane], [u1, v0, plane]];
    default: return [[u0, v1, plane], [u1, v1, plane], [u1, v0, plane], [u0, v0, plane]];
  }
}

const voxelFaces = [
  { dx: 1, dy: 0, dz: 0, normal: [1, 0, 0] },
  { dx: -1, dy: 0, dz: 0, normal: [-1, 0, 0] },
  { dx: 0, dy: 1, dz: 0, normal: [0, 1, 0] },
  { dx: 0, dy: -1, dz: 0, normal: [0, -1, 0] },
  { dx: 0, dy: 0, dz: 1, normal: [0, 0, 1] },
  { dx: 0, dy: 0, dz: -1, normal: [0, 0, -1] },
];

function isNonSolidVisualType(type) {
  return type === "water" || type === "swampWater" || type === "toxicWater";
}
