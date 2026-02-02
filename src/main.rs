mod cli;
mod commands;
mod config;
mod schema;
mod util;

use clap::Parser;

fn main() {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .init();
    let cli = cli::Cli::parse();
    if let Err(e) = commands::run(cli.command) {
        log::error!("{}", e);
        std::process::exit(1);
    }
}
