import * as THREE from "three";
import { blockAtlas } from "../data/blockAtlas.js";
import { applyVoxelMaterialDetail, materialStyles } from "./proceduralMaterials.js";
import { createWorldGeometryByType, createWorldMaterials } from "../world/rendering.js";

let blockPreviewState = null;
let materialPreviewState = null;
const blockPreviewSnapshots = new Map();
const materialPreviewSnapshots = new Map();
const maxPreviewSnapshots = 180;

export function createResourceBlockPreviewCanvas(blockType, { className = "block-preview-canvas", size = 112, previewDimensionsM = null } = {}) {
  const canvas = document.createElement("canvas");
  canvas.className = className;
  canvas.width = size;
  canvas.height = size;
  canvas.setAttribute("aria-hidden", "true");
  renderResourceBlockPreviewToCanvas(canvas, blockType, { size, previewDimensionsM });
  return canvas;
}

export function renderResourceBlockPreviewToCanvas(canvas, blockType, { size = 112, pixelRatio = 2, previewDimensionsM = null } = {}) {
  const targetSize = Math.max(48, Math.round(size * pixelRatio));
  const outputContext = canvas.getContext("2d");
  if (!outputContext) return false;

  canvas.width = targetSize;
  canvas.height = targetSize;
  const previewScale = previewScaleVector(previewDimensionsM);
  const snapshotKey = `block:${blockType ?? "stone"}:${targetSize}:${previewScaleKey(previewScale)}`;
  if (drawPreviewSnapshot(outputContext, blockPreviewSnapshots, snapshotKey, targetSize)) return true;

  const state = getResourceBlockPreviewState();
  state.renderer.setSize(targetSize, targetSize, false);
  state.camera.aspect = 1;
  state.camera.updateProjectionMatrix();

  if (state.mesh) {
    state.scene.remove(state.mesh);
    state.mesh = null;
  }

  const material = state.materials[blockType] ?? state.materials.stone;
  const geometry = state.geometryByType[blockType] ?? state.geometryByType.stone;
  const mesh = new THREE.Mesh(geometry, material);
  mesh.castShadow = true;
  mesh.receiveShadow = true;
  mesh.rotation.y = Math.PI * 0.22;
  if (geometry === state.waterGeometry) mesh.scale.setScalar(1.18);
  mesh.scale.multiply(previewScale);
  state.scene.add(mesh);
  state.mesh = mesh;

  state.renderer.render(state.scene, state.camera);
  outputContext.clearRect(0, 0, targetSize, targetSize);
  outputContext.drawImage(state.renderer.domElement, 0, 0, targetSize, targetSize);
  storePreviewSnapshot(blockPreviewSnapshots, snapshotKey, state.renderer.domElement, targetSize);
  return true;
}

export function createResourceMaterialPreviewCanvas(recipe, { className = "material-preview-canvas", size = 112, previewDimensionsM = null } = {}) {
  const canvas = document.createElement("canvas");
  canvas.className = className;
  canvas.width = size;
  canvas.height = size;
  canvas.setAttribute("aria-hidden", "true");
  renderResourceMaterialPreviewToCanvas(canvas, recipe, { size, previewDimensionsM });
  return canvas;
}

export function renderResourceMaterialPreviewToCanvas(canvas, recipe, { size = 112, pixelRatio = 2, previewDimensionsM = null } = {}) {
  const targetSize = Math.max(48, Math.round(size * pixelRatio));
  const outputContext = canvas.getContext("2d");
  if (!outputContext) return false;

  canvas.width = targetSize;
  canvas.height = targetSize;
  const previewScale = previewScaleVector(previewDimensionsM);
  const snapshotKey = `material:${materialPreviewCacheKey(recipe)}:${targetSize}:${previewScaleKey(previewScale)}`;
  if (drawPreviewSnapshot(outputContext, materialPreviewSnapshots, snapshotKey, targetSize)) return true;

  const state = getResourceMaterialPreviewState();
  state.renderer.setSize(targetSize, targetSize, false);
  state.camera.aspect = 1;
  state.camera.updateProjectionMatrix();

  if (state.model) {
    state.scene.remove(state.model);
    disposeThreeObject(state.model);
    state.model = null;
  }

  const model = createResourceMaterialPreviewObject(recipe, state, { previewScale });
  state.scene.add(model);
  state.model = model;
  state.renderer.render(state.scene, state.camera);
  outputContext.clearRect(0, 0, targetSize, targetSize);
  outputContext.drawImage(state.renderer.domElement, 0, 0, targetSize, targetSize);
  storePreviewSnapshot(materialPreviewSnapshots, snapshotKey, state.renderer.domElement, targetSize);
  return true;
}

export function createResourceMaterialPreviewObject(recipe, state = null, { previewScale = null } = {}) {
  const previewState = state ?? getResourceMaterialPreviewState();
  const colors = resourceMaterialColors(recipe);
  const patternTexture = createMaterialPatternTexture(recipe, colors[0]);
  const scale = previewScale ?? previewScaleVector(null);
  const group = new THREE.Group();
  group.rotation.y = Math.PI * 0.22;

  const transparent = recipe?.class === "glass" || recipe?.class === "crystal";
  const material = gameVoxelMaterial(colors[0], recipe, {
    map: patternTexture,
    opacity: transparent ? 0.72 : 1,
    style: transparent ? "ice" : undefined,
  });
  const mesh = new THREE.Mesh(previewState.boxGeometry, material);
  mesh.name = `material-preview:${recipe?.id ?? "custom"}`;
  mesh.scale.set(0.96 * scale.x, 0.96 * scale.y, 0.96 * scale.z);
  group.add(mesh);
  return group;
}

export function resourceMaterialColors(recipe) {
  const composition = resourceMaterialComposition(recipe);
  const fallbackColor = normalizeHexColor(recipe?.color) ?? "#8eeeff";
  if (!composition.length) {
    return [
      fallbackColor,
      mixHexColors(fallbackColor, "#ffffff", 0.18),
      mixHexColors(fallbackColor, "#111827", 0.14),
    ];
  }
  const blendedColor = blendCompositionColor(composition);
  const dominantColor = elementColor(composition[0]?.[0] ?? "O");
  const secondaryColor = elementColor(composition[1]?.[0] ?? composition[0]?.[0] ?? "O");
  return [
    blendedColor,
    mixHexColors(blendedColor, dominantColor, 0.34),
    mixHexColors(blendedColor, secondaryColor, 0.24),
  ];
}

export function resourceMaterialComposition(recipe) {
  if (Array.isArray(recipe?.composition) && recipe.composition.length) return recipe.composition;
  return (recipe?.rawInputs ?? []).flatMap((input) => blockCompositionByKey(input.key)).slice(0, 5);
}

export function elementColor(symbol) {
  const colors = {
    Al: "#9fc3d9",
    C: "#2c2c32",
    Ca: "#d7d0b2",
    Cl: "#b5f46c",
    Cu: "#d88748",
    Fe: "#a66b5b",
    H: "#d7f7ff",
    K: "#b981ff",
    Mg: "#d3e4c7",
    Mn: "#b48a92",
    N: "#72a8ff",
    Na: "#f2d36b",
    Ni: "#9bbf9f",
    O: "#78d8ff",
    S: "#f0da55",
    Si: "#c8b07a",
  };
  return colors[symbol] ?? "#8eeeff";
}

export function disposeThreeObject(object) {
  object?.traverse?.((child) => {
    if (!child.isMesh) return;
    if (Array.isArray(child.material)) {
      child.material.forEach(disposeThreeMaterial);
    } else {
      disposeThreeMaterial(child.material);
    }
  });
}

function getResourceBlockPreviewState() {
  if (blockPreviewState) return blockPreviewState;
  const renderer = createPreviewRenderer();
  const scene = createPreviewScene();
  const camera = createPreviewCamera();
  const cubeGeometry = new THREE.BoxGeometry(1, 1, 1);
  const waterGeometry = new THREE.PlaneGeometry(1, 1);
  waterGeometry.rotateX(-Math.PI / 2);
  blockPreviewState = {
    renderer,
    scene,
    camera,
    cubeGeometry,
    waterGeometry,
    geometryByType: createWorldGeometryByType({ THREE, cubeGeometry, waterGeometry }),
    materials: createWorldMaterials({ THREE }),
    mesh: null,
  };
  return blockPreviewState;
}

function getResourceMaterialPreviewState() {
  if (materialPreviewState) return materialPreviewState;
  materialPreviewState = {
    renderer: createPreviewRenderer(),
    scene: createPreviewScene(),
    camera: createPreviewCamera(),
    boxGeometry: new THREE.BoxGeometry(1, 1, 1),
    model: null,
  };
  return materialPreviewState;
}

function createPreviewRenderer() {
  const renderer = new THREE.WebGLRenderer({
    alpha: true,
    antialias: true,
    preserveDrawingBuffer: true,
  });
  renderer.setClearColor(0x000000, 0);
  renderer.shadowMap.enabled = false;
  return renderer;
}

function createPreviewScene() {
  const scene = new THREE.Scene();
  scene.add(new THREE.HemisphereLight(0xf4fbff, 0x526044, 2.8));
  const keyLight = new THREE.DirectionalLight(0xfff2bf, 2.2);
  keyLight.position.set(-3.4, 4.2, 3.8);
  scene.add(keyLight);
  const fillLight = new THREE.DirectionalLight(0x7be8ff, 0.5);
  fillLight.position.set(3, 2, -2);
  scene.add(fillLight);
  return scene;
}

function createPreviewCamera() {
  const camera = new THREE.PerspectiveCamera(32, 1, 0.1, 20);
  camera.position.set(2.45, 1.9, 2.7);
  camera.lookAt(0, 0, 0);
  return camera;
}

function previewScaleVector(dimensionsM) {
  const width = positivePreviewDimension(dimensionsM?.width);
  const height = positivePreviewDimension(dimensionsM?.height);
  const depth = positivePreviewDimension(dimensionsM?.depth);
  if (!width || !height || !depth) return new THREE.Vector3(1, 1, 1);
  const referenceM = 0.1;
  const raw = new THREE.Vector3(width / referenceM, height / referenceM, depth / referenceM);
  const maxAxis = Math.max(raw.x, raw.y, raw.z);
  const minVisibleAxis = 0.22;
  const maxVisibleAxis = 1.34;
  const fit = maxAxis > maxVisibleAxis ? maxVisibleAxis / maxAxis : 1;
  raw.multiplyScalar(fit);
  raw.x = Math.max(minVisibleAxis, raw.x);
  raw.y = Math.max(minVisibleAxis, raw.y);
  raw.z = Math.max(minVisibleAxis, raw.z);
  return raw;
}

function previewScaleKey(scale) {
  return [scale.x, scale.y, scale.z].map((value) => value.toFixed(4)).join(",");
}

function positivePreviewDimension(value) {
  const number = Number(value);
  return Number.isFinite(number) && number > 0 ? number : null;
}

function drawPreviewSnapshot(outputContext, snapshots, snapshotKey, targetSize) {
  const snapshot = snapshots.get(snapshotKey);
  if (!snapshot) return false;
  outputContext.clearRect(0, 0, targetSize, targetSize);
  outputContext.drawImage(snapshot, 0, 0, targetSize, targetSize);
  return true;
}

function storePreviewSnapshot(snapshots, snapshotKey, sourceCanvas, targetSize) {
  const snapshot = document.createElement("canvas");
  snapshot.width = targetSize;
  snapshot.height = targetSize;
  const context = snapshot.getContext("2d");
  if (!context) return;
  context.drawImage(sourceCanvas, 0, 0, targetSize, targetSize);
  snapshots.set(snapshotKey, snapshot);
  trimPreviewSnapshotCache(snapshots);
}

function trimPreviewSnapshotCache(snapshots) {
  while (snapshots.size > maxPreviewSnapshots) {
    const firstKey = snapshots.keys().next().value;
    snapshots.delete(firstKey);
  }
}

function materialPreviewCacheKey(recipe) {
  const composition = resourceMaterialComposition(recipe)
    .map(([symbol, range]) => `${symbol}:${range}`)
    .join(",");
  const rawInputs = (recipe?.rawInputs ?? [])
    .map((input) => `${input.key ?? ""}:${input.amount ?? ""}`)
    .join(",");
  const properties = recipe?.properties
    ? JSON.stringify(recipe.properties)
    : "";
  return [
    recipe?.id ?? "custom",
    recipe?.class ?? "",
    normalizeHexColor(recipe?.color) ?? "",
    composition,
    rawInputs,
    properties,
  ].join("|");
}

function createMaterialPatternTexture(recipe, baseColor) {
  const composition = resourceMaterialComposition(recipe);
  const canvas = document.createElement("canvas");
  canvas.width = 96;
  canvas.height = 96;
  const context = canvas.getContext("2d");
  if (!context) return null;

  context.fillStyle = baseColor;
  context.fillRect(0, 0, canvas.width, canvas.height);
  const random = seededRandom(`${recipe?.id ?? "custom"}:material-pattern`);
  composition.slice(0, 7).forEach(([symbol, range], index) => {
    const weight = Math.max(0.05, Math.min(0.9, compositionRangeMidpoint(range) / 100));
    const rgb = hexToRgb(elementColor(symbol));
    const patchCount = Math.max(2, Math.round(2 + weight * 7));
    for (let patchIndex = 0; patchIndex < patchCount; patchIndex++) {
      const x = random() * canvas.width;
      const y = random() * canvas.height;
      const radius = canvas.width * (0.08 + random() * 0.18 + weight * 0.08);
      const gradient = context.createRadialGradient(x, y, radius * 0.08, x, y, radius);
      gradient.addColorStop(0, `rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, ${0.34 + weight * 0.22})`);
      gradient.addColorStop(0.62, `rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, ${0.11 + weight * 0.12})`);
      gradient.addColorStop(1, `rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, 0)`);
      context.fillStyle = gradient;
      context.beginPath();
      context.arc(x, y, radius, 0, Math.PI * 2);
      context.fill();
    }

    context.strokeStyle = `rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, ${0.12 + weight * 0.14})`;
    context.lineWidth = 2.2 + weight * 4;
    context.lineCap = "round";
    context.lineJoin = "round";
    for (let strokeIndex = 0; strokeIndex < 2; strokeIndex++) {
      const startX = random() * canvas.width;
      const startY = random() * canvas.height;
      context.beginPath();
      context.moveTo(startX, startY);
      context.bezierCurveTo(
        startX + (random() - 0.5) * 70,
        startY + (random() - 0.5) * 70,
        random() * canvas.width,
        random() * canvas.height,
        random() * canvas.width,
        random() * canvas.height,
      );
      context.stroke();
    }

    if (index === 0) {
      context.fillStyle = `rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, ${0.08 + weight * 0.08})`;
      context.fillRect(0, 0, canvas.width, canvas.height);
    }
  });

  for (let index = 0; index < 90; index++) {
    const alpha = 0.035 + random() * 0.045;
    context.fillStyle = `rgba(255, 255, 255, ${alpha})`;
    context.fillRect(random() * canvas.width, random() * canvas.height, 1 + random() * 2, 1 + random() * 2);
  }

  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  texture.wrapS = THREE.ClampToEdgeWrapping;
  texture.wrapT = THREE.ClampToEdgeWrapping;
  texture.needsUpdate = true;
  return texture;
}

function gameVoxelMaterial(color, recipe, options = {}) {
  const transparent = Number.isFinite(options.opacity) && options.opacity < 1;
  const material = new THREE.MeshLambertMaterial({
    color: options.map ? 0xffffff : color,
    map: options.map ?? null,
    transparent,
    opacity: transparent ? options.opacity : 1,
    depthWrite: !transparent,
  });
  material.fog = true;
  const style = materialStyles[options.style ?? materialStyleKey(recipe)];
  if (style) applyVoxelMaterialDetail(material, style, materialPreviewSeed(recipe?.id ?? "custom"));
  return material;
}

function materialStyleKey(recipe) {
  const classStyles = {
    alloy: "deepStone",
    carbon: "coal",
    ceramic: "clay",
    chemical: "saltFlat",
    composite: "trunk",
    crystal: "ice",
    fiber: "trunk",
    glass: "ice",
    metal: "stone",
    polymer: "mud",
  };
  return classStyles[recipe?.class] ?? "stone";
}

function blockCompositionByKey(key) {
  const entry = blockAtlas.find((item) => item.key === key);
  return entry?.composition ?? [];
}

function blendCompositionColor(composition) {
  let totalWeight = 0;
  const mixed = { r: 0, g: 0, b: 0 };
  composition.forEach(([symbol, range]) => {
    const weight = compositionRangeMidpoint(range);
    if (!Number.isFinite(weight) || weight <= 0) return;
    const color = hexToRgb(elementColor(symbol));
    mixed.r += color.r * weight;
    mixed.g += color.g * weight;
    mixed.b += color.b * weight;
    totalWeight += weight;
  });
  if (totalWeight <= 0) return "#8eeeff";
  return rgbToHex({
    r: mixed.r / totalWeight,
    g: mixed.g / totalWeight,
    b: mixed.b / totalWeight,
  });
}

function compositionRangeMidpoint(range) {
  const values = String(range ?? "").match(/\d+(?:\.\d+)?/g)?.map(Number).filter(Number.isFinite) ?? [];
  if (!values.length) return 0;
  if (values.length === 1) return values[0];
  return (values[0] + values[1]) / 2;
}

function mixHexColors(base, overlay, overlayWeight) {
  const a = hexToRgb(base);
  const b = hexToRgb(overlay);
  const weight = Math.max(0, Math.min(1, overlayWeight));
  return rgbToHex({
    r: a.r * (1 - weight) + b.r * weight,
    g: a.g * (1 - weight) + b.g * weight,
    b: a.b * (1 - weight) + b.b * weight,
  });
}

function hexToRgb(hex) {
  const value = Number.parseInt(String(hex).replace("#", ""), 16);
  if (!Number.isFinite(value)) return { r: 142, g: 238, b: 255 };
  return {
    r: (value >> 16) & 255,
    g: (value >> 8) & 255,
    b: value & 255,
  };
}

function normalizeHexColor(color) {
  const value = String(color ?? "").trim();
  if (/^#[0-9a-f]{6}$/i.test(value)) return value;
  if (/^[0-9a-f]{6}$/i.test(value)) return `#${value}`;
  return null;
}

function rgbToHex({ r, g, b }) {
  const value = [r, g, b]
    .map((channel) => Math.max(0, Math.min(255, Math.round(channel))).toString(16).padStart(2, "0"))
    .join("");
  return `#${value}`;
}

function materialPreviewSeed(value = "") {
  let hash = 2166136261;
  for (let index = 0; index < value.length; index++) {
    hash ^= value.charCodeAt(index);
    hash = Math.imul(hash, 16777619);
  }
  return (hash >>> 0) / 4294967295;
}

function seededRandom(seedText) {
  let state = 2166136261;
  for (let index = 0; index < seedText.length; index++) {
    state ^= seedText.charCodeAt(index);
    state = Math.imul(state, 16777619);
  }
  return () => {
    state += 0x6d2b79f5;
    let value = state;
    value = Math.imul(value ^ (value >>> 15), value | 1);
    value ^= value + Math.imul(value ^ (value >>> 7), value | 61);
    return ((value ^ (value >>> 14)) >>> 0) / 4294967296;
  };
}

function disposeThreeMaterial(material) {
  if (!material) return;
  material.map?.dispose?.();
  material.dispose?.();
}
