<p align="center">
  <img src="https://img.shields.io/badge/rust-1.81+-orange?logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/license-MIT-blue" alt="License">
  <img src="https://img.shields.io/github/v/release/irfancode/tuba?logo=github" alt="Release">
  <img src="https://img.shields.io/badge/platform-linux%20%7C%20macOS-lightgrey" alt="Platform">
</p>

<h1 align="center">Tuba</h1>
<p align="center"><strong>Unified terminal browser — fast, efficient, feature-rich</strong></p>

<p align="center">
  <code>Tuba</code> is a modern TUI browser for the terminal. Browse the web, read articles,
  follow links, manage bookmarks — all without leaving your command line.
  <br>
  Powered by <a href="https://ratatui.rs">Ratatui</a>,
  <a href="https://crates.io/crates/reqwest">reqwest</a>, and
  <a href="https://github.com/rust-headless-chrome/rust-headless-chrome">headless_chrome</a>.
</p>

---

## Features

- **Tabbed browsing** — multiple pages, switch with `Tab` / `Shift+Tab`
- **HTML rendering** — cyberpunk-themed TUI with links, tables, code blocks, and more
- **Reader mode** — extract and print articles to stdout (`--reader`)
- **Ad blocking** — built-in blocker (toggle with `Ctrl+E`)
- **Bookmarks & History** — persistent across sessions
- **Privacy mode** — no history saved (`Ctrl+P`)
- **Search engine integration** — DuckDuckGo, Google, Bing, Startpage, Brave
- **Headless Chromium** — fallback for JavaScript-rendered pages
- **Proxy support** — HTTP, SOCKS5, etc.
- **Settings screen** — configure everything at runtime (`Ctrl+S`)

---

## Binary Releases

Pre-built binaries are available for Linux and macOS on the [releases page](https://github.com/irfancode/tuba/releases).

| Platform | Architecture | Download |
|----------|-------------|----------|
| Linux | x86_64 | [tuba-x86_64-linux.tar.gz](https://github.com/irfancode/tuba/releases/latest) |
| Linux | aarch64 | [tuba-aarch64-linux.tar.gz](https://github.com/irfancode/tuba/releases/latest) |
| macOS | x86_64 | [tuba-x86_64-macos.tar.gz](https://github.com/irfancode/tuba/releases/latest) |
| macOS | aarch64 | [tuba-aarch64-macos.tar.gz](https://github.com/irfancode/tuba/releases/latest) |

> **Note:** Chromium/Chrome is required for JavaScript-rendered pages. Set the path via `--chrome` or the `TUBA_CHROME_PATH` environment variable.

---

## Installation

### From source

```bash
git clone https://github.com/irfancode/tuba.git
cd tuba
cargo build --release
cp target/release/tuba ~/.local/bin/
```

### Using `cargo install`

```bash
cargo install --git https://github.com/irfancode/tuba
```

### Using Docker

```bash
docker build -t tuba .
docker run --rm -it tuba https://example.com
```

### Using the Makefile

```bash
make build      # debug build
make release    # release build (LTO-optimized)
make install    # build release & copy to ~/.local/bin/
```

---

## Usage

```bash
# Open the welcome screen
tuba

# Navigate to a URL
tuba https://example.com

# Reader mode — print article to stdout
tuba --reader https://example.com/article

# With debugging enabled
tuba --debug https://example.com

# Override Chrome/Chromium path
tuba --chrome /usr/bin/chromium https://example.com

# Enable privacy mode
tuba --private https://example.com

# Use a SOCKS5 proxy
tuba --proxy socks5://127.0.0.1:1080 https://example.com
```

### CLI Options

| Option | Description |
|--------|-------------|
| `URL` | Target URL (optional — starts at welcome screen if omitted) |
| `--reader` | Strip page to article and print to stdout |
| `--timeout <ms>` | Page load timeout (default: 30000) |
| `--delay <sec>` | Extra delay after page load for late JS (default: 2.0) |
| `--no-links` | Strip hyperlinks from output (reader mode) |
| `--no-tables` | Strip tables from output (reader mode) |
| `--debug` | Enable debug logging |
| `--chrome <path>` | Path to Chrome/Chromium binary |
| `--no-adblock` | Disable ad blocker |
| `--private` | Enable privacy mode |
| `--proxy <url>` | Proxy URL (e.g. `socks5://127.0.0.1:1080`) |

---

## Key Bindings

| Key | Action |
|-----|--------|
| `q` / `Ctrl+C` | Quit |
| `j` / `Down` | Scroll down |
| `k` / `Up` | Scroll up |
| `Ctrl+D` | Scroll down 20 lines |
| `Ctrl+U` | Scroll up 20 lines |
| `g` | Scroll to top |
| `G` | Scroll to bottom |
| `PgDn` | Scroll down 40 lines |
| `PgUp` | Scroll up 40 lines |
| `Tab` | Next tab |
| `Shift+Tab` | Previous tab |
| `Ctrl+T` | New tab |
| `Ctrl+W` | Close tab |
| `Ctrl+L` | Enter URL/edit URL |
| `Enter` | Reload page |
| `b` | Go back |
| `f` | Go forward |
| `r` | Reload current page |
| `R` | Edit & reload URL |
| `/` | Search in page |
| `n` | Next search result |
| `N` | Previous search result |
| `Ctrl+B` | Show bookmarks |
| `Ctrl+D` | Toggle bookmark |
| `Ctrl+H` | Show history |
| `Ctrl+S` | Open settings |
| `Ctrl+E` | Toggle ad blocker |
| `Ctrl+P` | Toggle privacy mode |
| `?` | Show help |
| `1`-`9` | Open numbered link |

---

## Configuration

Configuration is stored in `~/.config/tuba/config.toml`. Settings can be modified at runtime via the settings screen (`Ctrl+S`) or by editing the file directly.

| Key | Default | Description |
|-----|---------|-------------|
| `homepage` | `about:welcome` | Homepage URL |
| `search_engine` | DuckDuckGo | Search engine |
| `adblock_enabled` | `true` | Enable ad blocking |
| `privacy_mode` | `false` | Disable history |
| `page_load_timeout_ms` | `30000` | Load timeout |
| `post_load_delay_s` | `2.0` | JS delay |
| `chrome_path` | `chromium` | Chrome binary path |
| `proxy_enabled` | `false` | Enable proxy |
| `proxy_url` | — | Proxy URL |

---

## Building

```bash
# Debug build
cargo build

# Release build with optimizations
cargo build --release

# Run tests
cargo test

# Format code
cargo fmt

# Lint
cargo clippy -- -D warnings

# Build documentation
cargo doc --no-deps --open
```

---

## Docker

```bash
make docker        # Build image
make docker-run    # Run with args (make docker-run ARGS=https://example.com)
```

---

## Dependencies

- **Runtime:** Chromium/Chrome for JavaScript-rendered pages (optional)
- **Build:** Rust 1.81+, `pkg-config`, `libssl-dev`, `libfontconfig1-dev`

---

## Contributing

Contributions are welcome! Please open an issue or submit a pull request.

1. Fork the repository
2. Create a feature branch
3. Commit your changes
4. Push to your branch
5. Open a Pull Request

---

## License

MIT License — see [LICENSE](LICENSE) for details.
