# promviz

A Prometheus Visualizer for the terminal, written in Rust

This is a fork of [prom-tui](https://github.com/pastrami-turtles/prom-tui),
mostly to scratch my own itch. So, feature requests without a PR is very
unlikely to happen.

## Usage

Start with 'cargo run' and quit by pressing 'q'.

You can provide the endpoint to scrape in 2 ways:
  1. as CLI argument
  2. as env variable

In the case of the CLI argument run:

```bash
cargo run -- --endpoint "http://localhost:8081/metrics"
```

with the env variable
```bash
PROM_ENDPOINT=http://localhost:8081/metrics cargo run
```

If no endpoint is provided the default value is http://localhost:8080/metrics

