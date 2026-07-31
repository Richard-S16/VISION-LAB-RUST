# Third-Party Runtime Assets

| Asset | Source | License |
|---|---|---|
| RF-DETR Nano ONNX | `onnx-community/rfdetr_nano-ONNX` | Apache-2.0 |
| Babylon environment texture | `assets.babylonjs.com/core/environments/environmentSpecular.env` | Babylon.js asset; attribution retained here pending release notice review |
| JetBrains Mono | JetBrains / Google Fonts | SIL Open Font License 1.1 |
| Michroma | Google Fonts | SIL Open Font License 1.1 |
| Sora | Google Fonts | SIL Open Font License 1.1 |
| Microsoft Visual C++ Runtime | Visual Studio 2022 `14.44.35112` app-local x64 runtime | Microsoft Visual Studio license terms |

Runtime, renderer, model, and font attribution is recorded in
`THIRD-PARTY-NOTICES.md`. The complete notices directory ships with the desktop
application. Rust dependency notices are reproducibly generated with:

```powershell
cargo about generate --workspace --locked third-party/rust-licenses.hbs --output-file third-party/RUST-DEPENDENCY-LICENSES.md
```
