use clap::Parser;
use clap::ValueHint;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Prometheus endpoint to scrape
    ///
    /// The Prometheus endpoint used to scrape metrics from.
    #[arg(short, long, env="PROM_ENDPOINT", value_hint=ValueHint::Url, default_value="http://localhost:8080/metrics")]
    pub endpoint: String,

    /// Prometheus endpoint's port number
    ///
    /// The port number used in the default prometheus endpoint. Example: http://localhost:<PORT>/metrics
    #[arg(short, long, env="PROM_PORT", value_hint=ValueHint::Other)]
    pub port: Option<u16>,

    ///Scrape interval of the prometheus endpoint")
    ///
    /// The time interval between 2 consecutive scrapes.")
    #[arg(short='i', long, env="PROM_SCRAPE_INTERVAL", value_hint=ValueHint::Other, default_value="10")]
    pub scrape_interval: u16,

    /// Set the logging level
    ///
    /// Set the logging level to use when logging to the app.log file
    #[arg(short, long, env="LOG_LEVEL", value_hint=ValueHint::Other, default_value="INFO")]
    pub loglevel: log::LevelFilter,
}
