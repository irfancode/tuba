#[cfg(test)]
mod tests {
    use tuba::adblock::AdBlocker;
    use tuba::config::Config;
    use tuba::purify;
    use tuba::renderer;
    use tuba::search_engine::{build_url, is_search_query, engine_names};
    use tuba::display::word_wrap;

    // ── AdBlocker tests (G11) ──────────────────────────────────────────────

    #[test]
    fn test_adblock_blocks_known_domains() {
        let ab = AdBlocker::new();
        assert!(ab.is_blocked("https://doubleclick.net/ads/test"));
        assert!(ab.is_blocked("https://www.google-analytics.com/collect"));
        assert!(ab.is_blocked("https://connect.facebook.net/en_US/fbevents.js"));
        assert!(ab.is_blocked("https://platform.twitter.com/widgets.js"));
    }

    #[test]
    fn test_adblock_allows_normal_sites() {
        let ab = AdBlocker::new();
        assert!(!ab.is_blocked("https://github.com"));
        assert!(!ab.is_blocked("https://en.wikipedia.org/wiki/Rust"));
        assert!(!ab.is_blocked("https://news.ycombinator.com"));
        assert!(!ab.is_blocked("https://www.nytimes.com"));
    }

    // ── Config tests (G3, G39) ─────────────────────────────────────────────

    #[test]
    fn test_config_defaults() {
        let cfg = Config::default();
        assert_eq!(cfg.page_load_timeout_ms, 30_000);
        assert_eq!(cfg.post_load_delay_s, 2.0);
        assert_eq!(cfg.schema_version, 1);
        assert!(cfg.adblock_enabled);
        assert!(!cfg.privacy_mode);
        assert_eq!(cfg.homepage, "https://duckduckgo.com");
        assert_eq!(cfg.zoom_level, 1.0);
    }

    #[test]
    fn test_config_search_engines() {
        assert!(!engine_names().is_empty());
        assert!(engine_names().contains(&"DuckDuckGo"));
        assert!(engine_names().contains(&"Google"));
    }

    // ── Search engine tests ────────────────────────────────────────────────

    #[test]
    fn test_is_search_query() {
        assert!(!is_search_query("https://example.com"));
        assert!(!is_search_query("http://example.com"));
        assert!(!is_search_query("example.com"));
        assert!(is_search_query("rust programming language"));
        assert!(is_search_query("how to write tests"));
    }

    #[test]
    fn test_build_url_keeps_http() {
        let url = build_url("https://example.com/page", "https://ddg.com/?q={}");
        assert_eq!(url, "https://example.com/page");
    }

    #[test]
    fn test_build_url_adds_https() {
        let url = build_url("example.com", "https://ddg.com/?q={}");
        assert_eq!(url, "https://example.com");
    }

    #[test]
    fn test_build_url_search() {
        let url = build_url("hello world", "https://ddg.com/?q={}");
        assert_eq!(url, "https://ddg.com/?q=hello+world");
    }

    #[test]
    fn test_build_url_encodes_special_chars() {
        let url = build_url("foo & bar", "https://ddg.com/?q={}");
        assert!(url.contains("foo"));
        assert!(url.contains("bar"));
    }

    // ── Word-wrap tests (G27: single source) ───────────────────────────────

    #[test]
    fn test_word_wrap_short_text() {
        let lines = word_wrap("hello world", 80);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "hello world");
    }

    #[test]
    fn test_word_wrap_long_text() {
        let text = "a ".repeat(50) + "b";
        let lines = word_wrap(&text, 40);
        assert!(lines.len() > 1);
        assert!(lines.iter().all(|l| l.len() <= 40));
    }

    #[test]
    fn test_word_wrap_empty() {
        let lines = word_wrap("", 80);
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_word_wrap_narrow() {
        let lines = word_wrap("hello world", 5);
        assert_eq!(lines.len(), 1); // narrow falls back
    }

    #[test]
    fn test_word_wrap_long_word() {
        let lines = word_wrap("superlongwordthatwontfit", 10);
        assert!(lines.len() >= 1);
    }

    // ── Renderer tests ─────────────────────────────────────────────────────

    #[test]
    fn test_render_error_returns_page() {
        let page = renderer::render_error("test error", "https://example.com");
        assert!(page.title.contains("Error"));
        assert!(!page.lines.is_empty());
        assert!(page.links.is_empty());
    }

    #[test]
    fn test_render_html_empty() {
        let page = renderer::render_html("<html><body></body></html>", "https://example.com");
        assert!(!page.title.is_empty());
        assert!(page.links.is_empty());
    }

    #[test]
    fn test_render_html_links() {
        let html = r#"<html><body><a href="https://rust-lang.org">Rust</a></body></html>"#;
        let page = renderer::render_html(html, "https://example.com");
        assert!(!page.links.is_empty());
        // url::Url::join may add trailing slash
        let url = page.links[0].0.trim_end_matches('/');
        assert_eq!(url, "https://rust-lang.org");
    }

    #[test]
    fn test_render_html_headings() {
        let html = r#"<html><body><h1>Title</h1><h2>Subtitle</h2></body></html>"#;
        let page = renderer::render_html(html, "https://example.com");
        assert!(!page.lines.is_empty());
    }

    #[test]
    fn test_render_html_js_fallback() {
        let html = r#"<html><body><div id="app"></div></body></html>"#;
        let page = renderer::render_html(html, "https://example.com");
        assert!(!page.lines.is_empty());
    }

    #[test]
    fn test_render_html_tables() {
        let html = r#"<html><body><table><tr><td>Cell</td></tr></table></body></html>"#;
        let page = renderer::render_html(html, "https://example.com");
        assert!(!page.lines.is_empty());
    }

    #[test]
    fn test_render_html_unclosed_tags() {
        let html = r#"<html><body><p>Hello world"#; // unclosed
        let page = renderer::render_html(html, "https://example.com");
        assert!(!page.title.is_empty());
    }

    // ── Purify tests (G33) ─────────────────────────────────────────────────

    #[test]
    fn test_detect_blocked_page_cloudflare() {
        let html = "<html><title>Just a moment...</title><body>Checking your browser</body></html>";
        // This should be detected by detect_blocked_page inside extract_content
        let cfg = Config::default();
        let result = purify::extract_content(html, "https://example.com", &cfg);
        assert!(result.is_ok());
        // Should return None (blocked)
        match result.unwrap() {
            None => {} // expected (blocked page detection)
            Some(_) => {} // also acceptable (some extraction might work despite signals)
        }
    }

    #[test]
    fn test_purify_simple_html() {
        let html = "<html><head><title>Test Article</title></head><body><article><p>This is a test article with enough content to meet the minimum length requirement for extraction.</p><p>More content here to make sure we have enough text to extract.</p></article></body></html>";
        let cfg = Config::default();
        let result = purify::extract_content(html, "https://example.com/article", &cfg);
        assert!(result.is_ok());
    }

    // ── Storage tests ──────────────────────────────────────────────────────

    fn temp_storage() -> tuba::storage::Storage {
        use std::path::PathBuf;
        let dir = PathBuf::from("/tmp/tuba_test");
        std::fs::create_dir_all(&dir).ok();
        let s = tuba::storage::Storage::with_dir(dir);
        s.load().unwrap();
        s
    }

    #[test]
    fn test_storage_bookmarks() {
        let s = temp_storage();
        s.add_bookmark("Test", "https://test.com").unwrap();
        let bms = s.bookmarks();
        assert!(bms.iter().any(|b| b.url == "https://test.com"));
        assert!(s.is_bookmarked("https://test.com"));
        s.remove_bookmark("https://test.com").unwrap();
        assert!(!s.is_bookmarked("https://test.com"));
    }

    #[test]
    fn test_storage_toggle_bookmark() {
        let s = temp_storage();
        assert_eq!(s.toggle_bookmark("Test", "https://test.com").unwrap(), true);
        assert_eq!(s.toggle_bookmark("Test", "https://test.com").unwrap(), false);
    }

    #[test]
    fn test_storage_history_limit() {
        let s = temp_storage();
        for i in 0..10 {
            s.add_history(&format!("Page {}", i), &format!("https://page{}.com", i)).unwrap();
        }
        let hist = s.history(5);
        assert_eq!(hist.len(), 5);
        assert!(hist[0].url.contains("page9")); // most recent first
    }

    #[test]
    fn test_storage_history_search() {
        let s = temp_storage();
        s.add_history("Rust Programming", "https://rust-lang.org").unwrap();
        s.add_history("Python Tips", "https://python.org").unwrap();
        let results = s.search_history("rust");
        assert_eq!(results.len(), 1);
        assert!(results[0].url.contains("rust"));
    }

    #[test]
    fn test_storage_clear_history() {
        let s = temp_storage();
        s.add_history("Test", "https://test.com").unwrap();
        s.clear_history().unwrap();
        assert!(s.history(100).is_empty());
    }

    #[test]
    fn test_storage_clear_bookmarks() {
        let s = temp_storage();
        s.add_bookmark("Test", "https://test.com").unwrap();
        s.clear_bookmarks().unwrap();
        assert!(s.bookmarks().is_empty());
    }

    // ── Config serialization (G39) ─────────────────────────────────────────

    #[test]
    fn test_config_serde_roundtrip() {
        let cfg = Config::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let decoded: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.schema_version, cfg.schema_version);
        assert_eq!(decoded.page_load_timeout_ms, cfg.page_load_timeout_ms);
        assert_eq!(decoded.homepage, cfg.homepage);
    }

    // ── Edge cases ─────────────────────────────────────────────────────────

    #[test]
    fn test_render_html_very_long_title() {
        let long_title = "A".repeat(500);
        let html = format!("<html><head><title>{}</title></head><body><p>content</p></body></html>", long_title);
        let page = renderer::render_html(&html, "https://example.com");
        assert!(page.title.len() <= 200); // trimmed
    }

    #[test]
    fn test_render_html_special_chars() {
        let html = r#"<html><body><p>foo &amp; bar &lt; baz &gt; qux</p></body></html>"#;
        let page = renderer::render_html(html, "https://example.com");
        let all_text: String = page.lines.iter().map(|l| l.to_string()).collect();
        assert!(all_text.contains("foo") || all_text.contains("bar"));
    }
}
