# 📋 Clip Vault

> **A blazing-fast clipboard history manager built with Rust — silently records everything you copy, instantly recalled with a hotkey.**

![Rust](https://img.shields.io/badge/built%20with-Rust-orange?logo=rust)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-blue)
![License](https://img.shields.io/badge/license-MIT-green)
![Version](https://img.shields.io/badge/version-0.1.0-brightgreen)

---

## ✨ Why Clip Vault?

The system clipboard only holds one item. Every time you copy something new, the previous content is gone forever. **Clip Vault** runs silently in the background and remembers everything — text, code snippets, URLs, images — so you can retrieve any of it instantly.

- ⚡ **Always ready** — `Ctrl+Shift+V` summons the window from anywhere, instantly
- 🔍 **Instant search** — find any past copy in milliseconds, supports Chinese
- 📌 **Pin important items** — pinned entries never get pushed out by new copies
- 🖼 **Image support** — screenshots and copied images are captured and previewed
- 💾 **Persistent history** — survives restarts, up to 500 entries saved to disk
- 📦 **Zero dependencies** — single binary, no installation required

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
- **Sequence numbers** — entries 1–9 show their number for keyboard shortcuts

### Search
- **Real-time filtering** — type to instantly filter all entries
- **✕ Clear button** — one click to clear the search box
- **Chinese support** — full Unicode search, works with CJK text
- **Case-insensitive** — finds matches regardless of capitalization

### Keyboard Shortcuts
- **`Ctrl+Shift+V`** — toggle the window from anywhere, even inside other apps
- **`Esc`** — minimize the window
- **`1`–`9`** — instantly copy the nth visible entry and close the window
- **`↑` / `↓`** — navigate entries with arrow keys
- **`Enter`** — copy the selected entry and close the window

### Interface
- **Click to copy** — click any entry to copy it (window stays open)
- **Right-click menu** — copy, pin/unpin, or delete any entry
- **System tray icon** (Windows) — right-click for Show / Quit
- **Close button minimizes** — clicking ✕ minimizes to taskbar, program keeps running
- **Always on top** — window stays above other apps so you can paste immediately
- **Auto-focus search** — search box is ready to type the moment the window appears

### Data
- **Persistent storage**
  - Windows: `%LOCALAPPDATA%\clip-vault\history.json`
  - macOS: `~/Library/Application Support/clip-vault/history.json`
  - Linux: `~/.local/share/clip-vault/history.json`
- **Debounced writes** — disk writes are batched (max once per 2 seconds) to avoid I/O thrash
- **Safe shutdown** — pending writes are flushed immediately on exit, no data loss
- **Images not persisted** — image bytes are kept in memory only (too large for JSON)

---

## 📸 Screenshot

```
┌─────────────────────────────────────┐
│ 🔍 [搜索历史…         ] ✕  ⏸ 🗑    │
├─────────────────────────────────────┤
│ 📌 已固定                            │
│ ┌─────────────────────────────────┐ │
│ │ 1  14:22  3 行 87 字       📌 ✕ │ │
│ │ const MAX_RESULTS: usize = 2000 │ │
│ └─────────────────────────────────┘ │
├─────────────────────────────────────┤
│ 今天                                 │
│ ┌─────────────────────────────────┐ │
│ │ 2  15:41  128 字           📌 ✕ │ │
│ │ https://github.com/...          │ │
│ └─────────────────────────────────┘ │
│ ┌─────────────────────────────────┐ │
│ │ 3  15:38  1920×1080        📌 ✕ │ │
│ │ [图片预览]                       │ │
│ └─────────────────────────────────┘ │
├─────────────────────────────────────┤
│ 42 条记录  •  Ctrl+Shift+V  •  Esc  │
└─────────────────────────────────────┘
```

---

## 📥 Download & Run

### Windows
1. Go to [Releases](../../releases)
2. Download `clip-vault.exe`
3. Double-click to start — it runs silently in the background
4. Press `Ctrl+Shift+V` to open the history window

> ✅ No .NET, no Java, no Python, no Visual C++ Redistributable required.  
> Works on Windows 10 and above.

### macOS

macOS does not allow running unsigned binaries by default. Build from source:

```bash
# 1. Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# 2. Clone and build
git clone https://github.com/1716775457damn/clip-vault.git
cd clip-vault
cargo build --release

# 3. Run
./target/release/clip-vault
```

**Optional: package as a `.app` bundle**

```bash
cargo install cargo-bundle
cargo bundle --release
open "target/release/bundle/osx/Clip Vault.app"
```

> ℹ️ On first launch macOS may show a security warning.  
> Go to **System Settings → Privacy & Security** and click **Open Anyway**.

> ⚠️ **Accessibility permission required for the global hotkey (`Ctrl+Shift+V`)**  
> 1. Open **System Settings → Privacy & Security → Accessibility**  
> 2. Click the **+** button and add `clip-vault` (or `Clip Vault.app`)  
> 3. Make sure the toggle is **enabled**  
> Without this permission the hotkey will not work, but all other features function normally.

> 📝 **System tray icon** is not available on macOS in this version.  
> Use the hotkey or click the Dock icon to show the window.

> 🌏 CJK (Chinese/Japanese/Korean) fonts are embedded in the binary — no system font installation needed.

### Linux

```bash
git clone https://github.com/1716775457damn/clip-vault.git
cd clip-vault
cargo build --release
./target/release/clip-vault
```

---

## 🛠️ Build from Source

Requires [Rust](https://rustup.rs/) (stable toolchain).

```bash
git clone https://github.com/1716775457damn/clip-vault.git
cd clip-vault
cargo build --release
# Windows: target/release/clip-vault.exe
# macOS/Linux: target/release/clip-vault
```

---

## 🏗️ Architecture

```
src/
├── main.rs      # Entry point, hotkey registration, system tray, CJK font
├── app.rs       # GUI (egui): search, render, keyboard nav, pause toggle
├── store.rs     # Data model, persistence, debounced disk writes
└── monitor.rs   # Background clipboard polling thread
```

| Component | Crate | Why |
|-----------|-------|-----|
| GUI framework | `egui` / `eframe` | Immediate-mode, native, 60fps, no Electron |
| Clipboard access | `arboard` | Cross-platform clipboard read/write |
| Global hotkey | `global-hotkey` | System-wide `Ctrl+Shift+V` registration |
| System tray | `tray-icon` | Windows/Linux tray icon with menu |
| Serialization | `serde_json` | Persist history to JSON |
| Timestamps | `chrono` | Entry timestamps and time grouping |

---

## ⚡ Performance

- **200ms poll interval** — clipboard changes are captured within 200ms
- **FNV-1a hash** — image change detection without copying bytes
- **Debounced writes** — disk I/O batched to at most once per 2 seconds
- **Pre-computed fields** — `preview`, `stats`, char count, and lowercase search index computed once at capture time
- **O(1) deduplication** — `HashSet` lookup instead of linear scan
- **Virtual scroll** — log renders only visible rows regardless of entry count
- **Minimized = idle** — when minimized, UI polls at 200ms instead of 60fps, near-zero CPU usage

---

## 🗺️ Roadmap

- [x] System tray icon (Windows)
- [ ] System tray icon (macOS)
- [ ] Configurable hotkey
- [ ] Export history to file
- [ ] Max history size setting
- [ ] Dark / light theme toggle

---

## 📄 License

MIT © 2025
