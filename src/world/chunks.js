import { chunkSize, cloudMinHeight, cloudSectorSize, minBuildY } from "./config.js";
import { currentWorldSeed, terrainProfile } from "./generator.js";
import { canonicalAboveSurfaceBlocksInArea, canonicalRenderTypeAt, canonicalSurfaceHeightAt, canonicalWaterLevelAt } from "./canonicalResource.js";
import { WorldMapBlock, renderTypeForBlock } from "./blocks.js";
import { blockKey, parseCellKey } from "./keys.js";
import { isSolidCell } from "./state.js";

const waterSurfaceOffset = 0.56;
const waterVisualHeightScale = 2 / 3;
const waterVisualCenterOffset = -1 / 6;
const cubeHalfSize = 0.5;
const cavityNeighborOffsets = [
  [1, 0, 0],
  [-1, 0, 0],
  [0, 1, 0],
  [0, -1, 0],
  [0, 0, 1],
  [0, 0, -1],
];

export function createChunkGroup({ THREE, chunkX, chunkZ, state, geometryByType, materials, detailMode = "full" }) {
  const group = new THREE.Group();
  group.name = `chunk:${chunkX},${chunkZ}`;
  group.userData.detailMode = detailMode;
  group.userData.solidKeys = new Set();
  const fullDetail = detailMode === "full";
  const decorationDetail = detailMode !== "surface";
  const treeDetail = detailMode !== "surface";
  const castChunkShadows = chunkDetailCastsShadows(detailMode);
  const receiveChunkShadows = chunkDetailReceivesShadows(detailMode);
  const terrainColumnDepth = fullDetail ? 2 : 0;
  const surfaceAt = createSurfaceReader();
  const minX = chunkX * chunkSize;
  const maxX = minX + chunkSize - 1;
  const minZ = chunkZ * chunkSize;
  const maxZ = minZ + chunkSize - 1;
  const aboveSurfaceByColumn = treeDetail ? createAboveSurfaceColumnMap(canonicalAboveSurfaceBlocksInArea({ minX, maxX, minZ, maxZ })) : null;

  const matrices = {
    grass: [],
    dirt: [],
    stone: [],
    deepStone: [],
    coal: [],
    sand: [],
    sandstone: [],
    gravel: [],
    clay: [],
    mud: [],
    dryDirt: [],
    saltFlat: [],
    snow: [],
    ice: [],
    frozenSoil: [],
    basalt: [],
    ash: [],
    bedrock: [],
    water: [],
    swampWater: [],
    toxicWater: [],
    lava: [],
    quicksand: [],
    trunk: [],
    trunkDark: [],
    pineTrunk: [],
    deadWood: [],
    giantRoot: [],
    leaves: [],
    leavesDark: [],
    leavesLight: [],
    leavesTeal: [],
    leavesWarm: [],
    pineLeaves: [],
    snowLeaves: [],
    flowerStem: [],
    flowerRed: [],
    flowerYellow: [],
    flowerBlue: [],
    flowerWhite: [],
    grassPlant: [],
    dryGrass: [],
    bush: [],
    deadBush: [],
    cactus: [],
    reed: [],
    swampGrass: [],
    snowBush: [],
    thorn: [],
    moss: [],
    lichen: [],
    vine: [],
    glowMycelium: [],
    mushroom: [],
    seaweed: [],
    aquaticPlant: [],
    coral: [],
    deadCoral: [],
    shellBed: [],
    pebble: [],
    shoreDamp: [],
    shoreFoam: [],
    reedStem: [],
    reedTip: [],
    mushroomStem: [],
    mushroomCap: [],
    pendingMine: [],
  };
  const previousPendingMineMatrixList = state.pendingMineMatrixList;
  state.pendingMineMatrixList = matrices.pendingMine;

  const transform = new THREE.Matrix4();
  const scale = new THREE.Vector3(1, 1, 1);
  const waterScale = new THREE.Vector3(1, waterVisualHeightScale, 1);
  const rotation = new THREE.Quaternion();
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

  const sourceFluidCells = new Map();
  for (let localZ = 0; localZ < chunkSize; localZ++) {
    for (let localX = 0; localX < chunkSize; localX++) {
      const x = chunkX * chunkSize + localX;
      const z = chunkZ * chunkSize + localZ;
      const height = surfaceAt(x, z);
      const columnStart = exposedColumnStart(x, z, height, terrainColumnDepth, surfaceAt);

      if (fullDetail) {
        pushBlock(state, matrices.bedrock, "bedrock", x, minBuildY, z, transform, rotation, scale, THREE, group.userData.solidKeys);
      }

      for (let y = Math.max(minBuildY, columnStart); y <= height; y++) {
        const type = canonicalRenderTypeAt({ x, y, z });
        const key = blockKey(x, y, z);
        if (type && matrices[type] && !(state.placedBlocks.has(key) && !state.removedBlocks.has(key))) {
          pushBlock(state, matrices[type], type, x, y, z, transform, rotation, scale, THREE, group.userData.solidKeys);
        }
      }

      const canonicalWaterY = canonicalWaterLevelAt({ x, z, surface: height });
      const underCanonicalWater = canonicalWaterY !== null && canonicalWaterY > height;
      if (underCanonicalWater) {
        const fluidType = "water";
        const waterY = canonicalWaterY;
        const fluidKey = `${fluidType}:${waterY}`;
        if (!sourceFluidCells.has(fluidKey)) sourceFluidCells.set(fluidKey, { fluidType, waterY, cells: [] });
        sourceFluidCells.get(fluidKey).cells.push([localX, localZ]);
        if (fullDetail) addWaterEdgeDetail(matrices, decorationBudget, x, z, waterY, transform, THREE);
      }

      const surfaceSupportVisible = isSurfaceSupportVisible(state, x, height, z);
      const canonicalAboveSurface = treeDetail && surfaceSupportVisible && !underCanonicalWater
        ? addCanonicalAboveSurfaceBlocks(state, matrices, aboveSurfaceByColumn, x, z, transform, rotation, scale, THREE, group.userData.solidKeys)
        : false;

      if (decorationDetail && surfaceSupportVisible && !underCanonicalWater) {
        const profile = terrainProfile(x, z);
        if (
          !canonicalAboveSurface &&
          profile.vegetation &&
          height > 3 &&
          shouldPlaceVegetationDecoration(vegetationBudget, profile.vegetation, x, z)
        ) {
          addVegetationDecoration(matrices, x, height, z, profile.vegetation, transform, rotation, scale, THREE);
        }

        addSurfaceDetail(matrices, decorationBudget, x, height, z, profile, canonicalAboveSurface, transform, THREE);
      }
    }
  }

  for (const { fluidType, waterY, cells } of sourceFluidCells.values()) {
    if (cells.length && matrices[fluidType]) pushSourceWaterRects(matrices[fluidType], chunkX, chunkZ, cells, waterY, transform, rotation, THREE);
  }

  for (const key of state.dynamicWater) {
    const [x, y, z] = parseCellKey(key);
    if (Math.floor(x / chunkSize) !== chunkX || Math.floor(z / chunkSize) !== chunkZ) continue;
    if (isSolidCell(state, x, y, z)) continue;
    pushWaterVisualMatrix(matrices.water, x, y, z, transform, rotation, waterScale, THREE);
  }

  for (const [key, type] of state.placedBlocks) {
    const [x, y, z] = parseCellKey(key);
    if (Math.floor(x / chunkSize) !== chunkX || Math.floor(z / chunkSize) !== chunkZ) continue;
    if (!matrices[type]) continue;
    pushBlock(state, matrices[type], type, x, y, z, transform, rotation, scale, THREE, group.userData.solidKeys, false);
  }

  addRemovedBlockCavityShell(state, matrices, chunkX, chunkZ, transform, rotation, scale, THREE, group.userData.solidKeys);

  const chunkMeshSolidKeys = new Set();
  for (const list of Object.values(matrices)) {
    for (const entry of list) {
      if ((entry.meshable || entry.occludesVoxelFaces) && entry.block && !state.removedBlocks.has(entry.block.key)) {
        chunkMeshSolidKeys.add(entry.block.key);
      }
    }
  }

  const instanceColor = new THREE.Color();
  for (const [type, list] of Object.entries(matrices)) {
    if (!list.length) continue;
    const meshableEntries = list.filter((entry) => entry.meshable);
    const instanceEntries = list.filter((entry) => entry.matrix);
    if (meshableEntries.length) {
      const geometry = createVisibleVoxelGeometry(THREE, state, meshableEntries, chunkMeshSolidKeys, surfaceAt);
      if (geometry) {
        const material = geometry.userData.hasVertexColors ? materials[type].clone() : materials[type];
        if (geometry.userData.hasVertexColors) material.vertexColors = true;
        const mesh = new THREE.Mesh(geometry, material);
        mesh.castShadow = castChunkShadows && chunkTypeUsesShadows(type);
        mesh.receiveShadow = receiveChunkShadows && chunkTypeReceivesShadows(type);
        mesh.userData.blocks = [];
        mesh.userData.greedyVoxelMesh = true;
        mesh.userData.meshType = type;
        group.add(mesh);
      }
    }
    if (!instanceEntries.length) continue;
    const mesh = new THREE.InstancedMesh(geometryByType[type], materials[type], instanceEntries.length);
    let hasInstanceColors = false;
    instanceEntries.forEach((entry, index) => {
      mesh.setMatrixAt(index, entry.matrix);
      if (!entry.tint) return;
      mesh.setColorAt(index, instanceColor.setRGB(entry.tint[0], entry.tint[1], entry.tint[2]));
      hasInstanceColors = true;
    });
    if (hasInstanceColors && mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
    const interactive = instanceEntries.some((entry) => entry.block);
    mesh.castShadow = castChunkShadows && chunkTypeUsesShadows(type);
    mesh.receiveShadow = receiveChunkShadows && chunkTypeReceivesShadows(type);
    updateInstancedMeshBounds(mesh);
    if (!interactive) mesh.raycast = () => {};
    mesh.userData.blocks = interactive ? instanceEntries.map((entry) => entry.block) : [];
    mesh.userData.meshType = type;
    group.add(mesh);
  }

  if (previousPendingMineMatrixList === undefined) delete state.pendingMineMatrixList;
  else state.pendingMineMatrixList = previousPendingMineMatrixList;
  return group;
}

function chunkDetailCastsShadows(detailMode) {
  return detailMode === "full" || detailMode === "decorated";
}

function chunkDetailReceivesShadows(detailMode) {
  return detailMode === "full" || detailMode === "decorated";
}

function chunkTypeUsesShadows(type) {
  return type !== "water" && type !== "swampWater" && type !== "toxicWater" && type !== "lava" && type !== "shoreDamp" && type !== "shoreFoam";
}

function chunkTypeReceivesShadows(type) {
  return type !== "shoreDamp" && type !== "shoreFoam" && type !== "water" && type !== "swampWater" && type !== "toxicWater";
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

function addCanonicalAboveSurfaceBlocks(state, matrices, aboveSurfaceByColumn, x, z, transform, rotation, scale, THREE, solidKeys) {
  const blocks = aboveSurfaceByColumn?.get(x)?.get(z);
  if (!blocks?.length) return false;
  let added = false;
  for (const block of blocks) {
    if (!matrices[block.type]) continue;
    const key = blockKey(block.x, block.y, block.z);
    if (state.placedBlocks.has(key) && !state.removedBlocks.has(key)) continue;
    pushBlock(state, matrices[block.type], block.type, block.x, block.y, block.z, transform, rotation, scale, THREE, solidKeys, false);
    added = true;
  }
  return added;
}

function addRemovedBlockCavityShell(state, matrices, chunkX, chunkZ, transform, rotation, scale, THREE, solidKeys) {
  if (!state.removedBlocks.size) return;
  const minX = chunkX * chunkSize;
  const maxX = minX + chunkSize - 1;
  const minZ = chunkZ * chunkSize;
  const maxZ = minZ + chunkSize - 1;

  for (const removedKey of state.removedBlocks) {
    const [rx, ry, rz] = parseCellKey(removedKey);
    if (!Number.isFinite(rx) || !Number.isFinite(ry) || !Number.isFinite(rz)) continue;

    for (const [dx, dy, dz] of cavityNeighborOffsets) {
      const x = rx + dx;
      const y = ry + dy;
      const z = rz + dz;
      if (x < minX || x > maxX || z < minZ || z > maxZ) continue;

      const key = blockKey(x, y, z);
      if (state.removedBlocks.has(key) || state.placedBlocks.has(key) || solidKeys.has(key)) continue;

      const type = canonicalRenderTypeAt({ x, y, z });
      if (!type || !matrices[type] || isNonSolidVisualType(type)) continue;
      pushBlock(state, matrices[type], type, x, y, z, transform, rotation, scale, THREE, solidKeys, false);
    }
  }
}

function createVisibleVoxelGeometry(THREE, state, entries, chunkMeshSolidKeys, surfaceAt) {
  const positions = [];
  const normals = [];
  const indices = [];
  const occlusionMemo = new Map();
  const faceGroups = new Map();

  for (const entry of entries) {
    if (!entry.block) continue;
    const { x, y, z, key } = entry.block;
    if (state.removedBlocks.has(key)) continue;

    for (let faceIndex = 0; faceIndex < voxelFaces.length; faceIndex += 1) {
      const face = voxelFaces[faceIndex];
      if (isVoxelFaceOccluded(state, chunkMeshSolidKeys, occlusionMemo, surfaceAt, x + face.dx, y + face.dy, z + face.dz)) continue;
      addGreedyFaceCell(faceGroups, faceIndex, x, y, z);
    }
  }

  for (const group of faceGroups.values()) appendGreedyFaceGroup(group, positions, normals, indices);

  if (!indices.length) return null;

  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute("position", new THREE.Float32BufferAttribute(positions, 3));
  geometry.setAttribute("normal", new THREE.Float32BufferAttribute(normals, 3));
  geometry.setIndex(indices);
  geometry.computeBoundingSphere();
  geometry.userData.hasVertexColors = false;
  return geometry;
}

function addGreedyFaceCell(faceGroups, faceIndex, x, y, z) {
  const cell = greedyFaceCell(faceIndex, x, y, z);
  const groupKey = `${faceIndex}:${cell.plane}`;
  let group = faceGroups.get(groupKey);
  if (!group) {
    group = {
      faceIndex,
      plane: cell.plane,
      cellsByV: new Map(),
      minU: cell.u,
      maxU: cell.u,
      minV: cell.v,
      maxV: cell.v,
    };
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
    case 0:
      return { plane: x + cubeHalfSize, u: z, v: y };
    case 1:
      return { plane: x - cubeHalfSize, u: z, v: y };
    case 2:
      return { plane: y + cubeHalfSize, u: x, v: z };
    case 3:
      return { plane: y - cubeHalfSize, u: x, v: z };
    case 4:
      return { plane: z + cubeHalfSize, u: x, v: y };
    default:
      return { plane: z - cubeHalfSize, u: x, v: y };
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

      for (let dy = 0; dy < height; dy += 1) {
        for (let dx = 0; dx < width; dx += 1) addVisitedGreedyCell(visitedByV, u + dx, v + dy);
      }

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
    case 0:
      return [
        [plane, v1, u0],
        [plane, v1, u1],
        [plane, v0, u1],
        [plane, v0, u0],
      ];
    case 1:
      return [
        [plane, v1, u1],
        [plane, v1, u0],
        [plane, v0, u0],
        [plane, v0, u1],
      ];
    case 2:
      return [
        [u0, plane, v1],
        [u1, plane, v1],
        [u1, plane, v0],
        [u0, plane, v0],
      ];
    case 3:
      return [
        [u0, plane, v0],
        [u1, plane, v0],
        [u1, plane, v1],
        [u0, plane, v1],
      ];
    case 4:
      return [
        [u1, v1, plane],
        [u0, v1, plane],
        [u0, v0, plane],
        [u1, v0, plane],
      ];
    default:
      return [
        [u0, v1, plane],
        [u1, v1, plane],
        [u1, v0, plane],
        [u0, v0, plane],
      ];
  }
}

const voxelFaces = [
  {
    dx: 1,
    dy: 0,
    dz: 0,
    normal: [1, 0, 0],
    corners: [
      [cubeHalfSize, cubeHalfSize, -cubeHalfSize],
      [cubeHalfSize, cubeHalfSize, cubeHalfSize],
      [cubeHalfSize, -cubeHalfSize, cubeHalfSize],
      [cubeHalfSize, -cubeHalfSize, -cubeHalfSize],
    ],
  },
  {
    dx: -1,
    dy: 0,
    dz: 0,
    normal: [-1, 0, 0],
    corners: [
      [-cubeHalfSize, cubeHalfSize, cubeHalfSize],
      [-cubeHalfSize, cubeHalfSize, -cubeHalfSize],
      [-cubeHalfSize, -cubeHalfSize, -cubeHalfSize],
      [-cubeHalfSize, -cubeHalfSize, cubeHalfSize],
    ],
  },
  {
    dx: 0,
    dy: 1,
    dz: 0,
    normal: [0, 1, 0],
    corners: [
      [-cubeHalfSize, cubeHalfSize, cubeHalfSize],
      [cubeHalfSize, cubeHalfSize, cubeHalfSize],
      [cubeHalfSize, cubeHalfSize, -cubeHalfSize],
      [-cubeHalfSize, cubeHalfSize, -cubeHalfSize],
    ],
  },
  {
    dx: 0,
    dy: -1,
    dz: 0,
    normal: [0, -1, 0],
    corners: [
      [-cubeHalfSize, -cubeHalfSize, -cubeHalfSize],
      [cubeHalfSize, -cubeHalfSize, -cubeHalfSize],
      [cubeHalfSize, -cubeHalfSize, cubeHalfSize],
      [-cubeHalfSize, -cubeHalfSize, cubeHalfSize],
    ],
  },
  {
    dx: 0,
    dy: 0,
    dz: 1,
    normal: [0, 0, 1],
    corners: [
      [cubeHalfSize, cubeHalfSize, cubeHalfSize],
      [-cubeHalfSize, cubeHalfSize, cubeHalfSize],
      [-cubeHalfSize, -cubeHalfSize, cubeHalfSize],
      [cubeHalfSize, -cubeHalfSize, cubeHalfSize],
    ],
  },
  {
    dx: 0,
    dy: 0,
    dz: -1,
    normal: [0, 0, -1],
    corners: [
      [-cubeHalfSize, cubeHalfSize, -cubeHalfSize],
      [cubeHalfSize, cubeHalfSize, -cubeHalfSize],
      [cubeHalfSize, -cubeHalfSize, -cubeHalfSize],
      [-cubeHalfSize, -cubeHalfSize, -cubeHalfSize],
    ],
  },
];

function isVoxelFaceOccluded(state, chunkMeshSolidKeys, occlusionMemo, surfaceAt, x, y, z) {
  const key = blockKey(x, y, z);
  if (state.removedBlocks.has(key)) return false;
  if (chunkMeshSolidKeys.has(key)) return true;
  const placedType = state.placedBlocks.get(key);
  if (placedType && !isNonSolidVisualType(placedType)) return true;
  if (occlusionMemo.has(key)) return occlusionMemo.get(key);
  const occluded = y <= surfaceAt(x, z);
  occlusionMemo.set(key, occluded);
  return occluded;
}

export function createCloudSectorGroup({ THREE, sectorX, sectorZ, geometry, material }) {
  const lobes = [];
  addCloudCluster(lobes, sectorX, sectorZ);

  if (!lobes.length) return null;

  const group = new THREE.Group();
  group.name = `cloud:${sectorX},${sectorZ}`;
  const mesh = new THREE.InstancedMesh(geometry, material, lobes.length);
  const transform = new THREE.Matrix4();
  const rotation = new THREE.Quaternion();
  let index = 0;

  for (const lobe of lobes) {
    transform.compose(new THREE.Vector3(lobe.x, lobe.y, lobe.z), rotation, new THREE.Vector3(lobe.rx, lobe.ry, lobe.rz));
    mesh.setMatrixAt(index, transform);
    index++;
  }

  mesh.castShadow = false;
  mesh.receiveShadow = false;
  updateInstancedMeshBounds(mesh);
  mesh.frustumCulled = true;
  mesh.userData.blocks = Array.from({ length: lobes.length }, () => null);
  group.add(mesh);
  return group;
}

function updateInstancedMeshBounds(mesh) {
  mesh.instanceMatrix.needsUpdate = true;
  mesh.computeBoundingSphere?.();
  mesh.computeBoundingBox?.();
}

function exposedColumnStart(x, z, height, maxDepth = 5, surfaceAt = (surfaceX, surfaceZ) => canonicalSurfaceHeightAt({ x: surfaceX, z: surfaceZ })) {
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

function isSurfaceSupportVisible(state, x, y, z) {
  const key = blockKey(x, y, z);
  if (state.removedBlocks.has(key)) return false;
  return true;
}

function subsurfaceType(surfaceType, y, height) {
  if (surfaceType === "sand" && y >= height - 2) return "sand";
  if (surfaceType === "sandstone" && y >= height - 3) return "sandstone";
  if (surfaceType === "snow" && y === height - 1) return "snow";
  if (surfaceType === "snow" && y >= height - 4) return "sandstone";
  if (surfaceType === "grass" && y >= height - 2) return "dirt";
  return "stone";
}

export function generatedBlockTypeAt(x, y, z) {
  return canonicalRenderTypeAt({ x, y, z }) ?? null;
}

function addVegetationDecoration(matrices, x, height, z, vegetation, transform, rotation, scale, THREE) {
  const type = renderTypeForBlock(vegetation);
  if (!type || !matrices[type]) return;

  if (vegetation === WorldMapBlock.Cactus) {
    const cactusHeight = 1.4 + random2(x + 601, z - 607) * 1.2;
    pushDecorationBox(matrices.cactus, x, height + 0.5 + cactusHeight * 0.5, z, transform, THREE, 0.42, cactusHeight, 0.42);
    return;
  }

  if (vegetation === WorldMapBlock.Bush || vegetation === WorldMapBlock.DeadBush || vegetation === WorldMapBlock.SnowBush) {
    addBushCluster(matrices, x, height, z, type, vegetation, transform, THREE);
    return;
  }

  if (vegetation === WorldMapBlock.GiantRoot || vegetation === WorldMapBlock.DeadWood) {
    const yaw = random2(x + 617, z - 619) * Math.PI;
    pushDecorationOrientedBox(matrices[type], x, height + 0.64, z, transform, THREE, 0.32, 0.26, 1.45, yaw);
    return;
  }

  if (vegetation === WorldMapBlock.Reed) {
    addReedCluster(matrices, x, z, height + 0.5, transform, THREE);
    return;
  }

  if (vegetation === WorldMapBlock.Mushroom) {
    addMushroomCluster(matrices, x, height, z, transform, THREE);
    return;
  }

  if (vegetation === WorldMapBlock.Coral || vegetation === WorldMapBlock.DeadCoral) {
    const count = 2 + Math.floor(random2(x - 631, z + 641) * 3);
    for (let i = 0; i < count; i++) {
      const px = x + randomOffset(x, z, 61 + i * 2) * 0.72;
      const pz = z + randomOffset(x, z, 62 + i * 2) * 0.72;
      const sy = 0.34 + random2(x + i * 643, z - i * 647) * 0.34;
      pushDecorationBox(matrices[type], px, height + 0.52 + sy * 0.5, pz, transform, THREE, 0.18, sy, 0.18);
    }
    return;
  }

  if (vegetation === WorldMapBlock.Vine) {
    const yaw = random2(x + 653, z - 659) * Math.PI;
    pushDecorationFlatPlane(matrices.vine, x + randomOffset(x, z, 63), height + 0.54, z + randomOffset(x, z, 64), transform, THREE, 0.42, 0.9, yaw);
    return;
  }

  if (vegetation === WorldMapBlock.Seaweed || vegetation === WorldMapBlock.AquaticPlant || vegetation === WorldMapBlock.SwampGrass) {
    addTuft(matrices[type], x, height, z, transform, THREE, 0.52, 0.42);
    return;
  }

  if (vegetation === WorldMapBlock.ShellBed || vegetation === WorldMapBlock.Moss || vegetation === WorldMapBlock.Lichen || vegetation === WorldMapBlock.GlowMycelium) {
    addGroundPatch(matrices[type], x, height, z, transform, THREE, 0.58, 0.32, 67);
    return;
  }

  if (vegetation === WorldMapBlock.Thorn) {
    const yaw = random2(x + 661, z - 673) * Math.PI;
    pushDecorationOrientedBox(matrices.thorn, x, height + 0.78, z, transform, THREE, 0.1, 0.72, 0.48, yaw);
    return;
  }

  addTuft(matrices[type], x, height, z, transform, THREE, 0.46, 0.34);
}

function shouldPlaceVegetationDecoration(budget, vegetation, x, z) {
  const category = vegetationDecorationCategory(vegetation);
  if ((budget[category] ?? 0) <= 0) return false;
  if (random2(x + 2309, z - 2311) >= vegetationDecorationChance(category)) return false;
  budget[category]--;
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

function addSurfacePlants(matrices, x, height, z, transform, THREE) {
  const moisture = random2(Math.floor(x / 6) + 31, Math.floor(z / 6) - 43);
  const flowerRoll = random2(x - 211, z + 587);

  if (flowerRoll > 0.78 && moisture > 0.14) {
    const flowerTypes = ["flowerRed", "flowerYellow", "flowerBlue", "flowerWhite"];
    const type = flowerTypes[Math.floor(random2(x + 17, z + 23) * flowerTypes.length)];
    const px = x + randomOffset(x, z, 1);
    const pz = z + randomOffset(x, z, 2);
    pushDecorationBox(matrices.flowerStem, px, height + 0.82, pz, transform, THREE, 0.14, 0.72, 0.14);
    pushDecorationBox(matrices[type], px, height + 1.25, pz, transform, THREE, 0.38, 0.38, 0.38);
  }
}

function addSurfaceDetail(matrices, budget, x, height, z, profile, treeCell, transform, THREE) {
  if ((profile.terrain === WorldMapBlock.Grass || profile.terrain === WorldMapBlock.Mud) && profile.slope <= 2 && height > 3 && !treeCell) {
    const tuftRoll = random2(x + 149, z - 173);
    const forestBoost = profile.forest > 0.55 ? 0.08 : 0;
    if (tuftRoll > 0.985 - forestBoost * 0.25 && budget.grassTuft > 0) {
      addTuft(matrices.grassPlant, x, height, z, transform, THREE, 0.46, 0.34);
      budget.grassTuft--;
    } else if (tuftRoll < 0.025 && budget.dryGrassTuft > 0) {
      addTuft(matrices.dryGrass, x, height, z, transform, THREE, 0.34, 0.26);
      budget.dryGrassTuft--;
    }

    if (profile.forest > 0.62 && budget.dryGrassTuft > 0 && random2(x - 1487, z + 1511) > 0.985) {
      addGroundPatch(matrices.dryGrass, x, height, z, transform, THREE, 0.5, 0.22, 17);
      budget.dryGrassTuft--;
    }

    if (budget.mushroom > 0 && profile.forest > 0.5 && random2(x + 2221, z - 2237) > 0.99 && isMushroomClusterCell(x, z, profile)) {
      addMushroomCluster(matrices, x, height, z, transform, THREE);
      budget.mushroom--;
    }

    if (budget.fallenLog > 0 && profile.forest > 0.74 && profile.slope <= 1 && random2(x + 1931, z - 1949) > 0.47) {
      addFallenLog(matrices, x, height, z, transform, THREE);
      budget.fallenLog--;
    }
  }

  const stonySurface = [WorldMapBlock.Stone, WorldMapBlock.DeepStone, WorldMapBlock.Gravel, WorldMapBlock.Basalt, WorldMapBlock.Ash].includes(profile.terrain);
  const sandySurface = [WorldMapBlock.Sand, WorldMapBlock.DryDirt, WorldMapBlock.SaltFlat, WorldMapBlock.Quicksand].includes(profile.terrain);
  if ((sandySurface || stonySurface) && budget.pebble > 0) {
    const shardChance = stonySurface ? 0.43 - Math.min(profile.slope, 5) * 0.018 : 1;
    const pebbleChance = stonySurface ? 0.45 : 0.495;
    if (!sandySurface && random2(x + 2099, z - 2111) > shardChance) {
      addRockShard(matrices.pebble, x, height, z, profile, transform, THREE);
      budget.pebble--;
    } else if (random2(x - 509, z + 419) > pebbleChance) {
      addPebble(matrices.pebble, x, height, z, transform, THREE);
      budget.pebble--;
    }
  }

  if (stonySurface && profile.slope <= 1 && budget.grassTuft > 0) {
    if (random2(x + 1559, z - 1571) > 0.49) {
      addGroundPatch(matrices.lichen, x, height, z, transform, THREE, 0.28, 0.2, 19);
      budget.grassTuft--;
    }
  }

  if ((profile.terrain === WorldMapBlock.Snow || profile.terrain === WorldMapBlock.FrozenSoil) && profile.slope <= 3 && budget.shoreFoam > 0 && random2(x - 1607, z + 1613) > 0.9) {
    addGroundPatch(matrices.shoreFoam, x, height, z, transform, THREE, 0.62, 0.16, 23);
    budget.shoreFoam--;
  }

  if (profile.terrain === WorldMapBlock.Sand && budget.shoreDamp > 0 && isNearWater(x, z)) {
    addShoreDamp(matrices.shoreDamp, x, height, z, transform, THREE);
    budget.shoreDamp--;
  }

  if ((profile.terrain === WorldMapBlock.Sand || profile.terrain === WorldMapBlock.Mud || profile.terrain === WorldMapBlock.Clay) && budget.reed > 0 && isNearWater(x, z)) {
    if (random2(x + 1811, z - 1823) > 0.42) {
      addReedCluster(matrices, x, z, height + 0.5, transform, THREE);
      budget.reed--;
    }
  }
}

function addWaterEdgeDetail(matrices, budget, x, z, waterY, transform, THREE) {
  if (!isNearLand(x, z)) return;

  if (budget.shoreFoam > 0 && random2(x + 1291, z - 1301) >= 0.22) {
    const sx = 0.44 + random2(x - 1319, z + 1321) * 0.34;
    const sz = 0.16 + random2(x + 1327, z - 1361) * 0.16;
    const px = x + randomOffset(x, z, 13) * 0.5;
    const pz = z + randomOffset(x, z, 14) * 0.5;
    pushDecorationFlatPlane(matrices.shoreFoam, px, waterY + waterSurfaceOffset + 0.012, pz, transform, THREE, sx, sz, random2(x + 1367, z - 1373) * Math.PI);
    budget.shoreFoam--;
  }

  if (budget.reed > 0 && random2(x - 1709, z + 1721) > 0.42) {
    addReedCluster(matrices, x, z, waterY + waterSurfaceOffset + 0.04, transform, THREE);
    budget.reed--;
  }
}

function addReedCluster(matrices, x, z, baseY, transform, THREE) {
  const count = 1 + Math.floor(random2(x + 1733, z - 1741) * 3);
  for (let i = 0; i < count; i++) {
    const px = x + randomOffset(x, z, 25 + i * 2) * 0.72;
    const pz = z + randomOffset(x, z, 26 + i * 2) * 0.72;
    const height = 0.72 + random2(x + i * 1753, z - i * 1759) * 0.5;
    pushDecorationBox(matrices.reedStem, px, baseY + height * 0.5, pz, transform, THREE, 0.08, height, 0.08);
    if (random2(x - i * 1777, z + i * 1783) > 0.38) {
      pushDecorationBox(matrices.reedTip, px, baseY + 0.06 + height, pz, transform, THREE, 0.12, 0.18, 0.12);
    }
  }
}

function addBushCluster(matrices, x, height, z, leafType, vegetation, transform, THREE) {
  const centerX = x + randomOffset(x, z, 51) * 0.8;
  const centerZ = z + randomOffset(x, z, 52) * 0.8;
  const branchType = vegetation === WorldMapBlock.SnowBush ? "pineTrunk" : vegetation === WorldMapBlock.DeadBush ? "deadWood" : "trunkDark";
  const branchCount = 2 + Math.floor(random2(x + 1649, z - 1657) * 2);

  for (let i = 0; i < branchCount; i++) {
    const yaw = random2(x + i * 1663, z - i * 1667) * Math.PI * 2;
    const pitch = -0.28 + random2(x - i * 1693, z + i * 1697) * 0.56;
    const length = 0.42 + random2(x + i * 1709, z - i * 1721) * 0.34;
    const spread = 0.08 + random2(x - i * 1723, z + i * 1733) * 0.18;
    pushDecorationAngledBox(
      matrices[branchType],
      centerX + Math.sin(yaw) * spread,
      height + 0.58 + random2(x + i * 1741, z - i * 1747) * 0.24,
      centerZ + Math.cos(yaw) * spread,
      transform,
      THREE,
      0.075,
      0.085,
      length,
      yaw,
      pitch,
    );
  }

  const leafCount = vegetation === WorldMapBlock.DeadBush ? 1 : 2 + Math.floor(random2(x - 1753, z + 1759) * 2);
  for (let i = 0; i < leafCount; i++) {
    const yaw = (i / leafCount) * Math.PI * 2 + random2(x + i * 1787, z - i * 1789) * 0.42;
    const spread = random2(x - i * 1801, z + i * 1811) * 0.18;
    const width = (vegetation === WorldMapBlock.SnowBush ? 0.46 : 0.52) + random2(x + i * 1817, z - i * 1823) * 0.18;
    const leafHeight = (vegetation === WorldMapBlock.DeadBush ? 0.34 : 0.46) + random2(x - i * 1831, z + i * 1847) * 0.18;
    pushDecorationAngledBox(
      matrices[leafType],
      centerX + Math.sin(yaw) * spread,
      height + 0.78 + random2(x + i * 1861, z - i * 1867) * 0.18,
      centerZ + Math.cos(yaw) * spread,
      transform,
      THREE,
      width,
      leafHeight,
      0.08,
      yaw,
      -0.08 + random2(x - i * 1871, z + i * 1877) * 0.16,
    );
  }
}

function addMushroomCluster(matrices, x, height, z, transform, THREE) {
  const count = 1 + Math.floor(random2(x + 1871, z - 1877) * 3);
  for (let i = 0; i < count; i++) {
    const px = x + randomOffset(x, z, 31 + i * 2) * 0.76;
    const pz = z + randomOffset(x, z, 32 + i * 2) * 0.76;
    const stemHeight = 0.16 + random2(x + i * 1889, z - i * 1901) * 0.12;
    const capWidth = 0.22 + random2(x - i * 1907, z + i * 1913) * 0.12;
    pushDecorationBox(matrices.mushroomStem, px, height + 0.52 + stemHeight * 0.5, pz, transform, THREE, 0.08, stemHeight, 0.08);
    pushDecorationBox(matrices.mushroomCap, px, height + 0.56 + stemHeight, pz, transform, THREE, capWidth, 0.1, capWidth);
  }
}

function addFallenLog(matrices, x, height, z, transform, THREE) {
  const yaw = Math.round(random2(x - 1951, z + 1973) * 7) * (Math.PI / 4);
  const length = 1.35 + random2(x + 1979, z - 1987) * 0.85;
  const px = x + randomOffset(x, z, 37) * 0.42;
  const pz = z + randomOffset(x, z, 38) * 0.42;
  const trunkType = random2(x - 1993, z + 1997) > 0.78 ? "trunkDark" : "trunk";
  pushDecorationOrientedBox(matrices[trunkType], px, height + 0.62, pz, transform, THREE, 0.28, 0.28, length, yaw);
  if (random2(x + 2003, z - 2011) > 0.36) {
    const capOffset = length * 0.52;
    pushDecorationBox(
      matrices.mushroomCap,
      px + Math.sin(yaw) * capOffset,
      height + 0.72,
      pz + Math.cos(yaw) * capOffset,
      transform,
      THREE,
      0.2,
      0.08,
      0.2,
    );
  }
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

function addTuft(list, x, height, z, transform, THREE, width, depth) {
  const px = x + randomOffset(x, z, 7) * 0.72;
  const pz = z + randomOffset(x, z, 8) * 0.72;
  const yaw = random2(x + 701, z - 733) * Math.PI;
  pushDecorationFlatPlane(list, px, height + 0.516, pz, transform, THREE, width, depth, yaw);
}

function addGroundPatch(list, x, height, z, transform, THREE, width, depth, salt) {
  const px = x + randomOffset(x, z, salt) * 0.78;
  const pz = z + randomOffset(x, z, salt + 1) * 0.78;
  const yaw = random2(x + salt * 79, z - salt * 83) * Math.PI;
  pushDecorationFlatPlane(list, px, height + 0.516, pz, transform, THREE, width, depth, yaw);
}

function addPebble(list, x, height, z, transform, THREE) {
  const px = x + randomOffset(x, z, 9) * 0.88;
  const pz = z + randomOffset(x, z, 10) * 0.88;
  const sx = 0.12 + random2(x + 941, z - 967) * 0.13;
  const sy = 0.06 + random2(x - 983, z + 997) * 0.06;
  const sz = 0.1 + random2(x + 1013, z - 1031) * 0.12;
  pushDecorationBox(list, px, height + 0.5 + sy * 0.5, pz, transform, THREE, sx, sy, sz);
}

function addRockShard(list, x, height, z, profile, transform, THREE) {
  const px = x + randomOffset(x, z, 41) * 0.82;
  const pz = z + randomOffset(x, z, 42) * 0.82;
  const yaw = random2(x - 2131, z + 2137) * Math.PI;
  const longSide = 0.22 + random2(x + 2141, z - 2143) * 0.22;
  const shortSide = 0.08 + random2(x - 2153, z + 2161) * 0.08;
  const heightScale = 0.08 + Math.min(profile.slope, 5) * 0.012 + random2(x + 2179, z - 2203) * 0.04;
  pushDecorationOrientedBox(list, px, height + 0.5 + heightScale * 0.5, pz, transform, THREE, shortSide, heightScale, longSide, yaw);
}

function addShoreDamp(list, x, height, z, transform, THREE) {
  const roll = random2(x + 1109, z - 1151);
  const sx = 0.58 + roll * 0.28;
  const sz = 0.42 + random2(x - 1171, z + 1187) * 0.28;
  const px = x + randomOffset(x, z, 11) * 0.42;
  const pz = z + randomOffset(x, z, 12) * 0.42;
  pushDecorationFlatPlane(list, px, height + 0.516, pz, transform, THREE, sx, sz, random2(x + 1201, z - 1213) * Math.PI);
}

function isNearWater(x, z) {
  return (
    hasSurfaceWater(x + 1, z) ||
    hasSurfaceWater(x - 1, z) ||
    hasSurfaceWater(x, z + 1) ||
    hasSurfaceWater(x, z - 1)
  );
}

function isNearLand(x, z) {
  return (
    !hasSurfaceWater(x + 1, z) ||
    !hasSurfaceWater(x - 1, z) ||
    !hasSurfaceWater(x, z + 1) ||
    !hasSurfaceWater(x, z - 1)
  );
}

function hasSurfaceWater(x, z) {
  const surface = canonicalSurfaceHeightAt({ x, z });
  const waterLevel = canonicalWaterLevelAt({ x, z, surface });
  return waterLevel !== null && surface < waterLevel;
}

function randomOffset(x, z, salt) {
  return (random2(x + salt * 101, z - salt * 131) - 0.5) * 0.52;
}

function addCloudCluster(lobes, seedX, seedZ) {
  const seed = random2(seedX, seedZ);
  if (seed < 0.48) return;

  const centerX = seedX * cloudSectorSize + 12 + random2(seedX + 19, seedZ - 11) * (cloudSectorSize - 24);
  const centerZ = seedZ * cloudSectorSize + 12 + random2(seedX - 23, seedZ + 31) * (cloudSectorSize - 24);
  const baseY = cloudMinHeight + 4 + Math.floor(seed * 7);
  const mainRx = 11 + random2(seedX + 43, seedZ + 5) * 10;
  const mainRz = 8 + random2(seedX - 7, seedZ + 47) * 8;

  lobes.push({
    x: centerX,
    y: baseY,
    z: centerZ,
    rx: mainRx,
    ry: 4.2 + random2(seedX + 3, seedZ - 5) * 2.4,
    rz: mainRz,
  });

  const sideLobes = 5 + Math.floor(random2(seedX - 71, seedZ + 79) * 3);
  for (let i = 0; i < sideLobes; i++) {
    const angle = random2(seedX + i * 13, seedZ - i * 17) * Math.PI * 2;
    const offset = 7 + random2(seedX - i * 29, seedZ + i * 31) * 15;
    const size = 0.48 + random2(seedX + i * 37, seedZ + i * 41) * 0.38;
    lobes.push({
      x: centerX + Math.cos(angle) * offset,
      y: baseY - 1.2 + random2(seedX + i * 53, seedZ - i * 59) * 4,
      z: centerZ + Math.sin(angle) * offset,
      rx: mainRx * size,
      ry: 3.2 + random2(seedX - i * 43, seedZ - i * 47) * 2.5,
      rz: mainRz * (0.52 + random2(seedX + i * 61, seedZ + i * 67) * 0.32),
    });
  }
}

function pushBlock(state, list, type, x, y, z, transform, rotation, scale, THREE, solidKeys, canonicalize = true) {
  if (canonicalize) {
    const canonicalType = canonicalRenderTypeAt({ x, y, z });
    if (!canonicalType) return;
    type = canonicalType;
  }
  const key = blockKey(x, y, z);
  if (state.removedBlocks.has(key)) return;
  if (!isNonSolidVisualType(type)) trackGeneratedSolidKey(state, solidKeys, key);
  const visualY = isShortWaterVisualType(type) ? y + waterVisualCenterOffset : y;
  const visualScale = isShortWaterVisualType(type) ? new THREE.Vector3(1, waterVisualHeightScale, 1) : scale;
  const block = { x, y, z, type, key };
  if (state.pendingMinedBlocks?.has(key)) {
    const pendingList = state.pendingMineMatrixList ?? null;
    const targetList = pendingList || list;
    targetList.push({
      matrix: composeMatrix(x, visualY, z, transform, rotation, visualScale, THREE),
      block,
      tint: blockTint(type, x, y, z),
      occludesVoxelFaces: true,
    });
    return;
  }
  if (isMeshableVoxelType(type)) {
    list.push({
      x,
      y,
      z,
      type,
      block,
      tint: blockTint(type, x, y, z),
      meshable: true,
    });
    return;
  }
  list.push({
    matrix: composeMatrix(x, visualY, z, transform, rotation, visualScale, THREE),
    block,
    tint: blockTint(type, x, y, z),
  });
}

function isNonSolidVisualType(type) {
  return type === "water" || type === "swampWater" || type === "toxicWater";
}

function isShortWaterVisualType(type) {
  return type === "water" || type === "swampWater" || type === "toxicWater";
}

function isMeshableVoxelType(type) {
  return !isNonSolidVisualType(type) && type !== "lava";
}

function trackGeneratedSolidKey(state, solidKeys, key) {
  state.solidBlocks.add(key);
  if (!solidKeys) return;
  if (solidKeys.has(key)) return;
  solidKeys.add(key);
  if (!state.generatedSolidRefs) return;
  state.generatedSolidRefs.set(key, (state.generatedSolidRefs.get(key) ?? 0) + 1);
}

function pushMatrix(list, x, y, z, transform, rotation, scale, THREE) {
  const matrixScale = scale ?? new THREE.Vector3(1, 1, 1);
  list.push({
    matrix: composeMatrix(x, y, z, transform, rotation, matrixScale, THREE),
    block: null,
  });
}

function pushWaterVisualMatrix(list, x, y, z, transform, rotation, scale, THREE) {
  list.push({
    matrix: composeMatrix(x, y + waterVisualCenterOffset, z, transform, rotation, scale, THREE),
    block: null,
    tint: blockTint("water", x, y, z),
  });
}

function pushDecorationBox(list, x, y, z, transform, THREE, sx, sy, sz) {
  const rotation = new THREE.Quaternion();
  const scale = new THREE.Vector3(sx, sy, sz);
  list.push({
    matrix: composeMatrix(x, y, z, transform, rotation, scale, THREE),
    block: null,
  });
}

function pushDecorationOrientedBox(list, x, y, z, transform, THREE, sx, sy, sz, yaw = 0) {
  const rotation = new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(0, 1, 0), yaw);
  const scale = new THREE.Vector3(sx, sy, sz);
  list.push({
    matrix: composeMatrix(x, y, z, transform, rotation, scale, THREE),
    block: null,
  });
}

function pushDecorationAngledBox(list, x, y, z, transform, THREE, sx, sy, sz, yaw = 0, pitch = 0) {
  const rotation = new THREE.Quaternion().setFromEuler(new THREE.Euler(pitch, yaw, 0, "YXZ"));
  const scale = new THREE.Vector3(sx, sy, sz);
  list.push({
    matrix: composeMatrix(x, y, z, transform, rotation, scale, THREE),
    block: null,
  });
}

function pushDecorationFlatPlane(list, x, y, z, transform, THREE, sx, sz, yaw = 0) {
  const rotation = new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(0, 1, 0), yaw);
  const scale = new THREE.Vector3(sx, 1, sz);
  list.push({
    matrix: composeMatrix(x, y, z, transform, rotation, scale, THREE),
    block: null,
  });
}

function pushSourceWaterRects(list, chunkX, chunkZ, sourceWaterCells, waterY, transform, rotation, THREE) {
  const water = Array.from({ length: chunkSize }, () => Array(chunkSize).fill(false));
  for (const [localX, localZ] of sourceWaterCells) water[localZ][localX] = true;

  const used = Array.from({ length: chunkSize }, () => Array(chunkSize).fill(false));
  for (let localZ = 0; localZ < chunkSize; localZ++) {
    for (let localX = 0; localX < chunkSize; localX++) {
      if (!water[localZ][localX] || used[localZ][localX]) continue;

      let width = 1;
      while (localX + width < chunkSize && water[localZ][localX + width] && !used[localZ][localX + width]) {
        width++;
      }

      let depth = 1;
      let canGrow = true;
      while (localZ + depth < chunkSize && canGrow) {
        for (let dx = 0; dx < width; dx++) {
          if (!water[localZ + depth][localX + dx] || used[localZ + depth][localX + dx]) {
            canGrow = false;
            break;
          }
        }
        if (canGrow) depth++;
      }

      for (let dz = 0; dz < depth; dz++) {
        for (let dx = 0; dx < width; dx++) {
          used[localZ + dz][localX + dx] = true;
        }
      }

      pushWaterRect(
        list,
        chunkX,
        chunkZ,
        localX,
        localZ,
        localX + width - 1,
        localZ + depth - 1,
        waterY,
        transform,
        rotation,
        THREE,
      );
    }
  }
}

function pushWaterRect(list, chunkX, chunkZ, startX, startZ, endX, endZ, waterY, transform, rotation, THREE) {
  const width = endX - startX + 1;
  const depth = endZ - startZ + 1;
  const centerX = chunkX * chunkSize + startX + (width - 1) * 0.5;
  const centerZ = chunkZ * chunkSize + startZ + (depth - 1) * 0.5;
  const scale = new THREE.Vector3(width, 1, depth);
  list.push({
    matrix: composeMatrix(centerX, waterY + waterSurfaceOffset, centerZ, transform, rotation, scale, THREE),
    block: null,
  });
}

function composeMatrix(x, y, z, transform, rotation, scale, THREE) {
  transform.compose(new THREE.Vector3(x, y, z), rotation, scale);
  return transform.clone();
}

function random2(x, z) {
  let n = Math.imul(x | 0, 374761393) ^ Math.imul(z | 0, 668265263) ^ Math.imul(currentWorldSeed() | 0, 2246822519);
  n = Math.imul(n ^ (n >>> 13), 1274126177);
  return ((n ^ (n >>> 16)) >>> 0) / 4294967295;
}

export function blockTint(type, x, y, z) {
  const tone = random2(x + y * 17, z - y * 31);
  const fleck = random2(x - y * 7 + 311, z + y * 13 - 197);
  let shade = 0.965 + tone * 0.07;
  let warm = 1;
  let cool = 1;

  if (type === "water" || type === "swampWater" || type === "toxicWater") {
    const profile = terrainProfile(x, z);
    const waterSurface = profile.noise.waterSurfaceHeight;
    const waterFloor = profile.noise.waterFloorHeight;
    const depth = waterSurface !== null && waterFloor !== null ? Math.max(1, waterSurface - waterFloor) : 1;
    const depthFactor = Math.min(1, (depth - 1) / 3);
    shade = 1.05 - depthFactor * 0.34 + tone * 0.035;
    warm = 0.9 - depthFactor * 0.28;
    cool = 1.08 + depthFactor * 0.18 + fleck * 0.025;
  } else if (type === "sand" || type === "sandstone" || type === "quicksand") {
    warm = 0.985 + fleck * 0.045;
    cool = 0.965 + tone * 0.035;
    if (type === "quicksand") {
      shade = 0.9 + tone * 0.065;
      warm = 0.9 + fleck * 0.045;
      cool = 0.82 + tone * 0.04;
    }
  } else if (type === "saltFlat") {
    shade = 0.98 + tone * 0.05;
    warm = 1.02 + fleck * 0.025;
    cool = 0.96 + tone * 0.025;
  } else if (type === "grass" || type.includes("leaves")) {
    warm = 0.94 + tone * 0.055;
    cool = 0.96 + fleck * 0.08;
    shade = 0.95 + tone * 0.09;
  } else if (type === "snow" || type === "ice") {
    warm = 0.96 + tone * 0.035;
    cool = type === "ice" ? 1.08 + fleck * 0.08 : 1.015 + fleck * 0.055;
    shade = 0.97 + tone * 0.055;
  } else if (type === "frozenSoil") {
    shade = 0.92 + tone * 0.08;
    warm = 0.86 + fleck * 0.04;
    cool = 1.02 + tone * 0.06;
  } else if (type === "stone" || type === "deepStone" || type === "coal" || type === "gravel" || type === "basalt" || type === "ash" || type === "bedrock") {
    shade = 0.93 + tone * 0.11;
    warm = 0.97 + fleck * 0.035;
    cool = 0.97 + tone * 0.06;
    if (type === "coal") {
      shade = 0.78 + tone * 0.08;
      warm = 0.82 + fleck * 0.03;
      cool = 0.88 + tone * 0.05;
    } else if (type === "deepStone") {
      warm = 0.86 + fleck * 0.03;
      cool = 1.06 + tone * 0.08;
    } else if (type === "basalt" || type === "bedrock") {
      shade = 0.86 + tone * 0.1;
      warm = 0.9 + fleck * 0.03;
      cool = 1.02 + tone * 0.04;
    } else if (type === "ash") {
      shade = 0.98 + tone * 0.08;
      warm = 1.0 + fleck * 0.035;
      cool = 0.94 + tone * 0.04;
    } else if (type === "gravel") {
      shade = 0.9 + tone * 0.14;
    }
  } else if (type === "clay") {
    shade = 0.94 + tone * 0.08;
    warm = 1.05 + fleck * 0.04;
    cool = 0.88 + tone * 0.04;
  } else if (type === "mud") {
    shade = 0.82 + tone * 0.09;
    warm = 0.9 + fleck * 0.035;
    cool = 0.78 + tone * 0.035;
  } else if (type === "dryDirt") {
    shade = 0.94 + tone * 0.08;
    warm = 1.08 + fleck * 0.05;
    cool = 0.82 + tone * 0.03;
  } else if (type.startsWith("trunk")) {
    shade = 0.91 + tone * 0.12;
    warm = 1.01 + fleck * 0.055;
    cool = 0.91 + tone * 0.035;
  }

  return [shade * warm, shade, shade * cool];
}
