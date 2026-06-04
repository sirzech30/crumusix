# CrumusiX

CrumusiX is a lightweight desktop audio player that combines Spotify streaming and YouTube playback into a single app. It is built using Tauri v2, vanilla HTML/CSS/JS, and Rust.

By using Rust for the heavy lifting (via `librespot` and `rodio`), the player runs natively on your machine without requiring a heavy Chromium wrapper (like Electron) or official Spotify client instances, keeping its memory footprint well under 100MB RAM.

---

## Features

### Dual Playback Options
* **Native Spotify Integration:** Powered by `librespot` for direct connection and stream decoding. It handles loudness normalization (-14 LUFS), gapless track preloading, and local media caching.
* **YouTube Playback:** Resolves and streams audio from YouTube, with a small optional corner video overlay.

### Media Key & OS Integration
* Uses `souvlaki` for native media key mapping (supports MPRIS on Linux, Now Playing on macOS, and SMTC on Windows).
* Displays live track metadata (title, artist, album art, duration) in system media control widgets and lockscreens.

### Cache & Database
* **Smart Cache:** Track metadata is stored locally in an embedded key-value database (`sled`) and album art is downsampled to compressed WebP files to load instantly.
* **Offline Lyrics Database:** Syncs and caches song lyrics locally in an SQLite database for offline reading.

### Performance Controls
* Includes toggle controls to disable graphics blurs (`backdrop-filter`) and visualizer animations to reduce CPU/GPU usage on lower-spec hardware.
* Tauri V8 engine parameters are tuned for aggressive garbage collection to minimize memory leaks.

---

## Getting Started

### Prerequisites
Before running or compiling the project, make sure you have installed:
* [Rust toolchain](https://www.rust-lang.org/tools/install) (cargo, rustc)
* [Node.js & npm](https://nodejs.org/)
* Platform-specific build utilities (e.g., `build-essential` and WebKitGTK developer packages on Linux). See the [Tauri prerequisites guide](https://v2.tauri.app/start/prerequisites/) for your operating system.

### Installation
Clone the repository and install the dependencies:
```bash
npm install
```

### Development
To run the app locally with hot reloading enabled:
```bash
npm run tauri dev
```

### Build
To compile a production-ready, stripped standalone executable:
```bash
npm run tauri build
```
