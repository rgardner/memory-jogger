use anyhow::Result;
use chrono::NaiveDateTime;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct CLIArgs {
    #[structopt()]
    url: String,
    #[structopt(short, long)]
    time: Option<NaiveDateTime>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = CLIArgs::from_args();
    let archive =
        mj_repl::util::get_wayback_url(args.url, args.time, &reqwest::Client::new()).await?;
    match archive {
        Some(archive) => println!("{}", archive),
        None => println!("No archive found"),
    }
    Ok(())
}
