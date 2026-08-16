mod cli;
mod commands;
mod config;
mod context;
mod git;
mod ui;

use {
    anyhow::Result,
    clap::Parser,
    cli::{Cli, Command},
    console::style,
    context::Ctx,
    sigit_api::Client,
};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    if let Err(e) = run().await {
        eprintln!("{} {:#}", style("✘").red(), e);
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();

    let mut client = Client::new(cli.environment)?;
    if let Some(token) = config::read_token(cli.environment) {
        client = client.with_token(token);
    }

    let ctx = Ctx::new(client, cli.environment, cli.json);

    match cli.command {
        Command::Auth(command) => commands::auth::run(&ctx, command).await,
        Command::Me => commands::me::run(&ctx).await,
        Command::Repo(command) => commands::repo::run(&ctx, command).await,
        Command::Pr(command) => commands::pr::run(&ctx, command).await,
        Command::Issue(command) => commands::issue::run(&ctx, command).await,
        Command::Code(command) => commands::code::run(&ctx, command).await,
        Command::Hook(command) => commands::hook::run(&ctx, command).await,
        Command::Session(command) => commands::session::run(&ctx, command).await,
        Command::Billing(command) => commands::billing::run(&ctx, command).await,
        Command::Browse(args) => commands::browse::run(&ctx, args).await,
        Command::Api(args) => commands::api::run(&ctx, args).await,
    }
}
