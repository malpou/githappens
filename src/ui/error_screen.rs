use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph};

pub fn render(frame: &mut ratatui::Frame, area: Rect, message: &str) {
    let block = Block::default().borders(Borders::ALL).title(" Error ");

    let paragraph = Paragraph::new(format!("\n  {message}\n\n  Press r to retry, q to quit."))
        .block(block)
        .style(Style::default().add_modifier(Modifier::BOLD));

    frame.render_widget(paragraph, area);
}
