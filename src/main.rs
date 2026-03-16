mod catalog;
mod catalog_schema;
mod cli;
mod commands;
mod config;
mod schema;
mod util;

use clap::Parser;

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    let cli = cli::Cli::parse();
    if let Err(e) = commands::run(cli.command) {
        tracing::error!("{:?}", e);
        std::process::exit(1);
    }
}
