import assert from "node:assert/strict";
import { describe, it } from "mocha";

import { createForgedItemMesh, decodeForgeCode, forgeBytesToCode } from "../src/forgedItems.js";

const forgedAttributeCount = 12;

class BitWriter {
  private buffer: number[] = [];
  private current = 0;
  private bitCount = 0;

  write(value: number, bits: number): void {
    for (let index = bits - 1; index >= 0; index -= 1) {
      this.current = (this.current << 1) | ((value >> index) & 1);
      this.bitCount += 1;
      if (this.bitCount === 8) {
        this.buffer.push(this.current);
        this.current = 0;
        this.bitCount = 0;
      }
    }
  }

  bytes(): Uint8Array {
    if (this.bitCount > 0) this.buffer.push(this.current << (8 - this.bitCount));
    return Uint8Array.from(this.buffer);
  }
}

describe("forged item codec", () => {
  it("decodes compact v8 physical stats with derived density", () => {
    const bytes = compactStatsCodeBytes();
    const decoded = decodeForgeCode(forgeBytesToCode(bytes));

    assert.equal(decoded.version, 8);
    assert.equal(decoded.equipmentStats.massGrams, 12_000);
    assert.equal(decoded.equipmentStats.volumeCm3, 3_000);
    assert.equal(decoded.equipmentStats.densityKgM3, 4_000);
    assert.equal(decoded.equipmentStats.attributes.hardness, 70);
    assert.equal(decoded.components.length, 0);
  });

  it("keeps legacy v7 physical stats readable", () => {
    const decoded = decodeForgeCode(forgeBytesToCode(legacyStatsCodeBytes()));

    assert.equal(decoded.version, 7);
    assert.equal(decoded.equipmentStats.massGrams, 1_234);
    assert.equal(decoded.equipmentStats.volumeCm3, 567);
    assert.equal(decoded.equipmentStats.densityKgM3, 7_890);
    assert.equal(decoded.equipmentStats.attributes.hardness, 50);
    assert.equal(decoded.components.length, 0);
  });

  it("stores compact v8 stats in fewer bytes than the legacy stats block", () => {
    assert.equal(compactStatsCodeBytes().byteLength, 15);
    assert.equal(legacyStatsCodeBytes().byteLength, 19);
  });

  it("decodes v9 full-solid components without an RLE payload", () => {
    const decoded = decodeForgeCode(forgeBytesToCode(fullSolidComponentCodeBytes(9)));

    assert.equal(decoded.version, 9);
    assert.equal(decoded.components.length, 1);
    assert.equal(decoded.components[0].solid.length, 1_960);
    assert.ok(decoded.components[0].solid.every((value: number) => value === 1));
  });

  it("stores v9 full-solid components in fewer bytes than v8 RLE", () => {
    assert.ok(fullSolidComponentCodeBytes(9).byteLength < fullSolidComponentCodeBytes(8).byteLength);
  });

  it("decodes v10 zero-offset components without offset coordinates", () => {
    const decoded = decodeForgeCode(forgeBytesToCode(fullSolidComponentCodeBytes(10)));

    assert.equal(decoded.version, 10);
    assert.equal(decoded.components[0].offset.x, 0);
    assert.equal(decoded.components[0].offset.y, 0);
    assert.equal(decoded.components[0].offset.z, 0);
  });

  it("keeps v10 non-zero component offsets readable", () => {
    const decoded = decodeForgeCode(forgeBytesToCode(fullSolidComponentCodeBytes(10, { offsetX64: 64 })));

    assert.equal(decoded.components[0].offset.x, 1);
    assert.equal(decoded.components[0].offset.y, 0);
    assert.equal(decoded.components[0].offset.z, 0);
  });

  it("stores v10 zero-offset full-solid components in fewer bytes than v9", () => {
    assert.ok(fullSolidComponentCodeBytes(10).byteLength < fullSolidComponentCodeBytes(9).byteLength);
  });

  it("builds full-solid component geometry as one cuboid", () => {
    const mesh = createForgedItemMesh(forgeBytesToCode(fullSolidComponentCodeBytes(10)));
    try {
      assert.equal(mesh.geometry.getAttribute("position")?.count, 36);
    } finally {
      mesh.geometry.dispose();
      if (Array.isArray(mesh.material)) {
        for (const material of mesh.material) material.dispose();
      } else {
        mesh.material.dispose();
      }
    }
  });

  it("greedy-meshes drilled components into merged surface quads", () => {
    const solid = throughHoleSolid();
    const mesh = createForgedItemMesh(forgeBytesToCode(throughHoleComponentCodeBytes(solid)));
    try {
      const positionCount = mesh.geometry.getAttribute("position")?.count ?? 0;
      assert.equal(positionCount, 96);
      assert.ok(positionCount < naiveVoxelPositionCount(solid) / 10);
    } finally {
      mesh.geometry.dispose();
      if (Array.isArray(mesh.material)) {
        for (const material of mesh.material) material.dispose();
      } else {
        mesh.material.dispose();
      }
    }
  });

  it("decodes v12 cut-box solid components", () => {
    const solid = throughHoleSolid();
    const decoded = decodeForgeCode(forgeBytesToCode(throughHoleCutBoxComponentCodeBytes()));

    assert.equal(decoded.version, 12);
    assert.equal(decoded.components.length, 1);
    assert.deepEqual(Array.from(decoded.components[0].solid), Array.from(solid));
  });

  it("stores v12 cut-box components in fewer bytes than v11 RLE", () => {
    const solid = throughHoleSolid();

    assert.ok(throughHoleCutBoxComponentCodeBytes().byteLength < throughHoleComponentCodeBytes(solid).byteLength);
  });

  it("decodes v13 extruded-mask solid components", () => {
    const solid = diagonalPrismSolid();
    const decoded = decodeForgeCode(forgeBytesToCode(diagonalPrismExtrudedMaskComponentCodeBytes(solid)));

    assert.equal(decoded.version, 13);
    assert.deepEqual(Array.from(decoded.components[0].solid), Array.from(solid));
  });

  it("stores v13 extruded-mask components in fewer bytes than v11 RLE", () => {
    const solid = diagonalPrismSolid();

    assert.ok(diagonalPrismExtrudedMaskComponentCodeBytes(solid).byteLength < diagonalPrismRleComponentCodeBytes(solid).byteLength);
  });

  it("decodes v11 default resource color without a color payload", () => {
    const decoded = decodeForgeCode(forgeBytesToCode(fullSolidComponentCodeBytes(11, { defaultColor: true })));

    assert.equal(decoded.version, 11);
    assert.equal(decoded.components[0].color.getHexString(), "9ca4a2");
  });

  it("keeps v11 custom component colors readable", () => {
    const decoded = decodeForgeCode(forgeBytesToCode(fullSolidComponentCodeBytes(11)));

    assert.equal(Math.round(decoded.components[0].color.r * 15), 9);
    assert.equal(Math.round(decoded.components[0].color.g * 15), 9);
    assert.equal(Math.round(decoded.components[0].color.b * 15), 9);
  });

  it("stores v11 default-color components in fewer bytes than v10", () => {
    assert.ok(fullSolidComponentCodeBytes(11, { defaultColor: true }).byteLength < fullSolidComponentCodeBytes(10).byteLength);
  });

  it("decodes v14 painted component faces", () => {
    const decoded = decodeForgeCode(forgeBytesToCode(paintedFullSolidComponentCodeBytes()));

    assert.equal(decoded.version, 14);
    assert.equal(decoded.components.length, 1);
    assert.equal(decoded.components[0].paint, undefined);
    assert.equal(decoded.components[0].paintQuads.length, 1);
    assert.deepEqual(decoded.components[0].paintQuads[0], {
      axis: 0,
      side: 1,
      plane: 14,
      u0: 2,
      u1: 5,
      v0: 3,
      v1: 6,
      color: "#ff1122",
    });
  });

  it("keeps full-face v14 paint compact while rendering it", () => {
    const decoded = decodeForgeCode(forgeBytesToCode(paintedFullFaceComponentCodeBytes()));

    assert.equal(decoded.components[0].paint, undefined);
    assert.equal(decoded.components[0].paintQuads.length, 1);

    const mesh = createForgedItemMesh(forgeBytesToCode(paintedFullFaceComponentCodeBytes()));
    try {
      const colors = Array.from(mesh.geometry.getAttribute("color")?.array ?? []);
      const uniqueColors = new Set<string>();
      for (let index = 0; index < colors.length; index += 3) {
        uniqueColors.add(`${Number(colors[index]).toFixed(4)},${Number(colors[index + 1]).toFixed(4)},${Number(colors[index + 2]).toFixed(4)}`);
      }
      assert.equal(decoded.components[0].paintQuads.length, 1);
      assert.ok(uniqueColors.size > 1);
    } finally {
      mesh.geometry.dispose();
      if (Array.isArray(mesh.material)) {
        for (const material of mesh.material) material.dispose();
      } else {
        mesh.material.dispose();
      }
    }
  });

  it("renders v14 paint as vertex colors instead of a cuboid shortcut", () => {
    const mesh = createForgedItemMesh(forgeBytesToCode(paintedFullSolidComponentCodeBytes()));
    try {
      const colors = Array.from(mesh.geometry.getAttribute("color")?.array ?? []);
      const uniqueColors = new Set<string>();
      for (let index = 0; index < colors.length; index += 3) {
        uniqueColors.add(`${Number(colors[index]).toFixed(4)},${Number(colors[index + 1]).toFixed(4)},${Number(colors[index + 2]).toFixed(4)}`);
      }
      assert.ok((mesh.geometry.getAttribute("position")?.count ?? 0) > 36);
      assert.ok(uniqueColors.size > 1);
      assert.ok(colors.some((value, index) => index % 3 === 0 && Math.abs(Number(value) - 1) < 0.001));
    } finally {
      mesh.geometry.dispose();
      if (Array.isArray(mesh.material)) {
        for (const material of mesh.material) material.dispose();
      } else {
        mesh.material.dispose();
      }
    }
  });
});

function compactStatsCodeBytes(): Uint8Array {
  const writer = new BitWriter();
  writer.write(8, 4);
  writer.write(2_400, 16);
  writer.write(3_000, 16);
  for (let index = 0; index < forgedAttributeCount; index += 1) writer.write(44, 6);
  writer.write(0, 1);
  writer.write(0, 5);
  return writer.bytes();
}

function legacyStatsCodeBytes(): Uint8Array {
  const writer = new BitWriter();
  writer.write(7, 4);
  writer.write(1_234, 22);
  writer.write(567, 22);
  writer.write(7_890, 14);
  for (let index = 0; index < forgedAttributeCount; index += 1) writer.write(50, 7);
  writer.write(0, 1);
  writer.write(0, 5);
  return writer.bytes();
}

function fullSolidComponentCodeBytes(version: 8 | 9 | 10 | 11, { offsetX64 = 0, defaultColor = false } = {}): Uint8Array {
  const writer = new BitWriter();
  writer.write(version, 4);
  writer.write(2_400, 16);
  writer.write(3_000, 16);
  for (let index = 0; index < forgedAttributeCount; index += 1) writer.write(44, 6);
  writer.write(0, 1);
  writer.write(1, 5);
  writer.write(0, 3);
  if (version >= 11 && defaultColor) {
    writer.write(1, 1);
  } else {
    if (version >= 11) writer.write(0, 1);
    writer.write(0x999, 12);
  }
  writer.write(32, 8);
  writer.write(32, 8);
  writer.write(32, 8);
  if (version >= 10 && offsetX64 === 0) {
    writer.write(1, 1);
  } else {
    if (version >= 10) writer.write(0, 1);
    writer.write(offsetX64, 10);
    writer.write(0, 10);
    writer.write(0, 10);
  }
  writer.write(0, 1);
  if (version >= 9) {
    writer.write(1, 1);
  } else {
    writer.write(1, 1);
    writer.write(1, 11);
    writer.write(1_960, 11);
  }
  return writer.bytes();
}

function throughHoleComponentCodeBytes(solid: Uint8Array): Uint8Array {
  const writer = new BitWriter();
  writer.write(11, 4);
  writer.write(2_400, 16);
  writer.write(3_000, 16);
  for (let index = 0; index < forgedAttributeCount; index += 1) writer.write(44, 6);
  writer.write(0, 1);
  writer.write(1, 5);
  writer.write(0, 3);
  writer.write(1, 1);
  writer.write(32, 8);
  writer.write(32, 8);
  writer.write(32, 8);
  writer.write(1, 1);
  writer.write(0, 1);
  writer.write(0, 1);
  writeSolidRuns(writer, solid);
  return writer.bytes();
}

function throughHoleCutBoxComponentCodeBytes(): Uint8Array {
  const writer = new BitWriter();
  writer.write(12, 4);
  writer.write(2_400, 16);
  writer.write(3_000, 16);
  for (let index = 0; index < forgedAttributeCount; index += 1) writer.write(44, 6);
  writer.write(0, 1);
  writer.write(1, 5);
  writer.write(0, 3);
  writer.write(1, 1);
  writer.write(32, 8);
  writer.write(32, 8);
  writer.write(32, 8);
  writer.write(1, 1);
  writer.write(0, 1);
  writer.write(2, 2);
  writer.write(1, 5);
  writeSolidCutBox(writer, { x: 5, y: 0, z: 5, sx: 4, sy: 10, sz: 4 });
  return writer.bytes();
}

function diagonalPrismRleComponentCodeBytes(solid: Uint8Array): Uint8Array {
  const writer = componentHeaderWriter(11);
  writer.write(0, 1);
  writeSolidRuns(writer, solid);
  return writer.bytes();
}

function diagonalPrismExtrudedMaskComponentCodeBytes(solid: Uint8Array): Uint8Array {
  const writer = componentHeaderWriter(13);
  writer.write(3, 2);
  writer.write(1, 2);
  writeSolidMaskRuns(writer, diagonalPrismMask(solid));
  return writer.bytes();
}

function paintedFullSolidComponentCodeBytes(): Uint8Array {
  const writer = componentHeaderWriter(14);
  writer.write(1, 2);
  writer.write(1, 11);
  writer.write(0, 2);
  writer.write(1, 1);
  writer.write(14, 4);
  writer.write(2, 4);
  writer.write(5, 4);
  writer.write(3, 4);
  writer.write(6, 4);
  writer.write(0xf12, 12);
  return writer.bytes();
}

function paintedFullFaceComponentCodeBytes(): Uint8Array {
  const writer = componentHeaderWriter(14);
  writer.write(1, 2);
  writer.write(1, 11);
  writer.write(0, 2);
  writer.write(1, 1);
  writer.write(14, 4);
  writer.write(0, 4);
  writer.write(10, 4);
  writer.write(0, 4);
  writer.write(14, 4);
  writer.write(0x1f2, 12);
  return writer.bytes();
}

function componentHeaderWriter(version: 11 | 13 | 14): BitWriter {
  const writer = new BitWriter();
  writer.write(version, 4);
  writer.write(2_400, 16);
  writer.write(3_000, 16);
  for (let index = 0; index < forgedAttributeCount; index += 1) writer.write(44, 6);
  writer.write(0, 1);
  writer.write(1, 5);
  writer.write(0, 3);
  writer.write(1, 1);
  writer.write(32, 8);
  writer.write(32, 8);
  writer.write(32, 8);
  writer.write(1, 1);
  writer.write(0, 1);
  return writer;
}

function writeSolidCutBox(writer: BitWriter, box: { x: number; y: number; z: number; sx: number; sy: number; sz: number }): void {
  writer.write(box.x, 4);
  writer.write(box.y, 4);
  writer.write(box.z, 4);
  writer.write(box.sx, 4);
  writer.write(box.sy, 4);
  writer.write(box.sz, 4);
}

function diagonalPrismSolid(): Uint8Array {
  const grid = { x: 14, y: 10, z: 14 };
  const solid = new Uint8Array(grid.x * grid.y * grid.z);
  for (let z = 0; z < grid.z; z += 1) {
    for (let y = 0; y < grid.y; y += 1) {
      for (let x = 0; x < grid.x; x += 1) {
        if (x <= z) solid[voxelIndex(grid, x, y, z)] = 1;
      }
    }
  }
  return solid;
}

function diagonalPrismMask(solid: Uint8Array): Uint8Array {
  const grid = { x: 14, y: 10, z: 14 };
  const mask = new Uint8Array(grid.x * grid.z);
  for (let z = 0; z < grid.z; z += 1) {
    for (let x = 0; x < grid.x; x += 1) mask[x + grid.x * z] = solid[voxelIndex(grid, x, 0, z)];
  }
  return mask;
}

function writeSolidMaskRuns(writer: BitWriter, mask: Uint8Array): void {
  const runs: number[] = [];
  let current = mask[0] ?? 0;
  let length = 0;
  for (const value of mask) {
    if (value === current && length < 255) {
      length += 1;
      continue;
    }
    runs.push(length);
    current = value;
    length = 1;
  }
  runs.push(length);
  writer.write(mask[0] ?? 0, 1);
  writer.write(Math.min(runs.length, 255), 8);
  for (const run of runs.slice(0, 255)) writer.write(run, 8);
}

function throughHoleSolid(): Uint8Array {
  const grid = { x: 14, y: 10, z: 14 };
  const solid = new Uint8Array(grid.x * grid.y * grid.z).fill(1);
  for (let z = 5; z <= 8; z += 1) {
    for (let y = 0; y < grid.y; y += 1) {
      for (let x = 5; x <= 8; x += 1) {
        solid[voxelIndex(grid, x, y, z)] = 0;
      }
    }
  }
  return solid;
}

function writeSolidRuns(writer: BitWriter, solid: Uint8Array): void {
  const runs: number[] = [];
  let current = solid[0] ?? 0;
  let length = 0;
  for (const value of solid) {
    if (value === current && length < 2047) {
      length += 1;
      continue;
    }
    runs.push(length);
    current = value;
    length = 1;
  }
  runs.push(length);
  writer.write(solid[0] ?? 0, 1);
  writer.write(Math.min(runs.length, 2047), 11);
  for (const run of runs.slice(0, 2047)) writer.write(run, 11);
}

function naiveVoxelPositionCount(solid: Uint8Array): number {
  const grid = { x: 14, y: 10, z: 14 };
  let faces = 0;
  for (let z = 0; z < grid.z; z += 1) {
    for (let y = 0; y < grid.y; y += 1) {
      for (let x = 0; x < grid.x; x += 1) {
        if (solid[voxelIndex(grid, x, y, z)] !== 1) continue;
        if (!solidAt(solid, grid, x + 1, y, z)) faces += 1;
        if (!solidAt(solid, grid, x - 1, y, z)) faces += 1;
        if (!solidAt(solid, grid, x, y + 1, z)) faces += 1;
        if (!solidAt(solid, grid, x, y - 1, z)) faces += 1;
        if (!solidAt(solid, grid, x, y, z + 1)) faces += 1;
        if (!solidAt(solid, grid, x, y, z - 1)) faces += 1;
      }
    }
  }
  return faces * 6;
}

function solidAt(solid: Uint8Array, grid: { x: number; y: number; z: number }, x: number, y: number, z: number): boolean {
  if (x < 0 || y < 0 || z < 0 || x >= grid.x || y >= grid.y || z >= grid.z) return false;
  return solid[voxelIndex(grid, x, y, z)] === 1;
}

function voxelIndex(grid: { x: number; y: number; z: number }, x: number, y: number, z: number): number {
  return x + grid.x * (y + grid.y * z);
}
