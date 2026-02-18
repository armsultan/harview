<div align="center">

# harview

A fast, lightweight HTTP Archive (HAR) viewer for the terminal, written in Rust.

*Forked from [sheepla/harview](https://github.com/sheepla/harview) with added features and improvements. WIP with vibe code :-D*

[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

![harview demo](./media/demo.gif)

</div>

---

## Overview

**harview** lets you browse HAR files directly in your terminal—no browser required. It's designed for developers and security professionals who prefer the command line and want a quick, lightweight way to inspect HTTP traffic in the terminal.

## Features

### Core Functionality
- **Split-Pane Interface** — Request list on top, detailed view below
- **Search & Filter** — Vim-style `/` search with full regex support; filter by URL, host, headers, body, status code, method, size, duration, and more. Matches are highlighted in both the table and the detail pane
- **Tab Navigation** — Quickly switch between Headers, Cookies, Request, Response, and Help tabs
- **Syntax Highlighting** — Toggle with `h`. Supports JSON, HTML, XML, JavaScript, and CSS. Automatically skipped for bodies over 200KB to keep the UI responsive — use `b` to open large content in `bat` instead
- **Pretty Printing** — Auto-formatted JSON and XML/HTML for readability
- **Base64 Decoding** — Response bodies with base64 encoding are automatically decoded for display

### Request Table

The top pane displays a list of all entries with the following columns:

| Column | Description |
|--------|-------------|
| Status | HTTP status code (color-coded by class) |
| Method | HTTP method (GET, POST, etc.) |
| URL | Full request URL |
| ContentType | Response MIME type |
| Size | Response body size |
| Timestamp | Request start time (`HH:MM:SS.mmm`) |

### Mouse Support
- **Pane-Aware Scrolling** — Scroll independently in list or details pane based on cursor position
- **Visual Focus Indicator** — Active pane highlighted with a green border
- **Clickable Tabs** — Switch views with a single click
- **Clickable Rows** — Click a row in the request table to select it

### Integrations

External viewers are available on the **Request** and **Response** tabs. They open the current tab's body in an external program for full-featured viewing.

- **`fx`** — Open JSON bodies for advanced querying and exploration (only activates for JSON content)
- **`bat`** — Open bodies with syntax highlighting. File type is auto-detected from the response MIME type (json, html, js, css, xml)
- **`$EDITOR`** — Open bodies in your preferred editor (falls back to `vi`)

## Installation

### From Source

```sh
git clone https://github.com/armsultan/harview.git
cd harview
cargo install --path . --locked
```

If needed, add `$HOME/.cargo/bin` to your `PATH` to be able to run the installed binaries

```bash
#zsh
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc && source ~/.zshrc
# bash
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc && source ~/.bashrc

# To verify it worked:
which harview
```

### Prerequisites

- Rust (cargo)
- [`fx`](https://fx.wtf/) (optional, for JSON viewing): `npm install -g fx` or via package manager
- [`bat`](https://github.com/sharkdp/bat) (optional, for enhanced text viewing): `brew install bat` or equivalent

## Usage

### 1. Export a HAR File

Open your browser's DevTools (F12), go to the **Network** tab, and export the session as a HAR file.

### 2. View with harview

```sh
harview path/to/file.har
```

## Controls

### Keyboard

#### Search & Filter

| Key | Action |
|-----|--------|
| `/` | Enter search mode |
| `Tab` | Cycle search scope (see scopes below) |
| `Enter` | Confirm filter and return to normal mode |
| `Esc` (search mode) | Cancel — restores the previous filter state |
| `Esc` (normal mode) | Clear the active filter |

The search bar appears at the bottom of the request table while active:

```
/ [ALL] api\.example▏                              47/312
```

**Search scopes** (cycle with `Tab`):

| Scope | Searches |
|-------|----------|
| `ALL` | Every field listed below |
| `URL` | Full request URL |
| `Host` | Hostname only |
| `QueryStr` | Query string parameters (`key=value`) |
| `ReqHdrs` | Request headers (`Name: Value`) |
| `RespHdrs` | Response headers |
| `ReqBody` | Request body |
| `RespBody` | Response body (base64 decoded automatically) |
| `Method` | HTTP method (e.g. `GET`, `POST`) |
| `Status` | HTTP status code (e.g. `404`) |
| `ReqSize` | Request body size in bytes |
| `RespSize` | Response body size in bytes |
| `Duration` | Total request duration in ms |

Regex examples: `^GET`, `4\d{2}`, `application/json`, `api/v[0-9]+`

#### Navigation

| Key | Action |
|-----|--------|
| `j` / `↓` | Move selection down |
| `k` / `↑` | Move selection up |
| `d` | Move down by 3 |
| `u` | Move up by 3 |
| `g` | Jump to first entry |
| `G` | Jump to last entry |

#### Details Pane Scrolling

| Key | Action |
|-----|--------|
| `Shift+↑` | Scroll up by 1 line |
| `Shift+↓` | Scroll down by 1 line |
| `PageUp` | Scroll up by 10 lines |
| `PageDown` | Scroll down by 10 lines |

#### Tabs & Views

| Key | Action |
|-----|--------|
| `1` – `4` | Switch to tab (Headers, Cookies, Request, Response) |
| `←` / `→` | Cycle through tabs |
| `?` | Show help tab with all keybindings |
| `h` | Toggle syntax highlighting |

#### External Viewers (Request/Response tabs only)

| Key | Action |
|-----|--------|
| `J` | Open JSON in `fx` |
| `b` | Open body in `bat` |
| `o` | Open body in `$EDITOR` (default: `vi`) |

#### General

| Key | Action |
|-----|--------|
| `q` | Quit |
| `Ctrl+C` | Quit |

### Mouse

| Action | Effect |
|--------|--------|
| Scroll (top pane) | Move selection up/down |
| Scroll (bottom pane) | Scroll details content |
| Click row | Select that entry |
| Click tab | Switch to the clicked tab |

## Enhancement ideas

- [x] Search and filter requests
- [ ] Export selected entries
- [ ] Support for additional content types
- [ ] Configurable color themes

## References

- [HAR File Format — Wikipedia](https://en.wikipedia.org/wiki/HAR_%28file_format%29)
- [HAR Specification — W3C](https://w3c.github.io/web-performance/specs/HAR/Overview.html)

## Acknowledgments

Built with [ratatui](https://ratatui.rs/), a powerful TUI library for Rust.

## License

This project is licensed under the [MIT License](LICENSE).