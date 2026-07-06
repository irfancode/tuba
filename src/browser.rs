use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::Client;
use reqwest::cookie::Jar;

use crate::adblock::AdBlocker;
use crate::config::Config;

pub enum FetchResult {
    /// Plain HTTP fetch without JS rendering
    Http {
        html: String,
        final_url: String,
        status: u16,
        headers: Vec<(String, String)>,
    },
    /// Headless Chromium fetch with full JS execution
    Headless {
        html: String,
    },
    /// Page was blocked by adblock (G11)
    Blocked,
}

pub struct Browser {
    http_client: Client,
    adblocker: AdBlocker,
    cookie_jar: Arc<Jar>,
}

impl Browser {
    pub fn new(config: &Config) -> Result<Self> {
        let cookie_jar = Arc::new(Jar::default());

        let mut builder = Client::builder()
            .user_agent(&config.user_agent)
            .cookie_provider(cookie_jar.clone())
            .timeout(Duration::from_millis(config.page_load_timeout_ms))
            .danger_accept_invalid_certs(false) // G5: HTTPS cert validation
            .gzip(true)
            .brotli(true);

        // G12: Proxy support
        if config.proxy_enabled && !config.proxy_url.is_empty() {
            let proxy = reqwest::Proxy::all(&config.proxy_url)
                .context("Invalid proxy URL")?;
            builder = builder.proxy(proxy);
        }

        let http_client = builder.build()?;
        let adblocker = AdBlocker::new();

        Ok(Self {
            http_client,
            adblocker,
            cookie_jar,
        })
    }

    pub async fn fetch(&self, url: &str, config: &Config, use_headless: bool) -> Result<FetchResult> {
        // G11: Ad blocking check
        if config.adblock_enabled && self.adblocker.is_blocked(url) {
            return Ok(FetchResult::Blocked);
        }

        if use_headless {
            return self.fetch_headless(url, config).await;
        }

        self.fetch_http(url, config).await
    }

    pub async fn fetch_http(&self, url: &str, _config: &Config) -> Result<FetchResult> {
        let response = self.http_client
            .get(url)
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
            .header("Accept-Language", "en-US,en;q=0.9")
            .header("Sec-Fetch-Dest", "document")
            .header("Sec-Fetch-Mode", "navigate")
            .header("Sec-Fetch-Site", "none")
            .header("Sec-Fetch-User", "?1")
            .header("Upgrade-Insecure-Requests", "1")
            .send()
            .await?;

        let status = response.status().as_u16();
        let final_url = response.url().to_string();
        let headers: Vec<(String, String)> = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let html = response.text().await?;

        Ok(FetchResult::Http { html, final_url, status, headers })
    }

    /// Run headless Chrome in a blocking task (Tokio spawn_blocking) — G2 fix
    pub async fn fetch_headless(&self, url: &str, config: &Config) -> Result<FetchResult> {
        let url = url.to_string();
        let config = config.clone();

        let html = tokio::task::spawn_blocking(move || {
            fetch_headless_sync(&url, &config)
        })
        .await
        .context("Headless task panicked")?
        .context("Headless fetch failed")?;

        Ok(FetchResult::Headless { html })
    }

    pub fn cookie_jar(&self) -> Arc<Jar> {
        self.cookie_jar.clone()
    }
}

fn fetch_headless_sync(url: &str, config: &Config) -> Result<String> {
    use headless_chrome::{Browser, LaunchOptions};

    let timeout = Duration::from_millis(config.page_load_timeout_ms);

    let mut opts = LaunchOptions::default();
    opts.headless = true;
    opts.idle_browser_timeout = timeout;
    if let Some(ref path) = config.chrome_path {
        opts.path = Some(std::path::PathBuf::from(path));
    }

    let browser = Browser::new(opts).context("Failed to launch Chromium")?;

    let tab = browser.new_tab().context("Failed to create tab")?;

    // Anti-detection stealth JS (port from reader-rs)
    let stealth_js = r#"
        Object.defineProperty(navigator, 'webdriver', {get: () => undefined});
        Object.defineProperty(navigator, 'plugins', {get: () => [1, 2, 3, 4, 5]});
        Object.defineProperty(navigator, 'languages', {get: () => ['en-US', 'en']});
        window.chrome = {
            runtime: {
                connect: () => {},
                sendMessage: () => {},
                onMessage: {addListener: () => {}},
                onConnect: {addListener: () => {}},
            },
            loadTimes: () => {},
            csi: () => {},
            app: {isInstalled: false},
        };
    "#;
    tab.evaluate(stealth_js, false).ok();

    tab.navigate_to(url).context("Failed to navigate")?;
    tab.wait_until_navigated().context("Navigation timeout")?;

    if config.post_load_delay_s > 0.0 {
        std::thread::sleep(Duration::from_secs_f64(config.post_load_delay_s));
    }

    let html = tab.get_content().context("Failed to get page content")?;
    Ok(html)
}
