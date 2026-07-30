import {
  ArcRotateCamera,
  BackgroundMaterial,
  Color4,
  DirectionalLight,
  Engine,
  ImportMeshAsync,
  MeshBuilder,
  RenderTargetTexture,
  Scene,
  SceneLoader,
  ShadowGenerator,
  Vector3,
} from "@babylonjs/core";
import { FilesInputStore } from "@babylonjs/core/Misc/filesInputStore";
import "@babylonjs/loaders/glTF";

const MODEL_FILE = "CarConcept.glb";
const TARGET_SIZE = 4;

function importMesh(rootUrl, sceneFilename, scene) {
  return new Promise((resolve, reject) => {
    SceneLoader.ImportMesh(
      "",
      rootUrl,
      sceneFilename,
      scene,
      (meshes) => resolve({ meshes }),
      null,
      (_scene, message) => reject(new Error(message)),
    );
  });
}

export async function createBabylonScene(canvas) {
  const engine = new Engine(
    canvas,
    true,
    { preserveDrawingBuffer: true, stencil: true },
    false,
  );
  const scene = new Scene(engine);
  scene.clearColor = new Color4(0.028, 0.035, 0.047, 1);

  const camera = new ArcRotateCamera(
    "camera",
    -Math.PI / 2 - Math.PI / 6,
    1.05,
    TARGET_SIZE * 1.9,
    new Vector3(0, TARGET_SIZE * 0.35, 0),
    scene,
  );
  camera.attachControl(canvas, true);
  camera.lowerRadiusLimit = TARGET_SIZE * 1.1;
  camera.upperRadiusLimit = TARGET_SIZE * 4;
  camera.lowerBetaLimit = 0.35;
  camera.upperBetaLimit = 1.45;
  camera.wheelDeltaPercentage = 0.01;
  camera.pinchDeltaPercentage = 0.01;
  camera.useAutoRotationBehavior = true;
  camera.autoRotationBehavior.idleRotationSpeed = 0.12;
  camera.autoRotationBehavior.idleRotationWaitTime = 2000;

  scene.createDefaultEnvironment({
    createGround: false,
    createSkybox: false,
    environmentTexture: new URL("./environmentSpecular.env", document.baseURI).href,
  });
  scene.environmentIntensity = 0.8;

  const key = new DirectionalLight("key", new Vector3(-0.4, -1, -0.35), scene);
  key.position = new Vector3(4, 10, 4);
  key.intensity = 0.9;

  const shadowGenerator = new ShadowGenerator(1024, key);
  shadowGenerator.useBlurExponentialShadowMap = true;
  shadowGenerator.blurKernel = 32;
  shadowGenerator.darkness = 0.35;
  shadowGenerator.getShadowMap().refreshRate = RenderTargetTexture.REFRESHRATE_RENDER_ONCE;

  const ground = MeshBuilder.CreateGround(
    "ground",
    { width: TARGET_SIZE * 6, height: TARGET_SIZE * 6 },
    scene,
  );
  const shadowMaterial = new BackgroundMaterial("shadowOnly", scene);
  shadowMaterial.shadowOnly = true;
  ground.material = shadowMaterial;
  ground.receiveShadow = true;

  let modelMeshes = [];

  function applyModel(meshes) {
    for (const mesh of modelMeshes) mesh.dispose();
    modelMeshes = meshes;
    const root = meshes[0];
    const bounds = root.getHierarchyBoundingVectors(true);
    const size = bounds.max.subtract(bounds.min);
    const maxDimension = Math.max(size.x, size.y, size.z) || 1;
    root.scaling.scaleInPlace(TARGET_SIZE / maxDimension);
    const normalizedBounds = root.getHierarchyBoundingVectors(true);
    const center = normalizedBounds.max.add(normalizedBounds.min).scale(0.5);
    root.position.subtractInPlace(
      new Vector3(center.x, normalizedBounds.min.y, center.z),
    );
    camera.target.y = (normalizedBounds.max.y - normalizedBounds.min.y) * 0.45;

    for (const mesh of meshes) {
      if (mesh.getTotalVertices() > 0) shadowGenerator.addShadowCaster(mesh);
    }
    shadowGenerator.getShadowMap().resetRefreshCounter();
  }

  async function loadDefault() {
    const result = await ImportMeshAsync(MODEL_FILE, scene);
    applyModel(result.meshes);
  }

  async function loadFiles(files) {
    const list = [...files];
    const model = list.find((file) => /\.(glb|gltf)$/i.test(file.name));
    if (!model) throw new Error("no .glb or .gltf file in selection");

    for (const file of list) {
      FilesInputStore.FilesToLoad[file.name.toLowerCase()] = file;
    }
    try {
      const result = await importMesh("file:", model.name, scene);
      applyModel(result.meshes);
    } finally {
      for (const file of list) {
        delete FilesInputStore.FilesToLoad[file.name.toLowerCase()];
      }
    }
  }

  function dispose() {
    engine.stopRenderLoop();
    scene.dispose();
    engine.dispose();
    modelMeshes = [];
  }

  await loadDefault();
  return { engine, scene, camera, loadDefault, loadFiles, dispose };
}
