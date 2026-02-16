<div align="center">

# harview

A fast, lightweight HTTP Archive (HAR) viewer for the terminal, written in Rust.

*Forked from [sheepla/harview](https://github.com/sheepla/harview) with added features and improvements. WIP with vibe code :-D*

[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)


<!-- Add a screenshot or demo GIF here -->
<!-- ![harview demo](./assets/demo.gif) -->

</div>

---

## Overview

**harview** lets you browse HAR files directly in your terminal—no browser required. It's designed for developers and security professionals who prefer the command line and want a quick, lightweight way to inspect HTTP traffic in the terminal.

## Features

### Core Functionality
- **Split-Pane Interface** — Request list on top, detailed view below
- **Tab Navigation** — Quickly switch between Headers, Cookies, Request Body, and Response Body
- **Syntax Highlighting** — Support for JSON, HTML, XML, JavaScript, and CSS
- **Pretty Printing** — Auto-formatted JSON and HTML for readability

### Mouse Support
- **Pane-Aware Scrolling** — Scroll independently in list or details pane based on hover
- **Visual Focus Indicator** — Active pane highlighted with a green border
- **Clickable Tabs** — Switch views with a single click

### Integrations
- **External JSON Viewer** — Open JSON bodies in [`fx`](https://fx.wtf/) for advanced querying and exploration

## Installation

### From Source

```sh
git clone https://github.com/armsultan/harview.git
cd harview
cargo install --path .
```

### Prerequisites

- Rust (cargo)
- `fx` (optional, for JSON viewing): `npm install -g fx` or via package manager.
- `bat` (optional, for enhanced text viewing): `brew install bat` or equivalent.

## Usage

### 1. Export a HAR File

Open your browser's DevTools (F12), go to the **Network** tab, and export the session as a HAR file.

### 2. View with harview

```sh
harview path/to/file.har
```

## Controls

### Keyboard

| Key | Action |
|-----|--------|
| `j` / `↓` | Move selection down |
| `k` / `↑` | Move selection up |
| `d` | Move down by 3 |
| `u` | Move up by 3 |
| `g` / `Home` | Jump to top |
| `G` / `End` | Jump to bottom |
| `Shift+↓` / `PageDown` | Scroll details pane down |
| `Shift+↑` / `PageUp` | Scroll details pane up |
| `1` – `4` | Switch to tab (Headers, Cookies, Request, Response) |
| `Left` / `Right` | Navigate Tabs (Headers, Cookies, Request, Response) |
| `h` | Toggle Syntax Highlighting (Performance mode) |
| `J` | Open JSON in `fx` (External viewer) |
| `b` | Open Body in `bat` (External viewer) |
| `q` / `Ctrl+C` | Quit |

### Mouse

| Action | Effect |
|--------|--------|
| Hover | Focus the list or details pane |
| Scroll | Scroll within the focused pane |
| Click Tab | Switch to the clicked tab |

## Enhancement ideas

- [ ] Search and filter requests
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