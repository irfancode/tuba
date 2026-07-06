use std::path::PathBuf;
use std::sync::Mutex;

use anyhow::Result;
use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::config::{Config, CONFIG_DIR, CONFIG_FILE, HISTORY_FILE, BOOKMARKS_FILE, SCHEMA_VERSION};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub title: String,
    pub url: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub title: String,
    pub url: String,
    pub visited_at: String,
}

pub struct Storage {
    config_path: PathBuf,
    history_path: PathBuf,
    bookmarks_path: PathBuf,
    dir: PathBuf,

    config: Mutex<Config>,
    bookmarks: Mutex<Vec<Bookmark>>,
    history: Mutex<Vec<HistoryEntry>>,
}

impl Storage {
    pub fn new() -> Self {
        let dir = dirs();
        Self::with_dir(dir)
    }

    pub fn with_dir(dir: PathBuf) -> Self {
        let config_path = dir.join(CONFIG_FILE);
        let history_path = dir.join(HISTORY_FILE);
        let bookmarks_path = dir.join(BOOKMARKS_FILE);

        Self {
            config_path,
            history_path,
            bookmarks_path,
            dir,
            config: Mutex::new(Config::default()),
            bookmarks: Mutex::new(Vec::new()),
            history: Mutex::new(Vec::new()),
        }
    }

    pub fn config(&self) -> Config {
        self.config.lock().unwrap().clone()
    }

    pub fn save_config(&self, config: &Config) -> Result<()> {
        {
            let mut c = self.config.lock().unwrap();
            *c = config.clone();
        }
        let cfg = self.config.lock().unwrap().clone();
        let json = serde_json::to_string_pretty(&cfg)?;
        std::fs::write(&self.config_path, json)?;
        Ok(())
    }

    pub fn update_config<F>(&self, f: F) -> Result<()>
    where
        F: FnOnce(&mut Config),
    {
        let mut guard = self.config.lock().unwrap();
        f(&mut guard);
        let cfg = guard.clone();
        drop(guard);
        let json = serde_json::to_string_pretty(&cfg)?;
        std::fs::write(&self.config_path, json)?;
        Ok(())
    }

    pub fn bookmarks(&self) -> Vec<Bookmark> {
        self.bookmarks.lock().unwrap().clone()
    }

    pub fn add_bookmark(&self, title: &str, url: &str) -> Result<()> {
        let mut bms = self.bookmarks.lock().unwrap();
        if bms.iter().any(|b| b.url == url) {
            return Ok(()); // Already bookmarked
        }
        let max = self.config.lock().unwrap().max_bookmarks;
        bms.push(Bookmark {
            title: title.to_string(),
            url: url.to_string(),
            created_at: Local::now().format("%Y-%m-%d %H:%M").to_string(),
        });
        if bms.len() > max {
            bms.remove(0);
        }
        self.flush_bookmarks(&bms)?;
        Ok(())
    }

    pub fn remove_bookmark(&self, url: &str) -> Result<()> {
        let mut bms = self.bookmarks.lock().unwrap();
        bms.retain(|b| b.url != url);
        self.flush_bookmarks(&bms)?;
        Ok(())
    }

    pub fn is_bookmarked(&self, url: &str) -> bool {
        self.bookmarks.lock().unwrap().iter().any(|b| b.url == url)
    }

    pub fn toggle_bookmark(&self, title: &str, url: &str) -> Result<bool> {
        if self.is_bookmarked(url) {
            self.remove_bookmark(url)?;
            Ok(false) // removed
        } else {
            self.add_bookmark(title, url)?;
            Ok(true) // added
        }
    }

    pub fn history(&self, limit: usize) -> Vec<HistoryEntry> {
        let hist = self.history.lock().unwrap();
        hist.iter().rev().take(limit).cloned().collect()
    }

    pub fn search_history(&self, query: &str) -> Vec<HistoryEntry> {
        let q = query.to_lowercase();
        let hist = self.history.lock().unwrap();
        hist.iter()
            .rev()
            .filter(|h| h.title.to_lowercase().contains(&q) || h.url.to_lowercase().contains(&q))
            .take(100)
            .cloned()
            .collect()
    }

    pub fn add_history(&self, title: &str, url: &str) -> Result<()> {
        let config = self.config.lock().unwrap().clone();
        if config.privacy_mode {
            return Ok(()); // G16: Privacy mode — no history
        }
        let mut hist = self.history.lock().unwrap();
        hist.push(HistoryEntry {
            title: title.to_string(),
            url: url.to_string(),
            visited_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        });
        if hist.len() > config.max_history {
            hist.remove(0);
        }
        self.flush_history(&hist)?;
        Ok(())
    }

    pub fn clear_history(&self) -> Result<()> {
        self.history.lock().unwrap().clear();
        self.flush_history(&self.history.lock().unwrap())?;
        Ok(())
    }

    pub fn clear_bookmarks(&self) -> Result<()> {
        self.bookmarks.lock().unwrap().clear();
        self.flush_bookmarks(&self.bookmarks.lock().unwrap())?;
        Ok(())
    }

    // -- Load / Save ---------------------------------------------------------

    pub fn load(&self) -> Result<()> {
        std::fs::create_dir_all(&self.dir)?;

        // Config
        if self.config_path.exists() {
            let data = std::fs::read_to_string(&self.config_path)?;
            if let Ok(cfg) = serde_json::from_str::<Config>(&data) {
                if cfg.schema_version == SCHEMA_VERSION {
                    *self.config.lock().unwrap() = cfg;
                }
            }
        }

        // Bookmarks
        if self.bookmarks_path.exists() {
            let data = std::fs::read_to_string(&self.bookmarks_path)?;
            if let Ok(bms) = serde_json::from_str::<Vec<Bookmark>>(&data) {
                *self.bookmarks.lock().unwrap() = bms;
            }
        }

        // History
        if self.history_path.exists() {
            let data = std::fs::read_to_string(&self.history_path)?;
            if let Ok(hist) = serde_json::from_str::<Vec<HistoryEntry>>(&data) {
                *self.history.lock().unwrap() = hist;
            }
        }

        Ok(())
    }

    fn flush_bookmarks(&self, bms: &[Bookmark]) -> Result<()> {
        let json = serde_json::to_string_pretty(bms)?;
        std::fs::write(&self.bookmarks_path, json)?;
        Ok(())
    }

    fn flush_history(&self, hist: &[HistoryEntry]) -> Result<()> {
        let json = serde_json::to_string_pretty(hist)?;
        std::fs::write(&self.history_path, json)?;
        Ok(())
    }
}

fn dirs() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join(".config").join(CONFIG_DIR)
}
