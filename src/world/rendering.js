import { currentWorldSeed } from "./generator.js";
import { blockRenderProfileForType } from "./blockRendering.js";
import {
  applyVoxelMaterialDetail,
  materialStyles,
} from "../render/proceduralMaterials.js";

export function createWorldGeometryByType({ THREE, cubeGeometry, waterGeometry, cloudGeometry = null }) {
  return {
    grass: cubeGeometry,
    dirt: cubeGeometry,
    stone: cubeGeometry,
    deepStone: cubeGeometry,
    sand: cubeGeometry,
    sandstone: cubeGeometry,
    gravel: cubeGeometry,
    clay: cubeGeometry,
    mud: cubeGeometry,
    dryDirt: cubeGeometry,
    saltFlat: cubeGeometry,
    snow: cubeGeometry,
    ice: cubeGeometry,
    frozenSoil: cubeGeometry,
    basalt: cubeGeometry,
    ash: cubeGeometry,
    bedrock: cubeGeometry,
    water: cubeGeometry,
    swampWater: cubeGeometry,
    toxicWater: cubeGeometry,
    lava: waterGeometry,
    quicksand: cubeGeometry,
    trunk: cubeGeometry,
    trunkDark: cubeGeometry,
    pineTrunk: cubeGeometry,
    deadWood: cubeGeometry,
    giantRoot: cubeGeometry,
    leaves: cubeGeometry,
    leavesDark: cubeGeometry,
    leavesLight: cubeGeometry,
    leavesTeal: cubeGeometry,
    leavesWarm: cubeGeometry,
    pineLeaves: cubeGeometry,
    snowLeaves: cubeGeometry,
    grassPlant: waterGeometry,
    dryGrass: waterGeometry,
    bush: cubeGeometry,
    deadBush: cubeGeometry,
    cactus: cubeGeometry,
    reed: cubeGeometry,
    swampGrass: waterGeometry,
    snowBush: cubeGeometry,
    thorn: cubeGeometry,
    moss: waterGeometry,
    lichen: waterGeometry,
    vine: waterGeometry,
    glowMycelium: waterGeometry,
    mushroom: cubeGeometry,
    seaweed: waterGeometry,
    aquaticPlant: waterGeometry,
    coral: cubeGeometry,
    deadCoral: cubeGeometry,
    shellBed: waterGeometry,
    coal: cubeGeometry,
    flowerStem: cubeGeometry,
    flowerRed: cubeGeometry,
    flowerYellow: cubeGeometry,
    flowerBlue: cubeGeometry,
    flowerWhite: cubeGeometry,
    grassTuft: waterGeometry,
    dryGrassTuft: waterGeometry,
    pebble: cubeGeometry,
    shoreDamp: waterGeometry,
    shoreFoam: waterGeometry,
    reedStem: cubeGeometry,
    reedTip: cubeGeometry,
    mushroomStem: cubeGeometry,
    mushroomCap: cubeGeometry,
    pendingMine: cubeGeometry,
    ...(cloudGeometry ? { cloud: cloudGeometry } : {}),
  };
}

export function createWorldMaterials({ THREE, includeCloud = false } = {}) {
  const materials = {
    grass: voxelBlockMaterial(THREE, "grass"),
    dirt: voxelBlockMaterial(THREE, "dirt"),
    stone: voxelBlockMaterial(THREE, "stone"),
    deepStone: voxelBlockMaterial(THREE, "deepStone"),
    sand: voxelBlockMaterial(THREE, "sand"),
    sandstone: voxelBlockMaterial(THREE, "sandstone"),
    gravel: voxelBlockMaterial(THREE, "gravel"),
    clay: voxelBlockMaterial(THREE, "clay"),
    mud: voxelBlockMaterial(THREE, "mud"),
    dryDirt: voxelBlockMaterial(THREE, "dryDirt"),
    saltFlat: voxelBlockMaterial(THREE, "saltFlat"),
    snow: voxelBlockMaterial(THREE, "snow"),
    ice: voxelBlockMaterial(THREE, "ice"),
    frozenSoil: voxelBlockMaterial(THREE, "frozenSoil"),
    basalt: voxelBlockMaterial(THREE, "basalt"),
    ash: voxelBlockMaterial(THREE, "ash"),
    bedrock: voxelBlockMaterial(THREE, "bedrock"),
    water: waterBlockMaterial(THREE, "water"),
    swampWater: waterBlockMaterial(THREE, "swampWater"),
    toxicWater: waterBlockMaterial(THREE, "toxicWater"),
    lava: lavaBlockMaterial(THREE),
    quicksand: voxelBlockMaterial(THREE, "quicksand"),
    trunk: voxelBlockMaterial(THREE, "trunk"),
    trunkDark: voxelBlockMaterial(THREE, "trunkDark"),
    pineTrunk: voxelBlockMaterial(THREE, "pineTrunk"),
    deadWood: voxelBlockMaterial(THREE, "deadWood"),
    giantRoot: voxelBlockMaterial(THREE, "giantRoot"),
    leaves: voxelBlockMaterial(THREE, "leaves"),
    leavesDark: voxelBlockMaterial(THREE, "leavesDark"),
    leavesLight: voxelBlockMaterial(THREE, "leavesLight"),
    leavesTeal: voxelBlockMaterial(THREE, "leavesTeal"),
    leavesWarm: voxelBlockMaterial(THREE, "leavesWarm"),
    pineLeaves: voxelBlockMaterial(THREE, "pineLeaves"),
    snowLeaves: voxelBlockMaterial(THREE, "snowLeaves"),
    grassPlant: surfaceBlockMaterial(THREE, "grassPlant"),
    dryGrass: surfaceBlockMaterial(THREE, "dryGrass"),
    bush: plantBlockMaterial(THREE, "bush"),
    deadBush: plantBlockMaterial(THREE, "deadBush"),
    cactus: voxelBlockMaterial(THREE, "cactus"),
    reed: plantBlockMaterial(THREE, "reed"),
    swampGrass: surfaceBlockMaterial(THREE, "swampGrass"),
    snowBush: plantBlockMaterial(THREE, "snowBush"),
    thorn: plantBlockMaterial(THREE, "thorn"),
    moss: surfaceBlockMaterial(THREE, "moss"),
    lichen: surfaceBlockMaterial(THREE, "lichen"),
    vine: surfaceBlockMaterial(THREE, "vine"),
    glowMycelium: glowBlockMaterial(THREE, "glowMycelium"),
    mushroom: voxelBlockMaterial(THREE, "mushroom"),
    seaweed: surfaceBlockMaterial(THREE, "seaweed"),
    aquaticPlant: surfaceBlockMaterial(THREE, "aquaticPlant"),
    coral: plantBlockMaterial(THREE, "coral"),
    deadCoral: plantBlockMaterial(THREE, "deadCoral"),
    shellBed: surfaceBlockMaterial(THREE, "shellBed"),
    coal: voxelBlockMaterial(THREE, "coal"),
    flowerStem: plantBlockMaterial(THREE, "flowerStem"),
    flowerRed: plantBlockMaterial(THREE, "flowerRed"),
    flowerYellow: plantBlockMaterial(THREE, "flowerYellow"),
    flowerBlue: plantBlockMaterial(THREE, "flowerBlue"),
    flowerWhite: plantBlockMaterial(THREE, "flowerWhite"),
    grassTuft: surfaceBlockMaterial(THREE, "grassTuft"),
    dryGrassTuft: surfaceBlockMaterial(THREE, "dryGrassTuft"),
    pebble: voxelBlockMaterial(THREE, "pebble"),
    shoreDamp: surfaceBlockMaterial(THREE, "shoreDamp"),
    shoreFoam: surfaceBlockMaterial(THREE, "shoreFoam"),
    reedStem: plantBlockMaterial(THREE, "reedStem"),
    reedTip: plantBlockMaterial(THREE, "reedTip"),
    mushroomStem: voxelBlockMaterial(THREE, "mushroomStem"),
    mushroomCap: voxelBlockMaterial(THREE, "mushroomCap"),
    pendingMine: pendingMineMaterial(THREE),
  };

  if (includeCloud) materials.cloud = cloudMaterial(THREE);
  return materials;
}

function voxelMaterial(THREE, color, roughness = 0.1, transparent = false, options = {}) {
  const material = new THREE.MeshLambertMaterial({
    color,
    transparent,
    opacity: options.cloud ? 0.82 : transparent ? 0.7 : 1,
    depthWrite: !transparent,
  });
  material.fog = !options.cloud;
  if (options.style) applyVoxelMaterialDetail(material, options.style, currentWorldSeed());
  return material;
}

function voxelBlockMaterial(THREE, type) {
  const profile = blockRenderProfileForType(type);
  const material = voxelMaterial(THREE, profile.color, 0.1, Boolean(profile.transparent), {
    style: profile.style ? materialStyles[profile.style] : null,
  });
  material.name = `block:${type}`;
  return material;
}

function waterMaterial(THREE, color) {
  const material = new THREE.MeshBasicMaterial({
    color,
    transparent: true,
    opacity: 0.62,
    depthWrite: false,
  });
  material.fog = false;
  return material;
}

function waterBlockMaterial(THREE, type) {
  const profile = blockRenderProfileForType(type);
  const material = waterMaterial(THREE, profile.color);
  material.name = `block:${type}`;
  return material;
}

function lavaBlockMaterial(THREE) {
  const profile = blockRenderProfileForType("lava");
  const material = new THREE.MeshLambertMaterial({
    color: profile.color,
    emissive: profile.emissive,
    emissiveIntensity: 0.9,
    transparent: true,
    opacity: profile.opacity ?? 0.86,
    depthWrite: false,
  });
  material.fog = true;
  material.name = "block:lava";
  return material;
}

function pendingMineMaterial(THREE) {
  const material = new THREE.MeshLambertMaterial({
    color: 0xffffff,
    transparent: true,
    opacity: 0.42,
    depthWrite: false,
  });
  material.fog = true;
  material.name = "block:pendingMine";
  return material;
}

function plantMaterial(THREE, color) {
  const material = new THREE.MeshLambertMaterial({
    color,
    side: THREE.DoubleSide,
    alphaTest: 0.1,
  });
  material.fog = true;
  return material;
}

function plantBlockMaterial(THREE, type) {
  const material = plantMaterial(THREE, blockRenderProfileForType(type).color);
  material.name = `block:${type}`;
  return material;
}

function surfacePatchMaterial(THREE, color, opacity) {
  const material = new THREE.MeshLambertMaterial({
    color,
    transparent: true,
    opacity,
    depthWrite: false,
    side: THREE.DoubleSide,
  });
  material.fog = true;
  return material;
}

function surfaceBlockMaterial(THREE, type) {
  const profile = blockRenderProfileForType(type);
  const material = surfacePatchMaterial(THREE, profile.color, profile.opacity ?? 0.22);
  material.name = `block:${type}`;
  return material;
}

function glowBlockMaterial(THREE, type) {
  const profile = blockRenderProfileForType(type);
  const material = surfacePatchMaterial(THREE, profile.color, profile.opacity ?? 0.44);
  material.emissive = new THREE.Color(profile.emissive ?? profile.color);
  material.emissiveIntensity = 0.32;
  material.name = `block:${type}`;
  return material;
}

function cloudMaterial(THREE) {
  const material = new THREE.MeshLambertMaterial({
    color: 0xffffff,
    emissive: 0x9fb8d8,
    emissiveIntensity: 0.18,
  });
  material.fog = false;
  material.name = "block:cloud";
  return material;
}
