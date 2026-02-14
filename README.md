<div align="center">

# harview

HTTP Archive (HAR) Viewer on the terminal, written in Rust

*This tool is still under development. Please note that specifications are subject to change without notice.*

</div>

> [!NOTE]
> This is a fork of the original `harview` tool. I am building upon the original repository to add new features and improvements.

## About

**harview** is a viewer of HAR files that runs on the terminal. You can easily view HAR files exported from a web browser without opening the browser.

The goal of this tool is not to provide in-depth analysis capabilities like DevTool, but to provide the ability to browse HAR files with a lightweight UI like an easy-to-use pager for those familiar with the command line interface.

## New Features

This fork includes several enhancements over the original:

-   **Mouse Support**:
    -   **Split-Pane Scrolling**: Independent scrolling for the request list (top) and details pane (bottom) based on mouse hover.
    -   **Focus Highlighting**: The active pane is highlighted with a green border.
    -   **Clickable Tabs**: Switch between Headers, Cookies, Request, and Response by clicking the tabs.
-   **External Viewer Integration**: Open JSON bodies in `fx` for advanced exploration (requires `fx`).
-   **Enhanced UI**:
    -   **Timestamp Column**: Added to the request list.
    -   **Syntaxt Highlighting**: For JSON, HTML, XML, JS, and CSS.
    -   **Pretty Printing**: Auto-formatting for JSON and HTML.
    -   **Scrolling**: Proper scrolling support for Headers and Cookies views.

## Prerequisites

To use the external viewer feature, you need to have `fx` installed on your system.

```sh
npm install -g fx
```

## Usage

### Export HAR files from Browsers

Open your web browser's DevTools and export the HAR file.

### Use *harview* TUI

To use harview, specify the path of the HAR file as the first argument. Once the HAR file is loaded, entries in the HTTP log will appear in the table.

```sh
harview example.com.har
```

### Controls

**Keyboard Shortcuts**

| Key | Action |
|-----|---------|
| `k` / `Up` | Move focus up (List) |
| `j` / `Down` | Move focus down (List) |
| `u` | Move focus up by 3 |
| `d` | Move focus down by 3 |
| `g` / `Home` | Go to top |
| `G` / `End` | Go to bottom |
| `Shift+Up` / `PageUp` | Scroll Up (Details/Body) |
| `Shift+Down` / `PageDown` | Scroll Down (Details/Body) |
| `1` - `4` | Switch tab (Headers, Cookies, Request, Response) |
| `Left` / `Right` | Cycle through tabs |
| `Shift+j` / `J` | Open current JSON body in `fx` |
| `q` or `Ctrl-C` | Quit application |

**Mouse Actions**

| Action | Description |
|--------|-------------|
| **Hover** | Automatically focuses the top (List) or bottom (Details) pane. |
| **Scroll Wheel** | Scrolls the focused pane. |
| **Left Click** | Click on tabs (`[1] Headers`, etc.) to switch views. |

## Installation

Clone this repository then run `cargo install`

```sh
git clone https://github.com/sheepla/harview.git
cd harview
cargo install --path .
```

## References

- [HAR (file format) - Wikipedia](https://en.wikipedia.org/wiki/HAR_%28file_format%29)
- [HTTP Archive (HAR) Format Specifications - W3C](https://w3c.github.io/web-performance/specs/HAR/Overview.html)

## Thanks

- [ratatui](https://ratatui.rs/) - This tool was built with ratatui, a TUI library for Rust. Thank you for the amazing library and its ecosystem!
