use crate::logging::app_config;
use clap::Parser;
use cli::Cli;
use regex::Regex;

mod cli;
mod interactive;
mod logging;
mod prom;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // initialize the logger
    log4rs::init_config(app_config("log.out", cli.loglevel)).unwrap();
    log::info!("Starting the application!");

    let regex = Regex::new(":(\\d{2,5})/").unwrap();
    let endpoint = match cli.port {
        Some(port) => regex
            .replace(&cli.endpoint, format!(":{port}/", port = port))
            .to_string(),
        None => cli.endpoint,
    };
    log::info!("Reading metrics from endpoint: {}", endpoint);
    log::info!("Scraping interval is: {}s", cli.scrape_interval);

    // start dashboard
    log::info!("Showing the dashboard");
    interactive::show(endpoint.clone(), cli.scrape_interval as u64).await?;
    Ok(())
}
