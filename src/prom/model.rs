use std::collections::HashMap;

use chrono::{DateTime, Local};
use ratatui::widgets::{Bar, BarGroup};

use crate::interactive::format_value;

use super::parser::extract_labels_key_and_map;

#[derive(Debug)]
pub struct MetricHistory {
    pub metrics: HashMap<String, Metric>,
}

impl MetricHistory {
    pub fn new() -> Self {
        Self {
            metrics: HashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.metrics.len() == 0
    }

    pub fn get_metrics_headers(&self) -> Vec<String> {
        let mut headers: Vec<String> = self.metrics.keys().cloned().collect();
        headers.sort();
        headers
    }

    pub fn get_metric(&self, metric_name: &str) -> Option<&Metric> {
        self.metrics.get(metric_name)
    }
}

#[derive(Clone, Debug)]
pub enum MetricType {
    Gauge,
    Counter,
    Histogram,
    Summary,
}

pub struct SingleScrapeMetric {
    pub name: String,
    pub docstring: String,
    pub metric_type: MetricType,
    pub value_per_labels: HashMap<String, Sample>,
}

impl SingleScrapeMetric {
    pub fn into_metric(self) -> Metric {
        let mut metric = Metric {
            details: MetricDetails {
                name: self.name,
                docstring: self.docstring,
                metric_type: self.metric_type,
            },
            time_series: HashMap::new(),
        };
        self.value_per_labels
            .into_iter()
            .for_each(|(labels, sample)| {
                add_time_series_into_metric(labels, &mut metric.time_series, sample);
            });
        metric
    }
}

#[derive(Clone, Debug)]
pub struct Metric {
    pub details: MetricDetails,
    pub time_series: HashMap<String, TimeSeries>,
}

#[derive(Clone, Debug)]
pub struct MetricDetails {
    pub name: String,
    #[allow(dead_code)]
    pub docstring: String,
    pub metric_type: MetricType,
}

impl Metric {
    pub fn update_time_series(&mut self, value_per_labels: HashMap<String, Sample>) {
        value_per_labels.into_iter().for_each(|(key, value)| {
            if self.time_series.contains_key(&key) {
                self.time_series
                    .get_mut(&key)
                    .expect("should contain the value")
                    .samples
                    .push(value);
            } else {
                add_time_series_into_metric(key, &mut self.time_series, value);
            }
        })
    }

    pub fn get_labels(&self) -> Vec<&String> {
        let mut labels: Vec<&String> = self.time_series.keys().collect();
        labels.sort();
        labels
    }
}

#[derive(Clone, Debug)]
pub struct TimeSeries {
    #[allow(dead_code)]
    pub labels: HashMap<String, String>,
    pub samples: Vec<Sample>,
}

#[derive(Clone, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum Sample {
    GaugeSample(SingleValueSample),
    CounterSample(SingleValueSample),
    HistogramSample(HistogramValueSample),
    SummarySample(SummaryValueSample),
}

#[derive(Clone, Debug)]
pub struct SingleValueSample {
    pub timestamp: u64,
    pub value: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Bucket {
    pub name: String,
    pub value: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Quantil {
    pub name: String,
    pub value: f64,
}

impl Bucket {
    pub fn new(name: String, value: u64) -> Self {
        Self { name, value }
    }
}

#[derive(Clone, Debug)]
pub struct HistogramValueSample {
    pub timestamp: u64,
    pub bucket_values: Vec<Bucket>,
    pub sum: f64,
    pub count: u64,
}

#[derive(Clone, Debug)]
pub struct SummaryValueSample {
    pub time: DateTime<Local>,
    pub quantiles: Vec<Quantil>,
    pub sum: f64,
    pub count: u64,
}

impl<'a> From<&SummaryValueSample> for BarGroup<'a> {
    fn from(val: &SummaryValueSample) -> Self {
        let mut min = f64::MAX;
        let mut max = f64::MIN;
        val.quantiles.iter().for_each(|data_point| {
            if data_point.value > max {
                max = data_point.value;
            }
            if data_point.value < min {
                min = data_point.value;
            }
        });
        //Low level is the bar height for the min
        //value. For non-zero values we want a
        //small bar to be displayed.
        let low_level = if min != 0.0 { 5.0 } else { 0.0 };

        //Scale so we have at least 100 steps
        let scale_span = 100.0 - low_level;

        //3.25, 2.25
        let bars: Vec<Bar> = val
            .quantiles
            .iter()
            .map(|m| {
                let percent = (m.value - min) / (max - min);
                let new_val = (percent * scale_span) + low_level;
                Bar::default()
                    .value(new_val.round() as u64)
                    .text_value(format_value(m.value))
                    .label(m.name.clone().into())
            })
            .collect();
        BarGroup::default().bars(&bars)
    }
}

fn add_time_series_into_metric(
    labels: String,
    time_series: &mut HashMap<String, TimeSeries>,
    sample: Sample,
) {
    let mut labels_map = HashMap::new();
    let key;
    if labels.contains('=') {
        (labels_map, key) = extract_labels_key_and_map(Some(labels));
    } else {
        key = labels;
        labels_map.insert("key".to_string(), "value".to_string());
    }

    time_series.insert(
        key,
        TimeSeries {
            labels: labels_map,
            samples: vec![sample],
        },
    );
}

#[cfg(test)]
mod tests {
    use crate::prom::{
        parser::{decode_single_scrape_metric, split_metric_lines},
        test_data::generate_metric_lines,
    };

    use super::*;

    #[test]
    // TODO eventually at some point this test can be removed. As the logic is tested from the metric scraper.
    fn test_convert_single_scrape_metric_into_metric_and_update_metric() {
        use std::time::{SystemTime, UNIX_EPOCH};

        // simulate first scrape
        let lines = split_metric_lines(generate_metric_lines());
        let mut metrics: Vec<Metric> = Vec::new();
        for part in lines {
            let single_scrape_metric = decode_single_scrape_metric(
                part,
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            );
            let name_to_test = single_scrape_metric.name.clone();
            let labels_to_test = match single_scrape_metric.value_per_labels.keys().next() {
                Some(key) => key.clone(),
                None => String::new(),
            };
            let metric = single_scrape_metric.into_metric();
            assert_eq!(metric.details.name, name_to_test);
            assert_eq!(metric.time_series.contains_key(&labels_to_test), true);
            metrics.push(metric);
        }
        // simulate second scrape
        let lines = split_metric_lines(generate_metric_lines());
        for part in lines {
            let single_scrape_metric = decode_single_scrape_metric(
                part,
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            );
            // update existing metrics
            let metric_to_update_option = metrics
                .iter_mut()
                .find(|m| m.details.name == single_scrape_metric.name);
            match metric_to_update_option {
                Some(metric_to_update) => {
                    metric_to_update.update_time_series(single_scrape_metric.value_per_labels);
                    metric_to_update
                        .time_series
                        .values()
                        .for_each(|time_series| {
                            assert_eq!(time_series.samples.len(), 2);
                        });
                }
                None => {
                    panic!("no additional metric should be added");
                }
            }
        }
    }
}
