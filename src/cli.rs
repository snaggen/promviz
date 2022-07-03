use clap::{command, Arg, Command, ValueHint};

#[allow(clippy::too_many_lines)]
#[must_use]
pub fn build() -> Command<'static> {
    command!()
        .name("PROM TUI")
        .arg(
            Arg::new("Endpoint")
                .short('e')
                .long("endpoint")
                .env("PROM_ENDPOINT")
                .value_hint(ValueHint::Url)
                .value_name("ENDPOINT")
                .global(true)
                .takes_value(true)
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
                .takes_value(true)
                .use_value_delimiter(false)
                .help("Prometheus endpoint's port number")
                .long_help("The port number used in the default prometheus endpoint. Example: http://localhost:<PORT>/metrics")
                .validator(|v| v.to_string().parse::<u16>())
        )
        .arg(
            Arg::new("Scrape-Interval")
                .short('i')
                .long("scrape-interval")
                .env("PROM_SCRAPE_INTERVAL")
                .value_hint(ValueHint::Other)
                .value_name("SCRAPE_INTERVAL")
                .global(false)
                .takes_value(true)
                .use_value_delimiter(false)
                .help("Scrape interval of the prometheus endpoint")
                .long_help("The time interval between 2 consecutive scrapes. Default value is 10s")
                .default_value("10")
                .validator(|v| v.to_string().parse::<u16>())
        )
}

#[test]
fn verify() {
    build().debug_assert();
}
