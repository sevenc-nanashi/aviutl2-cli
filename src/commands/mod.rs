mod develop;
mod init;
mod prepare;
mod release;
mod schema;

use anyhow::Result;

use crate::cli::Commands;

pub fn run(command: Commands) -> Result<()> {
    match command {
        Commands::Init => init::run(),
        Commands::Prepare { force } => {
            schema::run()?;
            prepare::aviutl2()?;
            prepare::artifacts(force, None)
        }
        Commands::PrepareAviUtl2 => prepare::aviutl2(),
        Commands::PrepareArtifacts { force, profile } => prepare::artifacts(force, profile),
        Commands::Develop { profile } => develop::run(profile),
        Commands::PrepareSchema => schema::run(),
        Commands::Release {
            profile,
            set_version,
        } => release::run(profile, set_version),
    }
}
