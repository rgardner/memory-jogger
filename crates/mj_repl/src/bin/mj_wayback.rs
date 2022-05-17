use anyhow::Result;
use chrono::NaiveDateTime;
use clap::Parser;

#[derive(Debug, Parser)]
struct CLIArgs {
    #[clap()]
    url: String,
    #[clap(short, long)]
    time: Option<NaiveDateTime>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = CLIArgs::parse();
    let archive =
        mj_repl::util::get_wayback_url(args.url, args.time, &reqwest::Client::new()).await?;
    match archive {
        Some(archive) => println!("{}", archive),
        None => println!("No archive found"),
    }
    Ok(())
}
