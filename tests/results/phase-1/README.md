# Phase 1 Results

Generated native inference results. Files are grouped by requested execution provider and correspond to Phase 0 fixture basenames.

| Pattern | Meaning |
|---|---|
| `cpu-*.json` | Strict CPU session, one warmup, five measured runs. |
| `directml-*.json` | Strict DirectML device 0 session, one warmup, five measured runs. |
| `auto-invalid-device-fallback.json` | Forced invalid DirectML device 999 proving explicit CPU fallback. |

`crates/vision-lab-inference-spike/tests/phase1_parity.rs` compares CPU and DirectML outputs against Phase 0 Transformers.js results.
