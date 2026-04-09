# 📋 Clip Vault

> **A blazing-fast clipboard history manager built with Rust — silently records everything you copy, instantly recalled with a hotkey.**

![Rust](https://img.shields.io/badge/built%20with-Rust-orange?logo=rust)
![Platform](https://img.shields.io/badge/platform-Windows-blue?logo=windows)
![License](https://img.shields.io/badge/license-MIT-green)
![Version](https://img.shields.io/badge/version-0.1.0-brightgreen)

---

## ✨ Why Clip Vault?

The Windows clipboard only holds one item. Every time you copy something new, the previous content is gone forever. **Clip Vault** runs silently in the background and remembers everything — text, code snippets, URLs, images — so you can retrieve any of it instantly.

- ⚡ **Always ready** — `Ctrl+Shift+V` summons the window from anywhere, instantly
- 🔍 **Instant search** — find any past copy in milliseconds, supports Chinese
- 📌 **Pin important items** — pinned entries never get pushed out by new copies
- 🖼 **Image support** — screenshots and copied images are captured and previewed
- 💾 **Persistent history** — survives restarts, up to 500 entries saved to disk
- 📦 **Zero dependencies** — single `.exe`, no installation required

---

## 🚀 Features

### Recording
- **Auto-capture** — monitors clipboard every 200ms, records text and images automatically
- **Smart deduplication** — copying the same content twice only keeps one entry
- **⏸ Pause recording** — one click to temporarily stop capturing (for passwords, sensitive data)
- **500 entry limit** — oldest unpinned entries are automatically removed to keep things fast

### History
- **Time groups** — entries organized into 今天 / 昨天 / 更早 (Today / Yesterday / Earlier)
- **📌 Pin entries** — pinned items stay at the top and are never auto-deleted
- **Character & line count** — each text entry shows its size (e.g. `3 行 128 字`)
- **Image dimensions** — image entries show their pixel size (e.g. `1920×1080`)
- **Hover preview** — hover over any entry to see the full content (up to 2000 chars)

### Search
- **Real-time filtering** — type to instantly filter all entries
- **Chinese support** — full Unicode search, works with CJK text
- **Case-insensitive** — finds matches regardless of capitalization

### Interface
- **`Ctrl+Shift+V`** — toggle the window from anywhere, even inside other apps
- **`Esc`** — hide the window
- **Click to copy** — click any entry to copy it and hide the window
- **Right-click menu** — copy, pin/unpin, or delete any entry
- **Close button hides** — clicking ✕ hides to background, doesn't quit
- **Always on top** — window stays above other apps so you can paste immediately
- **Auto-focus search** — search box is ready to type the moment the window appears

### Data
- **Persistent storage** — history saved to `%LOCALAPPDATA%\clip-vault\history.json`
- **Debounced writes** — disk writes are batched (max once per 2 seconds) to avoid I/O thrash
- **Safe shutdown** — pending writes are flushed immediately on exit, no data loss
- **Images not persisted** — image bytes are kept in memory only (too large for JSON)

---

## 📸 Screenshot

```
┌─────────────────────────────────────┐
│ 🔍 [搜索历史…              ] ⏸ 🗑   │
├─────────────────────────────────────┤
│ 📌 已固定                            │
│ ┌─────────────────────────────────┐ │
│ │ 14:22  3 行 87 字          📌 ✕ │ │
│ │ const MAX_RESULTS: usize = 2000 │ │
│ └─────────────────────────────────┘ │
├─────────────────────────────────────┤
│ 今天                                 │
│ ┌─────────────────────────────────┐ │
│ │ 15:41  128 字              📌 ✕ │ │
│ │ https://github.com/...          │ │
│ └─────────────────────────────────┘ │
│ ┌─────────────────────────────────┐ │
│ │ 15:38  1920×1080           📌 ✕ │ │
│ │ [图片预览]                       │ │
│ └─────────────────────────────────┘ │
├─────────────────────────────────────┤
│ 42 条记录  •  Ctrl+Shift+V  •  Esc  │
└─────────────────────────────────────┘
```

---

## 📥 Download & Run

1. Go to [Releases](../../releases)
2. Download `clip-vault.exe`
3. Double-click to start — it runs silently in the background
4. Press `Ctrl+Shift+V` to open the history window

> ✅ No .NET, no Java, no Python, no Visual C++ Redistributable required.
> Works on Windows 10 and above.

---

## 🛠️ Build from Source

Requires [Rust](https://rustup.rs/) (stable toolchain).

```bash
git clone https://github.com/1716775457damn/clip-vault.git
cd clip-vault
cargo build --release
# Binary: target/release/clip-vault.exe
```

---

## 🏗️ Architecture

```
src/
├── main.rs      # Entry point, hotkey registration, CJK font loading
├── app.rs       # GUI (egui): search, render, actions, pause toggle
├── store.rs     # Data model, persistence, debounced disk writes
└── monitor.rs   # Background clipboard polling thread
```

| Component | Crate | Why |
|-----------|-------|-----|
| GUI framework | `egui` / `eframe` | Immediate-mode, native, 60fps, no Electron |
| Clipboard access | `arboard` | Cross-platform clipboard read/write |
| Global hotkey | `global-hotkey` | System-wide `Ctrl+Shift+V` registration |
| Serialization | `serde_json` | Persist history to JSON |
| Timestamps | `chrono` | Entry timestamps and time grouping |

---

## ⚡ Performance

- **200ms poll interval** — clipboard changes are captured within 200ms
- **FNV-1a hash** — image change detection without copying bytes
- **Debounced writes** — disk I/O batched to at most once per 2 seconds
- **Pre-computed fields** — `preview`, `stats`, and lowercase search index computed once at capture time, never recalculated during rendering
- **Hidden = idle** — when the window is hidden, the UI thread polls at 200ms instead of 60fps, near-zero CPU usage

---

## 🗺️ Roadmap

- [ ] System tray icon
- [ ] Configurable hotkey
- [ ] Export history to file
- [ ] Max history size setting
- [ ] Dark / light theme toggle

---

## 📄 License

MIT © 2025
