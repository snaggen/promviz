use clap::{command, Arg, Command, ValueHint};

#[allow(clippy::too_many_lines)]
#[must_use]
pub fn build() -> Command {
    command!()
        .name("PROMVIZ")
        .arg(
            Arg::new("Endpoint")
                .short('e')
                .long("endpoint")
                .env("PROM_ENDPOINT")
                .value_hint(ValueHint::Url)
                .value_name("ENDPOINT")
                .global(true)
                .num_args(1)
                .help("Prometheus endpoint to scrape")
                .long_help("The Prometheus endpoint used to scrape metrics from.")
                .default_value("http://localhost:8080/metrics")
        )
        .arg(
            Arg::new("Port")
                .short('p')
                .long("port")
                .env("PROM_PORT")
                .value_hint(ValueHint::Other)
                .value_name("PORT")
                .global(false)
                .num_args(1)
                .use_value_delimiter(false)
                .help("Prometheus endpoint's port number")
                .long_help("The port number used in the default prometheus endpoint. Example: http://localhost:<PORT>/metrics")
                .value_parser(clap::value_parser!(u16))
        )
        .arg(
            Arg::new("Scrape-Interval")
                .short('i')
                .long("scrape-interval")
                .env("PROM_SCRAPE_INTERVAL")
                .value_hint(ValueHint::Other)
                .value_name("SCRAPE_INTERVAL")
                .global(false)
                .num_args(1)
                .use_value_delimiter(false)
                .help("Scrape interval of the prometheus endpoint")
                .long_help("The time interval between 2 consecutive scrapes. Default value is 10s")
                .default_value("10")
                .value_parser(clap::value_parser!(u16))
        )
        .arg(
            Arg::new("Logging")
                .short('l')
                .long("logging")
                .value_name("LOG_LEVEL")
                .global(false)
                .num_args(1)
                .use_value_delimiter(false)
                .help("Set the logging level")
                .long_help("Set the logging level to one of these values: DEBUG,ERROR,WARN,INFO")
                .value_parser(["info" , "INFO" , "debug" , "DEBUG" , "error" , "ERROR" , "warn" , "WARN" ])
        )
}

#[test]
fn verify() {
    build().debug_assert();
}
