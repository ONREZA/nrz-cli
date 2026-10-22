use anyhow::{Context, Result, bail, ensure};
use std::path::PathBuf;

fn main() -> Result<()> {
    let mut directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("schemas");
    let mut check = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--check" => check = true,
            "--out" => directory = args.next().context("--out requires a directory")?.into(),
            _ => bail!("unknown argument: {arg}"),
        }
    }
    for (name, schema) in [
        (
            "source-bundle-v2",
            nrz_contract::artifact_schemas::source_bundle_v2(),
        ),
        (
            "application-package-v1",
            nrz_contract::artifact_schemas::application_package_v1(),
        ),
        (
            "runtime-artifact-graph-v3",
            nrz_contract::artifact_schemas::runtime_artifact_graph_v3(),
        ),
    ] {
        let path = directory.join(format!("{name}.schema.json"));
        if check {
            ensure!(
                serde_json::from_slice::<serde_json::Value>(
                    &std::fs::read(&path).with_context(|| format!("read {}", path.display()))?
                )? == schema,
                "artifact schema drift: {}; run export-artifact-schemas",
                path.display()
            );
        } else {
            let mut bytes = serde_json::to_vec_pretty(&schema)?;
            bytes.push(b'\n');
            std::fs::create_dir_all(&directory)?;
            std::fs::write(&path, bytes).with_context(|| format!("write {}", path.display()))?;
        }
        println!(
            "{} {}",
            if check { "verified" } else { "wrote" },
            path.display()
        );
    }
    Ok(())
}
