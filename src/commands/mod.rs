mod develop;
mod init;
mod prepare;
mod preview;
mod release;
mod schema;

use anyhow::Result;

use crate::cli::Commands;
use crate::config::ConfigLoadOpts;

pub fn run(command: Commands, opts: ConfigLoadOpts) -> Result<()> {
    match command {
        Commands::Init { force, use_pkl } => init::run(force, use_pkl, &opts),
        Commands::Prepare { force, refresh } => {
            schema::run()?;
            prepare::aviutl2(&opts)?;
            prepare::cleanup_data_generated_by_prepare(&opts)?;
            prepare::artifacts(force, None, refresh, &opts)
        }
        Commands::PrepareAviUtl2 => prepare::aviutl2(&opts),
        Commands::PrepareArtifacts {
            force,
            profile,
            refresh,
        } => {
            prepare::cleanup_data_generated_by_prepare(&opts)?;
            prepare::artifacts(force, profile, refresh, &opts)
        }
        Commands::Develop {
            profile,
            skip_start,
            detach,
            refresh,
            args,
        } => develop::run(profile, skip_start, detach, refresh, args, &opts),
        Commands::PrepareSchema => schema::run(),
        Commands::Release {
            profile,
            set_version,
        } => release::run(profile, set_version, &opts),
        Commands::Preview {
            profile,
            skip_start,
            refresh,
            args,
        } => preview::run(profile, skip_start, refresh, args, &opts),
    }
}
