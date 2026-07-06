use std::sync::Arc;

use crate::browser::{Browser, FetchResult};
use crate::config::Config;
use crate::display::word_wrap;
use crate::purify::extract_content;
use crate::renderer::{render_html, RenderedPage};
use crate::search_engine::build_url;
use crate::storage::Storage;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Normal,
    UrlInput,
    Search,
    Bookmarks,
    History,
    Settings,
    Help,
}

#[derive(Debug, Clone)]
pub struct Tab {
    pub url: String,
    pub title: String,
    pub rendered: Option<RenderedPage>,
    pub scroll: usize,
    pub scroll_max: usize,
    pub loading: bool,
    pub error: Option<String>,
    pub back_stack: Vec<String>,
    pub forward_stack: Vec<String>,
    pub display_lines: Vec<(String, bool)>, // (text, is_title)
}

impl Tab {
    fn new(url: String) -> Self {
        Self {
            url,
            title: "New Tab".into(),
            rendered: None,
            scroll: 0,
            scroll_max: 0,
            loading: false,
            error: None,
            back_stack: Vec::new(),
            forward_stack: Vec::new(),
            display_lines: vec![("Press Ctrl+L to open a URL or ? for help.".into(), false)],
        }
    }

    pub fn welcome() -> Self {
        let lines = vec![
            ("  Tuba 1.1 — Unified Terminal Browser ".into(), true),
            ("  ─────────────────────────────────────".into(), false),
            ("".into(), false),
            (" ┌─ Navigation ──────────────────────┐".into(), false),
            ("   Ctrl+L    Open URL".into(), false),
            ("   Ctrl+T    New tab".into(), false),
            ("   Ctrl+W    Close tab".into(), false),
            ("   Tab/S-Tab  Next/Prev tab".into(), false),
            ("   b/f       Back / Forward".into(), false),
            ("   r         Reload".into(), false),
            ("".into(), false),
            (" ┌─ Scrolling & Search ──────────────┐".into(), false),
            ("   j/k       Scroll down / up".into(), false),
            ("   g/G       Top / Bottom".into(), false),
            ("   /         Search in page".into(), false),
            ("   n/N       Next/Prev match".into(), false),
            ("".into(), false),
            (" ┌─ Tools ───────────────────────────┐".into(), false),
            ("   Ctrl+B    Bookmarks".into(), false),
            ("   Ctrl+H    History".into(), false),
            ("   Ctrl+S    Settings".into(), false),
            ("   Ctrl+D    Toggle bookmark".into(), false),
            ("   Ctrl+E    Toggle ad blocker".into(), false),
            ("   Ctrl+P    Toggle privacy mode".into(), false),
            ("".into(), false),
            (" ┌─ Other ───────────────────────────┐".into(), false),
            ("   ?         Help".into(), false),
            ("   q/Ctrl+C  Quit".into(), false),
            ("".into(), false),
            ("  ─────────────────────────────────────".into(), false),
            ("   Enter a URL or search term to begin  ".into(), false),
        ];
        Self {
            url: "about:welcome".into(),
            title: "Tuba".into(),
            rendered: None,
            scroll: 0,
            scroll_max: lines.len().saturating_sub(1),
            loading: false,
            error: None,
            back_stack: Vec::new(),
            forward_stack: Vec::new(),
            display_lines: lines,
        }
    }

    pub fn set_from_rendered(&mut self, page: RenderedPage) {
        self.title = page.title.clone();
        self.rendered = Some(page);
        self.error = None;
        self.loading = false;
        self.display_lines = vec![("(rendering...)".into(), false)];
        self.scroll = 0;
    }

    pub fn set_error(&mut self, msg: &str) {
        self.title = "Error".into();
        self.rendered = None;
        self.display_lines = vec![
            (format!("⚠  {}", msg), false),
            (String::new(), false),
            ("Press Ctrl+L to enter a different URL.".into(), false),
        ];
        self.error = Some(msg.into());
        self.loading = false;
        self.scroll = 0;
        self.scroll_max = self.display_lines.len().saturating_sub(1);
    }

    pub fn set_loading(&mut self) {
        self.loading = true;
        self.display_lines = vec![("⏳ Loading...".into(), false)];
        self.scroll = 0;
        self.scroll_max = 0;
    }

    pub fn wrap_content(&mut self, width: usize) {
        let body_width = width.saturating_sub(4);
        if body_width < 10 {
            return;
        }

        match &self.rendered {
            Some(page) => {
                let mut lines: Vec<(String, bool)> = Vec::new();
                // Title line
                lines.push((format!("  {}", page.title), true));
                lines.push((format!("  {}", page.url), false));
                lines.push((String::new(), false));

                for line in &page.lines {
                    let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
                    let is_title = line
                        .spans
                        .iter()
                        .any(|s| s.style.add_modifier.contains(ratatui::style::Modifier::BOLD));
                    if text.len() > body_width {
                        let wrapped = word_wrap(&text, body_width);
                        for w in wrapped {
                            lines.push((w, is_title));
                        }
                    } else {
                        lines.push((text, is_title));
                    }
                }
                self.display_lines = lines;
            }
            None => {
                // Keep existing lines (welcome, error, loading)
                if !self.display_lines.is_empty() {
                    // re-wrap if needed
                    let existing = std::mem::take(&mut self.display_lines);
                    let mut lines = Vec::new();
                    for (text, is_title) in existing {
                        if text.len() > body_width {
                            let wrapped = word_wrap(&text, body_width);
                            for w in wrapped {
                                lines.push((w, is_title));
                            }
                        } else {
                            lines.push((text, is_title));
                        }
                    }
                    self.display_lines = lines;
                }
            }
        }

        self.scroll_max = self.display_lines.len().saturating_sub(1);
        if self.scroll > self.scroll_max {
            self.scroll = self.scroll_max;
        }
    }
}

pub struct App {
    pub tabs: Vec<Tab>,
    pub active_tab: usize,
    pub mode: Mode,
    pub url_input: String,
    pub search_query: String,
    pub search_results: Vec<(usize, String)>,
    pub search_idx: usize,
    pub status_message: String,
    pub should_quit: bool,
    pub storage: Storage,

    pub browser: Arc<Browser>,
    pub config: Config,

    // Callback channel for page load results
    pub load_tx: tokio::sync::mpsc::UnboundedSender<LoadResult>,
    pub load_rx: tokio::sync::mpsc::UnboundedReceiver<LoadResult>,
}

pub struct LoadResult {
    pub tab_idx: usize,
    pub url: String,
    pub result: Result<FetchResult, String>,
}

impl App {
    pub fn new(config: Config, storage: Storage, browser: Browser) -> Self {
        let (load_tx, load_rx) = tokio::sync::mpsc::unbounded_channel();

        Self {
            tabs: vec![Tab::welcome()],
            active_tab: 0,
            mode: Mode::Normal,
            url_input: String::new(),
            search_query: String::new(),
            search_results: Vec::new(),
            search_idx: 0,
            status_message: "NORMAL  |  ? for help".into(),
            should_quit: false,
            storage,
            browser: Arc::new(browser),
            config,
            load_tx,
            load_rx,
        }
    }

    // ── Tab management ─────────────────────────────────────────────────────

    pub fn active_tab(&self) -> &Tab {
        &self.tabs[self.active_tab]
    }

    pub fn active_tab_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active_tab]
    }

    pub fn add_tab(&mut self, url: String) {
        let idx = self.tabs.len();
        self.tabs.push(Tab::new(url.clone()));
        self.active_tab = idx;
        if url != "about:welcome" && !url.is_empty() {
            self.navigate(url);
        }
    }

    pub fn close_tab(&mut self) {
        if self.tabs.len() <= 1 {
            self.should_quit = true;
            return;
        }
        self.tabs.remove(self.active_tab);
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
        self.update_status("");
    }

    pub fn next_tab(&mut self) {
        if self.active_tab + 1 < self.tabs.len() {
            self.active_tab += 1;
            self.update_status("");
        }
    }

    pub fn prev_tab(&mut self) {
        if self.active_tab > 0 {
            self.active_tab -= 1;
            self.update_status("");
        }
    }

    // ── Navigation (G2: async via channel) ─────────────────────────────────

    pub fn navigate(&mut self, raw_url: String) {
        let url = build_url(&raw_url, &self.config.search_engine);

        let old_url = self.active_tab().url.clone();
        if old_url != "about:welcome" && old_url != url {
            let tab = self.active_tab_mut();
            tab.back_stack.push(old_url);
            tab.forward_stack.clear();
            tab.url = url.clone();
        } else {
            self.active_tab_mut().url = url.clone();
        }

        self.active_tab_mut().set_loading();
        self.status_message = format!("⏳ Loading {}...", url);

        // Spawn async fetch
        let tab_idx = self.active_tab;
        let url_clone = url.clone();
        let browser = self.browser.clone();
        let config = self.config.clone();
        let tx = self.load_tx.clone();

        tokio::spawn(async move {
            // Determine if headless is needed based on URL patterns
            let js_heavy = is_js_heavy_site(&url_clone);
            let result = browser.fetch(&url_clone, &config, js_heavy).await;
            let fetch_result = match result {
                Ok(fr) => fr,
                Err(_) => FetchResult::Blocked,
            };
            tx.send(LoadResult {
                tab_idx,
                url: url_clone,
                result: Ok(fetch_result),
            })
            .ok();
        });
    }

    pub fn go_back(&mut self) {
        let url = {
            let tab = self.active_tab_mut();
            if tab.back_stack.is_empty() {
                self.status_message = "No back history".into();
                return;
            }
            let url = tab.back_stack.pop().unwrap();
            tab.forward_stack.push(tab.url.clone());
            url
        };
        self.active_tab_mut().url = url.clone();
        self.active_tab_mut().set_loading();
        let tab_idx = self.active_tab;
        let url_clone = url.clone();
        let browser = self.browser.clone();
        let config = self.config.clone();
        let tx = self.load_tx.clone();

        tokio::spawn(async move {
            let result = browser.fetch(&url_clone, &config, false).await;
            tx.send(LoadResult {
                tab_idx,
                url: url_clone,
                result: result.map_err(|e| e.to_string()),
            })
            .ok();
        });
    }

    pub fn go_forward(&mut self) {
        let url = {
            let tab = self.active_tab_mut();
            if tab.forward_stack.is_empty() {
                self.status_message = "No forward history".into();
                return;
            }
            let url = tab.forward_stack.pop().unwrap();
            tab.back_stack.push(tab.url.clone());
            url
        };
        self.active_tab_mut().url = url.clone();
        self.active_tab_mut().set_loading();
        let tab_idx = self.active_tab;
        let url_clone = url.clone();
        let browser = self.browser.clone();
        let config = self.config.clone();
        let tx = self.load_tx.clone();

        tokio::spawn(async move {
            let result = browser.fetch(&url_clone, &config, false).await;
            tx.send(LoadResult {
                tab_idx,
                url: url_clone,
                result: result.map_err(|e| e.to_string()),
            })
            .ok();
        });
    }

    pub fn reload(&mut self) {
        let url = self.active_tab().url.clone();
        if url == "about:welcome" || url.is_empty() {
            return;
        }
        self.navigate(url);
    }

    pub fn refresh(&mut self) {
        self.reload();
    }

    // ── Process load results (called from event loop) ──────────────────────

    pub fn process_load_result(&mut self, result: LoadResult) {
        if result.tab_idx >= self.tabs.len() {
            return;
        }
        let tab = &mut self.tabs[result.tab_idx];
        if tab.url != result.url {
            return; // stale navigation
        }

        match result.result {
            Ok(FetchResult::Http { html, final_url, .. }) => {
                tab.url = final_url.clone();
                let page = render_html(&html, &final_url);
                let title = page.title.clone();
                tab.set_from_rendered(page);
                tab.wrap_content(80);

                // Save to history (G16: respects privacy mode)
                self.storage.add_history(&title, &final_url).ok();
                self.status_message = format!("Loaded: {}", title);
            }
            Ok(FetchResult::Headless { html }) => {
                // Try readability extraction first; fall back to HTML render
                let config = self.config.clone();
                match extract_content(&html, &tab.url, &config) {
                    Ok(Some(article)) => {
                        let title = if article.title.is_empty() {
                            tab.url.clone()
                        } else {
                            article.title.clone()
                        };
                        let page = render_html(&html, &tab.url);
                        tab.set_from_rendered(page);
                        tab.title = title.clone();
                        tab.wrap_content(80);
                        self.storage.add_history(&title, &tab.url).ok();
                        self.status_message = format!("Loaded (JS): {}", title);
                    }
                    _ => {
                        // Fallback to plain render
                        let page = render_html(&html, &tab.url);
                        let title = page.title.clone();
                        tab.set_from_rendered(page);
                        tab.wrap_content(80);
                        self.storage.add_history(&title, &tab.url).ok();
                        self.status_message = format!("Loaded: {}", title);
                    }
                }
            }
            Ok(FetchResult::Blocked) => {
                tab.set_error("Blocked by ad blocker (Ctrl+E to disable)");
                self.status_message = "Blocked by ad blocker.".into();
            }
            Err(e) => {
                tab.set_error(&e);
                self.status_message = format!("Error: {}", e);
            }
        }
    }

    // ── Search (in-page) ───────────────────────────────────────────────────

    pub fn run_search(&mut self) {
        let query = self.search_query.to_lowercase();
        self.search_results.clear();
        self.search_idx = 0;

        let results: Vec<(usize, String)> = self.active_tab()
            .display_lines
            .iter()
            .enumerate()
            .filter(|(_, (text, _))| text.to_lowercase().contains(&query))
            .map(|(i, (text, _))| (i, text.clone()))
            .collect();

        self.search_results = results;

        if !self.search_results.is_empty() {
            self.status_message = format!("Found {} matches", self.search_results.len());
            self.active_tab_mut().scroll = self.search_results[0].0;
        } else {
            self.status_message = "No matches found.".into();
        }
    }

    pub fn next_search(&mut self) {
        if self.search_results.is_empty() {
            return;
        }
        self.search_idx = (self.search_idx + 1) % self.search_results.len();
        self.active_tab_mut().scroll = self.search_results[self.search_idx].0;
    }

    pub fn prev_search(&mut self) {
        if self.search_results.is_empty() {
            return;
        }
        self.search_idx = if self.search_idx == 0 {
            self.search_results.len() - 1
        } else {
            self.search_idx - 1
        };
        self.active_tab_mut().scroll = self.search_results[self.search_idx].0;
    }

    // ── Bookmarks ──────────────────────────────────────────────────────────

    pub fn toggle_bookmark(&mut self) {
        let url = self.active_tab().url.clone();
        if url == "about:welcome" || url.is_empty() {
            self.status_message = "No page to bookmark.".into();
            return;
        }
        let title = self.active_tab().title.clone();
        match self.storage.toggle_bookmark(&title, &url) {
            Ok(true) => self.status_message = "★ Bookmark added!".into(),
            Ok(false) => self.status_message = "★ Bookmark removed.".into(),
            Err(e) => self.status_message = format!("Bookmark error: {}", e),
        }
    }

    pub fn open_bookmark(&mut self, idx: usize) {
        let bms = self.storage.bookmarks();
        if idx < bms.len() {
            self.navigate(bms[idx].url.clone());
        }
    }

    // ── History ────────────────────────────────────────────────────────────

    pub fn open_history_entry(&mut self, idx: usize) {
        let hist = self.storage.history(100);
        if idx < hist.len() {
            self.navigate(hist[idx].url.clone());
        }
    }

    // ── Settings ───────────────────────────────────────────────────────────

    pub fn set_search_engine(&mut self, name: &str) {
        if let Some(url) = crate::search_engine::engine_url(name) {
            self.config.search_engine = url.to_string();
            self.storage.update_config(|c| c.search_engine = url.to_string()).ok();
            self.status_message = format!("Search engine set to {}", name);
        }
    }

    pub fn set_homepage(&mut self, url: &str) {
        self.config.homepage = url.to_string();
        self.storage.update_config(|c| c.homepage = url.to_string()).ok();
        self.status_message = "Homepage updated.".into();
    }

    pub fn toggle_adblock(&mut self) {
        self.config.adblock_enabled = !self.config.adblock_enabled;
        let state = if self.config.adblock_enabled { "ON" } else { "OFF" };
        self.storage.update_config(|c| c.adblock_enabled = self.config.adblock_enabled).ok();
        self.status_message = format!("Ad blocker: {}", state);
    }

    pub fn toggle_privacy_mode(&mut self) {
        self.config.privacy_mode = !self.config.privacy_mode;
        let state = if self.config.privacy_mode { "ON" } else { "OFF" };
        self.storage.update_config(|c| c.privacy_mode = self.config.privacy_mode).ok();
        self.status_message = format!("Privacy mode: {}", state);
    }

    pub fn set_proxy(&mut self, url: &str) {
        let enabled = !url.is_empty();
        self.config.proxy_enabled = enabled;
        self.config.proxy_url = url.to_string();
        self.storage
            .update_config(|c| {
                c.proxy_enabled = enabled;
                c.proxy_url = url.to_string();
            })
            .ok();
        self.status_message = if enabled {
            format!("Proxy set to {}", url)
        } else {
            "Proxy disabled.".into()
        };
    }

    // ── Utilities ──────────────────────────────────────────────────────────

    pub fn update_status(&mut self, msg: &str) {
        if msg.is_empty() {
            self.status_message = "NORMAL  |  ? for help".into();
        } else {
            self.status_message = msg.into();
        }
    }

    pub fn scroll_down(&mut self, amount: usize) {
        let tab = self.active_tab_mut();
        tab.scroll = tab.scroll.saturating_add(amount).min(tab.scroll_max);
    }

    pub fn scroll_up(&mut self, amount: usize) {
        let tab = self.active_tab_mut();
        tab.scroll = tab.scroll.saturating_sub(amount);
    }

    pub fn scroll_to_top(&mut self) {
        self.active_tab_mut().scroll = 0;
    }

    pub fn scroll_to_bottom(&mut self) {
        let tab = self.active_tab_mut();
        tab.scroll = tab.scroll_max;
    }

    pub fn open_link_by_number(&mut self, num: usize) {
        if let Some(ref rendered) = self.active_tab().rendered {
            if num > 0 && num <= rendered.links.len() {
                let (href, _) = &rendered.links[num - 1];
                self.navigate(href.clone());
            }
        }
    }
}

fn is_js_heavy_site(url: &str) -> bool {
    let lower = url.to_lowercase();
    // Sites that are known to require JavaScript
    let js_heavy = [
        "chatgpt.com",
        "claude.ai",
        "chat.deepseek.com",
        "discord.com",
        "x.com",
        "twitter.com",
        "instagram.com",
        "reddit.com",
        "tiktok.com",
        "pinterest.com",
        "spotify.com",
        "netflix.com",
        "twitch.tv",
        "snapchat.com",
        "medium.com",
        "quora.com",
        "stackoverflow.com",
        "stackexchange.com",
        "zoom.us",
        "openai.com",
        "gemini.google.com",
        "messenger.com",
        "t.me",
        "whatsapp.com",
        "robinhood.com",
        "coinbase.com",
        " Figma",
        "miro.com",
        "notion.so",
        "linear.app",
        "excalidraw.com",
        "replit.com",
        "codesandbox.io",
        "glitch.com",
        "vercel.com",
        "netlify.com",
    ];
    js_heavy.iter().any(|site| lower.contains(site))
}
