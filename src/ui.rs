use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    symbols::border,
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, List, ListItem, Paragraph, Wrap,
    },
    Frame,
};

use unicode_width::UnicodeWidthStr;

use crate::app::{App, Mode};

// ── Color palette (refined cyberpunk theme) ────────────────────────────────

const BG: Color = Color::Rgb(0x0a, 0x0e, 0x0a);
const SURFACE: Color = Color::Rgb(0x14, 0x1c, 0x14);
const PRIMARY: Color = Color::Rgb(0x00, 0xff, 0xaa);
const PRIMARY_DIM: Color = Color::Rgb(0x00, 0xbb, 0x77);
const ACCENT: Color = Color::Rgb(0x44, 0xff, 0x88);
const BODY: Color = Color::Rgb(0xd0, 0xe0, 0xd0);
const DIM: Color = Color::Rgb(0x55, 0x77, 0x55);
const DIM_BRIGHT: Color = Color::Rgb(0x66, 0x88, 0x66);
const ERROR: Color = Color::Rgb(0xff, 0x33, 0x55);
const WARN: Color = Color::Rgb(0xff, 0xaa, 0x00);
const INFO: Color = Color::Rgb(0x44, 0xbb, 0xff);
const TAB_ACTIVE_BG: Color = Color::Rgb(0x00, 0xcc, 0x77);
const TAB_ACTIVE_FG: Color = Color::Rgb(0x00, 0x00, 0x00);
const TAB_INACTIVE_BG: Color = Color::Rgb(0x12, 0x2a, 0x12);
const HIGHLIGHT_BG: Color = Color::Rgb(0xff, 0xff, 0x44);
const HIGHLIGHT_FG: Color = Color::Rgb(0x00, 0x00, 0x00);
const BORDER_DIM: Color = Color::Rgb(0x22, 0x3a, 0x22);

// ── Main draw ──────────────────────────────────────────────────────────────

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    if area.width < 20 || area.height < 6 {
        return;
    }

    // Re-wrap if terminal size changed
    if app.active_tab().scroll_max == 0
        || area.width as usize
            != app
                .active_tab()
                .display_lines
                .get(0)
                .map(|_| area.width as usize)
                .unwrap_or(0)
    {
        let w = area.width as usize;
        for tab in &mut app.tabs {
            tab.wrap_content(w);
        }
    }

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area);

    draw_tab_bar(frame, layout[0], app);
    draw_url_bar(frame, layout[1], app);
    draw_content(frame, layout[2], app);
    draw_status_bar(frame, layout[3], app);

    match app.mode {
        Mode::UrlInput => draw_url_input(frame, area, app),
        Mode::Search => draw_search_input(frame, area, app),
        Mode::Bookmarks => draw_bookmarks(frame, area, app),
        Mode::History => draw_history(frame, area, app),
        Mode::Settings => draw_settings(frame, area, app),
        Mode::Help => draw_help(frame, area),
        Mode::Normal => {}
    }
}

// ── Tab bar ────────────────────────────────────────────────────────────────

fn draw_tab_bar(frame: &mut Frame, area: Rect, app: &App) {
    let bg = Style::default().bg(BG);
    let mut spans: Vec<Span> = Vec::new();

    for (i, tab) in app.tabs.iter().enumerate() {
        let active = i == app.active_tab;

        let prefix = if tab.loading {
            " ◌ "
        } else if active {
            " ▸ "
        } else {
            "   "
        };

        let label = truncate(&tab.title, 18);
        let tab_text = format!("{}{}. {}", prefix, i + 1, label);

        let style = if active {
            Style::default()
                .bg(TAB_ACTIVE_BG)
                .fg(TAB_ACTIVE_FG)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().bg(TAB_INACTIVE_BG).fg(DIM_BRIGHT)
        };

        spans.push(Span::styled(tab_text, style));
        spans.push(Span::styled(" ", bg));
    }

    // New tab button
    let new_tab = Span::styled("  +  ", Style::default().bg(TAB_INACTIVE_BG).fg(DIM));
    spans.push(new_tab);

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line)
        .style(bg)
        .left_aligned();
    frame.render_widget(paragraph, area);
}

// ── URL bar ────────────────────────────────────────────────────────────────

fn draw_url_bar(frame: &mut Frame, area: Rect, app: &App) {
    let url = &app.active_tab().url;

    let is_secure = url.starts_with("https://");
    let is_welcome = url == "about:welcome";
    let is_error = app.active_tab().error.is_some();

    let (icon, icon_color) = if is_welcome {
        (" ◆ ", DIM)
    } else if is_error {
        (" ✗ ", ERROR)
    } else if is_secure {
        (" ◉ ", PRIMARY)
    } else {
        (" ○ ", WARN)
    };

    let style = Style::default().bg(SURFACE);

    let url_style = Style::default()
        .bg(SURFACE)
        .fg(if is_error { ERROR } else { BODY });

    let display = if is_welcome {
        Span::styled("  Tuba 1.1  —  press Ctrl+L to open a URL", Style::default().bg(SURFACE).fg(DIM).italic())
    } else {
        let display_url = if url.chars().count() > area.width as usize - 6 {
            let chars: String = url
                .chars()
                .skip(url.chars().count().saturating_sub(area.width as usize - 7))
                .collect();
            format!("…{}", chars)
        } else {
            url.clone()
        };
        Span::styled(format!(" {}{} ", icon, display_url), url_style)
    };

    let icon_span = Span::styled(icon, Style::default().bg(SURFACE).fg(icon_color));

    let line = if is_welcome {
        Line::from(display)
    } else {
        Line::from(vec![icon_span, display])
    };

    let paragraph = Paragraph::new(line).style(style);
    frame.render_widget(paragraph, area);
}

// ── Content area ───────────────────────────────────────────────────────────

fn draw_content(frame: &mut Frame, area: Rect, app: &App) {
    let tab = app.active_tab();

    if tab.loading {
        draw_loading(frame, area);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    let query = app.search_query.to_lowercase();

    for (i, (line_text, is_title)) in tab.display_lines.iter().enumerate() {
        let base_style = if *is_title {
            Style::default()
                .fg(PRIMARY)
                .add_modifier(Modifier::BOLD)
        } else if line_text.starts_with("⚠") {
            Style::default()
                .fg(ERROR)
                .add_modifier(Modifier::BOLD)
        } else if line_text.starts_with("  [") && line_text.contains(']') {
            Style::default().fg(ACCENT)
        } else if line_text.starts_with("▎") {
            Style::default().fg(Color::Rgb(0xff, 0xdd, 0x44))
        } else if line_text.starts_with("  ─") {
            Style::default().fg(DIM)
        } else {
            Style::default().fg(BODY)
        };

        let is_search_match = app.search_results.iter().any(|(idx, _)| *idx == i);
        let is_current = app
            .search_results
            .get(app.search_idx)
            .map(|(idx, _)| *idx == i)
            .unwrap_or(false);

        if !query.is_empty() && is_search_match {
            let lower = line_text.to_lowercase();
            let mut spans = Vec::new();
            let mut last_end = 0;

            for (m_start, _) in lower.match_indices(&query) {
                if m_start > last_end {
                    spans.push(Span::styled(&line_text[last_end..m_start], base_style));
                }
                let m_end = m_start + query.len();
                let hl_style = if is_current {
                    Style::default()
                        .fg(HIGHLIGHT_FG)
                        .bg(HIGHLIGHT_BG)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                        .fg(HIGHLIGHT_BG)
                        .add_modifier(Modifier::BOLD)
                };
                spans.push(Span::styled(&line_text[m_start..m_end], hl_style));
                last_end = m_end;
            }
            if last_end < line_text.len() {
                spans.push(Span::styled(&line_text[last_end..], base_style));
            }
            lines.push(Line::from(spans));
        } else {
            lines.push(Line::from(Span::styled(line_text.as_str(), base_style)));
        }
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (empty page)",
            Style::default().fg(DIM).italic(),
        )));
    }

    let scroll = tab.scroll.min(tab.scroll_max);

    let paragraph = Paragraph::new(Text::from(lines))
        .style(Style::default().bg(BG))
        .scroll((scroll as u16, 0))
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

fn draw_loading(frame: &mut Frame, area: Rect) {
    let block = Block::default().style(Style::default().bg(BG));

    let lines = vec![
        Line::from(Span::styled(
            "  ◌ Loading...",
            Style::default().fg(PRIMARY_DIM).italic(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Fetching and rendering page...",
            Style::default().fg(DIM),
        )),
    ];

    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .style(Style::default().bg(BG));

    frame.render_widget(paragraph, area);
}

// ── Status bar ─────────────────────────────────────────────────────────────

fn draw_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let tab = app.active_tab();
    let mode_str = match app.mode {
        Mode::Normal => " NORMAL ",
        Mode::UrlInput => " URL ",
        Mode::Search => " SEARCH ",
        Mode::Bookmarks => " BOOKMARKS ",
        Mode::History => " HISTORY ",
        Mode::Settings => " SETTINGS ",
        Mode::Help => " HELP ",
    };

    let mode_style = match app.mode {
        Mode::Normal => Style::default().bg(PRIMARY_DIM).fg(Color::Black).bold(),
        Mode::UrlInput => Style::default().bg(INFO).fg(Color::Black).bold(),
        Mode::Search => Style::default().bg(WARN).fg(Color::Black).bold(),
        Mode::Bookmarks => Style::default().bg(PRIMARY).fg(Color::Black).bold(),
        Mode::History => Style::default().bg(ACCENT).fg(Color::Black).bold(),
        Mode::Settings => Style::default().bg(Color::Rgb(0x88, 0x44, 0xff)).fg(Color::Black).bold(),
        Mode::Help => Style::default().bg(Color::Rgb(0x44, 0x88, 0xff)).fg(Color::Black).bold(),
    };

    let tab_count = format!(" Tab {}/{} ", app.active_tab + 1, app.tabs.len());
    let scroll_pct = if tab.scroll_max > 0 {
        format!(" {}% ", (tab.scroll as f64 / tab.scroll_max as f64 * 100.0) as usize)
    } else {
        " TOP ".into()
    };

    let status = if tab.loading {
        " ◌ loading "
    } else if app.active_tab().error.is_some() {
        " ✗ error "
    } else {
        " ● ready "
    };

    let status_color = if tab.loading {
        WARN
    } else if app.active_tab().error.is_some() {
        ERROR
    } else {
        PRIMARY
    };

    let mut spans: Vec<Span> = Vec::new();

    // Mode indicator
    spans.push(Span::styled(mode_str, mode_style));
    spans.push(Span::styled(" ", Style::default().bg(SURFACE)));

    // Tab info
    spans.push(Span::styled(
        tab_count,
        Style::default().bg(SURFACE).fg(DIM_BRIGHT),
    ));
    spans.push(Span::styled("│", Style::default().bg(SURFACE).fg(BORDER_DIM)));

    // Scroll position
    spans.push(Span::styled(
        format!(" {} ", scroll_pct),
        Style::default().bg(SURFACE).fg(DIM_BRIGHT),
    ));
    spans.push(Span::styled("│", Style::default().bg(SURFACE).fg(BORDER_DIM)));

    // Status
    spans.push(Span::styled(
        status,
        Style::default().bg(SURFACE).fg(status_color).italic(),
    ));
    spans.push(Span::styled("│", Style::default().bg(SURFACE).fg(BORDER_DIM)));

    // Message
    let msg_color = if app.status_message.contains("Error")
        || app.status_message.contains("error")
        || app.status_message.contains("Blocked")
    {
        ERROR
    } else if app.status_message.contains("★")
        || app.status_message.contains("Found")
    {
        WARN
    } else {
        DIM
    };
    spans.push(Span::styled(
        format!(" {} ", app.status_message),
        Style::default().bg(SURFACE).fg(msg_color),
    ));

    // Fill remaining
    spans.push(Span::styled(
        " ".repeat(area.width.saturating_sub(
            spans.iter().map(|s| s.content.width()).sum::<usize>() as u16,
        ) as usize),
        Style::default().bg(SURFACE),
    ));

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).style(Style::default().bg(SURFACE));
    frame.render_widget(paragraph, area);
}

// ── Overlay helpers ────────────────────────────────────────────────────────

fn centered_rect(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let pad_x = (100 - percent_x) as u16;
    let pad_y = (100 - percent_y) as u16;

    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(pad_y / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage(pad_y / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(pad_x / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage(pad_x / 2),
        ])
        .split(vert[1])[1]
}

fn overlay_block(title: &str, border_color: Color) -> Block<'static> {
    Block::default()
        .title(format!(" {} ", title))
        .title_style(Style::default().fg(border_color).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_set(border::ROUNDED)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(SURFACE))
}

// ── URL input overlay ──────────────────────────────────────────────────────

fn draw_url_input(frame: &mut Frame, area: Rect, app: &App) {
    let rect = centered_rect(area, 70, 12);
    frame.render_widget(Clear, rect);

    let block = overlay_block(" URL ", PRIMARY);

    let input = Paragraph::new(app.url_input.as_str())
        .style(Style::default().fg(BODY).bg(SURFACE))
        .block(block);

    frame.render_widget(input, rect);

    let cursor_x = rect.x + text_width(&app.url_input).min(rect.width as usize - 2) as u16 + 1;
    frame.set_cursor_position((cursor_x, rect.y + 1));
}

// ── Search input overlay ───────────────────────────────────────────────────

fn draw_search_input(frame: &mut Frame, area: Rect, app: &App) {
    let rect = centered_rect(area, 50, 10);
    frame.render_widget(Clear, rect);

    let block = overlay_block(" Search ", WARN);

    let input = Paragraph::new(app.search_query.as_str())
        .style(Style::default().fg(BODY).bg(SURFACE))
        .block(block);

    frame.render_widget(input, rect);

    let cursor_x = rect.x + text_width(&app.search_query).min(rect.width as usize - 2) as u16 + 1;
    frame.set_cursor_position((cursor_x, rect.y + 1));
}

// ── Bookmarks overlay ──────────────────────────────────────────────────────

fn draw_bookmarks(frame: &mut Frame, area: Rect, app: &App) {
    let rect = centered_rect(area, 70, 60);
    frame.render_widget(Clear, rect);

    let bms = app.storage.bookmarks();
    let items: Vec<ListItem> = bms
        .iter()
        .enumerate()
        .map(|(i, bm)| {
            let num = i + 1;
            let text = format!(" {:>2}. {} │ {} ", num, bm.title, bm.url);
            ListItem::new(text).style(Style::default().fg(BODY).bg(SURFACE))
        })
        .collect();

    let empty_msg = if bms.is_empty() {
        vec![ListItem::new("  No bookmarks yet. Press Ctrl+D to add the current page.")
            .style(Style::default().fg(DIM).italic().bg(SURFACE))]
    } else {
        items
    };

    let list = List::new(empty_msg)
        .block(overlay_block(" Bookmarks (1-9 open, d delete, Esc close) ", PRIMARY))
        .highlight_style(Style::default().fg(HIGHLIGHT_FG).bg(PRIMARY).bold());

    frame.render_widget(list, rect);
}

// ── History overlay ────────────────────────────────────────────────────────

fn draw_history(frame: &mut Frame, area: Rect, app: &App) {
    let rect = centered_rect(area, 75, 65);
    frame.render_widget(Clear, rect);

    let hist = app.storage.history(100);
    let items: Vec<ListItem> = hist
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let num = i + 1;
            let text = format!(" {:>2}. {} │ {} │ {}", num, h.visited_at, h.title, h.url);
            ListItem::new(text).style(Style::default().fg(BODY).bg(SURFACE))
        })
        .collect();

    let empty_msg = if hist.is_empty() {
        vec![ListItem::new("  No history yet. Start browsing!")
            .style(Style::default().fg(DIM).italic().bg(SURFACE))]
    } else {
        items
    };

    let list = List::new(empty_msg)
        .block(overlay_block(" History (1-9 open, c clear, Esc close) ", ACCENT));

    frame.render_widget(list, rect);
}

// ── Settings overlay ───────────────────────────────────────────────────────

fn draw_settings(frame: &mut Frame, area: Rect, app: &App) {
    let rect = centered_rect(area, 65, 55);
    frame.render_widget(Clear, rect);

    let cfg = &app.config;
    let engine_names = crate::search_engine::engine_names();
    let engine_name = engine_names
        .iter()
        .find(|n| crate::search_engine::engine_url(n) == Some(&cfg.search_engine))
        .copied()
        .unwrap_or("Custom");

    let lines = vec![
        format!("  KEY     SETTING              VALUE"),
        format!("  ──────  ───────────────────  ─────────────────────"),
        format!("  h       Homepage              {}  ", cfg.homepage),
        format!("  s       Search engine         {}  ", engine_name),
        format!("  a       Ad blocker            {}  ", if cfg.adblock_enabled { "●  ON " } else { "○  OFF" }),
        format!("  p       Privacy mode          {}  ", if cfg.privacy_mode { "●  ON " } else { "○  OFF" }),
        format!("  z       Zoom level            {:.1}x  ", cfg.zoom_level),
        format!("  t       Page load timeout     {} ms  ", cfg.page_load_timeout_ms),
        format!("  d       Post-load delay       {:.1}s  ", cfg.post_load_delay_s),
        format!("  P       Proxy                 {}  ", if cfg.proxy_url.is_empty() { "(none)" } else { &cfg.proxy_url }),
        format!("  ──────  ───────────────────  ─────────────────────"),
        format!("  Esc     Close settings"),
    ];

    let items: Vec<ListItem> = lines
        .iter()
        .map(|line| ListItem::new(line.as_str()))
        .collect();

    let list = List::new(items)
        .block(overlay_block(" Settings ", PRIMARY))
        .style(Style::default().fg(BODY));

    frame.render_widget(list, rect);
}

// ── Help overlay ───────────────────────────────────────────────────────────

fn draw_help(frame: &mut Frame, area: Rect) {
    let rect = centered_rect(area, 58, 78);
    frame.render_widget(Clear, rect);

    let help_lines = vec![
        "  KEY               ACTION",
        "  ────────────────  ────────────────────────────────",
        "",
        "  ┌─ Navigation ──────────────────────────────────┐",
        "  Ctrl+L            Open URL",
        "  Ctrl+T            New tab",
        "  Ctrl+W            Close tab",
        "  Tab / Shift+Tab   Next / Previous tab",
        "  q / Ctrl+C        Quit",
        "",
        "  ┌─ Scrolling ───────────────────────────────────┐",
        "  j / Down          Scroll down",
        "  k / Up            Scroll up",
        "  Ctrl+D / Ctrl+U   Page down / Page up",
        "  g / G             Top / Bottom of page",
        "",
        "  ┌─ Page Actions ────────────────────────────────┐",
        "  b / f             Back / Forward",
        "  r / Ctrl+R        Reload",
        "  Enter             Reload current URL",
        "",
        "  ┌─ Search ──────────────────────────────────────┐",
        "  /                 Search in page",
        "  n / N             Next / Previous match",
        "",
        "  ┌─ Tools ───────────────────────────────────────┐",
        "  Ctrl+B            Bookmarks",
        "  Ctrl+D            Toggle bookmark",
        "  Ctrl+H            History",
        "  Ctrl+S            Settings",
        "  Ctrl+E            Toggle ad blocker",
        "  Ctrl+P            Toggle privacy mode",
        "",
        "  ┌─ Links ───────────────────────────────────────┐",
        "  1-9               Open link / bookmark by number",
        "  ? / Esc           Close help",
    ];

    let items: Vec<ListItem> = help_lines
        .into_iter()
        .map(|line| {
            if line.starts_with("  ┌─") || line.starts_with("  └─") {
                ListItem::new(line).style(Style::default().fg(PRIMARY_DIM).bold())
            } else {
                ListItem::new(line).style(Style::default().fg(BODY))
            }
        })
        .collect();

    let list = List::new(items)
        .block(overlay_block(" Help / Keybindings ", INFO));

    frame.render_widget(list, rect);
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn text_width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

fn truncate(s: &str, max: usize) -> String {
    if s.width() <= max {
        s.to_string()
    } else {
        let mut out = String::new();
        let mut w = 0;
        for c in s.chars() {
            let cw = unicode_width::UnicodeWidthStr::width(c.to_string().as_str());
            if w + cw > max - 1 {
                out.push('…');
                break;
            }
            out.push(c);
            w += cw;
        }
        out
    }
}
