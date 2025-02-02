use chrono::{Local, TimeZone};
use regex::Regex;

use super::model::{Bucket, MetricType, Quantil, SingleScrapeMetric, SummaryValueSample};
use super::Sample;
use super::{HistogramValueSample, SingleValueSample};
use log::error;
use std::collections::HashMap;
use std::slice::Iter;

pub fn decode_single_scrape_metric(lines: Vec<String>, timestamp: u64) -> SingleScrapeMetric {
    let mut lines_iter = lines.iter();
    let mut doc_name: Option<String> = None;
    let mut docstring: Option<String> = None;
    let mut type_name: Option<String> = None;
    let mut metric_type: Option<String> = None;
    #[allow(clippy::while_let_on_iterator)]
    while let Some(line) = lines_iter.next() {
        if line.starts_with("# HELP ") {
            if let Some((name, docstr)) = extract_name_docstring(line) {
                doc_name = Some(name);
                docstring = Some(docstr);
            }
        } else if line.starts_with("# TYPE ") {
            if let Some((tmp_name, tmp_type)) = extract_type(line) {
                type_name = Some(tmp_name);
                metric_type = Some(tmp_type);
            }
            break;
        } else if !line.starts_with('#') {
            panic!("Invalid metric data, TYPE must be present");
        }
    }
    let name = doc_name.unwrap_or(type_name.expect("No name found for metric"));
    let metric_type = metric_type.expect("TYPE not set for metric");
    let docstring = docstring.unwrap_or_default();
    let mut single_scrape_metric = SingleScrapeMetric {
        name,
        docstring,
        metric_type: MetricType::Gauge,
        value_per_labels: HashMap::new(),
    };
    match metric_type.as_str() {
        "gauge" => {
            for line in lines_iter {
                if line.is_empty() {
                    continue;
                }
                let labels = extract_labels(line);
                let (_, key) = extract_labels_key_and_map(labels);
                let value = extract_value(line);
                single_scrape_metric.value_per_labels.insert(
                    key,
                    Sample::GaugeSample(SingleValueSample { timestamp, value }),
                );
            }
        }
        "counter" => {
            for line in lines_iter {
                if line.is_empty() {
                    continue;
                }
                let labels = extract_labels(line);
                let (_, key) = extract_labels_key_and_map(labels);
                let value = extract_value(line);
                single_scrape_metric.metric_type = MetricType::Counter;
                single_scrape_metric.value_per_labels.insert(
                    key,
                    Sample::CounterSample(SingleValueSample { timestamp, value }),
                );
            }
        }
        // TODO handle also histogram with no additional labels
        "histogram" => {
            let splitted_lines_for_histogram = further_split_metric_lines_for_histogram(lines_iter);
            for group_lines in splitted_lines_for_histogram.iter() {
                let mut bucket_values = Vec::new();
                // retrieve buckets values
                for line in group_lines.iter().take(group_lines.len() - 2) {
                    let labels = extract_labels(line);
                    let (labels_map, _) = extract_labels_key_and_map(labels);
                    let bucket_value = labels_map.get("le").unwrap();
                    let value = extract_value(line);
                    bucket_values.push(Bucket::new(bucket_value.clone(), value as u64));
                }
                // retrieve sum value
                let sum = extract_value(&group_lines[group_lines.len() - 2]);
                // retrieve count value and labels
                let count_line = group_lines[group_lines.len() - 1].clone();
                let labels = extract_labels(&count_line);
                let (_, key) = extract_labels_key_and_map(labels);
                let count = extract_value(&count_line) as u64;
                single_scrape_metric.metric_type = MetricType::Histogram;
                single_scrape_metric.value_per_labels.insert(
                    key,
                    Sample::HistogramSample(HistogramValueSample {
                        timestamp,
                        bucket_values,
                        sum,
                        count,
                    }),
                );
            }
        }
        "summary" => {
            let splitted_lines_for_histogram = further_split_metric_lines_for_histogram(lines_iter);
            for group_lines in splitted_lines_for_histogram.iter() {
                let mut quantiles = Vec::new();
                // retrieve buckets values
                for line in group_lines.iter().take(group_lines.len() - 2) {
                    let labels = extract_labels(line);
                    let (labels_map, _) = extract_labels_key_and_map(labels);
                    let bucket_value = labels_map.get("quantile").unwrap();
                    let value = extract_value(line);
                    quantiles.push(Quantil {
                        name: bucket_value.clone(),
                        value,
                    });
                }
                // retrieve sum value
                let sum = extract_value(&group_lines[group_lines.len() - 2]);
                // retrieve count value and labels
                let count_line = group_lines[group_lines.len() - 1].clone();
                let labels = extract_labels(&count_line);
                let (_, key) = extract_labels_key_and_map(labels);
                let count = extract_value(&count_line) as u64;
                single_scrape_metric.metric_type = MetricType::Summary;
                let time = Local.timestamp_opt(timestamp as i64, 0).unwrap();
                single_scrape_metric.value_per_labels.insert(
                    key,
                    Sample::SummarySample(SummaryValueSample {
                        time,
                        quantiles,
                        sum,
                        count,
                    }),
                );
            }
        }
        _ => {
            error!("invalid metric type: {}", metric_type);
        }
    }
    single_scrape_metric
}

pub fn extract_labels_key_and_map(labels: Option<String>) -> (HashMap<String, String>, String) {
    match labels {
        Some(labels) => (decode_labels(&labels), labels),
        None => (
            HashMap::from([("key".to_string(), "single-value-with-no-labels".to_string())]),
            String::from("single-value-with-no-labels"),
        ),
    }
}

pub fn split_metric_lines(lines: Vec<String>) -> Vec<Vec<String>> {
    let mut metrics: Vec<Vec<String>> = Vec::new();
    let mut metric_lines: Vec<String> = Vec::new();

    for (index, line) in lines.iter().enumerate() {
        if !metric_lines.is_empty()
            && (index + 1 == lines.len()
                || (!line.starts_with('#') && lines[index + 1].starts_with('#')))
        {
            metric_lines.push(line.to_string());
            metrics.push(metric_lines);
            metric_lines = Vec::new();
            continue;
        }
        metric_lines.push(line.to_string());
    }

    metrics
}

pub fn further_split_metric_lines_for_histogram(lines_iter: Iter<'_, String>) -> Vec<Vec<String>> {
    let mut metrics: Vec<Vec<String>> = Vec::new();
    let mut metric_lines: Vec<String> = Vec::new();

    for line in lines_iter {
        if line.contains("_count{") || line.contains("_count ") {
            metric_lines.push(line.to_string());
            metrics.push(metric_lines);
            metric_lines = Vec::new();
            continue;
        }
        metric_lines.push(line.to_string());
    }
    metrics
}

fn extract_name_docstring(line: &str) -> Option<(String, String)> {
    if let Some(name_desc) = line.strip_prefix("# HELP ") {
        name_desc
            .split_once(' ')
            .map(|(n, d)| (n.to_string(), d.trim().to_string()))
    } else {
        None
    }
}

fn extract_type(line: &str) -> Option<(String, String)> {
    if let Some(line) = line.strip_prefix("# TYPE ") {
        line.rsplit_once(' ')
            .map(|(name, metric_type)| (name.to_string(), metric_type.trim().to_string()))
    } else {
        None
    }
}

pub fn extract_labels(line: &str) -> Option<String> {
    match line.find('{') {
        Some(firs_index) => match line.find('}') {
            Some(second_index) => {
                let labels = line
                    .split_at(firs_index + 1)
                    .1
                    .split_at(second_index - firs_index - 1)
                    .0;
                Some(String::from(labels))
            }
            None => None,
        },
        None => None,
    }
}

#[allow(dead_code)]
pub fn extract_labels_with_rgx(line: &str) -> Option<String> {
    let regex = Regex::new(r"\{(.*?)}").unwrap();
    if let Some(caps) = regex.captures_iter(line).next() {
        return Some(caps[1].to_string());
    }
    None
}

pub fn decode_labels(labels: &str) -> HashMap<String, String> {
    let parts: Vec<String> = labels
        .split(',')
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let mut labels = HashMap::new();
    for label in parts {
        let split: Vec<&str> = label.split('=').collect();
        if split.len() != 2 {
            error!("failed to split this value: {:?}", split);
            continue;
        }

        let key_value: Vec<String> = split
            .iter()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let value = key_value[1].clone().replace('"', "");
        labels.insert(key_value[0].clone(), value);
    }
    labels
}

#[allow(dead_code)]
pub fn decode_labels_with_rgx(labels_to_split: &str) -> HashMap<String, String> {
    let regex = Regex::new(r#"(\w+)="(\w+)""#).unwrap(); // using the global "/g" mode to capture all the occurrences without stopping at the first match
    let mut labels = HashMap::new();
    for cap in regex.captures_iter(labels_to_split) {
        labels.insert(cap[1].to_string(), cap[2].to_string());
    }
    labels
}

fn extract_value(line: &str) -> f64 {
    line.split_whitespace()
        .last()
        .unwrap()
        .parse::<f64>()
        .unwrap()
}

#[cfg(test)]
mod tests {
    use crate::prom::test_data::generate_metric_lines;

    use super::*;

    #[test]
    fn test_decode_labels() {
        let labels = decode_labels(&String::from("key1=\"value1\",key2=\"0\""));
        assert_eq!(labels.keys().count(), 2);
        assert_eq!(labels.get("key1").unwrap(), "value1");
        assert_eq!(labels.get("key2").unwrap(), "0");
    }

    #[test]
    fn test_extract_name_docstring() {
        let line = String::from("# HELP metric_1 Description of the metric");
        let name_desc = extract_name_docstring(&line);
        match name_desc {
            Some((name, description)) => {
                assert_eq!(name, "metric_1");
                assert_eq!(description, "Description of the metric");
            }
            None => panic!("Failed to extract name and description"),
        }
    }

    #[test]
    fn test_extract_type() {
        let line = String::from("# TYPE vectorized_pandaproxy_request_latency histogram");
        let (_name, metric_type) = extract_type(&line).expect("Failed to extract metric type");
        assert_eq!(metric_type, "histogram");
    }

    #[test]
    fn test_split_metric_lines() {
        let lines = generate_metric_lines();
        let splitted_lines = split_metric_lines(lines);
        assert_eq!(splitted_lines.len(), 6);
        assert_eq!(splitted_lines[0].len(), 3);
        assert_eq!(splitted_lines[1].len(), 3);
        assert_eq!(splitted_lines[2].len(), 3);
        assert_eq!(splitted_lines[3].len(), 3);
        assert_eq!(splitted_lines[4].len(), 22);
        assert_eq!(splitted_lines[5].len(), 12);
    }

    #[test]
    fn test_further_split_metric_lines_for_histogram() {
        let lines = generate_metric_lines();
        let splitted_lines = split_metric_lines(lines);
        let further_splitted_metrics_for_hist =
            further_split_metric_lines_for_histogram(splitted_lines[4][2..].iter());
        assert_eq!(further_splitted_metrics_for_hist.len(), 2);
        assert_eq!(further_splitted_metrics_for_hist[0].len(), 10);
        assert_eq!(further_splitted_metrics_for_hist[1].len(), 10);
        let further_splitted_metrics_for_hist =
            further_split_metric_lines_for_histogram(splitted_lines[5][2..].iter());
        assert_eq!(further_splitted_metrics_for_hist.len(), 1);
        assert_eq!(further_splitted_metrics_for_hist[0].len(), 10);
    }

    #[test]
    fn test_extract_labels() {
        let mut lines = Vec::new();
        lines.push(String::from("metric_1{shard=\"0\"} 10.000007"));
        lines.push(String::from("metric_2{shard=\"0\",label1=\"test1\"} 5"));
        lines.push(String::from("incoming_requests 10"));
        let line = &lines[0];
        let labels = extract_labels(&line);
        match labels {
            Some(labels) => {
                assert_eq!(labels, "shard=\"0\"");
            }
            None => panic!("Failed to extract labels"),
        }
        let line = &lines[1];
        let labels = extract_labels(&line);
        match labels {
            Some(labels) => {
                assert_eq!(labels, "shard=\"0\",label1=\"test1\"");
            }
            None => panic!("Failed to extract labels"),
        }
        let line = &lines[2];
        let labels = extract_labels(&line);
        match labels {
            Some(_) => {
                panic!("Should have not extracted any label");
            }
            None => (),
        }
    }

    #[test]
    fn test_decode_metric() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let mut lines = Vec::new();
        lines.push(String::from("# HELP metric_1 Description of the metric"));
        lines.push(String::from("# TYPE metric_1 gauge"));
        lines.push(String::from("metric_1{shard=\"0\"} 10.000007"));
        // insert to check if empty lines can be handled
        lines.push(String::from(""));
        let metric = decode_single_scrape_metric(
            lines,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        );
        assert_eq!(metric.name, "metric_1");
    }

    #[test]
    fn test_decode_single_scrape_metric() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let mut lines = Vec::new();
        lines.push(String::from("# HELP metric_1 Description of the metric"));
        lines.push(String::from("# TYPE metric_1 gauge"));
        lines.push(String::from("metric_1{shard=\"0\"} 10.000007"));
        // insert to check if empty lines can be handled
        lines.push(String::from(""));
        let metric = decode_single_scrape_metric(
            lines,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        );
        assert_eq!(metric.name, "metric_1");
    }
    #[test]
    fn test_decode_single_scrape_metric_with_histogram() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let mut lines = Vec::new();
        lines.push(String::from("# HELP response_time Response Times"));
        lines.push(String::from("# TYPE response_time histogram"));
        lines.push(String::from(
            "response_time_bucket{env=\"production\",le=\"0.005\"} 3",
        ));
        lines.push(String::from(
            "response_time_bucket{env=\"production\",le=\"0.01\"} 4",
        ));
        lines.push(String::from(
            "response_time_bucket{env=\"production\",le=\"0.025\"} 13",
        ));
        lines.push(String::from(
            "response_time_bucket{env=\"production\",le=\"+Inf\"} 6563",
        ));
        lines.push(String::from(
            "response_time_sum{env=\"production\"} 32899.06535799631",
        ));
        lines.push(String::from("response_time_count{env=\"production\"} 6563"));
        lines.push(String::from(
            "response_time_bucket{env=\"testing\",le=\"0.005\"} 4",
        ));
        lines.push(String::from(
            "response_time_bucket{env=\"testing\",le=\"0.01\"} 4",
        ));
        lines.push(String::from(
            "response_time_bucket{env=\"testing\",le=\"0.025\"} 13",
        ));
        lines.push(String::from(
            "response_time_bucket{env=\"testing\",le=\"+Inf\"} 6451",
        ));
        lines.push(String::from(
            "response_time_sum{env=\"testing\"} 32157.055112958977",
        ));
        lines.push(String::from("response_time_count{env=\"testing\"} 6451"));
        // insert to check if empty lines can be handled
        lines.push(String::from(""));
        let metric = decode_single_scrape_metric(
            lines,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        );
        assert_eq!(metric.name, "response_time");
        let metric_hist_1 = metric.value_per_labels.get("env=\"production\"").unwrap();
        let expected_1 = Vec::from([
            Bucket::new(String::from("0.005"), 3),
            Bucket::new(String::from("0.01"), 4),
            Bucket::new(String::from("0.025"), 13),
            Bucket::new(String::from("+Inf"), 6563),
        ]);
        let metric_hist_2 = metric.value_per_labels.get("env=\"testing\"").unwrap();
        let expected_2 = Vec::from([
            Bucket::new(String::from("0.005"), 4),
            Bucket::new(String::from("0.01"), 4),
            Bucket::new(String::from("0.025"), 13),
            Bucket::new(String::from("+Inf"), 6451),
        ]);
        match metric_hist_1 {
            Sample::HistogramSample(hist_metric_value) => {
                assert_eq!(hist_metric_value.bucket_values, expected_1);
                assert_eq!(hist_metric_value.sum, 32899.06535799631);
                assert_eq!(hist_metric_value.count, 6563);
            }
            _ => panic!("Failed to decode histogram"),
        }
        match metric_hist_2 {
            Sample::HistogramSample(hist_metric_value) => {
                assert_eq!(hist_metric_value.bucket_values, expected_2);
                assert_eq!(hist_metric_value.sum, 32157.055112958977);
                assert_eq!(hist_metric_value.count, 6451);
            }
            _ => panic!("Failed to decode histogram"),
        }
    }
    #[test]
    fn test_decode_single_scrape_metric_with_histogram_with_no_labels() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let mut lines = Vec::new();
        lines.push(String::from("# HELP response_time Response Times"));
        lines.push(String::from("# TYPE response_time histogram"));
        lines.push(String::from("response_time_bucket{le=\"0.005\"} 3"));
        lines.push(String::from("response_time_bucket{le=\"0.01\"} 4"));
        lines.push(String::from("response_time_bucket{le=\"0.025\"} 13"));
        lines.push(String::from("response_time_bucket{le=\"+Inf\"} 6563"));
        lines.push(String::from("response_time_sum 32899.06535799631"));
        lines.push(String::from("response_time_count 6563"));
        // insert to check if empty lines can be handled
        lines.push(String::from(""));
        let metric = decode_single_scrape_metric(
            lines,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        );
        assert_eq!(metric.name, "response_time");
        let metric_hist_1 = metric
            .value_per_labels
            .get("single-value-with-no-labels")
            .unwrap();
        let expected_1 = Vec::from([
            Bucket::new(String::from("0.005"), 3),
            Bucket::new(String::from("0.01"), 4),
            Bucket::new(String::from("0.025"), 13),
            Bucket::new(String::from("+Inf"), 6563),
        ]);
        match metric_hist_1 {
            Sample::HistogramSample(hist_metric_value) => {
                assert_eq!(hist_metric_value.bucket_values, expected_1);
                assert_eq!(hist_metric_value.sum, 32899.06535799631);
                assert_eq!(hist_metric_value.count, 6563);
            }
            _ => panic!("Failed to decode histogram"),
        }
    }
}
