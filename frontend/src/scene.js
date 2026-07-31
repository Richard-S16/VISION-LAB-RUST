import {
  ArcRotateCamera,
  BackgroundMaterial,
  Color4,
  DirectionalLight,
  Engine,
  LoadAssetContainerAsync,
  MeshBuilder,
  RenderTargetTexture,
  Scene,
  ShadowGenerator,
  Vector3,
} from "@babylonjs/core";
import { FilesInputStore } from "@babylonjs/core/Misc/filesInputStore";
import "@babylonjs/loaders/glTF";

const MODEL_FILE = "CarConcept.glb";
const TARGET_SIZE = 4;

function aggregateBounds(meshes) {
  let minimum = null;
  let maximum = null;
  for (const mesh of meshes) {
    mesh.computeWorldMatrix(true);
    mesh.refreshBoundingInfo(true);
    const box = mesh.getBoundingInfo().boundingBox;
    minimum = minimum ? Vector3.Minimize(minimum, box.minimumWorld) : box.minimumWorld.clone();
    maximum = maximum ? Vector3.Maximize(maximum, box.maximumWorld) : box.maximumWorld.clone();
  }
  if (!minimum || !maximum) throw new Error("model contains no renderable meshes");
  return { minimum, maximum };
}

function normalizedUri(uri) {
  let decoded;
  try {
    decoded = decodeURIComponent(uri);
  } catch {
    decoded = uri;
  }
  return decoded.split(/[?#]/, 1)[0].replace(/\\/g, "/").replace(/^\.\//, "");
}

function basename(path) {
  return path.split("/").pop().toLowerCase();
}

async function createFileStore(files, model) {
  const byBasename = new Map();
  for (const file of files) {
    const key = file.name.toLowerCase();
    if (byBasename.has(key)) {
      throw new Error(`duplicate selected filename: ${file.name}`);
    }
    byBasename.set(key, file);
  }

  const aliases = new Map();
  const addAlias = (key, file) => {
    const normalized = key.toLowerCase();
    const existing = aliases.get(normalized);
    if (existing && existing !== file) {
      throw new Error(`duplicate model resource path: ${key}`);
    }
    aliases.set(normalized, file);
  };
  for (const file of files) addAlias(file.name, file);

  if (/\.gltf$/i.test(model.name)) {
    let gltf;
    try {
      gltf = JSON.parse(await model.text());
    } catch (error) {
      throw new Error(`invalid glTF JSON: ${error.message}`);
    }
    const uris = [
      ...(gltf.buffers ?? []).map((entry) => entry.uri),
      ...(gltf.images ?? []).map((entry) => entry.uri),
    ].filter((uri) => typeof uri === "string" && !/^(data:|blob:)/i.test(uri));
    for (const uri of uris) {
      if (/^[a-z][a-z\d+.-]*:/i.test(uri)) {
        throw new Error(`external glTF resource is not allowed: ${uri}`);
      }
      const path = normalizedUri(uri);
      const resource = byBasename.get(basename(path));
      if (!resource) throw new Error(`missing glTF resource: ${uri}`);
      addAlias(path, resource);
      addAlias(uri, resource);
    }
  }

  for (const [key, file] of aliases) FilesInputStore.FilesToLoad[key] = file;
  return () => {
    for (const [key, file] of aliases) {
      if (FilesInputStore.FilesToLoad[key] === file) delete FilesInputStore.FilesToLoad[key];
    }
  };
}

async function createBabylonScene(canvas) {
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

  let activeModel = null;
  let latestGeneration = 1;
  let replacementCount = 0;
  let disposalCount = 0;

  function publishDiagnostics(bounds = null) {
    const visibleBounds = bounds ?? activeModel?.bounds ?? null;
    window.__visionLabSceneDiagnostics = {
      activeGeneration: activeModel?.generation ?? null,
      latestGeneration,
      renderableMeshes: activeModel?.meshes.length ?? 0,
      replacementCount,
      disposalCount,
      normalizedSize: visibleBounds
        ? {
            x: visibleBounds.maximum.x - visibleBounds.minimum.x,
            y: visibleBounds.maximum.y - visibleBounds.minimum.y,
            z: visibleBounds.maximum.z - visibleBounds.minimum.z,
          }
        : null,
      minimumY: visibleBounds?.minimum.y ?? null,
    };
  }

  function disposeModel(model) {
    if (!model) return;
    for (const mesh of model.meshes) shadowGenerator.removeShadowCaster(mesh, false);
    model.container.removeAllFromScene();
    model.container.dispose();
    disposalCount += 1;
  }

  function installContainer(container, generation) {
    const root = container.createRootMesh();
    root.name = `vision-lab-model-${generation}`;
    container.addAllToScene();
    try {
      const meshes = container.meshes.filter(
        (mesh) => mesh !== root && mesh.getTotalVertices() > 0,
      );
      const initial = aggregateBounds(meshes);
      const size = initial.maximum.subtract(initial.minimum);
      const maxDimension = Math.max(size.x, size.y, size.z);
      if (!Number.isFinite(maxDimension) || maxDimension <= 0) {
        throw new Error("model bounds are invalid");
      }
      root.scaling.setAll(TARGET_SIZE / maxDimension);
      const scaled = aggregateBounds(meshes);
      const center = scaled.maximum.add(scaled.minimum).scale(0.5);
      root.position.subtractInPlace(new Vector3(center.x, scaled.minimum.y, center.z));
      const normalized = aggregateBounds(meshes);

      const nextModel = { container, root, meshes, bounds: normalized, generation };
      for (const mesh of meshes) shadowGenerator.addShadowCaster(mesh, false);
      const previous = activeModel;
      activeModel = nextModel;
      replacementCount += 1;
      camera.target.y = (normalized.maximum.y - normalized.minimum.y) * 0.45;
      disposeModel(previous);
      shadowGenerator.getShadowMap().resetRefreshCounter();
      publishDiagnostics(normalized);
    } catch (error) {
      container.removeAllFromScene();
      container.dispose();
      throw error;
    }
  }

  async function replaceModel(load, generation) {
    let container;
    try {
      container = await load();
      if (generation !== latestGeneration) {
        container.dispose();
        return;
      }
      const loaded = container;
      container = null;
      installContainer(loaded, generation);
    } catch (error) {
      if (container && container !== activeModel?.container) container.dispose();
      throw error;
    }
  }

  async function loadDefault(generation) {
    await replaceModel(() => LoadAssetContainerAsync(MODEL_FILE, scene), generation);
  }

  async function loadFiles(files, generation) {
    const list = [...files];
    const models = list.filter((file) => /\.(glb|gltf)$/i.test(file.name));
    if (models.length === 0) throw new Error("no .glb or .gltf file in selection");
    if (models.length > 1) throw new Error("select exactly one .glb or .gltf model");
    const model = models[0];
    const cleanup = await createFileStore(list, model);
    try {
      await replaceModel(
        () => LoadAssetContainerAsync(model.name, scene, { rootUrl: "file:" }),
        generation,
      );
    } finally {
      cleanup();
    }
  }

  function beginReplacement(generation) {
    latestGeneration = generation;
    publishDiagnostics();
  }

  function dispose() {
    engine.stopRenderLoop();
    disposeModel(activeModel);
    activeModel = null;
    scene.dispose();
    engine.dispose();
  }

  await loadDefault(1);
  return {
    engine,
    scene,
    beginReplacement,
    loadDefault,
    loadFiles,
    dispose,
  };
}

let activeScene = null;

export async function initializeScene(canvas) {
  activeScene = await createBabylonScene(canvas);
  await activeScene.scene.whenReadyAsync();
  activeScene.engine.runRenderLoop(() => activeScene.scene.render());
}

export function beginModelReplacement(generation) {
  activeScene.beginReplacement(generation);
}

export function loadDefaultModel(generation) {
  return activeScene.loadDefault(generation);
}

export function loadModelFiles(files, generation) {
  return activeScene.loadFiles(files, generation);
}

export function resizeScene() {
  activeScene?.engine.resize();
}

export function getSceneFps() {
  return activeScene?.engine.getFps() ?? 0;
}

export function disposeScene() {
  activeScene?.dispose();
  activeScene = null;
}
