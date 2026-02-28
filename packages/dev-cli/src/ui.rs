//! Ratatui rendering — bordered layer blocks, table-aligned service rows,
//! context-sensitive command bar, and action feedback.

use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, MenuTarget, Mode, Status};
use crate::services::{self, Layer, SERVICES};

// ── Colors ──────────────────────────────────────────────────────────

const CYAN: Color = Color::Cyan;
const GREEN: Color = Color::Green;
const YELLOW: Color = Color::Yellow;
const RED: Color = Color::Red;
const DARK_GRAY: Color = Color::DarkGray;
const GRAY: Color = Color::Gray;

// ── Top-level render ────────────────────────────────────────────────

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Outer layout: title | body | command bar
    let chunks = Layout::vertical([
        Constraint::Length(1),  // Title
        Constraint::Min(10),   // Service sections
        Constraint::Length(4), // Command bar + feedback + legend
    ])
    .split(area);

    render_title(frame, chunks[0]);
    render_services(frame, chunks[1], app);
    render_footer(frame, chunks[2], app);
}

// ── Title ───────────────────────────────────────────────────────────

fn render_title(frame: &mut Frame, area: Rect) {
    let now = chrono::Local::now().format("%H:%M:%S").to_string();

    let title = Line::from(vec![
        Span::styled("  Root Editorial Dev", Style::default().bold()),
        Span::raw("  "),
        Span::styled(now, Style::default().fg(DARK_GRAY)),
    ]);

    frame.render_widget(Paragraph::new(title), area);
}

// ── Service sections ────────────────────────────────────────────────

fn render_services(frame: &mut Frame, area: Rect, app: &App) {
    // Count services per layer for dynamic sizing
    let infra_count = SERVICES.iter().filter(|s| s.layer == Layer::Infra).count() as u16;
    let backend_count = SERVICES.iter().filter(|s| s.layer == Layer::Backend).count() as u16;
    let frontend_count = SERVICES.iter().filter(|s| s.layer == Layer::Frontend).count() as u16;

    let chunks = Layout::vertical([
        Constraint::Length(infra_count + 2),    // +2 for block borders
        Constraint::Length(backend_count + 2),
        Constraint::Length(frontend_count + 2),
        Constraint::Min(0),                     // absorb remaining space
    ])
    .split(area);

    render_layer_block(frame, chunks[0], Layer::Infra, app);
    render_layer_block(frame, chunks[1], Layer::Backend, app);
    render_layer_block(frame, chunks[2], Layer::Frontend, app);
}

fn render_layer_block(frame: &mut Frame, area: Rect, layer: Layer, app: &App) {
    let is_active = match app.mode {
        Mode::LayerAction(MenuTarget::Layer(l)) => l == layer,
        Mode::LayerAction(MenuTarget::All) => true,
        Mode::Main => false,
    };

    let border_style = if is_active {
        Style::default().fg(CYAN)
    } else {
        Style::default().fg(DARK_GRAY)
    };

    // Build title: "▸ Backend" or " Infrastructure [i]"
    let title = if is_active {
        format!(" ▸ {} ", layer.label())
    } else {
        format!(" {} [{}] ", layer.label(), layer.key_hint())
    };

    let title_style = if is_active {
        Style::default().fg(CYAN).bold()
    } else {
        Style::default().bold()
    };

    // Inline action hint on the right side when not active
    let right_title = if !is_active && matches!(app.mode, Mode::Main) {
        let ops = if layer.has_rebuild() {
            "start · stop · restart · rebuild"
        } else {
            "start · stop · restart"
        };
        format!(" {ops} ")
    } else {
        String::new()
    };

    let block = Block::bordered()
        .title(Line::styled(title, title_style))
        .title(
            Line::styled(right_title, Style::default().fg(DARK_GRAY))
                .alignment(Alignment::Right),
        )
        .border_type(BorderType::Rounded)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Service rows as a table
    let services: Vec<_> = SERVICES.iter().filter(|s| s.layer == layer).collect();

    let rows: Vec<Row> = services
        .iter()
        .map(|svc| {
            let state = app
                .states
                .get(&svc.id)
                .cloned()
                .unwrap_or_default();

            let (dot, dot_style) = status_dot(state.status);
            let cpu_text = state.cpu.as_deref().unwrap_or("--");
            let hint_text = state.hint.clone().unwrap_or_default();
            let hint_color = match state.status {
                Status::Fail => RED,
                Status::Starting => YELLOW,
                _ => DARK_GRAY,
            };

            Row::new(vec![
                Cell::from(Span::styled(dot, dot_style)),
                Cell::from(svc.label),
                Cell::from(format!("localhost:{}", svc.display_port))
                    .style(if state.status == Status::Ok {
                        Style::default()
                    } else {
                        Style::default().fg(DARK_GRAY)
                    }),
                Cell::from(format!("cpu: {cpu_text}")).style(cpu_style(cpu_text)),
                Cell::from(Span::styled(hint_text, Style::default().fg(hint_color))),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(2),  // Status dot
            Constraint::Length(18), // Service name
            Constraint::Length(16), // Port
            Constraint::Length(12), // CPU
            Constraint::Min(0),    // Hint (diagnostic info for non-ok services)
        ],
    )
    .column_spacing(1);

    frame.render_widget(table, inner);
}

fn status_dot(status: Status) -> (&'static str, Style) {
    match status {
        Status::Ok => ("●", Style::default().fg(GREEN)),
        Status::Starting => ("●", Style::default().fg(YELLOW)),
        Status::Fail => ("●", Style::default().fg(RED)),
        Status::Stopped => ("○", Style::default().fg(DARK_GRAY)),
    }
}

fn cpu_style(cpu: &str) -> Style {
    let num: f64 = cpu.trim_end_matches('%').parse().unwrap_or(0.0);
    if num > 100.0 {
        Style::default().fg(RED).bold()
    } else if num > 20.0 {
        Style::default().fg(YELLOW)
    } else {
        Style::default().fg(DARK_GRAY)
    }
}

// ── Footer: command bar + feedback + legend ─────────────────────────

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(1), // Command bar
        Constraint::Length(1), // Action feedback / pending op
        Constraint::Length(1), // Legend
        Constraint::Min(0),
    ])
    .split(area);

    // Command bar
    let cmd_line = match &app.mode {
        Mode::Main => main_menu_line(),
        Mode::LayerAction(target) => submenu_line(target),
    };
    frame.render_widget(Paragraph::new(cmd_line), chunks[0]);

    // Action feedback / pending operation
    let feedback = if let Some((desc, started)) = &app.pending_op {
        let dots = animated_dots(started.elapsed().as_millis());
        Line::from(Span::styled(
            format!("  {desc}{dots}"),
            Style::default().fg(YELLOW),
        ))
    } else if let Some((msg, created)) = &app.action_msg {
        if created.elapsed().as_secs() < 15 {
            Line::from(Span::styled(
                format!("  {msg}"),
                Style::default().fg(GREEN),
            ))
        } else {
            Line::default()
        }
    } else {
        Line::default()
    };
    frame.render_widget(Paragraph::new(feedback), chunks[1]);

    // Legend
    let mut legend_spans = vec![
        Span::styled("  cpu: ", Style::default().fg(DARK_GRAY)),
        Span::styled(">100%", Style::default().fg(RED).bold()),
        Span::styled("  >20%", Style::default().fg(YELLOW)),
        Span::styled("  normal", Style::default().fg(DARK_GRAY)),
        Span::styled("  Open: ", Style::default().fg(DARK_GRAY)),
    ];
    for (i, svc) in services::url_services().iter().enumerate() {
        legend_spans.push(key_span(&format!("{}", i + 1)));
        legend_spans.push(Span::styled(
            format!("{} ", svc.label),
            Style::default().fg(DARK_GRAY),
        ));
    }
    let legend = Line::from(legend_spans);
    frame.render_widget(Paragraph::new(legend), chunks[2]);
}

fn main_menu_line() -> Line<'static> {
    Line::from(vec![
        Span::raw("  "),
        key_span("i"),
        Span::raw(" Infra  "),
        key_span("b"),
        Span::raw(" Backend  "),
        key_span("f"),
        Span::raw(" Frontend  "),
        key_span("a"),
        Span::raw(" All    "),
        key_span("d"),
        Span::raw(" reset db  "),
        key_span("l"),
        Span::raw(" logs  "),
        key_span("q"),
        Span::raw(" quit"),
    ])
}

fn submenu_line(target: &MenuTarget) -> Line<'static> {
    let label = target.label().to_string();
    let mut spans = vec![
        Span::styled(format!("  {label}"), Style::default().fg(CYAN).bold()),
        Span::styled(" ▸ ", Style::default().fg(CYAN)),
        key_span("s"),
        Span::raw(" start  "),
        key_span("x"),
        Span::raw(" stop  "),
        key_span("r"),
        Span::raw(" restart  "),
    ];

    if target.has_rebuild() && !matches!(target, MenuTarget::Layer(Layer::Infra)) {
        spans.push(key_span("b"));
        spans.push(Span::raw(" rebuild  "));
    }

    spans.push(key_span("l"));
    spans.push(Span::raw(" logs  "));
    spans.push(key_span("esc"));
    spans.push(Span::raw(" back"));

    Line::from(spans)
}

fn key_span(key: &str) -> Span<'static> {
    Span::styled(
        format!("[{key}]"),
        Style::default().fg(GRAY),
    )
}

fn animated_dots(elapsed_ms: u128) -> &'static str {
    match (elapsed_ms / 400) % 4 {
        0 => "",
        1 => ".",
        2 => "..",
        _ => "...",
    }
}
