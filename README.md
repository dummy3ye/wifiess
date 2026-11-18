# wifiess

A remarkable WiFi manager for the terminal, written in pure Rust.

> **Work in progress.**

## Features

- Beautiful TUI with live-updating signal data
- Spectrum / channel view
- Connect to WPA2, WPA3, and open networks
- Signal history sparklines
- Animated transitions and scan indicators
- Configurable themes and keybindings

## Requirements

- Linux with NetworkManager
- A WiFi adapter

## Quick Start

```bash
cargo build --release
sudo ./target/release/wifiess
```

For UI development without NetworkManager:

```bash
cargo run -- --demo
```

## License

MIT
