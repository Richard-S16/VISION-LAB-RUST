# VISION/LAB

Windows-first desktop object detection for interactive 3D models. VISION/LAB renders glTF assets with Babylon.js inside a Tauri 2 WebView and runs RF-DETR Nano natively through ONNX Runtime.

Detection works offline from bundled resources. DirectML is preferred on compatible Windows hardware, with explicit CPU fallback when DirectML cannot create the model session.

## Features

- Studio-lit 3D model viewer with orbit, zoom, pinch, shadows, and idle rotation.
- Bundled concept car loaded on startup.
- `.glb` upload and multi-file `.gltf` loading with sibling buffers and textures.
- Drag-and-drop model replacement and reset.
- Native RF-DETR Nano fp32 inference.
- DirectML acceleration with CPU fallback.
- Configurable confidence threshold.
- Detection latency, render FPS, and object count HUD.
- DPR-aware detection overlay.
- Single-flight inference with a 300 ms minimum capture interval.
- Request and model-generation checks that reject stale results.
- No browser inference worker, model download, or required runtime network access.

## Architecture

```text
Tauri WebView2
  Babylon.js scene
      -> reusable 384 x 384 capture canvas
      -> raw RGBA8 Tauri IPC body

Tauri native process
  typed command validation
      -> dedicated detector thread
      -> persistent ONNX Runtime session
      -> DirectML or CPU
      -> normalized detections and timings
      -> WebView overlay and HUD
```

Native detector responsibilities are separated under `src-tauri/src/detector/`:

| Module | Responsibility |
|---|---|
| `resources.rs` | Bundled resource resolution and SHA-256 verification |
| `session.rs` | ONNX session, provider fallback, schema checks, and warmup |
| `worker.rs` | Dedicated thread, single-flight backpressure, timeout, and shutdown |
| `preprocess.rs` | RGBA to rescaled RGB/NCHW tensor conversion |
| `postprocess.rs` | Softmax, labels, normalized boxes, clamping, and sorting |
| `types.rs` | Serializable IPC requests, responses, diagnostics, and timings |

## Requirements

- Windows 10 or Windows 11 x86_64.
- WebView2 Runtime.
- Node.js 22 or newer.
- Rust 1.97.1 with the MSVC target and Visual Studio C++ build tools.
- NSIS support supplied through the Tauri CLI for installer builds.

Rust version and components are pinned in `rust-toolchain.toml`. JavaScript and Rust dependencies are locked by `package-lock.json` and `Cargo.lock`.

## Model Resources

The application expects these files under `models/rfdetr-nano/`:

```text
model.onnx
config.json
preprocessor_config.json
checksums.json
```

`model.onnx` exceeds GitHub's regular file limit and is not stored in Git. Download the pinned model before building or running detector tests:

```powershell
Invoke-WebRequest `
  -Uri "https://huggingface.co/onnx-community/rfdetr_nano-ONNX/resolve/eae21cee0687a91bcf9fa071605c48d7705d2d91/onnx/model.onnx" `
  -OutFile "models/rfdetr-nano/model.onnx"
```

Verify the downloaded file:

```powershell
(Get-FileHash "models/rfdetr-nano/model.onnx" -Algorithm SHA256).Hash.ToLower()
```

Expected SHA-256: `9cbac6b11ce34a03034e4d5a24cfac5f18632fd6761d1311dd640232088d7fee`.

Model identity:

| Item | Value |
|---|---|
| Repository | `onnx-community/rfdetr_nano-ONNX` |
| Revision | `eae21cee0687a91bcf9fa071605c48d7705d2d91` |
| Precision | fp32 |
| Input | `1 x 3 x 384 x 384` |
| Model size | 108,074,865 bytes |
| SHA-256 | `9cbac6b11ce34a03034e4d5a24cfac5f18632fd6761d1311dd640232088d7fee` |

The native service validates model and configuration checksums before creating a session. See `models/rfdetr-nano/README.md` for provenance.

## Setup

Install frontend dependencies:

```powershell
npm ci
npm ci --prefix tools
```

Run the desktop application in development mode:

```powershell
npm run tauri -- dev
```

The detector initializes lazily when `DETECT` is pressed. Startup stages report model validation, ONNX Runtime loading, provider selection, graph optimization, and warmup.

## Builds

Build frontend assets:

```powershell
npm run build
```

Build a debug Windows installer:

```powershell
npm run tauri -- build --debug
```

Build a release installer:

```powershell
npm run tauri -- build
```

NSIS output is written under:

```text
target/<profile>/bundle/nsis/
```

## Verification

Run Rust unit, golden, fallback, and concurrency tests:

```powershell
cargo test --workspace
```

Run Rust quality checks:

```powershell
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

Run frontend and integration checks:

```powershell
npm run build
npm audit --omit=dev
npm run phase2 --prefix tools
npm run phase4 --prefix tools
```

Golden fixtures under `tests/fixtures/` compare native CPU output with recorded Transformers.js WebGPU fp32 results. DirectML is not required by ordinary tests; fallback coverage deliberately requests an invalid DirectML device and verifies CPU inference.

## Project Layout

```text
frontend/                 WebView presentation and Babylon integration
src-tauri/                Tauri host and production native detector
crates/                   Native inference feasibility spike
models/rfdetr-nano/       Pinned model configuration and checksum manifest
public/                   Bundled 3D, environment, font, and icon assets
tests/fixtures/           Immutable detector and visual parity fixtures
tests/results/            Recorded phase verification artifacts
tools/                    Playwright parity and IPC integration tests
docs/                     Completed implementation-phase reports
third-party/              Asset provenance and license material
```

## Current Status

Completed work includes baseline capture, native inference feasibility, Tauri parity shell, native detector service, and native frontend integration. Remaining work covers the Rust/WASM application controller, hardened Babylon adapter lifecycle, and clean-machine release verification.

Detailed implementation reports are available in `docs/`.

## Licensing

Repository code uses `MIT OR Apache-2.0` as declared by the Rust workspace. RF-DETR Nano and its selected ONNX export use Apache-2.0. Bundled fonts use the SIL Open Font License 1.1.

See `third-party/` for asset provenance and available license texts. Complete release notices still require final packaging review.
