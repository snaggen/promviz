use log::error;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::Span,
    widgets::{Axis, BarChart, Block, Borders, Chart, Dataset, GraphType, Row, Table, TableState},
    Frame,
};

use crate::prom::{Metric, MetricType, Sample};
use chrono::prelude::*;

use super::{graph_data::GraphData, histogram_data::HistogramData};

pub fn draw(
    f: &mut Frame,
    chunk_right: Rect,
    chunk_left: Rect,
    metric: &Metric,
    selected_label: &str,
) {
    match metric.details.metric_type {
        MetricType::Histogram => {
            if let Some(histogram_data) = HistogramData::parse(metric, selected_label) {
                draw_histogram_table(f, chunk_left, &histogram_data);
                draw_histogram(f, chunk_right, &histogram_data);
            }
        }
        _ => {
            if let Some(graph_data) = GraphData::parse(metric, selected_label) {
                draw_graph(f, chunk_right, &graph_data);
            }
            draw_table(f, chunk_left, metric, selected_label);
        }
    }
}

#[allow(clippy::cast_precision_loss)]
fn draw_table(f: &mut Frame, area: Rect, metric: &Metric, selected_label: &str) {
    let samples = &metric
        .time_series
        .get(selected_label)
        .expect("values for selected label")
        .samples;
    let title = format!("History ({})", samples.len());

    let rows = samples.iter().map(|entry| {
        let (timestamp, value) = match entry {
            Sample::GaugeSample(single_value) => (single_value.timestamp, single_value.value),
            Sample::CounterSample(single_value) => (single_value.timestamp, single_value.value),
            _ => {
                error!("History table is not implemented for this kind of sample.");
                unimplemented!();
            }
        };
        let time = Local.timestamp(timestamp as i64, 0).to_rfc2822();
        Row::new(vec![time, format!("{:+.4e}", value)])
    });

    let t = Table::new(
        rows,
        &[
            Constraint::Length(50),
            Constraint::Length(15),
            Constraint::Percentage(100),
        ],
    )
    .block(Block::default().borders(Borders::ALL).title(title))
    .header(Row::new(vec!["Time", "Value"]).style(Style::default().add_modifier(Modifier::BOLD)))
    .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    let mut state = TableState::default();
    state.select(Some(samples.len() - 1));

    f.render_stateful_widget(t, area, &mut state);
}

fn draw_graph(f: &mut Frame, area: Rect, points: &GraphData) {
    let datasets = vec![Dataset::default()
        .marker(symbols::Marker::Braille)
        .style(Style::default().fg(Color::LightGreen))
        .graph_type(GraphType::Line)
        .data(&points.data)];

    let mut five_percent_span = (points.y_max - points.y_min) * 0.05;
    if five_percent_span == 0.0 {
        five_percent_span = 1.0;
    }
    let y_min_axis = points.y_min - five_percent_span;
    let y_max_axis = points.y_max + five_percent_span;
    let y_labels = if y_max_axis < 0.0001 && y_min_axis < 0.0001 {
        vec![
            Span::raw(format!("{0:.1$e}", y_min_axis, 4)),
            Span::raw(format!("{0:.1$e}", y_max_axis, 4)),
        ]
    } else {
        //Use 3 digits precision unless for close to integer values
        let y_min_prec = if (y_min_axis - y_min_axis.floor()) < 0.0001 {
            0
        } else {
            3
        };
        let y_max_prec = if (y_max_axis - y_max_axis.floor()) < 0.0001 {
            0
        } else {
            3
        };
        vec![
            Span::raw(format!("{:.1$}", y_min_axis, y_min_prec)),
            Span::raw(format!("{:.1$}", y_max_axis, y_max_prec)),
        ]
    };

    let chart = Chart::new(datasets)
        .block(Block::default().title("Graph").borders(Borders::ALL))
        .x_axis(
            Axis::default()
                .labels(vec![
                    Span::raw(points.first_time.format("%H:%M:%S").to_string()),
                    Span::raw(points.last_time.format("%H:%M:%S").to_string()),
                ])
                .bounds([points.x_min, points.x_max]),
        )
        .y_axis(Axis::default().labels(y_labels).bounds([
            points.y_min - five_percent_span,
            points.y_max + five_percent_span,
        ]));
    f.render_widget(chart, area);
}

fn draw_histogram_table(f: &mut Frame, area: Rect, histogram_data: &HistogramData) {
    let chunks = Layout::default()
        .constraints([Constraint::Percentage(25), Constraint::Min(8)].as_ref())
        .split(area);

    // Draw histogram details
    let title_details = "Histogram Details".to_string();

    let row_details = [Row::new(vec![
        histogram_data.time.to_rfc2822(),
        histogram_data.count.to_string(),
        format!("{:.2}", histogram_data.sum),
    ])];

    let t = Table::new(
        row_details,
        &[
            Constraint::Length(40),
            Constraint::Length(15),
            Constraint::Length(15),
            Constraint::Percentage(100),
        ],
    )
    .block(Block::default().borders(Borders::ALL).title(title_details))
    .header(
        Row::new(vec!["Time", "Count", "Sum"]).style(Style::default().add_modifier(Modifier::BOLD)),
    )
    .highlight_style(Style::default().add_modifier(Modifier::BOLD));
    f.render_widget(t, chunks[0]);

    // Draw histogram buckets details
    let title = "Histogram Buckets Details".to_string();

    let rows = histogram_data.data.iter().map(|entry| {
        Row::new(vec![
            entry.get_bucket().clone(),
            entry.get_value().to_string(),
            format!("{:.2}", entry.get_percentage()),
            entry.get_inc_per_bucket().to_string(),
            format!("{:.2}", entry.get_inc_per_bucket_percentage()),
        ])
    });

    let t = Table::new(
        rows,
        &[
            Constraint::Length(15),
            Constraint::Length(15),
            Constraint::Length(15),
            Constraint::Length(15),
            Constraint::Percentage(100),
        ],
    )
    .block(Block::default().borders(Borders::ALL).title(title))
    .header(
        Row::new(vec!["Bucket", "Count", "Count %", "Inc", "Inc %"])
            .style(Style::default().add_modifier(Modifier::BOLD)),
    )
    .highlight_style(Style::default().add_modifier(Modifier::BOLD));
    f.render_widget(t, chunks[1]);
}

fn draw_histogram(f: &mut Frame, area: Rect, histogram_data: &HistogramData) {
    let data: Vec<(&str, u64)> = histogram_data
        .data
        .iter()
        .map(|bucket_value| {
            (
                bucket_value.get_bucket().as_str(),
                bucket_value.get_inc_per_bucket(),
            )
        })
        .collect();
    let bar_width = area.width / (data.len() + 1) as u16;
    let t = BarChart::default()
        .block(Block::default().title("Histogram").borders(Borders::ALL))
        .data(&data)
        .bar_width(bar_width)
        .bar_style(Style::default().fg(Color::LightGreen))
        .value_style(Style::default().fg(Color::Black).bg(Color::LightGreen));
    f.render_widget(t, area);
}
