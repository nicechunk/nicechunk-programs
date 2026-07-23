import assert from "node:assert";
// @ts-expect-error The canonical block atlas is an intentionally shared JavaScript module.
import { blockAtlas } from "../src/data/blockAtlas.js";
import {
  materialPhysicsDefinitions,
  materialPhysicsRecords,
  validateMaterialPhysicsDefinitions,
} from "../scripts/material-physics-rules.ts";

describe("material physics deployment rules", () => {
  it("covers every natural block, manufactured material, and the legacy forged fallback", () => {
    assert.equal(blockAtlas.length, 53);
    assert.equal(materialPhysicsDefinitions.filter((entry) => entry.source === "natural").length, 53);
    assert.equal(materialPhysicsDefinitions.filter((entry) => entry.source === "manufactured").length, 59);
    assert.equal(materialPhysicsDefinitions.at(-1)?.materialId, 0xffff);
    assert.equal(materialPhysicsDefinitions.length, 113);
    assert.equal(materialPhysicsRecords.length, materialPhysicsDefinitions.length);
  });

  it("keeps canonical IDs sorted, unique, and valid for the PDA binary search", () => {
    assert.doesNotThrow(() => validateMaterialPhysicsDefinitions());
    for (let index = 1; index < materialPhysicsRecords.length; index += 1) {
      assert.ok(materialPhysicsRecords[index].materialId > materialPhysicsRecords[index - 1].materialId);
    }
  });

  it("uses stable item codes and objective density values for critical outputs", () => {
    const byKey = new Map(materialPhysicsDefinitions.map((entry) => [entry.key, entry]));
    assert.deepEqual(pick(byKey.get("stone")), { materialId: 3, unitVolumeMm3: 1_000_000, densityKgM3: 2_600 });
    assert.deepEqual(pick(byKey.get("cotton_cloth")), { materialId: 1025, unitVolumeMm3: 1_000_000, densityKgM3: 150 });
    assert.deepEqual(pick(byKey.get("wooden_plank")), { materialId: 1031, unitVolumeMm3: 20_000, densityKgM3: 550 });
    assert.deepEqual(pick(byKey.get("stone_brick")), { materialId: 1040, unitVolumeMm3: 31_250, densityKgM3: 2_400 });
    assert.deepEqual(pick(byKey.get("blasting_charge")), { materialId: 1060, unitVolumeMm3: 750_000, densityKgM3: 1_250 });
  });
});

function pick(value: { materialId: number; unitVolumeMm3: number; densityKgM3: number } | undefined) {
  assert.ok(value);
  return {
    materialId: value.materialId,
    unitVolumeMm3: value.unitVolumeMm3,
    densityKgM3: value.densityKgM3,
  };
}
