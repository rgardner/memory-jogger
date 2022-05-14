use std::process::Command;

use anyhow::Result;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(about = "Memory Jogger build system.")]
enum CLIArgs {
    BuildDockerImage,
    Test {
        #[structopt(long)]
        backends: Vec<String>,
        #[structopt(long)]
        large: bool,
    },
    Lint {
        #[structopt(long)]
        backends: Vec<String>,
    },
}

fn cargo_features<S>(backends: &[S], large: Option<bool>) -> Vec<String>
where
    S: AsRef<str> + PartialEq<&'static str>,
{
    let mut args: Vec<String> = Vec::new();
    if backends == &["sqlite"] {
        args.extend_from_slice(&["--no-default-features".into(), "--features=sqlite".into()]);
    } else if backends == &["postgres"] {
        args.extend_from_slice(&["--no-default-features".into(), "--features=postgres".into()]);
    }
    if large == Some(true) {
        args.push("--features=large_tests".into());
    }
    args
}

fn main() -> Result<()> {
    let opt = CLIArgs::from_args();
    match opt {
        CLIArgs::BuildDockerImage => Command::new("docker-compose").arg("build").status()?,
        CLIArgs::Test { backends, large } => Command::new("cargo")
            .arg("test")
            .args(&cargo_features(&backends, Some(large)))
            .status()?,
        CLIArgs::Lint { backends } => Command::new("cargo")
            .args(&["clippy", "--all"])
            .args(&cargo_features(&backends, None))
            .args(&["--", "-D", "warnings"])
            .status()?,
    };

    Ok(())
}
