# RF-DETR Nano ONNX

| Item | Value |
|---|---|
| Repository | `onnx-community/rfdetr_nano-ONNX` |
| Revision | `eae21cee0687a91bcf9fa071605c48d7705d2d91` |
| Model path | `onnx/model.onnx` |
| License | Apache-2.0 |
| Expected bytes | 108,074,865 |
| Expected SHA-256 | `9cbac6b11ce34a03034e4d5a24cfac5f18632fd6761d1311dd640232088d7fee` |

Files in this directory are fetched from exact revision above for Phase 1 compatibility testing. `checksums.json` records verified local artifacts.

`model.onnx` is intentionally ignored by Git because it exceeds GitHub's 100 MB regular-file limit. Download the exact revision before development or packaging:

```powershell
Invoke-WebRequest `
  -Uri "https://huggingface.co/onnx-community/rfdetr_nano-ONNX/resolve/eae21cee0687a91bcf9fa071605c48d7705d2d91/onnx/model.onnx" `
  -OutFile "models/rfdetr-nano/model.onnx"
```

Confirm its SHA-256 matches the value above before use.
