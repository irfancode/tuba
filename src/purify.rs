use anyhow::{Context, Result};
use readability::extractor;
use scraper::{Html, Selector};
use url::Url;

use crate::config::Config;

#[derive(Debug, Clone)]
pub struct Article {
    pub title: String,
    pub author: String,
    pub date: String,
    pub text: String,
    pub url: String,
}

const BLOCKED_SIGNALS: &[(&str, &str)] = &[
    ("Just a moment...", "Cloudflare challenge"),
    ("Checking your browser", "anti-bot check"),
    ("Please verify you are a human", "CAPTCHA challenge"),
    ("api-engagement", "API engagement platform"),
    ("cdn-cgi/challenge-platform", "Cloudflare challenge"),
    ("_cf_chl_opt", "Cloudflare challenge"),
    ("DataDome", "DataDome anti-bot"),
    ("geo.captcha-delivery.com", "CAPTCHA challenge"),
];

pub fn extract_content(html: &str, url_str: &str, config: &Config) -> Result<Option<Article>> {
    // 0. Check for blocked/detection pages (G33: ported from Rust reader)
    if let Some(reason) = detect_blocked_page(html) {
        log::info!("Page blocked: {}", reason);
        return Ok(None);
    }

    let parsed_url = Url::parse(url_str).context("Invalid URL")?;

    // 1. Run Mozilla Readability
    let mut cursor = std::io::Cursor::new(html.as_bytes());
    let product = extractor::extract(&mut cursor, &parsed_url)
        .context("Readability extraction failed")?;

    // 2. Convert to plain text with optional links
    let text_content = if config.include_links {
        html2text::from_read(product.content.as_bytes(), 8888)
    } else {
        product.text.clone()
    };

    if text_content.trim().len() >= config.min_extracted_length {
        let (author, date) = extract_metadata(html);
        return Ok(Some(Article {
            title: product.title,
            author,
            date,
            text: text_content.trim().to_string(),
            url: url_str.to_string(),
        }));
    }

    // 3. Fallback: scraper-based extraction
    log::info!("Readability returned too little — trying scraper fallback");
    fallback_extract(html, url_str, config)
}

fn detect_blocked_page(html: &str) -> Option<&'static str> {
    let lower = html.to_lowercase();
    for &(pattern, reason) in BLOCKED_SIGNALS {
        if lower.contains(pattern) {
            return Some(reason);
        }
    }
    None
}

fn fallback_extract(html: &str, url_str: &str, config: &Config) -> Result<Option<Article>> {
    let doc = Html::parse_document(html);

    let mut parts: Vec<String> = Vec::new();
    let content_selectors = [
        "p", "h1", "h2", "h3", "h4", "h5", "h6", "li", "td", "th",
        "blockquote", "pre", "code", "article", "section",
    ];

    for sel_str in &content_selectors {
        if let Ok(sel) = Selector::parse(sel_str) {
            for el in doc.select(&sel) {
                let text: String = el.text().collect();
                let text = text.trim().to_string();
                if !text.is_empty() && text.len() > 2 {
                    parts.push(text);
                }
            }
        }
    }

    let text = parts.join("\n");
    if text.trim().len() < config.min_extracted_length {
        // Last resort: raw html2text on full page
        log::info!("Scraper fallback also thin — trying html2text on full page");
        let raw = html2text::from_read(html.as_bytes(), 8888);
        if raw.trim().len() < config.min_extracted_length {
            return Ok(None);
        }
        let title = extract_title(&doc).unwrap_or_else(|| url_str.to_string());
        let (author, date) = extract_metadata(html);
        return Ok(Some(Article {
            title,
            author,
            date,
            text: raw.trim().to_string(),
            url: url_str.to_string(),
        }));
    }

    // Boilerplate trimming (G33: ported from Rust reader)
    let text = trim_boilerplate(&text, config.min_extracted_length);

    let title = extract_title(&doc).unwrap_or_else(|| url_str.to_string());
    let (author, date) = extract_metadata(html);

    Ok(Some(Article {
        title,
        author,
        date,
        text,
        url: url_str.to_string(),
    }))
}

fn trim_boilerplate(text: &str, min_len: usize) -> String {
    let blocks: Vec<&str> = text.split('\n').collect();
    if blocks.len() < 3 {
        return text.to_string();
    }

    let mut best_start = 0usize;
    let mut best_len = 0usize;

    for i in 0..blocks.len() {
        let mut total = 0usize;
        for j in i..blocks.len() {
            let b = blocks[j].trim();
            if b.is_empty() && (j - i) > 3 {
                break;
            }
            total += b.len();
            if total > best_len {
                best_len = total;
                best_start = i;
            }
        }
    }

    if best_len < min_len {
        return text.to_string();
    }

    blocks[best_start..]
        .iter()
        .take_while(|b| !b.trim().is_empty() || best_len < min_len)
        .map(|b| b.to_string())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn extract_title(doc: &Html) -> Option<String> {
    // og:title
    if let Ok(sel) = Selector::parse("meta[property='og:title']") {
        for el in doc.select(&sel) {
            if let Some(val) = el.value().attr("content") {
                let t = val.trim().to_string();
                if !t.is_empty() {
                    return Some(t);
                }
            }
        }
    }
    // <title>
    if let Ok(sel) = Selector::parse("title") {
        for el in doc.select(&sel) {
            let t: String = el.text().collect();
            let t = t.trim().to_string();
            if !t.is_empty() {
                return Some(t);
            }
        }
    }
    None
}

fn extract_metadata(html: &str) -> (String, String) {
    let doc = Html::parse_document(html);
    let mut author = String::new();
    let mut date = String::new();

    if let Ok(sel) = Selector::parse("meta[name=author]") {
        for el in doc.select(&sel) {
            if let Some(val) = el.value().attr("content") {
                author = val.to_string();
                break;
            }
        }
    }

    if let Ok(sel) = Selector::parse("meta[property='article:published_time']") {
        for el in doc.select(&sel) {
            if let Some(val) = el.value().attr("content") {
                date = val.to_string();
                break;
            }
        }
    }

    if date.is_empty() {
        if let Ok(sel) = Selector::parse("meta[name=date]") {
            for el in doc.select(&sel) {
                if let Some(val) = el.value().attr("content") {
                    date = val.to_string();
                    break;
                }
            }
        }
    }

    (author, date)
}
