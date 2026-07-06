use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use regex::Regex;
use scraper::{ElementRef, Html, Selector};

#[derive(Debug, Clone)]
pub struct RenderedPage {
    pub title: String,
    pub url: String,
    pub lines: Vec<Line<'static>>,
    pub links: Vec<(String, String)>,
}

// Color palette (refined for readability)
const C_PRIMARY: Color = Color::Rgb(0x00, 0xff, 0xaa);
const C_SECTION: Color = Color::Rgb(0xff, 0x55, 0xff);
const C_BODY: Color = Color::Rgb(0xd0, 0xe0, 0xd0);
const C_BODY_BRIGHT: Color = Color::Rgb(0xe0, 0xf0, 0xe0);
const C_DIM: Color = Color::Rgb(0x55, 0x77, 0x55);
const C_LINK: Color = Color::Rgb(0x66, 0xdd, 0xff);
const C_H1: Color = Color::Rgb(0x00, 0xdd, 0xff);
const C_H2: Color = Color::Rgb(0x00, 0xbb, 0xdd);
const C_H3: Color = Color::Rgb(0x44, 0x99, 0xdd);
const C_CODE: Color = Color::Rgb(0x00, 0xff, 0x88);
const C_CODE_BG: Color = Color::Rgb(0x0d, 0x14, 0x0d);
const C_QUOTE: Color = Color::Rgb(0xff, 0xdd, 0x44);
const C_IMG: Color = Color::Rgb(0xff, 0x66, 0xbb);
const C_ERROR: Color = Color::Rgb(0xff, 0x33, 0x55);
const C_WARN: Color = Color::Rgb(0xff, 0xaa, 0x00);
const C_SEP: Color = Color::Rgb(0x22, 0x3a, 0x22);
const C_TABLE_HEADER: Color = Color::Rgb(0x00, 0xcc, 0xaa);
const C_META: Color = Color::Rgb(0x66, 0x88, 0x66);

fn strip_non_content(html: &str) -> String {
    let non_content_tags = [
        "script", "style", "noscript", "iframe", "svg", "canvas",
        "nav", "footer", "header", "aside",
    ];
    let mut s = html.to_string();
    for tag in &non_content_tags {
        let re =
            Regex::new(&format!(r"(?is)<\s*{}[^>]*>.*?</\s*{}\s*>", tag, tag)).unwrap();
        s = re.replace_all(&s, "").to_string();
    }
    let re = Regex::new(r"(?is)<\s*(link|meta|input)\b[^>]*/?\s*>").unwrap();
    re.replace_all(&s, "").to_string()
}

pub fn render_html(html: &str, url: &str) -> RenderedPage {
    let cleaned = strip_non_content(html);
    let doc = Html::parse_document(&cleaned);
    let title = extract_title(&doc, url);
    let links = extract_links(&doc, url);
    let lines = render_body(&doc, url);

    if lines.is_empty() {
        return render_fallback(&doc, &title, &links, url);
    }

    let mut all_lines: Vec<Line<'static>> = Vec::new();

    // Title
    all_lines.push(Line::from(""));
    all_lines.push(Line::from(Span::styled(
        format!("  {}", title),
        Style::new().fg(C_PRIMARY).add_modifier(Modifier::BOLD),
    )));
    all_lines.push(Line::from(Span::styled(
        format!("  {}", url),
        Style::new().fg(C_META).italic(),
    )));
    all_lines.push(Line::from(""));

    all_lines.extend(lines);

    // Links section
    if !links.is_empty() {
        all_lines.push(Line::from(Span::styled(
            format!("  {} ", "━".repeat(40)),
            Style::new().fg(C_SEP),
        )));
        all_lines.push(Line::from(""));
        all_lines.push(Line::from(Span::styled(
            format!(
                "  Links on this page ({} total, showing first 30):",
                links.len()
            ),
            Style::new().fg(C_SECTION).add_modifier(Modifier::BOLD),
        )));
        all_lines.push(Line::from(""));

        for (i, (href, text)) in links.iter().take(30).enumerate() {
            let label = if text.chars().count() > 60 {
                format!("{}…", text.chars().take(60).collect::<String>())
            } else {
                text.clone()
            };
            all_lines.push(Line::from(vec![
                Span::styled(
                    format!("  [{:>2}] ", i + 1),
                    Style::new().fg(C_DIM),
                ),
                Span::styled(label, Style::new().fg(C_LINK)),
                Span::styled(format!("  {}", href), Style::new().fg(C_META)),
            ]));
        }
    }

    RenderedPage {
        title,
        url: url.to_string(),
        lines: all_lines,
        links,
    }
}

pub fn render_error(message: &str, url: &str) -> RenderedPage {
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  ⚠ Connection Error",
            Style::new().fg(C_ERROR).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", message),
            Style::new().fg(C_ERROR),
        )),
        Line::from(""),
        Line::from(Span::styled(format!("  URL: {}", url), Style::new().fg(C_DIM))),
        Line::from(""),
        Line::from(Span::styled(
            "  Check the URL and try again.",
            Style::new().fg(C_DIM),
        )),
        Line::from(""),
    ];
    RenderedPage {
        title: format!("Error - {}", url),
        url: url.to_string(),
        lines,
        links: vec![],
    }
}

fn extract_title(doc: &Html, fallback_url: &str) -> String {
    if let Ok(sel) = Selector::parse("meta[property='og:title']") {
        for el in doc.select(&sel) {
            if let Some(val) = el.value().attr("content") {
                let t = val.trim();
                if !t.is_empty() {
                    return t.to_string();
                }
            }
        }
    }
    if let Ok(sel) = Selector::parse("title") {
        for el in doc.select(&sel) {
            let t: String = el.text().collect();
            let t = t.trim().to_string();
            if !t.is_empty() && t != "\u{a0}" {
                return t.chars().take(200).collect();
            }
        }
    }
    for tag in &["h1", "h2"] {
        if let Ok(sel) = Selector::parse(tag) {
            for el in doc.select(&sel) {
                let t: String = el.text().collect();
                let t = t.trim().to_string();
                if t.len() > 3 {
                    return t.chars().take(200).collect();
                }
            }
        }
    }
    fallback_url.to_string()
}

fn extract_links(doc: &Html, base_url: &str) -> Vec<(String, String)> {
    let mut links = Vec::new();
    let mut seen = std::collections::HashSet::new();
    if let Ok(sel) = Selector::parse("a[href]") {
        for el in doc.select(&sel) {
            let href = el.value().attr("href").unwrap_or("");
            let href = href.trim();
            if href.is_empty()
                || href.starts_with('#')
                || href.starts_with("javascript:")
            {
                continue;
            }
            let full = resolve_url(base_url, href);
            let text: String = el.text().collect();
            let text = if text.trim().is_empty() {
                href.to_string()
            } else {
                text.trim().to_string()
            };
            let key = (full.clone(), text.clone());
            if seen.insert(key) {
                links.push((full, text));
            }
        }
    }
    links
}

fn resolve_url(base: &str, rel: &str) -> String {
    if let Ok(base_url) = url::Url::parse(base) {
        if let Ok(resolved) = base_url.join(rel) {
            let mut s = resolved.to_string();
            if let Some(pos) = s.find('#') {
                s.truncate(pos);
            }
            return s;
        }
    }
    rel.to_string()
}

// ── Body rendering ─────────────────────────────────────────────────────────

fn render_body(doc: &Html, base_url: &str) -> Vec<Line<'static>> {
    let sel = Selector::parse("body").ok();
    if sel.is_none() {
        return vec![];
    }
    let sel = sel.unwrap();

    for body in doc.select(&sel) {
        let mut lines = Vec::new();
        render_node(&body, base_url, 0, &mut lines);
        if !lines.is_empty() {
            return lines;
        }
    }
    vec![]
}

fn render_node(
    node: &ElementRef,
    base_url: &str,
    indent: usize,
    out: &mut Vec<Line<'static>>,
) {
    let tag = node.value().name();

    match tag {
        "script" | "style" | "noscript" | "iframe" | "svg" | "canvas"
        | "nav" | "footer" | "header" | "aside" => return,
        "br" => {
            out.push(Line::from(""));
            return;
        }
        "hr" => {
            out.push(Line::from(Span::styled(
                format!("  {} ", "━".repeat(40)),
                Style::new().fg(C_SEP),
            )));
            return;
        }
        _ => {}
    }

    let pfx = "  ".repeat(indent);

    match tag {
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            let text = node_text(node);
            if !text.is_empty() {
                let style = match tag {
                    "h1" => Style::new().fg(C_H1).add_modifier(Modifier::BOLD),
                    "h2" => Style::new().fg(C_H2).add_modifier(Modifier::BOLD),
                    _ => Style::new().fg(C_H3).add_modifier(Modifier::BOLD),
                };
                let sep_color = match tag {
                    "h1" => C_H1,
                    "h2" => C_H2,
                    _ => C_H3,
                };
                out.push(Line::from(""));
                out.push(Line::from(Span::styled(
                    format!("{}{}", pfx, text),
                    style,
                )));
                if tag == "h1" || tag == "h2" {
                    out.push(Line::from(Span::styled(
                        format!("{}{}", pfx, "─".repeat(text.chars().count().min(60))),
                        Style::new().fg(sep_color).dim(),
                    )));
                }
                out.push(Line::from(""));
            }
        }

        "p" => {
            let mut spans: Vec<Span> = Vec::new();
            spans.push(Span::styled(
                pfx.clone(),
                Style::new().fg(C_BODY),
            ));
            collect_inline(node, base_url, &mut spans);
            let text_only: String = spans.iter().map(|s| s.content.as_ref()).collect();
            let trimmed = text_only.trim();
            if !trimmed.is_empty() && trimmed != pfx {
                out.push(Line::from(spans));
                out.push(Line::from(""));
            }
        }

        "a" => {
            let href = node.value().attr("href").unwrap_or("");
            let text = node_text(node);
            let display = if text.is_empty() { href } else { &text };
            if !href.is_empty()
                && !href.starts_with('#')
                && !href.starts_with("javascript:")
            {
                out.push(Line::from(vec![
                    Span::styled(pfx, Style::new().fg(C_BODY)),
                    Span::styled(
                        format!("[{}]", display.chars().take(80).collect::<String>()),
                        Style::new().fg(C_LINK),
                    ),
                ]));
            } else {
                let t = node_text(node);
                if !t.is_empty() {
                    out.push(Line::from(Span::styled(
                        format!("{}{}", pfx, t),
                        Style::new().fg(C_BODY),
                    )));
                }
            }
        }

        "b" | "strong" => {
            let text = node_text(node);
            if !text.is_empty() {
                out.push(Line::from(Span::styled(
                    format!("{}{}", pfx, text),
                    Style::new().fg(C_BODY_BRIGHT).add_modifier(Modifier::BOLD),
                )));
            }
        }

        "i" | "em" => {
            let text = node_text(node);
            if !text.is_empty() {
                out.push(Line::from(Span::styled(
                    format!("{}{}", pfx, text),
                    Style::new().fg(C_BODY).add_modifier(Modifier::ITALIC),
                )));
            }
        }

        "ul" | "ol" => {
            let is_ol = tag == "ol";
            let mut idx = 0;
            for child in node.children() {
                if let Some(child_el) = ElementRef::wrap(child) {
                    if child_el.value().name() == "li" {
                        idx += 1;
                        let text = node_text(&child_el);
                        if !text.is_empty() {
                            let bullet = if is_ol {
                                format!("{}.", idx)
                            } else {
                                "•".to_string()
                            };
                            out.push(Line::from(Span::styled(
                                format!("{}  {}  {}", pfx, bullet, text),
                                Style::new().fg(C_BODY),
                            )));
                        }
                    }
                }
            }
            if idx > 0 {
                out.push(Line::from(""));
            }
        }

        "blockquote" => {
            let text = node_text(node);
            if !text.is_empty() {
                for line in text.lines() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        out.push(Line::from(Span::styled(
                            format!("{}  ▎ {}", pfx, trimmed),
                            Style::new()
                                .fg(C_QUOTE)
                                .add_modifier(Modifier::ITALIC),
                        )));
                    }
                }
                out.push(Line::from(""));
            }
        }

        "pre" => {
            let code = node_text_raw(node);
            if !code.is_empty() {
                let code_style = Style::new().fg(C_CODE).bg(C_CODE_BG);
                let line_nums = code.lines().count() > 1;
                for (li, line) in code.lines().enumerate() {
                    let num = if line_nums {
                        format!("{:>3} ", li + 1)
                    } else {
                        String::new()
                    };
                    out.push(Line::from(Span::styled(
                        format!("{}  {}{}", pfx, num, line),
                        code_style,
                    )));
                }
                out.push(Line::from(""));
            }
        }

        "code" => {
            let text = node_text(node);
            if !text.is_empty() {
                out.push(Line::from(Span::styled(
                    format!("{}{}", pfx, text),
                    Style::new().fg(C_CODE).bg(C_CODE_BG),
                )));
            }
        }

        "img" => {
            let alt = node.value().attr("alt").unwrap_or("[image]");
            let src = node.value().attr("src").unwrap_or("");
            out.push(Line::from(Span::styled(
                format!("{}🖼  {} ({})", pfx, alt, src),
                Style::new().fg(C_IMG).add_modifier(Modifier::ITALIC),
            )));
        }

        "table" => {
            render_table(node, &pfx, out);
        }

        "body" | "div" | "section" | "article" | "main" | "span"
        | "form" | "li" | "td" | "th" => {
            for child in node.children() {
                if let Some(child_el) = ElementRef::wrap(child) {
                    render_node(&child_el, base_url, indent, out);
                }
            }
        }

        _ => {
            let mut has_element_children = false;
            for child in node.children() {
                if let Some(child_el) = ElementRef::wrap(child) {
                    has_element_children = true;
                    render_node(&child_el, base_url, indent, out);
                }
            }
            if !has_element_children {
                let text = node_text(node);
                if !text.is_empty() {
                    out.push(Line::from(Span::styled(
                        format!("{}{}", pfx, text),
                        Style::new().fg(C_BODY),
                    )));
                }
            }
        }
    }
}

// ── Inline content collector ───────────────────────────────────────────────
// Collects text and inline elements (a, strong, em, code, span) into a single
// span vector for rendering within a paragraph.

fn collect_inline(
    node: &ElementRef,
    base_url: &str,
    spans: &mut Vec<Span<'static>>,
) {
    for child in node.children() {
        if let Some(child_el) = ElementRef::wrap(child) {
            let tag = child_el.value().name();
            match tag {
                "a" => {
                    let href = child_el.value().attr("href").unwrap_or("");
                    let text: String = child_el.text().collect();
                    let text = text.trim().to_string();
                    if !text.is_empty()
                        && !href.starts_with('#')
                        && !href.starts_with("javascript:")
                    {
                        spans.push(Span::styled(
                            text,
                            Style::new().fg(C_LINK).add_modifier(Modifier::UNDERLINED),
                        ));
                    } else if !text.is_empty() {
                        spans.push(Span::styled(text, Style::new().fg(C_BODY)));
                    }
                }
                "b" | "strong" => {
                    let text: String = child_el.text().collect();
                    let text = text.trim().to_string();
                    if !text.is_empty() {
                        spans.push(Span::styled(
                            text,
                            Style::new()
                                .fg(C_BODY_BRIGHT)
                                .add_modifier(Modifier::BOLD),
                        ));
                    }
                }
                "i" | "em" => {
                    let text: String = child_el.text().collect();
                    let text = text.trim().to_string();
                    if !text.is_empty() {
                        spans.push(Span::styled(
                            text,
                            Style::new()
                                .fg(C_BODY)
                                .add_modifier(Modifier::ITALIC),
                        ));
                    }
                }
                "code" => {
                    let text: String = child_el.text().collect();
                    let text = text.trim().to_string();
                    if !text.is_empty() {
                        spans.push(Span::styled(
                            format!("`{}`", text),
                            Style::new().fg(C_CODE),
                        ));
                    }
                }
                "br" => {
                    spans.push(Span::raw(" "));
                }
                "img" => {
                    let alt = child_el.value().attr("alt").unwrap_or("[img]");
                    spans.push(Span::styled(
                        format!("[{}]", alt),
                        Style::new().fg(C_IMG).italic(),
                    ));
                }
                _ => {
                    collect_inline(&child_el, base_url, spans);
                }
            }
        } else {
            // Text node
            let text = child.value();
            if let scraper::node::Node::Text(t) = text {
                let t = t.text.trim();
                if !t.is_empty() {
                    spans.push(Span::styled(t.to_string(), Style::new().fg(C_BODY)));
                }
            }
        }
    }
}

// ── Table rendering ────────────────────────────────────────────────────────

fn render_table(node: &ElementRef, pfx: &str, out: &mut Vec<Line<'static>>) {
    let rows: Vec<Vec<String>> = node
        .select(&Selector::parse("tr").unwrap())
        .map(|row| {
            row.select(&Selector::parse("td, th").unwrap())
                .map(|cell| {
                    cell.text()
                        .collect::<String>()
                        .trim()
                        .to_string()
                })
                .collect()
        })
        .collect();

    if rows.is_empty() || rows.iter().all(|r| r.is_empty()) {
        return;
    }

    let max_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    if max_cols == 0 {
        return;
    }

    // Calculate column widths
    let mut col_widths = vec![0usize; max_cols];
    for row in &rows {
        for (i, cell) in row.iter().enumerate() {
            col_widths[i] = col_widths[i].max(cell.chars().count());
        }
    }
    let total_width: usize = col_widths.iter().sum::<usize>() + (max_cols - 1) * 3 + 4;

    // Don't render tables that are too wide
    if total_width > 120 {
        let text: String = rows
            .iter()
            .flat_map(|r| r.iter())
            .cloned()
            .collect::<Vec<_>>()
            .join(" ");
        if !text.is_empty() {
            out.push(Line::from(""));
            out.push(Line::from(Span::styled(
                format!("{}[table: {}]", pfx, text.chars().take(120).collect::<String>()),
                Style::new().fg(C_DIM).italic(),
            )));
            out.push(Line::from(""));
        }
        return;
    }

    out.push(Line::from(""));

    for (ri, row) in rows.iter().enumerate() {
        let mut spans = vec![Span::styled(
            pfx.to_string(),
            Style::new().fg(C_BODY),
        )];

        for (ci, cell) in row.iter().enumerate() {
            let is_header = ri == 0;
            let cell_style = if is_header {
                Style::new().fg(C_TABLE_HEADER).add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(C_BODY)
            };

            let padded = format!(
                " {:width$} ",
                cell,
                width = col_widths[ci]
            );
            spans.push(Span::styled(padded, cell_style));

            if ci < max_cols - 1 {
                spans.push(Span::styled(" │ ", Style::new().fg(C_SEP)));
            }
        }

        out.push(Line::from(spans));

        // Header separator
        if ri == 0 && rows.len() > 1 {
            let mut sep_spans = vec![Span::styled(
                pfx.to_string(),
                Style::new().fg(C_DIM),
            )];
            for ci in 0..max_cols {
                let sep = format!(" {} ", "─".repeat(col_widths[ci]));
                sep_spans.push(Span::styled(sep, Style::new().fg(C_SEP)));
                if ci < max_cols - 1 {
                    sep_spans.push(Span::styled(" ┼ ", Style::new().fg(C_SEP)));
                }
            }
            out.push(Line::from(sep_spans));
        }
    }

    out.push(Line::from(""));
}

// ── Text extraction helpers ────────────────────────────────────────────────

fn node_text(node: &ElementRef) -> String {
    let t: String = node.text().collect();
    let t = t.trim().to_string();
    t.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
}

fn node_text_raw(node: &ElementRef) -> String {
    use scraper::node::Node as ScraperNode;
    let mut text = String::new();
    for child in node.children() {
        let child_ref = child;
        match child_ref.value() {
            ScraperNode::Text(t) => text.push_str(&t.text),
            ScraperNode::Element(_) => {
                if let Some(el) = ElementRef::wrap(child_ref) {
                    text.push_str(&node_text_raw(&el));
                }
            }
            _ => {}
        }
    }
    text
}

// ── Fallback rendering ─────────────────────────────────────────────────────

fn render_fallback(
    doc: &Html,
    title: &str,
    links: &[(String, String)],
    url: &str,
) -> RenderedPage {
    let mut lines = Vec::new();

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!("  {}", title),
        Style::new().fg(C_H1).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    // Meta description
    if let Ok(sel) = Selector::parse("meta[name=description], meta[property='og:description']") {
        for el in doc.select(&sel) {
            if let Some(val) = el.value().attr("content") {
                if !val.is_empty() {
                    lines.push(Line::from(Span::styled(
                        format!("  {}", val.chars().take(300).collect::<String>()),
                        Style::new().fg(C_META).italic(),
                    )));
                    lines.push(Line::from(""));
                    break;
                }
            }
        }
    }

    // Extract any meaningful text blocks
    let texts: Vec<String> = doc
        .select(
            &Selector::parse(
                "p, h1, h2, h3, h4, li, blockquote, pre, td, th, div",
            )
            .unwrap(),
        )
        .filter_map(|el| {
            let t: String = el.text().collect();
            let t = t.trim().to_string();
            if t.len() > 30 {
                Some(t)
            } else {
                None
            }
        })
        .take(15)
        .collect();

    if !texts.is_empty() {
        for t in texts {
            let truncated = if t.chars().count() > 200 {
                format!("{}…", t.chars().take(200).collect::<String>())
            } else {
                t
            };
            lines.push(Line::from(Span::styled(
                format!("  {}", truncated),
                Style::new().fg(C_BODY),
            )));
            lines.push(Line::from(""));
        }
    } else {
        lines.push(Line::from(Span::styled(
            "  ⚡ This page requires JavaScript to display its content.",
            Style::new().fg(C_WARN),
        )));
        lines.push(Line::from(Span::styled(
            "  Try opening it with headless Chromium or use reader mode.",
            Style::new().fg(C_DIM),
        )));
        lines.push(Line::from(""));
    }

    RenderedPage {
        title: title.to_string(),
        url: url.to_string(),
        lines,
        links: links.to_vec(),
    }
}
