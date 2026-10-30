# `auto-editor-rs` 🦀🎬

[![Rust](https://img.shields.io/badge/Rust-1.80+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/License-Unlicense%20OR%20MIT-blue.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/Tests-25%20Passing-brightgreen.svg)]()

A high-performance, native pure **Rust** port of [WyattBlue/auto-editor](https://github.com/wyattblue/auto-editor) — effort-free automatic video and audio editing powered by acoustic volume analysis, inter-frame motion tracking, multi-class moment labeling, and seamless NLE timeline interchange (Final Cut Pro, Premiere Pro, DaVinci Resolve, Shotcut, Kdenlive).

---

## ⚡ Highlights & Architecture

- **🚀 Zero-Cost Performance**: Native compiled Rust binary with sub-millisecond timeline chunking and zero garbage collection overhead.
- **🔊 Acoustic Loudness & Decibels**: Full support for linear amplitude (`0.04`), decibels (`-19dB`), percentage thresholds (`4%`), and RMS/Peak calculations.
- **👁️ Motion Detection**: Inter-frame pixel variance and motion thresholding (`motion:0.02`).
- **🧠 Expression Grammar**: Recursive descent boolean expression parser evaluating complex conditions across audio and video streams:
  ```bash
  auto-editor example.mp4 --edit "(or audio:-19dB motion:0.06)"
  ```
- **✂️ Smart Margins & Bridging**: Customizable pre/post speech padding (`--margin 0.2s,1.5s`) and automatic short-silence gap bridging.
- **🏷️ Priority Labeling Engine**: Multi-class moment labels (0 = cut/inactive, 1 = keep/active, 2..255 = speed modifiers).
- **📦 NLE Project Interchange**:
  - **Final Cut Pro XML** (`.fcpxml` v1.9 / v1.10)
  - **Adobe Premiere Pro XML** (`<xmeml version="4">`)
  - **DaVinci Resolve XML** (`<xmeml version="5">`)
  - **Shotcut MLT XML** (`.mlt` sequences)
  - **Kdenlive Project XML** (`.kdenlive` multi-track MLT)
  - **CMX 3600 EDL** & **CSV** Cut Lists
  - **Structured JSON** Timeline

---

## 🚀 Installation & Build

```bash
git clone https://github.com/bhubbard/auto-editor-rs.git
cd auto-editor-rs
cargo build --release
```

Binary will be available at `./target/release/auto-editor`.

---

## 💻 CLI Usage

### 1. Basic Editing with Audio Threshold
```bash
# Cut out silence below 4% volume (default)
auto-editor example.mp4

# Set volume threshold in decibels with custom margin padding
auto-editor example.mp4 --edit audio:-19dB --margin 0.3s,1.0s
```

### 2. Multi-Method Boolean Cuts
```bash
# Keep moments with audio above -19dB OR motion above 6%
auto-editor example.mp4 --edit "(or audio:-19dB motion:0.06)"
```

### 3. Export to Professional Video Editors
```bash
# Export to Final Cut Pro (FCPXML)
auto-editor example.mp4 --export final-cut-pro

# Export to Adobe Premiere Pro
auto-editor example.mp4 --premiere

# Export to DaVinci Resolve
auto-editor example.mp4 --resolve

# Export to Shotcut
auto-editor example.mp4 --shotcut

# Export to Kdenlive
auto-editor example.mp4 --kdenlive
```

### 4. Speed Up Inactive Sections (Timelapse / Lecture Mode)
```bash
# Speed up silent sections by 4x instead of cutting completely
auto-editor lecture.mp4 --silent-speed 4.0
```

### 5. Preview & Statistics
```bash
# Preview cuts and calculate time savings without rendering
auto-editor podcast.mp4 --preview

# Output machine-readable JSON statistics
auto-editor podcast.mp4 --stats
```

---

## 🧪 Upstream Parity & Test Suite

Includes 25 automated integration and unit tests covering:
- Decibel, linear, and percentage audio threshold conversions
- Frame difference motion metric calculations
- Boolean expression AST lexing, parsing, and multi-stream evaluation
- Margin expansion and short-gap silence bridging
- Multi-label priority arbitration and custom speed actions
- XML roundtripping for FCPXML, Premiere, Resolve, Shotcut, Kdenlive, EDL, and CSV

Run test suite:
```bash
cargo test
```

---

## 📜 License

Licensed under either of:
- [The Unlicense](LICENSE) (matching upstream public domain)
- [MIT License](LICENSE)

at your option. Based on the concept and project by [Wyatt Blue](https://github.com/wyattblue/auto-editor).
