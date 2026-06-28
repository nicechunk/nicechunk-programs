export const materialStyles = {
  grass: 1,
  dirt: 2,
  sand: 3,
  sandstone: 4,
  stone: 5,
  snow: 6,
  trunk: 7,
  leaves: 8,
  gravel: 9,
  clay: 10,
  mud: 11,
  dryDirt: 12,
  saltFlat: 13,
  ice: 14,
  frozenSoil: 15,
  basalt: 16,
  ash: 17,
  quicksand: 18,
  deepStone: 19,
  bedrock: 20,
  coal: 21,
};

export function applyVoxelMaterialDetail(material, style, seed = 0) {
  const uniforms = {
    uProcStyle: { value: style },
    uProcSeed: { value: normalizeSeed(seed) },
  };

  material.userData.proceduralUniforms = uniforms;
  material.onBeforeCompile = (shader) => {
    shader.uniforms.uProcStyle = uniforms.uProcStyle;
    shader.uniforms.uProcSeed = uniforms.uProcSeed;
    shader.vertexShader = shader.vertexShader.replace(
      "void main() {",
      `
varying vec3 vProcWorldPosition;
varying vec3 vProcWorldNormal;

void main() {
      `,
    );
    shader.vertexShader = shader.vertexShader.replace(
      "#include <defaultnormal_vertex>",
      `
#include <defaultnormal_vertex>
vProcWorldNormal = inverseTransformDirection(transformedNormal, viewMatrix);
      `,
    );
    shader.vertexShader = shader.vertexShader.replace(
      "#include <begin_vertex>",
      `
#include <begin_vertex>
vec4 procWorldPosition = vec4(transformed, 1.0);
#ifdef USE_INSTANCING
  procWorldPosition = instanceMatrix * procWorldPosition;
#endif
procWorldPosition = modelMatrix * procWorldPosition;
vProcWorldPosition = procWorldPosition.xyz;
      `,
    );
    shader.fragmentShader = shader.fragmentShader.replace(
      "void main() {",
      `
uniform float uProcStyle;
uniform float uProcSeed;
varying vec3 vProcWorldPosition;
varying vec3 vProcWorldNormal;

float procHash(vec2 p) {
  vec3 p3 = fract(vec3(p.xyx) * 0.1031);
  p3 += dot(p3, p3.yzx + 33.33 + uProcSeed * 19.17);
  return fract((p3.x + p3.y) * p3.z);
}

float procEdge(vec2 uv, float width) {
  vec2 q = abs(fract(uv + 0.5) - 0.5);
  return smoothstep(0.5 - width, 0.5, max(q.x, q.y));
}

vec3 procTint(vec3 color, vec3 tint, float amount) {
  return mix(color, color * tint, clamp(amount, 0.0, 1.0));
}

vec3 procBlend(vec3 color, vec3 target, float amount) {
  return mix(color, target, clamp(amount, 0.0, 1.0));
}

vec3 applyProcVoxelDetail(vec3 color) {
  vec3 normal = normalize(vProcWorldNormal);
  vec3 world = vProcWorldPosition;
  vec3 cell = floor(world + 0.5);
  float topFace = smoothstep(0.55, 0.9, normal.y);
  float sideFace = 1.0 - topFace;
  vec2 faceUv = topFace > 0.5
    ? world.xz
    : (abs(normal.x) > abs(normal.z) ? world.zy : world.xy);
  float edge = procEdge(faceUv, 0.055);
  vec2 faceCell = abs(fract(faceUv + 0.5) - 0.5);
  float bevelBand = smoothstep(0.36, 0.49, max(faceCell.x, faceCell.y));
  float bevelHighlight = smoothstep(0.31, 0.39, max(faceCell.x, faceCell.y)) * (1.0 - smoothstep(0.43, 0.49, max(faceCell.x, faceCell.y)));
  float cornerWear = smoothstep(0.32, 0.49, min(faceCell.x, faceCell.y)) * bevelBand;
  float chip = step(0.88, procHash(floor(faceUv * 2.0) + cell.xy * 0.23 + cell.z * 0.17)) * bevelBand;
  float localY = clamp(world.y - cell.y + 0.5, 0.0, 1.0);
  float lowerSide = sideFace * (1.0 - smoothstep(0.12, 0.46, localY));
  float upperSide = sideFace * smoothstep(0.64, 0.96, localY);
  float topLip = smoothstep(0.72, 0.96, fract(world.y + 0.5));
  float cellTone = procHash(cell.xz + vec2(uProcStyle * 13.1, uProcSeed * 5.7));
  float fleck = procHash(floor(faceUv * 5.0) + cell.xz * 0.37 + uProcStyle * 23.0);
  float broadLayer = smoothstep(0.42, 0.54, fract(world.y * 0.72 + cell.x * 0.07 + cell.z * 0.11));
  float fineLayer = smoothstep(0.48, 0.54, fract(world.y * 3.25 + cell.x * 0.13 - cell.z * 0.09));
  float longCrack = step(0.9, procHash(vec2(floor(faceUv.x * 2.0), floor(world.y * 1.35)) + cell.xz * 0.19));
  float windLine = smoothstep(0.46, 0.5, fract((world.x * 0.52 + world.z * 0.2) * 2.4 + uProcSeed * 3.0));
  float sketchLine = smoothstep(0.47, 0.5, fract((faceUv.x * 0.74 + faceUv.y * 0.31 + uProcSeed * 4.0) * 4.0));

  color *= 0.965 + cellTone * 0.07;
  color *= 1.0 - edge * mix(0.035, 0.11, sideFace);
  color *= 1.0 - lowerSide * 0.08 - cornerWear * 0.045 - chip * 0.035;
  color *= mix(0.84 + topLip * 0.18 + upperSide * 0.06, 1.0, topFace);
  color = procTint(color, vec3(1.12, 1.08, 0.94), topFace * bevelHighlight * 0.09);
  color = procTint(color, vec3(1.08, 1.04, 0.92), upperSide * bevelHighlight * 0.05);

  if (uProcStyle < 1.5) {
    float colorPatch = procHash(floor(world.xz * 2.0) + cell.xz * 0.11);
    float bladeLine = smoothstep(0.46, 0.5, fract((world.x * 1.35 - world.z * 0.75 + cellTone * 2.0) * 2.8));
    float sideSegment = floor(faceUv.x * 8.0);
    float fringeSeed = procHash(vec2(sideSegment, cell.x * 0.37 + cell.z * 0.61 + uProcSeed * 13.0));
    float fringeLength = 0.26 + fringeSeed * 0.14;
    float fringe = sideFace * smoothstep(1.0 - fringeLength, 0.98, localY);
    float fringeBlade = smoothstep(0.38, 0.5, fract(faceUv.x * 10.0 + cellTone * 2.0));
    float fringeBreak = step(0.22, procHash(vec2(sideSegment, floor(localY * 7.0)) + cell.xz * 0.19));
    float fringeNotch = step(0.76, procHash(vec2(sideSegment, floor(localY * 11.0)) + cell.zx * 0.23));
    float fringeMask = fringe * mix(0.58, 1.0, fringeBlade) * mix(fringeBreak, 0.42, fringeNotch);
    float capCore = sideFace * smoothstep(0.67, 0.82, localY);
    float grassSide = max(capCore, fringeMask * 0.92);
    float dirtBody = sideFace * (1.0 - grassSide);
    float dirtNoise = procHash(floor(faceUv * vec2(5.0, 7.0)) + cell.xz * 0.41 + uProcSeed * 17.0);
    float dirtLayer = smoothstep(0.42, 0.52, fract(localY * 4.8 + cell.x * 0.09 + cell.z * 0.13));
    float dirtPocket = step(0.78, procHash(floor(faceUv * vec2(5.0, 4.0)) + cell.zx * 0.31 + uProcSeed * 7.0));
    float layerCrease = smoothstep(0.48, 0.52, fract(localY * 5.2 + cellTone * 0.25));
    vec3 dirtColor = mix(vec3(0.39, 0.24, 0.13), vec3(0.6, 0.4, 0.2), dirtNoise * 0.62 + dirtLayer * 0.24);
    dirtColor = mix(dirtColor, vec3(0.27, 0.17, 0.09), lowerSide * 0.34 + longCrack * 0.16 + dirtPocket * 0.18 + layerCrease * 0.1);
    vec3 sideGrassColor = mix(vec3(0.23, 0.49, 0.19), vec3(0.42, 0.67, 0.24), fringeBlade * 0.42 + cellTone * 0.22);
    float rootShadow = sideFace * smoothstep(0.52, 0.66, localY) * (1.0 - smoothstep(0.7, 0.86, localY));
    float rootFleck = step(0.64, procHash(vec2(sideSegment, floor(faceUv.y * 9.0)) + cell.xz * 0.27));
    float fringeShadow = rootShadow * (0.68 + rootFleck * 0.32);
    color = procTint(color, vec3(1.08, 1.14, 0.86), topFace * smoothstep(0.58, 0.92, colorPatch) * 0.12);
    color = procTint(color, vec3(0.78, 0.95, 0.68), topFace * (1.0 - smoothstep(0.2, 0.55, colorPatch)) * 0.08);
    color = procTint(color, vec3(1.16, 1.12, 0.72), topFace * bladeLine * 0.045);
    color = procBlend(color, dirtColor, dirtBody * 0.94);
    color = procBlend(color, sideGrassColor, grassSide * 0.84);
    color = procTint(color, vec3(0.4, 0.29, 0.16), fringeShadow * 0.28);
    color = procTint(color, vec3(0.76, 0.6, 0.36), lowerSide * 0.08);
  } else if (uProcStyle < 2.5) {
    float soilBand = smoothstep(0.18, 0.28, fract(world.y * 2.0 + cell.x * 0.07 + cell.z * 0.11));
    color = procTint(color, vec3(0.72, 0.58, 0.42), sideFace * soilBand * 0.18);
    color = procTint(color, vec3(1.08, 0.92, 0.68), sideFace * fineLayer * 0.08);
    color = procTint(color, vec3(1.15, 1.05, 0.82), topFace * step(0.84, fleck) * 0.08);
  } else if (uProcStyle < 3.5) {
    float dune = sin((world.x * 0.85 + world.z * 0.42 + uProcSeed * 9.0) * 3.14159);
    color = procTint(color, vec3(1.12, 1.04, 0.82), topFace * (dune * 0.5 + 0.5) * 0.08);
    color = procTint(color, vec3(1.16, 1.09, 0.78), topFace * windLine * 0.08);
    color = procTint(color, vec3(0.78, 0.64, 0.38), topFace * step(0.9, fleck) * 0.1);
    color = procTint(color, vec3(0.82, 0.68, 0.42), sideFace * broadLayer * 0.08);
    color = procTint(color, vec3(0.72, 0.58, 0.36), sideFace * sketchLine * 0.06);
  } else if (uProcStyle < 4.5) {
    float layer = smoothstep(0.44, 0.52, fract(world.y * 3.0 + cell.x * 0.05));
    float hairline = step(0.94, procHash(floor(faceUv * vec2(3.0, 7.0)) + cell.xz * 0.21));
    color = procTint(color, vec3(1.18, 1.05, 0.8), sideFace * layer * 0.18);
    color = procTint(color, vec3(0.66, 0.55, 0.4), sideFace * (1.0 - broadLayer) * 0.08);
    color = procTint(color, vec3(0.72, 0.62, 0.48), step(0.86, fleck) * 0.08 + sideFace * (longCrack + hairline) * 0.1);
    color = procTint(color, vec3(1.12, 0.98, 0.72), topFace * bevelHighlight * 0.08);
  } else if (uProcStyle < 5.5) {
    float mineral = step(0.92, procHash(floor(faceUv * 3.0) + cell.zy * 0.29));
    float lichen = topFace * step(0.86, procHash(floor(world.xz * 1.35) + cell.xz * 0.13));
    color = procTint(color, vec3(0.7, 0.76, 0.82), sideFace * broadLayer * 0.18);
    color = procTint(color, vec3(0.56, 0.6, 0.64), sideFace * fineLayer * 0.1);
    color = procTint(color, vec3(0.72, 0.76, 0.78), step(0.8, fleck) * 0.14 + sideFace * longCrack * 0.16 + mineral * 0.08);
    color = procTint(color, vec3(0.62, 0.82, 0.58), lichen * 0.09);
    color = procTint(color, vec3(1.18, 1.16, 1.08), topFace * step(fleck, 0.14) * 0.08);
  } else if (uProcStyle < 6.5) {
    float snowRidge = smoothstep(0.45, 0.5, fract((world.x * 0.34 - world.z * 0.52 + uProcSeed * 6.0) * 3.0));
    color = procTint(color, vec3(0.78, 0.93, 1.16), sideFace * (0.18 + broadLayer * 0.12) + step(0.9, fleck) * 0.08);
    color = procTint(color, vec3(1.1, 1.12, 1.08), topFace * (windLine + snowRidge * 0.7) * 0.1);
    color = procTint(color, vec3(1.08, 1.12, 1.12), topFace * edge * 0.18);
  } else if (uProcStyle < 7.5) {
    float bark = procHash(vec2(floor(faceUv.x * 4.0), cell.z + floor(world.y * 2.0)));
    float barkLine = smoothstep(0.42, 0.52, fract(faceUv.x * 6.0 + cellTone * 2.0));
    color = procTint(color, vec3(0.58, 0.42, 0.28), sideFace * (smoothstep(0.56, 0.94, bark) + barkLine * 0.45) * 0.18);
    color = procTint(color, vec3(1.18, 0.96, 0.68), topFace * step(0.8, fleck) * 0.1);
  } else if (uProcStyle < 8.5) {
    float leaf = procHash(floor(faceUv * 3.0) + cell.xy * 0.17 + cell.z);
    float leafVein = smoothstep(0.45, 0.5, fract((faceUv.x - faceUv.y + cellTone) * 3.0));
    color = procTint(color, vec3(0.72, 0.9, 0.62), smoothstep(0.64, 0.94, leaf) * 0.14);
    color = procTint(color, vec3(1.12, 1.12, 0.76), step(0.9, fleck) * 0.08);
    color = procTint(color, vec3(0.62, 0.82, 0.5), leafVein * 0.04);
  } else if (uProcStyle < 9.5) {
    float pebbleCell = procHash(floor(faceUv * 4.0) + cell.xz * 0.31);
    float pebbleEdge = smoothstep(0.36, 0.5, max(abs(fract(faceUv.x * 4.0) - 0.5), abs(fract(faceUv.y * 4.0) - 0.5)));
    color = procTint(color, vec3(0.72, 0.72, 0.68), pebbleEdge * 0.18);
    color = procTint(color, vec3(1.2, 1.16, 1.02), step(0.78, pebbleCell) * 0.1);
    color = procTint(color, vec3(0.56, 0.58, 0.58), step(pebbleCell, 0.18) * 0.12);
  } else if (uProcStyle < 10.5) {
    float deposit = smoothstep(0.38, 0.54, fract(world.y * 2.6 + cell.x * 0.06 - cell.z * 0.04));
    float crack = step(0.92, procHash(floor(faceUv * vec2(5.0, 3.0)) + cell.xz * 0.25));
    color = procTint(color, vec3(1.16, 0.92, 0.76), sideFace * deposit * 0.18 + topFace * step(0.82, fleck) * 0.08);
    color = procTint(color, vec3(0.62, 0.48, 0.42), crack * 0.14 + lowerSide * 0.08);
  } else if (uProcStyle < 11.5) {
    float wetPatch = smoothstep(0.44, 0.76, procHash(floor(world.xz * 2.0) + cell.xz * 0.39));
    float slick = smoothstep(0.48, 0.52, fract((world.x * 0.37 + world.z * 0.61 + uProcSeed * 8.0) * 2.0));
    color = procTint(color, vec3(0.48, 0.38, 0.28), wetPatch * 0.26 + lowerSide * 0.12);
    color = procTint(color, vec3(1.12, 0.98, 0.78), topFace * slick * wetPatch * 0.08);
  } else if (uProcStyle < 12.5) {
    float dryCrack = step(0.86, procHash(floor(faceUv * vec2(4.0, 4.0)) + cell.xz * 0.29));
    float paleDust = smoothstep(0.56, 0.92, fleck);
    color = procTint(color, vec3(1.2, 0.98, 0.62), topFace * paleDust * 0.14);
    color = procTint(color, vec3(0.54, 0.38, 0.24), dryCrack * 0.16 + sideFace * broadLayer * 0.1);
  } else if (uProcStyle < 13.5) {
    float saltPlate = procEdge(faceUv * 0.42 + cell.xz * 0.03, 0.065);
    float chalk = step(0.72, procHash(floor(faceUv * 3.0) + cell.xz * 0.17));
    color = procTint(color, vec3(1.16, 1.14, 1.02), topFace * (0.18 + chalk * 0.1));
    color = procTint(color, vec3(0.72, 0.68, 0.56), saltPlate * 0.22 + sideFace * fineLayer * 0.08);
  } else if (uProcStyle < 14.5) {
    float trappedAir = step(0.88, procHash(floor(faceUv * 4.0) + cell.zx * 0.22));
    float iceLine = smoothstep(0.46, 0.5, fract((faceUv.x * 0.68 - faceUv.y * 0.24 + uProcSeed * 5.0) * 4.0));
    color = procTint(color, vec3(0.62, 0.9, 1.22), 0.24 + sideFace * 0.12);
    color = procTint(color, vec3(1.28, 1.28, 1.22), topFace * bevelHighlight * 0.18 + trappedAir * 0.08);
    color = procTint(color, vec3(0.56, 0.78, 1.18), iceLine * 0.08);
  } else if (uProcStyle < 15.5) {
    float frozenLayer = smoothstep(0.42, 0.52, fract(world.y * 3.4 + cell.x * 0.05 + cell.z * 0.07));
    float frost = step(0.82, procHash(floor(faceUv * 3.0) + cell.xz * 0.33));
    color = procTint(color, vec3(0.66, 0.82, 0.9), sideFace * frozenLayer * 0.18 + frost * 0.08);
    color = procTint(color, vec3(1.1, 1.12, 1.05), topFace * step(0.84, fleck) * 0.08);
  } else if (uProcStyle < 16.5) {
    float column = smoothstep(0.42, 0.5, fract((faceUv.x + cellTone * 0.18) * 2.0));
    float vesicle = step(0.88, procHash(floor(faceUv * 5.0) + cell.xz * 0.41));
    color = procTint(color, vec3(0.48, 0.5, 0.52), column * sideFace * 0.14 + vesicle * 0.08);
    color = procTint(color, vec3(1.14, 1.04, 0.88), topFace * step(0.9, fleck) * 0.06);
  } else if (uProcStyle < 17.5) {
    float powder = smoothstep(0.44, 0.86, procHash(floor(world.xz * 3.0) + cell.xz * 0.12));
    color = procTint(color, vec3(1.12, 1.08, 1.0), topFace * powder * 0.2);
    color = procTint(color, vec3(0.62, 0.58, 0.54), sideFace * broadLayer * 0.18 + lowerSide * 0.08);
  } else if (uProcStyle < 18.5) {
    float swirl = smoothstep(0.45, 0.5, fract((world.x * 0.28 + world.z * 0.44 + uProcSeed * 9.0) * 3.0));
    color = procTint(color, vec3(0.78, 0.66, 0.42), 0.12 + topFace * swirl * 0.14);
    color = procTint(color, vec3(1.12, 0.96, 0.64), topFace * windLine * 0.08);
    color = procTint(color, vec3(0.52, 0.42, 0.26), sideFace * lowerSide * 0.12);
  } else if (uProcStyle < 19.5) {
    float deepVein = smoothstep(0.46, 0.5, fract((faceUv.x * 0.42 + faceUv.y * 0.73 + cellTone) * 3.0));
    float glimmer = step(0.965, procHash(floor(faceUv * 4.0) + cell.xy * 0.19 + cell.z));
    color = procTint(color, vec3(0.56, 0.62, 0.76), sideFace * broadLayer * 0.2 + deepVein * 0.1);
    color = procTint(color, vec3(1.16, 1.2, 1.18), glimmer * 0.08);
    color = procTint(color, vec3(0.46, 0.48, 0.56), lowerSide * 0.12);
  } else if (uProcStyle < 20.5) {
    float hardEdge = procEdge(faceUv * 0.68 + cell.xz * 0.02, 0.052);
    float blackFleck = step(0.84, procHash(floor(faceUv * 4.0) + cell.xz * 0.44));
    color = procTint(color, vec3(0.38, 0.38, 0.44), hardEdge * 0.18 + blackFleck * 0.1);
    color = procTint(color, vec3(0.82, 0.82, 0.9), topFace * bevelHighlight * 0.06);
  } else {
    float seam = smoothstep(0.43, 0.52, fract((faceUv.x * 0.38 + faceUv.y * 0.71 + cellTone) * 3.0));
    float gloss = step(0.94, procHash(floor(faceUv * 5.0) + cell.xz * 0.37 + cell.y * 0.19));
    float fracture = step(0.9, procHash(floor(faceUv * vec2(3.0, 6.0)) + cell.zy * 0.23));
    color = procTint(color, vec3(0.42, 0.4, 0.38), seam * 0.18 + sideFace * broadLayer * 0.12);
    color = procTint(color, vec3(1.32, 1.24, 1.08), gloss * 0.16 + topFace * bevelHighlight * 0.08);
    color = procTint(color, vec3(0.24, 0.22, 0.2), fracture * 0.2 + lowerSide * 0.16);
  }

  return color;
}

void main() {
      `,
    );
    shader.fragmentShader = shader.fragmentShader.replace(
      "#include <color_fragment>",
      `
#include <color_fragment>
diffuseColor.rgb = applyProcVoxelDetail(diffuseColor.rgb);
      `,
    );
  };
  return material;
}

export function applyWaterMaterialDetail(material, seed = 0) {
  const uniforms = {
    uWaterSeed: { value: normalizeSeed(seed) },
    uWaterTime: { value: 0 },
  };

  material.userData.proceduralUniforms = uniforms;
  material.onBeforeCompile = (shader) => {
    shader.uniforms.uWaterSeed = uniforms.uWaterSeed;
    shader.uniforms.uWaterTime = uniforms.uWaterTime;
    shader.vertexShader = shader.vertexShader.replace(
      "void main() {",
      `
varying vec3 vWaterWorldPosition;

void main() {
      `,
    );
    shader.vertexShader = shader.vertexShader.replace(
      "#include <begin_vertex>",
      `
#include <begin_vertex>
vec4 waterWorldPosition = vec4(transformed, 1.0);
#ifdef USE_INSTANCING
  waterWorldPosition = instanceMatrix * waterWorldPosition;
#endif
waterWorldPosition = modelMatrix * waterWorldPosition;
vWaterWorldPosition = waterWorldPosition.xyz;
      `,
    );
    shader.fragmentShader = shader.fragmentShader.replace(
      "void main() {",
      `
uniform float uWaterSeed;
uniform float uWaterTime;
varying vec3 vWaterWorldPosition;

float waterHash(vec2 p) {
  vec3 p3 = fract(vec3(p.xyx) * 0.1031);
  p3 += dot(p3, p3.yzx + 31.37 + uWaterSeed * 11.0);
  return fract((p3.x + p3.y) * p3.z);
}

void main() {
      `,
    );
    shader.fragmentShader = shader.fragmentShader.replace(
      "#include <color_fragment>",
      `
#include <color_fragment>
float rippleA = sin((vWaterWorldPosition.x * 0.72 + vWaterWorldPosition.z * 1.17 + uWaterSeed * 7.0 + uWaterTime * 0.42) * 3.14159) * 0.5 + 0.5;
float rippleB = sin((vWaterWorldPosition.x * -0.38 + vWaterWorldPosition.z * 0.84 + uWaterSeed * 11.0 - uWaterTime * 0.28) * 3.14159) * 0.5 + 0.5;
float ripple = mix(rippleA, rippleB, 0.18);
float drift = waterHash(floor(vWaterWorldPosition.xz * 1.25 + vec2(uWaterTime * 0.05, -uWaterTime * 0.04)));
float glint = step(0.985, drift) * smoothstep(0.76, 0.98, ripple);
vec3 baseWater = diffuseColor.rgb;
vec3 depthTint = mix(baseWater * vec3(0.72, 0.86, 1.06), baseWater * vec3(0.92, 1.06, 0.92), smoothstep(0.22, 0.72, baseWater.g));
diffuseColor.rgb = mix(baseWater, depthTint, 0.34);
diffuseColor.rgb = mix(diffuseColor.rgb * vec3(0.9, 0.98, 1.08), diffuseColor.rgb * vec3(1.08, 1.08, 1.0), ripple * 0.035 + glint * 0.018);
diffuseColor.a *= 0.98 + ripple * 0.015;
      `,
    );
  };
  return material;
}

export function updateProceduralMaterialTime(materials, time) {
  for (const material of Object.values(materials)) {
    const uniforms = material?.userData?.proceduralUniforms;
    if (uniforms?.uWaterTime) uniforms.uWaterTime.value = time;
  }
}

export function updateProceduralMaterialSeed(materials, seed) {
  const value = normalizeSeed(seed);
  for (const material of Object.values(materials)) {
    const uniforms = material?.userData?.proceduralUniforms;
    if (!uniforms) continue;
    if (uniforms.uProcSeed) uniforms.uProcSeed.value = value;
    if (uniforms.uWaterSeed) uniforms.uWaterSeed.value = value;
  }
}

function normalizeSeed(seed) {
  const value = Number(seed) || 0;
  return (Math.abs(value) % 100000) / 100000;
}
