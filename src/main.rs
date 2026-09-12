use crate::app::App;
use clap::Parser;
use cli::Cli;
use color_eyre::Result;

mod action;
mod app;
mod cli;
mod components;
mod config;
mod errors;
mod logging;
mod mods;
mod tui;

#[tokio::main]
async fn main() -> Result<()> {
    crate::errors::init()?;

    let args = Cli::parse();

    if let Some(arg) = args.install_mod {
        let msg = if arg.starts_with("http://") || arg.starts_with("https://") {
            balatui::install_mod_from_url(&arg).await
        } else {
            balatui::install_mod_by_name(&arg).await
        }
        .map_err(|e| color_eyre::eyre::eyre!(e))?;
        println!("{msg}");
        return Ok(());
    }

    let mut app = App::new(args.tick_rate, args.frame_rate)?;

    // Set max_log_level to Info
    tui_logger::init_logger(log::LevelFilter::Trace)?;

    // Set default level for unknown targets to Info
    tui_logger::set_default_level(log::LevelFilter::Info);
    let _config = config::Config::new()?;

    // let mut temp_file = download_to_tmp("https://github.com/colonthreeing/SealSealBalatro/releases/download/1.1.0/SealSeal.zip").await;
    //
    // let file = temp_file.as_file();
    //
    // unzip(file, &get_balatro_appdata_dir().join("Mods"), "SealSeal");

    app.run().await?;

    Ok(())
}
