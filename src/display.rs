use std::cmp::min;
use std::process::Command;

use colored::*;

use crate::config::Config;
use crate::purify::Article;

pub fn render_article(article: &Option<Article>, config: &Config) {
    let width = config
        .terminal_width
        .unwrap_or_else(|| terminal_width().unwrap_or(100));

    let article = match article {
        Some(a) => a,
        None => {
            println!();
            println!(
                "{}",
                "Could not extract any meaningful content from the page."
                    .yellow()
                    .bold()
            );
            println!();
            println!("{}", "Possible reasons:".truecolor(85, 119, 85));
            println!(
                "  {}",
                "• The page is behind a login wall or paywall".truecolor(85, 119, 85)
            );
            println!(
                "  {}",
                "• The site uses anti-bot protection (Cloudflare, DataDome, etc.)"
                    .truecolor(85, 119, 85)
            );
            println!(
                "  {}",
                "• The page is a JavaScript-only SPA".truecolor(85, 119, 85)
            );
            println!(
                "  {}",
                "• The page has no article content (video, image gallery, etc.)"
                    .truecolor(85, 119, 85)
            );
            println!();
            return;
        }
    };

    // Title
    if !article.title.is_empty() {
        println!();
        let title_line = format!("  {}", article.title);
        println!("{}", title_line.bold().truecolor(0, 255, 170));
        println!(
            "{}",
            "  ".to_string()
                + &"─".repeat(
                    min(article.title.chars().count(), width.saturating_sub(4))
                )
                .truecolor(0, 200, 130)
        );
        println!();
    }

    // Metadata
    let mut meta_parts = Vec::new();
    if !article.author.is_empty() {
        meta_parts.push(format!("✍  {}", article.author));
    }
    if !article.date.is_empty() {
        meta_parts.push(format!("📅 {}", article.date));
    }
    if !article.url.is_empty() {
        meta_parts.push(format!("🔗 {}", article.url));
    }
    if !meta_parts.is_empty() {
        println!(
            "  {}",
            meta_parts.join("  │  ").truecolor(85, 119, 85).italic()
        );
        println!();
    }

    // Separator
    println!(
        "{}",
        "━".repeat(width).truecolor(25, 50, 25)
    );
    println!();

    // Body
    let body_width = min(width.saturating_sub(4), config.body_text_width);
    let paragraphs: Vec<&str> = article.text.split("\n\n").collect();

    for para in &paragraphs {
        let lines: Vec<&str> = para.lines().collect();
        for line in &lines {
            let stripped = line.trim();
            if stripped.is_empty() {
                continue;
            }
            let wrapped = word_wrap(stripped, body_width);
            for wline in &wrapped {
                println!("  {}", wline.truecolor(208, 224, 208));
            }
        }
        println!();
    }

    // Footer
    println!(
        "{}",
        "━".repeat(width).truecolor(25, 50, 25)
    );
    let char_count = article.text.chars().count();
    let word_count = article.text.split_whitespace().count();
    println!(
        "  {}",
        format!(
            "{} chars  ·  {} words  ·  {}",
            format_count(char_count),
            format_count(word_count),
            article.url,
        )
        .truecolor(85, 119, 85)
        .italic()
    );
    println!();
}

pub fn render_error(message: &str, url: &str) {
    println!();
    println!("⚠  Error: {}", message.red().bold());
    println!("  {}", url.truecolor(85, 119, 85));
    println!();
}

pub fn word_wrap(text: &str, width: usize) -> Vec<String> {
    if width < 10 {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        let w_chars: Vec<char> = word.chars().collect();
        if w_chars.len() > width {
            if !current.is_empty() {
                lines.push(current);
            }
            let mut remaining: Vec<char> = w_chars;
            while !remaining.is_empty() {
                let split_at = min(remaining.len(), width);
                let chunk: String = remaining.drain(..split_at).collect();
                lines.push(chunk);
            }
            current = String::new();
            continue;
        }

        let cur_chars: Vec<char> = current.chars().collect();
        if cur_chars.len() + w_chars.len() + 1 > width {
            if !current.is_empty() {
                lines.push(current);
            }
            current = word.to_string();
        } else {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    if lines.is_empty() {
        lines.push(text.to_string());
    }

    lines
}

fn format_count(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    result.chars().rev().collect()
}

fn terminal_width() -> Option<usize> {
    if let Ok(out) = Command::new("stty")
        .arg("size")
        .arg("-F")
        .arg("/dev/stderr")
        .output()
    {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout);
            let parts: Vec<&str> = s.trim().split_whitespace().collect();
            if parts.len() == 2 {
                return parts[1].parse().ok();
            }
        }
    }
    None
}
