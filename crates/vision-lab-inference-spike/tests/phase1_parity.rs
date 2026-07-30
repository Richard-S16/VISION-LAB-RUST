use std::{fs, path::PathBuf};

use serde::Deserialize;

#[derive(Deserialize)]
struct NativeResult {
    detections: Vec<NativeDetection>,
}

#[derive(Deserialize)]
struct NativeDetection {
    label: String,
    score: f32,
    bounding_box: NativeBox,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NativeBox {
    origin_x: f32,
    origin_y: f32,
    width: f32,
    height: f32,
}

#[derive(Deserialize)]
struct BaselineResult {
    detections: Vec<BaselineDetection>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BaselineDetection {
    bounding_box: BaselineBox,
    categories: Vec<BaselineCategory>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BaselineBox {
    origin_x: f32,
    origin_y: f32,
    width: f32,
    height: f32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BaselineCategory {
    category_name: String,
    score: f32,
}

#[test]
fn cpu_and_directml_results_match_transformers_js_baseline() {
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fixtures = [
        "car-front-three-quarter",
        "car-rear-three-quarter",
        "car-side",
        "laptop-front-three-quarter",
        "laptop-side",
    ];

    for provider in ["cpu", "directml"] {
        for fixture in fixtures {
            let baseline: BaselineResult =
                read_json(repository.join(format!("tests/fixtures/expected/{fixture}.json")));
            let native: NativeResult = read_json(
                repository.join(format!("tests/results/phase-1/{provider}-{fixture}.json")),
            );

            assert_eq!(
                native.detections.len(),
                baseline.detections.len(),
                "{provider} {fixture}: detection count"
            );

            for (index, (native, baseline)) in native
                .detections
                .iter()
                .zip(&baseline.detections)
                .enumerate()
            {
                let category = &baseline.categories[0];
                assert_eq!(
                    native.label, category.category_name,
                    "{provider} {fixture} detection {index}: label"
                );
                assert!(
                    (native.score - category.score).abs() <= 0.000_01,
                    "{provider} {fixture} detection {index}: score {} != {}",
                    native.score,
                    category.score
                );
                assert_box_matches(
                    &native.bounding_box,
                    &baseline.bounding_box,
                    &format!("{provider} {fixture} detection {index}"),
                );
            }
        }
    }
}

fn read_json<T: for<'de> Deserialize<'de>>(path: PathBuf) -> T {
    serde_json::from_slice(&fs::read(&path).unwrap_or_else(|error| {
        panic!("failed to read {}: {error}", path.display());
    }))
    .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()))
}

fn assert_box_matches(native: &NativeBox, baseline: &BaselineBox, context: &str) {
    let native_x = native.origin_x.trunc();
    let native_y = native.origin_y.trunc();
    let native_width = (native.origin_x + native.width).trunc() - native_x;
    let native_height = (native.origin_y + native.height).trunc() - native_y;

    assert_eq!(native_x, baseline.origin_x, "{context}: origin x");
    assert_eq!(native_y, baseline.origin_y, "{context}: origin y");
    assert_eq!(native_width, baseline.width, "{context}: width");
    assert_eq!(native_height, baseline.height, "{context}: height");
}
