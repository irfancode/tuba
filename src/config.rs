use serde::{Deserialize, Serialize};

pub const CONFIG_DIR: &str = "tuba";
pub const CONFIG_FILE: &str = "settings.json";
pub const HISTORY_FILE: &str = "history.json";
pub const BOOKMARKS_FILE: &str = "bookmarks.json";
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    // Schema
    pub schema_version: u32,

    // Navigation
    pub page_load_timeout_ms: u64,
    pub post_load_delay_s: f64,
    pub navigation_wait_until: String,

    // Viewport
    pub viewport_width: u32,
    pub viewport_height: u32,

    // User agent
    pub user_agent: String,

    // Extraction
    pub min_extracted_length: usize,
    pub include_links: bool,
    pub include_tables: bool,
    pub include_formatting: bool,
    pub include_images: bool,

    // Browser
    pub homepage: String,
    pub search_engine: String,
    pub chrome_path: Option<String>,

    // Ad blocking (G11)
    pub adblock_enabled: bool,

    // Proxy (G12)
    pub proxy_enabled: bool,
    pub proxy_url: String,

    // Privacy (G16)
    pub privacy_mode: bool,
    pub max_history: usize,
    pub max_bookmarks: usize,

    // Display
    pub terminal_width: Option<usize>,
    pub body_text_width: usize,
    pub theme: String,

    // Zoom / font (G22)
    pub zoom_level: f64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,

            page_load_timeout_ms: 30_000,
            post_load_delay_s: 2.0,
            navigation_wait_until: "load".into(),

            viewport_width: 1440,
            viewport_height: 900,

            user_agent:
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36"
                    .into(),

            min_extracted_length: 50,
            include_links: true,
            include_tables: true,
            include_formatting: true,
            include_images: false,

            homepage: "https://duckduckgo.com".into(),
            search_engine: "https://duckduckgo.com/?q={}".into(),
            chrome_path: None,

            adblock_enabled: true,

            proxy_enabled: false,
            proxy_url: String::new(),

            privacy_mode: false,
            max_history: 500,
            max_bookmarks: 200,

            terminal_width: None,
            body_text_width: 88,
            theme: "cyberpunk".into(),

            zoom_level: 1.0,
        }
    }
}

pub static SEARCH_ENGINES: &[(&str, &str)] = &[
    ("DuckDuckGo", "https://duckduckgo.com/?q={}"),
    ("Google", "https://www.google.com/search?q={}"),
    ("Bing", "https://www.bing.com/search?q={}"),
    ("Brave", "https://search.brave.com/search?q={}"),
    ("SearXNG", "https://searx.be/search?q={}"),
];
