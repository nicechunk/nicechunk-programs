// @ts-expect-error The canonical block atlas is an intentionally shared JavaScript module.
import { blockAtlas } from "../src/data/blockAtlas.js";
import type { MaterialPhysicsRecord } from "../sdk/nicechunk-backpack.ts";

export interface MaterialPhysicsDefinition extends MaterialPhysicsRecord {
  key: string;
  unitVolumeMm3: number;
  source: "natural" | "manufactured" | "legacy";
}

interface BlockAtlasEntry {
  id: string;
  key: string;
  physical?: {
    densityKgM3?: number;
    volumeM3?: number;
  };
}

const manufacturedDefinitions = [
  [1001, "charcoal", 750_000, 250],
  [1002, "biochar_compost", 1_000_000, 450],
  [1004, "resin_binder", 250_000, 1_100],
  [1005, "ceramic_brick", 1_000_000, 1_900],
  [1006, "lime_ceramic", 1_000_000, 1_750],
  [1007, "quicklime", 500_000, 900],
  [1008, "salt_flux", 250_000, 1_200],
  [1009, "ash_cement", 1_000_000, 1_300],
  [1010, "glass_ingot", 250_000, 2_500],
  [1011, "obsidian_glass", 250_000, 2_400],
  [1012, "silicon_wafer", 20_000, 2_330],
  [1013, "ice_crystal", 250_000, 917],
  [1014, "iron_bloom", 250_000, 7_000],
  [1015, "copper_bloom", 250_000, 8_200],
  [1016, "alumina_plate", 60_000, 3_900],
  [1017, "nickel_iron", 250_000, 8_100],
  [1018, "carbon_plate", 60_000, 1_600],
  [1019, "carbon_steel", 250_000, 7_850],
  [1020, "basalt_fiber", 250_000, 2_670],
  [1021, "basalt_composite", 250_000, 2_100],
  [1022, "geopolymer_block", 1_000_000, 2_200],
  [1023, "coral_lime", 500_000, 900],
  [1024, "toxic_glass", 250_000, 2_550],
  [1025, "cotton_cloth", 1_000_000, 150],
  [1026, "white_dye", 20_000, 1_200],
  [1027, "yellow_dye", 20_000, 1_100],
  [1028, "red_dye", 20_000, 1_150],
  [1029, "blue_dye", 20_000, 1_250],
  [1030, "pink_dye", 20_000, 1_120],
  [1031, "wooden_plank", 20_000, 550],
  [1032, "wooden_stick", 4_900, 550],
  [1033, "squared_timber", 48_400, 520],
  [1034, "clear_glass_panel", 60_000, 2_500],
  [1035, "ice_blue_glass_panel", 60_000, 2_520],
  [1036, "amber_glass_panel", 60_000, 2_520],
  [1037, "basalt_reinforced_glass", 60_000, 2_600],
  [1038, "fired_clay_brick", 31_250, 1_800],
  [1039, "adobe_brick", 31_250, 1_700],
  [1040, "stone_brick", 31_250, 2_400],
  [1041, "deep_stone_brick", 31_250, 2_700],
  [1042, "basalt_brick", 31_250, 2_900],
  [1043, "sandstone_block", 250_000, 2_200],
  [1044, "cobblestone", 250_000, 2_500],
  [1045, "polished_stone_slab", 60_000, 2_600],
  [1046, "lime_plaster", 80_000, 1_600],
  [1047, "clay_plaster", 80_000, 1_700],
  [1048, "rammed_earth", 500_000, 1_900],
  [1049, "shell_terrazzo", 100_000, 2_400],
  [1050, "white_ceramic_tile", 10_000, 2_200],
  [1051, "blue_ceramic_tile", 10_000, 2_250],
  [1052, "volcanic_ash_concrete", 250_000, 2_300],
  [1053, "salt_crystal_block", 250_000, 2_160],
  [1054, "roof_tile_terracotta", 40_000, 1_900],
  [1055, "roof_tile_ice_blue", 40_000, 2_050],
  [1056, "roof_tile_shell_white", 40_000, 2_070],
  [1057, "roof_tile_charcoal", 40_000, 2_040],
  [1058, "roof_tile_ash_gray", 40_000, 2_120],
  [1059, "roof_tile_mycelium", 40_000, 2_030],
  [1060, "blasting_charge", 750_000, 1_250],
] as const;

const naturalDefinitions = (blockAtlas as readonly BlockAtlasEntry[]).map((entry) => {
  const match = /^NCK_(\d+)$/.exec(entry.id);
  const materialId = Number(match?.[1] ?? 0);
  const densityKgM3 = Math.trunc(Number(entry.physical?.densityKgM3) || 0);
  const unitVolumeMm3 = Math.round((Number(entry.physical?.volumeM3) || 0) * 1_000_000_000);
  return {
    materialId,
    key: entry.key,
    unitVolumeMm3,
    densityKgM3,
    source: "natural" as const,
  };
});

const definitions: MaterialPhysicsDefinition[] = [
  ...naturalDefinitions,
  ...manufacturedDefinitions.map(([materialId, key, unitVolumeMm3, densityKgM3]) => ({
    materialId,
    key,
    unitVolumeMm3,
    densityKgM3,
    source: "manufactured" as const,
  })),
  {
    materialId: 0xffff,
    key: "legacy_forged_item",
    unitVolumeMm3: 250_000,
    densityKgM3: 3_500,
    source: "legacy",
  },
];

export const materialPhysicsDefinitions: readonly MaterialPhysicsDefinition[] = Object.freeze(
  definitions.sort((left, right) => left.materialId - right.materialId),
);

export const materialPhysicsRecords: readonly MaterialPhysicsRecord[] = Object.freeze(
  materialPhysicsDefinitions.map(({ materialId, densityKgM3 }) => Object.freeze({ materialId, densityKgM3 })),
);

export function validateMaterialPhysicsDefinitions(
  definitions: readonly MaterialPhysicsDefinition[] = materialPhysicsDefinitions,
): void {
  if (!definitions.length || definitions.length > 240) {
    throw new Error("Material physics definitions must contain 1-240 records.");
  }
  let previousId = 0;
  const keys = new Set<string>();
  for (const definition of definitions) {
    if (!Number.isInteger(definition.materialId)
      || definition.materialId <= previousId
      || definition.materialId > 0xffff
      || !Number.isInteger(definition.densityKgM3)
      || definition.densityKgM3 <= 0
      || definition.densityKgM3 > 0xffff
      || !Number.isInteger(definition.unitVolumeMm3)
      || definition.unitVolumeMm3 <= 0
      || keys.has(definition.key)) {
      throw new Error(`Invalid material physics definition: ${definition.key}.`);
    }
    previousId = definition.materialId;
    keys.add(definition.key);
  }
}

validateMaterialPhysicsDefinitions();
