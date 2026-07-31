# Third-Party Notices

VISION/LAB includes the following third-party software and assets.

| Component | Version or revision | License |
|---|---|---|
| Babylon.js Core and Loaders | 9.17.0 | Apache-2.0 |
| Tauri API and runtime | 2.11.x | Apache-2.0 OR MIT |
| ONNX Runtime, via `ort` | ONNX Runtime 1.23.2 / `ort` 2.0.0-rc.10 | MIT |
| RF-DETR Nano ONNX model | `eae21cee0687a91bcf9fa071605c48d7705d2d91` | Apache-2.0 |
| Microsoft Visual C++ Runtime | 14.44.35112, x64 app-local deployment | Microsoft Visual Studio license terms |
| JetBrains Mono | bundled font files | SIL Open Font License 1.1 |
| Michroma | bundled font files | SIL Open Font License 1.1 |
| Sora | bundled font files | SIL Open Font License 1.1 |

Full shared license texts are distributed in this directory under `licenses/`:

- `Apache-2.0.txt`
- `MIT.txt`
- `JetBrains-Mono-OFL.txt`
- `Michroma-OFL.txt`
- `Sora-OFL.txt`

The complete locked Rust dependency inventory, package attribution, and exact
license texts are in `RUST-DEPENDENCY-LICENSES.md`, generated with
`cargo-about` from `Cargo.lock`.

The application links ONNX Runtime statically. Its absence from the installed
directory as a separate DLL is therefore expected. Microsoft DirectML is an
optional Windows execution provider loaded from the operating system; the app
falls back to the ONNX Runtime CPU provider when DirectML is unavailable.
The app-local Microsoft Visual C++ Runtime files remove the need for a
separately installed Visual C++ Redistributable.
