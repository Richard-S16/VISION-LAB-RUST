use std::{env, fs, path::PathBuf};

use anyhow::{Context, Result, bail};
use vision_lab_inference_spike::{Provider, RunOptions, run};

struct Args {
    model: PathBuf,
    config: PathBuf,
    fixture: PathBuf,
    provider: Provider,
    directml_device_id: i32,
    threshold: f32,
    warmup_runs: usize,
    measured_runs: usize,
    output: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = parse_args()?;
    let result = run(
        &args.model,
        &args.config,
        &args.fixture,
        RunOptions {
            provider: args.provider,
            directml_device_id: args.directml_device_id,
            threshold: args.threshold,
            warmup_runs: args.warmup_runs,
            measured_runs: args.measured_runs,
        },
    )?;
    let json = format!("{}\n", serde_json::to_string_pretty(&result)?);
    if let Some(output) = args.output {
        fs::write(&output, json)
            .with_context(|| format!("failed to write {}", output.display()))?;
    } else {
        print!("{json}");
    }
    Ok(())
}

fn parse_args() -> Result<Args> {
    let mut model = None;
    let mut config = None;
    let mut fixture = None;
    let mut provider = Provider::Cpu;
    let mut directml_device_id = 0;
    let mut threshold = 0.5;
    let mut warmup_runs = 0;
    let mut measured_runs = 1;
    let mut output = None;
    let mut args = env::args().skip(1);

    while let Some(argument) = args.next() {
        let mut value = || {
            args.next()
                .with_context(|| format!("missing value for {argument}"))
        };
        match argument.as_str() {
            "--model" => model = Some(PathBuf::from(value()?)),
            "--config" => config = Some(PathBuf::from(value()?)),
            "--fixture" => fixture = Some(PathBuf::from(value()?)),
            "--provider" => provider = Provider::parse(&value()?)?,
            "--directml-device" => {
                directml_device_id = value()?.parse().context("invalid DirectML device ID")?
            }
            "--threshold" => threshold = value()?.parse().context("invalid threshold")?,
            "--warmup-runs" => warmup_runs = value()?.parse().context("invalid warmup count")?,
            "--runs" => measured_runs = value()?.parse().context("invalid measured run count")?,
            "--output" => output = Some(PathBuf::from(value()?)),
            "--help" | "-h" => {
                println!(
                    "usage: vision-lab-inference-spike --model PATH --config PATH --fixture PATH [--provider auto|cpu|directml] [--directml-device N] [--threshold 0.5] [--warmup-runs N] [--runs N] [--output PATH]"
                );
                std::process::exit(0);
            }
            _ => bail!("unknown argument '{argument}'"),
        }
    }

    Ok(Args {
        model: model.context("--model is required")?,
        config: config.context("--config is required")?,
        fixture: fixture.context("--fixture is required")?,
        provider,
        directml_device_id,
        threshold,
        warmup_runs,
        measured_runs,
        output,
    })
}
