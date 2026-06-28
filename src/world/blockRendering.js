export const blockRenderProfiles = Object.freeze({
  grass: { color: 0x62a744, style: "grass" },
  dirt: { color: 0x8a5a35, style: "dirt" },
  stone: { color: 0x7d8278, style: "stone" },
  deepStone: { color: 0x343743, style: "deepStone" },
  coal: { color: 0x1b1a1d, style: "coal" },
  sand: { color: 0xcdbb73, style: "sand" },
  sandstone: { color: 0xb58d54, style: "sandstone" },
  gravel: { color: 0x8a8579, style: "gravel" },
  clay: { color: 0xa77d64, style: "clay" },
  mud: { color: 0x433123, style: "mud" },
  dryDirt: { color: 0xa36d3b, style: "dryDirt" },
  saltFlat: { color: 0xebe6d4, style: "saltFlat" },
  snow: { color: 0xe7f2f5, style: "snow" },
  ice: { color: 0x9ed8ff, style: "ice", transparent: true, opacity: 0.7 },
  frozenSoil: { color: 0x82929c, style: "frozenSoil" },
  basalt: { color: 0x28292d, style: "basalt" },
  ash: { color: 0x77736c, style: "ash" },
  bedrock: { color: 0x1e1e24, style: "bedrock" },
  water: { color: 0x377fb9, transparent: true, opacity: 0.62 },
  swampWater: { color: 0x3d5f45, transparent: true, opacity: 0.62 },
  toxicWater: { color: 0x72c442, transparent: true, opacity: 0.62, emissive: 0x72c442 },
  lava: { color: 0xff5a1f, transparent: true, opacity: 0.86, emissive: 0xff2b00 },
  quicksand: { color: 0xad8f4f, style: "quicksand" },
  trunk: { color: 0x7a4b28, style: "trunk" },
  trunkDark: { color: 0x52311f, style: "trunk" },
  pineTrunk: { color: 0x5a3a25, style: "trunk" },
  deadWood: { color: 0x4a3a2a, style: "trunk" },
  giantRoot: { color: 0x5b351f, style: "trunk" },
  leaves: { color: 0x3f8c3d, style: "leaves" },
  leavesDark: { color: 0x256c38, style: "leaves" },
  leavesLight: { color: 0x73b846, style: "leaves" },
  leavesTeal: { color: 0x1f9b82, style: "leaves" },
  leavesWarm: { color: 0x6aa242, style: "leaves" },
  pineLeaves: { color: 0x1d6654, style: "leaves" },
  snowLeaves: { color: 0xbfd9d8, style: "leaves" },
  cactus: { color: 0x2f9b57, style: "leaves" },
  pebble: { color: 0x8d887a, style: "stone" },
  mushroom: { color: 0xb85b5b },
  mushroomStem: { color: 0xe7d5a1 },
  mushroomCap: { color: 0xc84b3f },
  flowerStem: { color: 0x4d9636 },
  flowerRed: { color: 0xd94a4a },
  flowerYellow: { color: 0xf1d04b },
  flowerBlue: { color: 0x5d8ee8 },
  flowerWhite: { color: 0xf4f1d8 },
  grassPlant: { color: 0x65b84a, transparent: true, opacity: 0.22 },
  dryGrass: { color: 0xc2a44e, transparent: true, opacity: 0.18 },
  grassTuft: { color: 0x4f9d36, transparent: true, opacity: 0.22 },
  dryGrassTuft: { color: 0xb8a85d, transparent: true, opacity: 0.18 },
  bush: { color: 0x3d8b3d },
  deadBush: { color: 0x8b6a3e },
  reed: { color: 0x8ba94e },
  swampGrass: { color: 0x4f7f3c, transparent: true, opacity: 0.22 },
  snowBush: { color: 0xc9d8d2 },
  thorn: { color: 0x5d4a34 },
  moss: { color: 0x3f8f4a, transparent: true, opacity: 0.2 },
  lichen: { color: 0x9ca66b, transparent: true, opacity: 0.18 },
  vine: { color: 0x2f7a35, transparent: true, opacity: 0.22 },
  glowMycelium: { color: 0x6ef0c2, transparent: true, opacity: 0.44, emissive: 0x6ef0c2 },
  seaweed: { color: 0x227c5c, transparent: true, opacity: 0.24 },
  aquaticPlant: { color: 0x3fa878, transparent: true, opacity: 0.24 },
  coral: { color: 0xff7f7f },
  deadCoral: { color: 0x9a8f86 },
  shellBed: { color: 0xe4d6b5, transparent: true, opacity: 0.2 },
  shoreDamp: { color: 0x8f7f4d, transparent: true, opacity: 0.28 },
  shoreFoam: { color: 0xdff5d8, transparent: true, opacity: 0.34 },
  reedStem: { color: 0x6f9a3d },
  reedTip: { color: 0xb88f4a },
});

export function blockRenderProfileForType(type) {
  return blockRenderProfiles[type] ?? blockRenderProfiles.stone;
}

export function blockVisualProfileForType(type) {
  const profile = blockRenderProfileForType(type);
  const color = profile.color ?? blockRenderProfiles.stone.color;
  return {
    type,
    color: hexColor(color),
    frontColor: hexColor(color),
    sideColor: hexColor(scaleColor(color, 0.68)),
    topColor: hexColor(scaleColor(color, profile.transparent ? 1.08 : 1.18)),
    glowColor: profile.emissive ? hexColor(profile.emissive) : "transparent",
    transparent: Boolean(profile.transparent),
    emissive: Boolean(profile.emissive),
    opacity: profile.opacity ?? 1,
  };
}

function hexColor(color) {
  return `#${Math.max(0, Math.min(0xffffff, color | 0)).toString(16).padStart(6, "0")}`;
}

function scaleColor(color, amount) {
  const r = Math.max(0, Math.min(255, Math.round(((color >> 16) & 255) * amount)));
  const g = Math.max(0, Math.min(255, Math.round(((color >> 8) & 255) * amount)));
  const b = Math.max(0, Math.min(255, Math.round((color & 255) * amount)));
  return (r << 16) | (g << 8) | b;
}
