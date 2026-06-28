import assert from "node:assert";
// @ts-expect-error JavaScript world modules are intentionally tested through their public ESM exports.
import { BiomeType, EMPTY_BLOCK, WorldMapBlock } from "../src/world/blocks.js";
// @ts-expect-error JavaScript world modules are intentionally tested through their public ESM exports.
import {
  canonicalAboveSurfaceBlocksInArea,
  canonicalBlockIdAt,
  canonicalRenderTypeAt,
  canonicalSurfaceHeightAt,
  canonicalWaterLevelAt,
  setCanonicalWorldConfig,
} from "../src/world/canonicalResource.js";
// @ts-expect-error JavaScript world modules are intentionally tested through their public ESM exports.
import { getGeneratedBlock, setWorldSeed, terrainProfile } from "../src/world/generator.js";

const CLIENT_GOLDEN_SEED = "nicechunk-golden-review-v1";
const CANONICAL_WORLD_CONFIG = {
  worldSeedHex: "6e6963656368756e6b2d676f6c64656e2d7265766965772d76310000000000",
  minBuildY: -32,
  maxBuildY: 256,
  maxTerrainHeight: 160,
  seaLevel: 96,
};

describe("nicechunk deterministic worldgen golden fixtures", () => {
  beforeEach(() => {
    setWorldSeed(CLIENT_GOLDEN_SEED);
    setCanonicalWorldConfig(CANONICAL_WORLD_CONFIG);
  });

  it("keeps client terrain profiles stable across representative coordinates", () => {
    const fixtures = [
      { x: 0, z: 0, height: 102, biome: BiomeType.Swamp, terrain: WorldMapBlock.Mud, subsurface: WorldMapBlock.Clay },
      { x: -48, z: 96, height: 101, biome: BiomeType.River, terrain: WorldMapBlock.Water, subsurface: WorldMapBlock.Mud },
      { x: 120, z: 24, height: 101, biome: BiomeType.Beach, terrain: WorldMapBlock.Sand, subsurface: WorldMapBlock.Sand },
      { x: 512, z: -320, height: 96, biome: BiomeType.Ocean, terrain: WorldMapBlock.Water, subsurface: WorldMapBlock.Sand },
      { x: -512, z: 384, height: 115, biome: BiomeType.Forest, terrain: WorldMapBlock.Grass, subsurface: WorldMapBlock.Dirt },
      { x: 1024, z: -1024, height: 106, biome: BiomeType.Rainforest, terrain: WorldMapBlock.Grass, subsurface: WorldMapBlock.Dirt },
    ];

    for (const fixture of fixtures) {
      const profile = terrainProfile(fixture.x, fixture.z);
      assert.equal(profile.height, fixture.height, `height at ${fixture.x},${fixture.z}`);
      assert.equal(profile.biome, fixture.biome, `biome at ${fixture.x},${fixture.z}`);
      assert.equal(profile.terrain, fixture.terrain, `terrain at ${fixture.x},${fixture.z}`);
      assert.equal(profile.subsurface, fixture.subsurface, `subsurface at ${fixture.x},${fixture.z}`);
    }
  });

  it("keeps client generated block outputs stable around surface depth", () => {
    const fixtures = [
      { x: 8, z: 8, y: 102, terrain: WorldMapBlock.Mud, fluid: null },
      { x: 8, z: 8, y: 98, terrain: WorldMapBlock.Clay, fluid: null },
      { x: 512, z: -320, y: 96, terrain: WorldMapBlock.Water, fluid: null },
      { x: 512, z: -320, y: 92, terrain: WorldMapBlock.Sand, fluid: null },
    ];

    for (const fixture of fixtures) {
      const block = getGeneratedBlock(fixture.x, fixture.y, fixture.z);
      assert.ok(block, `generated block at ${fixture.x},${fixture.y},${fixture.z}`);
      assert.equal(block.terrain, fixture.terrain, `terrain at ${fixture.x},${fixture.y},${fixture.z}`);
      assert.equal(block.fluid ?? null, fixture.fluid, `fluid at ${fixture.x},${fixture.y},${fixture.z}`);
    }

    assert.equal(getGeneratedBlock(8, 103, 8), null);
  });

  it("keeps canonical protocol block IDs stable for mining verification", () => {
    const fixtures = [
      { x: 0, z: 0, height: 126, water: null, surface: WorldMapBlock.FrozenSoil, below4: WorldMapBlock.Stone, above1: EMPTY_BLOCK, render: "frozenSoil" },
      { x: 32, z: -16, height: 116, water: null, surface: WorldMapBlock.Grass, below4: WorldMapBlock.Stone, above1: EMPTY_BLOCK, render: "grass" },
      { x: -160, z: -80, height: 96, water: null, surface: WorldMapBlock.Sand, below4: WorldMapBlock.Stone, above1: EMPTY_BLOCK, render: "sand" },
      { x: 256, z: 256, height: 89, water: 96, surface: WorldMapBlock.Sand, below4: WorldMapBlock.Stone, above1: WorldMapBlock.Water, render: "sand" },
      { x: -512, z: 384, height: 94, water: 96, surface: WorldMapBlock.Sand, below4: WorldMapBlock.Stone, above1: WorldMapBlock.Water, render: "sand" },
    ];

    for (const fixture of fixtures) {
      const height = canonicalSurfaceHeightAt({ x: fixture.x, z: fixture.z });
      assert.equal(height, fixture.height, `canonical height at ${fixture.x},${fixture.z}`);
      assert.equal(canonicalWaterLevelAt({ x: fixture.x, z: fixture.z, surface: height }), fixture.water, `canonical water at ${fixture.x},${fixture.z}`);
      assert.equal(canonicalBlockIdAt({ x: fixture.x, y: height, z: fixture.z }), fixture.surface, `canonical surface at ${fixture.x},${fixture.z}`);
      assert.equal(canonicalBlockIdAt({ x: fixture.x, y: height - 4, z: fixture.z }), fixture.below4, `canonical below4 at ${fixture.x},${fixture.z}`);
      assert.equal(canonicalBlockIdAt({ x: fixture.x, y: height + 1, z: fixture.z }), fixture.above1, `canonical above1 at ${fixture.x},${fixture.z}`);
      assert.equal(canonicalRenderTypeAt({ x: fixture.x, y: height, z: fixture.z }), fixture.render, `canonical render at ${fixture.x},${fixture.z}`);
    }
  });

  it("keeps canonical above-surface tree blocks stable", () => {
    const blocks = canonicalAboveSurfaceBlocksInArea({ minX: -512, maxX: -481, minZ: -416, maxZ: -385 });
    const firstTen = blocks.slice(0, 10).map((block: { x: number; y: number; z: number; block: number; type: string }) => ({
      x: block.x,
      y: block.y,
      z: block.z,
      block: block.block,
      type: block.type,
    }));

    assert.deepEqual(firstTen, [
      { x: -502, y: 118, z: -414, block: WorldMapBlock.Leaves, type: "leaves" },
      { x: -502, y: 119, z: -414, block: WorldMapBlock.Leaves, type: "leaves" },
      { x: -502, y: 120, z: -414, block: WorldMapBlock.Leaves, type: "leaves" },
      { x: -501, y: 118, z: -414, block: WorldMapBlock.Leaves, type: "leaves" },
      { x: -501, y: 119, z: -414, block: WorldMapBlock.Leaves, type: "leaves" },
      { x: -500, y: 118, z: -414, block: WorldMapBlock.Leaves, type: "leaves" },
      { x: -500, y: 119, z: -414, block: WorldMapBlock.Leaves, type: "leaves" },
      { x: -500, y: 120, z: -414, block: WorldMapBlock.Leaves, type: "leaves" },
      { x: -503, y: 118, z: -413, block: WorldMapBlock.Leaves, type: "leaves" },
      { x: -503, y: 119, z: -413, block: WorldMapBlock.Leaves, type: "leaves" },
    ]);
  });
});
