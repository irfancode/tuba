use std::io;

use anyhow::Result;
use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use tuba::app::{App, Mode};
use tuba::browser::Browser;
use tuba::config::Config;
use tuba::display;
use tuba::storage::Storage;

#[derive(Parser)]
#[command(
    name = "tuba",
    about = "Tuba — Unified terminal browser (fast, efficient, feature-rich)",
    version = "1.1.0",
)]
struct Cli {
    /// Target URL (optional — if omitted, starts at the welcome screen)
    url: Option<String>,

    /// Reader mode: strip page to article and print to stdout
    #[arg(long, short)]
    reader: bool,

    /// Page load timeout in milliseconds
    #[arg(long, default_value_t = 30_000)]
    timeout: u64,

    /// Extra delay after page load for late JS (seconds)
    #[arg(long, default_value_t = 2.0)]
    delay: f64,

    /// Strip hyperlinks from output (reader mode only)
    #[arg(long)]
    no_links: bool,

    /// Strip tables from output (reader mode only)
    #[arg(long)]
    no_tables: bool,

    /// Enable debug logging
    #[arg(long, short)]
    debug: bool,

    /// Path to Chrome/Chromium binary
    #[arg(long)]
    chrome: Option<String>,

    /// Disable ad blocker (G11)
    #[arg(long)]
    no_adblock: bool,

    /// Enable privacy mode — no history saved (G16)
    #[arg(long)]
    private: bool,

    /// Proxy URL (e.g. socks5://127.0.0.1:1080) (G12)
    #[arg(long)]
    proxy: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Logging (G34)
    if cli.debug {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
            .init();
    } else {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
            .init();
    }

    // Load config & storage
    let storage = Storage::new();
    storage.load()?;

    // Apply CLI overrides to config
    storage.update_config(|c| {
        c.page_load_timeout_ms = cli.timeout;
        c.post_load_delay_s = cli.delay;
        c.include_links = !cli.no_links;
        c.include_tables = !cli.no_tables;
        if let Some(ref path) = cli.chrome {
            c.chrome_path = Some(path.clone());
        }
        if cli.no_adblock {
            c.adblock_enabled = false;
        }
        if cli.private {
            c.privacy_mode = true;
        }
        if let Some(ref proxy_url) = cli.proxy {
            c.proxy_enabled = true;
            c.proxy_url = proxy_url.clone();
        }
    })?;

    let config = storage.config();
    let browser = Browser::new(&config)?;

    // ── Reader mode (CLI, no TUI) ────────────────────────────────────────────
    if cli.reader {
        if let Some(url) = cli.url {
            return run_reader(&url, &config, &browser).await;
        }
        eprintln!("Error: --reader requires a URL");
        std::process::exit(1);
    }

    // ── TUI browser mode ────────────────────────────────────────────────────
    let mut app = App::new(config, storage, browser);

    // If a URL was provided, navigate to it
    if let Some(url) = cli.url {
        let full_url = if url.starts_with("http://") || url.starts_with("https://") {
            url
        } else {
            format!("https://{}", url)
        };
        app.add_tab(full_url);
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let res = run_app(&mut terminal, &mut app).await;

    // Cleanup
    disable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = res {
        eprintln!("Error: {}", e);
    }

    Ok(())
}

async fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| tuba::ui::draw(f, app))?;

        // Process any pending load results (non-blocking)
        while let Ok(result) = app.load_rx.try_recv() {
            app.process_load_result(result);
        }

        // Check if we should quit
        if app.should_quit {
            break;
        }

        // Wait for input (with timeout so we can check load results)
        if !crossterm::event::poll(std::time::Duration::from_millis(50))? {
            continue;
        }

        let event = match event::read() {
            Ok(e) => e,
            Err(_) => continue,
        };

        let Event::Key(key) = event else {
            continue;
        };

        handle_key(app, key);
    }
    Ok(())
}

fn handle_key(app: &mut App, key: KeyEvent) {
    match app.mode {
        Mode::Normal => handle_normal(app, key),
        Mode::UrlInput => handle_url_input(app, key),
        Mode::Search => handle_search_input(app, key),
        Mode::Bookmarks => handle_bookmarks(app, key),
        Mode::History => handle_history(app, key),
        Mode::Settings => handle_settings(app, key),
        Mode::Help => {
            if matches!(key.code, KeyCode::Esc) || key.code == KeyCode::Char('?') {
                app.mode = Mode::Normal;
                app.update_status("");
            }
        }
    }
}

// ── Normal mode ────────────────────────────────────────────────────────────

fn handle_normal(app: &mut App, key: KeyEvent) {
    // Quit
    if key.code == KeyCode::Char('q')
        || (key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL)
    {
        app.should_quit = true;
        return;
    }

    match key.code {
        // Tabs
        KeyCode::Char('t') if key.modifiers == KeyModifiers::CONTROL => {
            app.add_tab("about:welcome".into());
            app.mode = Mode::UrlInput;
            app.url_input = String::new();
        }
        KeyCode::Char('w') if key.modifiers == KeyModifiers::CONTROL => {
            app.close_tab();
        }
        KeyCode::Tab => app.next_tab(),
        KeyCode::BackTab => app.prev_tab(),

        // Navigation
        KeyCode::Char('l') if key.modifiers == KeyModifiers::CONTROL => {
            app.mode = Mode::UrlInput;
            app.url_input = if app.active_tab().url == "about:welcome" {
                String::new()
            } else {
                app.active_tab().url.clone()
            };
        }
        KeyCode::Enter => {
            let url = app.active_tab().url.clone();
            if url != "about:welcome" {
                app.navigate(url);
            }
        }
        KeyCode::Char('b') => app.go_back(),
        KeyCode::Char('f') => app.go_forward(),
        KeyCode::Char('r') => app.reload(),
        KeyCode::Char('R') => {
            app.mode = Mode::UrlInput;
            app.url_input = app.active_tab().url.clone();
        }

        // Scroll
        KeyCode::Char('j') | KeyCode::Down => app.scroll_down(1),
        KeyCode::Char('k') | KeyCode::Up => app.scroll_up(1),
        KeyCode::Char('d') if key.modifiers == KeyModifiers::CONTROL => app.scroll_down(20),
        KeyCode::Char('u') if key.modifiers == KeyModifiers::CONTROL => app.scroll_up(20),
        KeyCode::Char('g') => app.scroll_to_top(),
        KeyCode::Char('G') => app.scroll_to_bottom(),
        KeyCode::PageDown => app.scroll_down(40),
        KeyCode::PageUp => app.scroll_up(40),

        // Search
        KeyCode::Char('/') => {
            app.mode = Mode::Search;
            app.search_query = String::new();
            app.search_results.clear();
            app.search_idx = 0;
        }
        KeyCode::Char('n') => app.next_search(),
        KeyCode::Char('N') => app.prev_search(),

        // Bookmarks
        KeyCode::Char('B') if key.modifiers == KeyModifiers::CONTROL => {
            app.mode = Mode::Bookmarks;
        }
        KeyCode::Char('d') if key.modifiers == KeyModifiers::CONTROL => {
            app.toggle_bookmark();
        }

        // History
        KeyCode::Char('h') if key.modifiers == KeyModifiers::CONTROL => {
            app.mode = Mode::History;
        }

        // Settings (G25: external settings screen)
        KeyCode::Char('s') if key.modifiers == KeyModifiers::CONTROL => {
            app.mode = Mode::Settings;
        }

        // Ad blocker toggle (G11)
        KeyCode::Char('e') if key.modifiers == KeyModifiers::CONTROL => {
            app.toggle_adblock();
        }

        // Privacy mode toggle (G16)
        KeyCode::Char('p') if key.modifiers == KeyModifiers::CONTROL => {
            app.toggle_privacy_mode();
        }

        // Help
        KeyCode::Char('?') => {
            app.mode = Mode::Help;
        }

        // Link navigation by number (1-9)
        KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
            let num = (c as usize) - ('0' as usize);
            app.open_link_by_number(num);
        }

        _ => {}
    }
}

// ── URL input mode ─────────────────────────────────────────────────────────

fn handle_url_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            let url = app.url_input.trim().to_string();
            if !url.is_empty() {
                app.navigate(url);
            }
            app.mode = Mode::Normal;
        }
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.update_status("");
        }
        KeyCode::Char(c) => {
            app.url_input.push(c);
        }
        KeyCode::Backspace => {
            app.url_input.pop();
        }
        KeyCode::Tab => {
            app.next_tab();
            app.mode = Mode::UrlInput;
            app.url_input = app.active_tab().url.clone();
        }
        _ => {}
    }
}

// ── Search mode ────────────────────────────────────────────────────────────

fn handle_search_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            app.run_search();
            app.mode = Mode::Normal;
        }
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.search_query = String::new();
            app.search_results.clear();
            app.update_status("");
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
        }
        KeyCode::Backspace => {
            app.search_query.pop();
        }
        _ => {}
    }
}

// ── Bookmarks mode ─────────────────────────────────────────────────────────

fn handle_bookmarks(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.update_status("");
        }
        KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
            let idx = (c as usize) - ('1' as usize);
            app.open_bookmark(idx);
            app.mode = Mode::Normal;
        }
        KeyCode::Char('d') => {
            let bms = app.storage.bookmarks();
            if let Some(last) = bms.last() {
                app.storage.remove_bookmark(&last.url).ok();
                app.status_message = "Last bookmark removed.".into();
            }
        }
        _ => {}
    }
}

// ── History mode ───────────────────────────────────────────────────────────

fn handle_history(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.update_status("");
        }
        KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
            let idx = (c as usize) - ('1' as usize);
            app.open_history_entry(idx);
            app.mode = Mode::Normal;
        }
        KeyCode::Char('c') | KeyCode::Char('C') => {
            app.storage.clear_history().ok();
            app.status_message = "History cleared.".into();
            app.mode = Mode::Normal;
        }
        _ => {}
    }
}

// ── Settings mode ──────────────────────────────────────────────────────────

fn handle_settings(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.update_status("");
        }
        KeyCode::Char('a') => app.toggle_adblock(),
        KeyCode::Char('p') => app.toggle_privacy_mode(),
        KeyCode::Char('h') => {
            app.mode = Mode::UrlInput;
            app.url_input = app.config.homepage.clone();
            app.status_message = "Enter new homepage URL and press Enter.".into();
        }
        KeyCode::Char('s') => {
            // Cycle through search engines
            let engines = tuba::search_engine::engine_names();
            let current = engines
                .iter()
                .position(|n| tuba::search_engine::engine_url(n) == Some(&app.config.search_engine))
                .unwrap_or(0);
            let next = (current + 1) % engines.len();
            app.set_search_engine(engines[next]);
        }
        _ => {}
    }
}

// ── Reader mode (CLI) ──────────────────────────────────────────────────────

async fn run_reader(url: &str, config: &Config, browser: &Browser) -> Result<()> {
    log::info!("Tuba reader mode: fetching {}", url);

    let fetch_result = browser.fetch(url, config, true).await?;

    match fetch_result {
        tuba::browser::FetchResult::Http { html, .. } | tuba::browser::FetchResult::Headless { html } => {
            log::info!("Fetching complete, extracting content...");
            let article = tuba::purify::extract_content(&html, url, config)?;
            display::render_article(&article, config);
        }
        tuba::browser::FetchResult::Blocked => {
            display::render_error("Blocked by ad blocker.", url);
        }
    }

    Ok(())
}
