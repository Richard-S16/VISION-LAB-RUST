# Baseline Fixtures

These fixtures were captured from `../VISION-LAB` commit `9653e1588608d37ace01b9544622bfd5334a92da` on 2026-07-29.

## Contents

| Path | Purpose |
|---|---|
| `frames/*.png` | Exact 384 x 384 detector inputs. |
| `expected/*.json` | Transformers.js WebGPU fp32 outputs at threshold 0.5. |
| `visual/*.png` | Full 1440 x 900 visual parity references. |
| `baseline-metadata.json` | Browser, GPU, dependency, load, and capture metadata. |

Expected JSON uses source overlay shape and pixel coordinates because it records source output exactly. Native Phase 1 tests may adapt these records to normalized target-domain boxes while retaining original files as immutable evidence.

## Integrity

| File | Bytes | SHA-256 |
|---|---:|---|
| `frames/car-front-three-quarter.png` | 41,290 | `4d26592bc9cbf4baefcf3a7eed2ce4796596b086e4774b22d18b47b99e51032b` |
| `frames/car-rear-three-quarter.png` | 41,971 | `2e769ed5608f58bf508216be0592a212a8ae527b922b9c63033b50f6f5ef7248` |
| `frames/car-side.png` | 39,083 | `924592ad8143354683f01140a1039b5b7764396f1900dd6cbd8925d7ef02da2e` |
| `frames/laptop-front-three-quarter.png` | 29,582 | `d886b7a9a0594d1d3c968df20222259f7d5c9d756bf4bd4b57b33c28f9248183` |
| `frames/laptop-side.png` | 32,711 | `c8a5909676bf77c0435c1bf2ce4a34f9d0f788ad20ed156a5130abfa4a8defa6` |
| `visual/default-car-1440x900.png` | 217,928 | `ceb54d7b1e5204393b6c69f536cba49017e52c2f83316ff1a2c74b01631db677` |
| `visual/laptop-1440x900.png` | 106,935 | `30c7145960dfd1aeb2718536bb63f7c22c7c42a5b82d8b7c65d2727d1bc6e168` |

Regeneration is an explicit baseline update. Do not regenerate fixtures as an incidental part of native implementation or routine tests.
