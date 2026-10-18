# TBrowser

[![Rust](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org/)
[![License: GPL v3+](https://img.shields.io/badge/License-GPLv3%2B-blue.svg)](LICENSE)
[![Version](https://img.shields.io/badge/version-0.99-blue.svg)]()

A terminal-based browser for **TNet** — a decentralized text network built on lightweight text protocols. TBrowser renders **TMkUp** documents directly in your terminal, converts images to ASCII art, and provides a fast, keyboard-driven browsing experience without JavaScript, ads, or trackers.

---

## Features

- **TNet protocols support** — TMUTP (text) and TBIP (binary info)
- **TMkUp rendering** — headings, ANSI colors, lists, tables, forms, spoilers, blocks
- **Inline image rendering** — PNG, JPEG, GIF, BMP, WebP converted to ASCII/Unicode half-blocks
- **Syntax highlighting** — powered by `syntect` with theme support
- **Bookmarks & history** — persistent navigation
- **Configurable** — TOML-based settings
- **Async I/O** — built on `tokio`
- **Terminal UI** — smooth experience with `ratatui` + `crossterm`
- **No JavaScript, no trackers, no bloat** — pure text

---

## Screenshots

> Screenshots coming soon. Run TBrowser and see it in action.

---

## Requirements

- **Rust** 1.85+ (edition 2024)
- A unicode terminal with ANSI color support
- **Linux**, **macOS**, or **Windows** (WSL recommended on Windows) 
- *(But in general, you can use it on any OS supported by Rust 2024 / Ratatui / CrossTerm.)*

---

## Installation

### From source

```bash
git clone https://github.com/TimeStop34/TBrowser.git
cd TBrowser
cargo build --release
./target/release/tbrowser
```

### Install to PATH

```bash
cargo install --path .
```

Then just run:

```bash
tbrowser
```

---

## Usage

### Open a TMUTP document

```bash
tbrowser tmutp://tnet.example/index.tmkup
```

### Fetch binary info (image)

```bash
tbrowser tbip://tnet.example/logo.png
```

### Interactive mode

```bash
tbrowser
```

Then type URLs directly in the address bar.

> **Note:** UTNP is a discovery protocol and cannot be used in URLs. It runs on port 34 and is used internally to negotiate which protocol the client should connect to. TBrowser handles this automatically when resolving TNet addresses.

---

## TMkUp Support

TBrowser fully supports **TMkUp v2.0** — the markup language of TNet.

Supported elements:

| Element | Syntax |
|---------|--------|
| Headings | `# H1 #` … `###### H6 ######` |
| Styles | `*italic*`, `**bold**`, `~~strike~~`, `==highlight==` |
| ANSI colors | `{red}…{/red}`, `{bg-blue}…{/bg-blue}` |
| Lists | `- item`, `1. item`, `- [x] done` |
| Links | `[text](url)` |
| Images | `![alt](url {height=90})` |
| Code | `` `inline` ``, ` ```lang ` |
| Info blocks | `!!! {type="info"} … !!!` |
| Color blocks | `%% {color="blue"} … %%` |
| Spoilers | `??? {title="…"} … ???` |
| Forms | `@@ {action="…"} … @@` |
| Tables | `\| a \| b \|` |
| Footnotes | `[^1]`, `[^1]: text` |
| Comments | `// line`, `/* block */` |
| Variables | `{{ variable }}` |

---

## Configuration

TBrowser stores its data and configuration at:

- **Linux/macOS:** `~/.config/tbrowser/`
- **Windows:** `%APPDATA%\tbrowser\`

Configuration files are managed by TBrowser itself and are not meant to be edited manually.

---

## Building

### Development build

```bash
cargo build
```

### Release build

```bash
cargo build --release
```

---

## License

This project is licensed under the **GNU General Public License v3.0 or later** — see the [LICENSE](LICENSE) file for details.

---

## Acknowledgments

- Inspired by **Gopher**, **Gemini**, and the early text-based internet
- Built with [ratatui](https://github.com/ratatui-org/ratatui), [crossterm](https://github.com/crossterm-rs/crossterm), [tokio](https://github.com/tokio-rs/tokio), [syntect](https://github.com/trishume/syntect)

---

**Enjoy browsing the text web!** 🌐