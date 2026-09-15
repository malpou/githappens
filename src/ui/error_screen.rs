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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn render_error_screen() {
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| render(f, f.area(), "Something went wrong"))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let text = buffer
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(text.contains("Error"));
        assert!(text.contains("Something went wrong"));
        assert!(text.contains("Press r to retry"));
    }
}
