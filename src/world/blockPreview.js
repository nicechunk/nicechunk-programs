import * as THREE from "three";
import { createWorldGeometryByType, createWorldMaterials } from "./rendering.js";

let previewState = null;

export function createBlockPreviewCanvas(blockType, { className = "block-preview-canvas", size = 112 } = {}) {
  const canvas = document.createElement("canvas");
  canvas.className = className;
  canvas.width = size;
  canvas.height = size;
  canvas.setAttribute("aria-hidden", "true");
  renderBlockPreviewToCanvas(canvas, blockType, { size });
  return canvas;
}

export function renderBlockPreviewToCanvas(canvas, blockType, { size = 112, pixelRatio = 2 } = {}) {
  const state = getPreviewState();
  const targetSize = Math.max(48, Math.round(size * pixelRatio));
  const outputContext = canvas.getContext("2d");
  if (!outputContext) return false;

  canvas.width = targetSize;
  canvas.height = targetSize;
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
  if (geometry === state.waterGeometry) {
    mesh.scale.setScalar(1.18);
  }
  state.scene.add(mesh);
  state.mesh = mesh;

  state.renderer.render(state.scene, state.camera);
  outputContext.clearRect(0, 0, targetSize, targetSize);
  outputContext.drawImage(state.renderer.domElement, 0, 0, targetSize, targetSize);
  return true;
}

function getPreviewState() {
  if (previewState) return previewState;

  const renderer = new THREE.WebGLRenderer({
    alpha: true,
    antialias: true,
    preserveDrawingBuffer: true,
  });
  renderer.setClearColor(0x000000, 0);
  renderer.shadowMap.enabled = false;

  const scene = new THREE.Scene();
  const camera = new THREE.PerspectiveCamera(32, 1, 0.1, 20);
  camera.position.set(2.45, 1.9, 2.7);
  camera.lookAt(0, 0, 0);

  scene.add(new THREE.HemisphereLight(0xf4fbff, 0x526044, 2.8));
  const keyLight = new THREE.DirectionalLight(0xfff2bf, 2.2);
  keyLight.position.set(-3.4, 4.2, 3.8);
  scene.add(keyLight);
  const fillLight = new THREE.DirectionalLight(0x7be8ff, 0.5);
  fillLight.position.set(3, 2, -2);
  scene.add(fillLight);

  const cubeGeometry = new THREE.BoxGeometry(1, 1, 1);
  const waterGeometry = new THREE.PlaneGeometry(1, 1);
  waterGeometry.rotateX(-Math.PI / 2);
  const geometryByType = createWorldGeometryByType({ THREE, cubeGeometry, waterGeometry });
  const materials = createWorldMaterials({ THREE });

  previewState = {
    renderer,
    scene,
    camera,
    cubeGeometry,
    waterGeometry,
    geometryByType,
    materials,
    mesh: null,
  };
  return previewState;
}
