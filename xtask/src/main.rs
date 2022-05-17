use std::process::Command;

use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
#[clap(about = "Memory Jogger build system.")]
enum CLIArgs {
    BuildDockerImage,
    Test {
        #[clap(long)]
        backends: Vec<String>,
        #[clap(long)]
        large: bool,
    },
    Lint {
        #[clap(long)]
        backends: Vec<String>,
    },
    CI {
        #[clap(long)]
        backends: Vec<String>,
    },
}

fn cargo_features(backends: &[String], large: Option<bool>) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();
    if backends == ["sqlite"] {
        args.extend_from_slice(&["--no-default-features".into(), "--features=sqlite".into()]);
    } else if backends == ["postgres"] {
        args.extend_from_slice(&["--no-default-features".into(), "--features=postgres".into()]);
    }
    if large == Some(true) {
        args.push("--features=large_tests".into());
    }
    args
}

fn build(backends: &[String]) -> Result<()> {
    let status = Command::new("cargo")
        .arg("build")
        .args(&cargo_features(backends, None))
        .status()?;
    anyhow::ensure!(status.success(), "cargo build failed");
    Ok(())
}

fn build_docker() -> Result<()> {
    let status = Command::new("docker-compose").arg("build").status()?;
    anyhow::ensure!(status.success(), "docker-compose failed");
    Ok(())
}

fn fmt(backends: &[String], check: bool) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.args(&["fmt", "--all"]);
    cmd.args(&cargo_features(backends, None));
    if check {
        cmd.arg("--check");
    }
    let status = cmd.status()?;
    anyhow::ensure!(status.success(), "cargo fmt failed");
    Ok(())
}

fn lint(backends: &[String]) -> Result<()> {
    let status = Command::new("cargo")
        .args(&["clippy", "--all"])
        .args(&cargo_features(backends, None))
        .args(&["--", "-D", "warnings"])
        .status()?;
    anyhow::ensure!(status.success(), "cargo clippy failed");
    Ok(())
}

fn test(backends: &[String], large: Option<bool>) -> Result<()> {
    let status = Command::new("cargo")
        .arg("test")
        .args(&cargo_features(backends, large))
        .status()?;
    anyhow::ensure!(status.success(), "cargo test failed");
    Ok(())
}

fn main() -> Result<()> {
    let opt = CLIArgs::parse();
    match opt {
        CLIArgs::BuildDockerImage => build_docker()?,
        CLIArgs::Test { backends, large } => test(&backends, Some(large))?,
        CLIArgs::Lint { backends } => lint(&backends)?,
        CLIArgs::CI { backends } => {
            build(&backends)?;
            fmt(&backends, true)?;
            lint(&backends)?;
            test(&backends, Some(true))?;
            build_docker()?
        }
    };

    Ok(())
}
