mod develop;
mod init;
mod preview;
mod prepare;
mod release;
mod schema;

use anyhow::Result;

use crate::cli::Commands;

pub fn run(command: Commands) -> Result<()> {
    match command {
        Commands::Init => init::run(),
        Commands::Prepare { force, refresh } => {
            schema::run()?;
            prepare::aviutl2()?;
            prepare::artifacts(force, None, refresh)
        }
        Commands::PrepareAviUtl2 => prepare::aviutl2(),
        Commands::PrepareArtifacts {
            force,
            profile,
            refresh,
        } => prepare::artifacts(force, profile, refresh),
        Commands::Develop {
            profile,
            skip_start,
            refresh,
        } => develop::run(profile, skip_start, refresh),
        Commands::PrepareSchema => schema::run(),
        Commands::Release {
            profile,
            set_version,
        } => release::run(profile, set_version),
        Commands::Preview {
            profile,
            skip_start,
            refresh,
        } => preview::run(profile, skip_start, refresh),
    }
}
