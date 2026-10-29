//! Build tooling follows the SDK selected by the committed Cargo lockfile.

use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

fn sdk_version(metadata: &Value) -> Result<&str, &'static str> {
    let missing = "locked server SDK not found";
    let packages = metadata["packages"].as_array().ok_or(missing)?;
    let members = metadata["workspace_members"].as_array().ok_or(missing)?;
    let server = packages
        .iter()
        .find(|p| p["name"] == "time-wise-server" && members.contains(&p["id"]))
        .ok_or(missing)?;
    let nodes = metadata["resolve"]["nodes"].as_array().ok_or(missing)?;
    let node = nodes
        .iter()
        .find(|node| node["id"] == server["id"])
        .ok_or(missing)?;
    let dependencies = node["deps"].as_array().ok_or(missing)?;
    let sdk = dependencies
        .iter()
        .find(|dep| dep["name"] == "worker")
        .ok_or(missing)?;
    packages
        .iter()
        .find(|p| p["id"] == sdk["pkg"])
        .and_then(|p| p["version"].as_str())
        .filter(|v| !v.is_empty())
        .ok_or(missing)
}

fn cargo() -> Command {
    Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
}

fn matches_version(tool: &Path, expected: &str) -> bool {
    Command::new(tool)
        .arg("--version")
        .output()
        .is_ok_and(|result| {
            result.status.success() && String::from_utf8_lossy(&result.stdout).trim() == expected
        })
}

fn execute() -> Result<(), Box<dyn std::error::Error>> {
    let action = std::env::args().nth(1).unwrap_or_else(|| "build".into());
    if !matches!(action.as_str(), "build" | "version") {
        return Err("expected build or version".into());
    }
    let server = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = server.join("../..");
    let metadata = cargo()
        .args([
            "metadata",
            "--locked",
            "--format-version",
            "1",
            "--filter-platform",
            "wasm32-unknown-unknown",
        ])
        .current_dir(&root)
        .output()?;
    if !metadata.status.success() {
        return Err("cargo metadata failed".into());
    }
    let metadata: Value = serde_json::from_slice(&metadata.stdout)?;
    let version = sdk_version(&metadata)?;
    if action == "version" {
        println!("{version}");
        return Ok(());
    }

    let tool = if let Some(tool) = std::env::var_os("WORKER_BUILD") {
        let tool = PathBuf::from(tool);
        if !matches_version(&tool, version) {
            return Err(format!("WORKER_BUILD must match locked worker SDK {version}").into());
        }
        tool
    } else {
        let destination = root.join("target/worker-tools");
        let tool = destination
            .join("bin/worker-build")
            .with_extension(std::env::consts::EXE_EXTENSION);
        if !matches_version(&tool, version) {
            let status = cargo()
                .args([
                    "install",
                    "worker-build",
                    "--version",
                    version,
                    "--locked",
                    "--force",
                    "--root",
                ])
                .arg(&destination)
                .arg("--target-dir")
                .arg(root.join("target/worker-tools-build"))
                .status()?;
            if !status.success() || !matches_version(&tool, version) {
                return Err(
                    format!("unable to install worker-build matching SDK {version}").into(),
                );
            }
        }
        tool
    };
    let status = Command::new(tool)
        .args(["--release", "--locked"])
        .current_dir(server)
        .status()?;
    if !status.success() {
        return Err("Worker build failed".into());
    }
    Ok(())
}

fn main() -> ExitCode {
    match execute() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn metadata() -> Value {
        json!({
            "packages": [
                {"id":"other", "name":"worker", "version":"1.0.0"},
                {"id":"sdk", "name":"worker", "version":"9.8.7"},
                {"id":"server", "name":"time-wise-server"}
            ],
            "workspace_members":["server"],
            "resolve":{"nodes":[{"id":"server", "deps":[{"name":"worker", "pkg":"sdk"}]}]}
        })
    }

    #[test]
    fn selects_the_servers_resolved_sdk_not_an_unrelated_package() {
        assert_eq!(sdk_version(&metadata()), Ok("9.8.7"));
    }

    #[test]
    fn missing_or_unresolved_sdk_fails_closed() {
        assert!(sdk_version(&json!({})).is_err());
        let mut metadata = metadata();
        metadata["resolve"]["nodes"][0]["deps"] = json!([]);
        assert!(sdk_version(&metadata).is_err());
    }
}
