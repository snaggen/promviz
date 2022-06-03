use tui::{
    backend::Backend,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

pub fn render<B: Backend>(f: &mut Frame<B>, metrics: &mut crate::model::MetricStore) {
    let size = f.size();

    let items: Vec<ListItem> = metrics
        .items
        .iter()
        .map(|i| ListItem::new(i.as_ref()))
        .collect();

    let list = List::new(items)
        .block(Block::default().title("Metrics").borders(Borders::ALL))
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .bg(Color::White)
                .fg(Color::Black)
                .add_modifier(Modifier::ITALIC),
        );

    f.render_stateful_widget(list, size, &mut metrics.state);
}
