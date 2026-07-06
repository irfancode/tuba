use crate::config::SEARCH_ENGINES;

pub fn is_search_query(input: &str) -> bool {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return false;
    }
    // If it looks like a URL, it's not a search
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return false;
    }
    if trimmed.contains('.') && !trimmed.contains(' ') {
        return false; // Likely a domain name
    }
    true
}

pub fn build_url(input: &str, search_engine_url: &str) -> String {
    let trimmed = input.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return trimmed.to_string();
    }
    if trimmed.contains('.') && !trimmed.contains(' ') {
        return format!("https://{}", trimmed);
    }
    // Search query
    let encoded: String = trimmed
        .chars()
        .map(|c| match c {
            ' ' => '+'.to_string(),
            c if c.is_alphanumeric() || c == '-' || c == '_' => c.to_string(),
            c => urlencoding(c),
        })
        .collect();
    search_engine_url.replace("{}", &encoded)
}

fn urlencoding(c: char) -> String {
    let bytes = c.to_string().as_bytes().to_vec();
    let mut out = String::new();
    for b in bytes {
        out.push_str(&format!("%{:02X}", b));
    }
    out
}

pub fn engine_names() -> Vec<&'static str> {
    SEARCH_ENGINES.iter().map(|(name, _)| *name).collect()
}

pub fn engine_url(name: &str) -> Option<&'static str> {
    SEARCH_ENGINES
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, url)| *url)
}
