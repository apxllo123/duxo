<div align="center">

[<img src="https://raw.githubusercontent.com/apxllo123/duxo/main/resources/icon.png?v=12" width="144" alt="Duxo icon"/>](https://github.com/apxllo123/duxo)

#  ✦ Duxo ✦

<strong>Universal Game Analysis & Adaptation</strong>

<p>Duxo analyzes a game's files, executables, dependencies, runtime requirements, graphics stack, and protection signals before an adaptation plan is generated.</p>

[![⌘ Electron](https://img.shields.io/github/actions/workflow/status/apxllo123/duxo/%E2%8C%98.yml?label=%E2%8C%98%20Electron&style=for-the-badge&color=8b5cf6)](https://github.com/apxllo123/duxo/actions/workflows/%E2%8C%98.yml)
[![⚙ Rust](https://img.shields.io/github/actions/workflow/status/apxllo123/duxo/%E2%9A%99.yml?label=%E2%9A%99%20Rust&style=for-the-badge&color=64748b)](https://github.com/apxllo123/duxo/actions/workflows/%E2%9A%99.yml)
[![✦ Release](https://img.shields.io/github/actions/workflow/status/apxllo123/duxo/%E2%9C%A6.yml?label=%E2%9C%A6%20Release&style=for-the-badge&color=f59e0b)](https://github.com/apxllo123/duxo/actions/workflows/%E2%9C%A6.yml)
[![Release](https://img.shields.io/github/v/release/apxllo123/duxo?display_name=tag&style=for-the-badge&color=7c5cff)](https://github.com/apxllo123/duxo/releases)
[![License](https://img.shields.io/github/license/apxllo123/duxo?style=for-the-badge&color=2ea043)](LICENSE)

</div>

---

## Overview

Duxo is an experimental game analysis and adaptation platform built around a simple rule: **understand the game first, then decide what can be adapted**.

Instead of immediately attempting to transform a game, Duxo builds a structured model of the game directory and its Windows binaries. That model can then be consumed by the Adapter to produce an explicit per-game plan, including dependencies that still need to be resolved and capabilities that are not yet implemented.

The repository currently contains the foundation of that pipeline: recursive analysis, PE inspection, dependency modeling, protection detection, capability profiling, adaptation-plan generation, and a desktop shell that connects the pieces together.

## ✨ What Duxo Does Today

### 🔎 Analyzer

The Rust Analyzer builds a machine-readable manifest without modifying the source game files. It currently supports:

- Recursive game-directory inventory
- File metadata and SHA-256 hashing
- Executable candidate detection
- PE format and architecture detection
- PE headers and section inspection
- PE import inspection
- Binary dependency relationships
- Protection-signal detection, including packer/protector and anti-cheat indicators
- Per-game capability profiling

### 🧩 Adapter

The Rust Adapter consumes the Analyzer's profile and produces a structured adaptation plan.

Plans can describe:

- Graphics requirements and backend needs
- Windows API families
- Runtime requirements
- Dependency-resolution work
- Unresolved imports
- Protection information
- Capabilities that are currently blocked because their executor is not implemented yet

Duxo intentionally reports unfinished conversion work as **blocked** instead of presenting it as supported functionality.

### 🖥️ Desktop

The desktop application provides the user-facing layer for the analysis pipeline.

Current desktop functionality includes:

- Native game-folder selection
- Analyzer execution from the UI
- Adapter-plan execution from the UI
- Controlled Electron preload communication
- Native packaging for macOS ARM64 and Windows

## 🏗️ Architecture

```text
                                Duxo Desktop
                           Electron + TypeScript
                                     │
                              Controlled Bridge
                                     │
                           ┌─────────┴─────────┐
                           │                   │
                    Duxo Analyzer       Duxo Adapter
                         Rust                  Rust
                           │                   │
                           └─────────┬─────────┘
                                     │
                              Game directory
                                     │
                        ┌────────────┴────────────┐
                        │                         │
                     Manifest                GameProfile
                        │                         │
                        └────────────┬────────────┘
                                     │
                              Adaptation Plan
```

### Analyzer → Profile → Plan

```text
Game files
   ↓
Manifest
   ↓
PE / dependency / protection analysis
   ↓
GameProfile
   ↓
Adaptation planning
   ↓
Explicit Ready / Blocked steps
```

This separation keeps evidence gathering independent from conversion decisions and gives future conversion executors a stable input model.

## 🎯 Per-Game Capability Profiles

Duxo is designed to make game-specific requirements visible before adaptation begins.

A capability profile can capture information such as:

- Direct3D 9 / 10 / 11 / 12 requirements discovered from PE imports
- DXGI requirements
- Windows API families such as windowing/input and networking
- Microsoft Visual C++ runtime requirements
- Binary-to-binary dependencies
- Unresolved imports that still require implementation or resolution
- Protection and anti-cheat signals

The resulting profile becomes the source of truth for adaptation planning rather than relying on assumptions about what a game needs.

## 🛡️ Protection & Safety Signals

The Analyzer can identify evidence associated with packers/protectors and anti-cheat technologies.

These signals are **diagnostic**. Duxo does not treat detection as automatic permission to bypass or remove a protection mechanism, and a detected technology can cause an adaptation step to remain blocked until a legitimate implementation exists.

## 🎨 Branding & Assets

The canonical application artwork lives at:

```text
resources/icon.png
```

The original source artwork is retained as:

```text
resources/icon.jpeg
```

The PNG is the packaging source for generated macOS and Windows icon assets. The repository README references the canonical PNG directly so the project branding and packaged application icon use the same source artwork.

## 🚦 Project Status

### Working

- [x] Recursive game analysis
- [x] File inventory and SHA-256 hashing
- [x] Executable detection
- [x] PE architecture detection
- [x] PE headers, sections, and imports
- [x] Binary dependency graph generation
- [x] Protection-signal detection
- [x] Per-game capability profiles
- [x] Per-game adaptation plans
- [x] Explicit dependency-resolution states
- [x] Rust Analyzer library and CLI
- [x] Rust Adapter library and CLI
- [x] Electron desktop shell
- [x] Native game-folder selection
- [x] Analyzer/Adapter execution from the desktop app
- [x] macOS ARM64 packaging pipeline
- [x] Windows packaging pipeline
- [x] Rust CI and Electron CI
- [x] Automated release workflow

### In Progress / Planned

- [ ] Full adaptation executors
- [ ] Native D3D/graphics adaptation modules
- [ ] Win32 API implementation coverage
- [ ] Runtime resolution and installation coverage
- [ ] Source-to-target file transformation
- [ ] x86 → ARM64 recompilation backend
- [ ] Automatic converted-game validation and launch testing
- [ ] Broader graphics backend support
- [ ] Production signing and notarization
- [ ] Full online/API service layer

These items are intentionally separated from the implemented feature set so the README does not overstate the current capabilities of the project.

## 🧪 Development & Verification

### Requirements

- Rust stable toolchain
- Node.js 24+
- npm

### Rust

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo build --workspace --release
```

### Desktop

```bash
cd desktop
npm install
npm run typecheck
npm run compile
npm run build
```

### Run the Analyzer

```bash
cargo run -p duxo-analyzer -- /path/to/game
```

The command outputs the generated game manifest as JSON.

### Run the Adapter

```bash
printf '%s\n' '<GameProfile JSON>' | cargo run -p duxo-adapter
```

The Adapter reads a serialized `GameProfile` from standard input and writes the generated adaptation plan as JSON.

### Run the Desktop App

```bash
cd desktop
npm install
npm start
```

## 📦 Releases

Duxo uses short public release tags while keeping full semantic versions in the application metadata.

### Current release

**[v4.6](https://github.com/apxllo123/duxo/releases/tag/v4.6)** — current release tag

### Release history

Each release is represented by a Git tag and a GitHub Release, so a version points directly to the exact source snapshot used for that release.

```text
v0.1 → v0.2 → … → v0.9 → v1.0 → v1.1 → … → v4.6
```

The short tag is the public version shown in releases, while the application keeps a valid semantic version internally (`4.6.0`, for example).

### Release pipeline

The **✦ Release** workflow is the only workflow responsible for publishing a versioned application release. It:

1. Detects substantive product changes since the latest release.
2. Calculates the next public release tag.
3. Updates the desktop package and macOS bundle versions together.
4. Creates or reuses the matching Git tag.
5. Runs the tagged ** Build** workflow.
6. Verifies the produced macOS artifacts.
7. Creates or reuses the GitHub Release only after the Mac build succeeds.
8. Attaches the ZIP, DMG, and SHA-256 files to that release.

Documentation-only and workflow-only changes do not create a new product release.

## ⚙️ CI & Automation

Duxo separates verification from packaging while keeping both automated.

###  Build

`.github/workflows/.yml` handles the macOS application build and packaging pipeline.

### ⌘ Electron

`.github/workflows/⌘.yml` handles Electron and renderer verification across the supported desktop targets.

### ⚙ Rust

`.github/workflows/⚙.yml` handles Rust formatting, compilation, Clippy, tests, and release builds.

### ✦ Release

`.github/workflows/✦.yml` is the single release orchestrator. It prepares the version and tag, waits for the tagged ` Build` workflow to pass, then publishes and attaches the resulting release artifacts.

## 📁 Repository Layout

```text
Duxo/
├── Duxo Analyzer/           # Rust game analysis engine
│   └── src/
├── Duxo Adapter/            # Rust adaptation planning engine
│   └── src/
├── desktop/                 # Electron + TypeScript application
│   ├── src/
│   ├── renderer/
│   ├── scripts/
│   └── macos/
├── resources/               # Project artwork and shared assets
│   ├── icon.png
│   └── icon.jpeg
├── tests/                   # Integration and future compatibility tests
├── docs/                    # Architecture and platform documentation
└── .github/workflows/       # CI, build, and release automation
```

## 🧭 Engineering Approach

Duxo follows a verification-first development loop:

```text
Inspect
  ↓
Reproduce / Measure
  ↓
Implement
  ↓
Format + Typecheck + Compile
  ↓
Clippy + Tests
  ↓
Build
  ↓
Inspect the artifact
  ↓
Fix and refine
```

The core principles are:

- **Analysis before adaptation** — establish evidence before making conversion decisions.
- **Explicit capabilities** — represent what is known, unknown, ready, or blocked.
- **Root-cause fixes** — correct the implementation instead of suppressing diagnostics.
- **Reproducible builds** — packaging and versioning should be deterministic and automatable.
- **Honest status reporting** — unfinished conversion functionality stays clearly marked as unfinished.

## 🗺️ Roadmap

### Phase 1 — Understand

Create a complete, reliable description of the game's files, binaries, dependencies, and runtime assumptions.

### Phase 2 — Model

Turn analysis evidence into capability profiles and adaptation requirements.

### Phase 3 — Adapt

Implement the platform-specific executors required by the generated plan.

### Phase 4 — Convert

Produce target-platform application packages while preserving game content and behavior wherever technically possible.

### Phase 5 — Validate

Automatically launch and exercise converted titles, capture diagnostics, and compare expected versus observed behavior.

### Phase 6 — Improve

Use measured compatibility and failure data to expand coverage and reduce manual intervention.

## 🖥️ Platform Focus

```text
macOS / Apple Silicon   → primary
Windows                 → secondary
Linux                   → deferred
```

Linux is deferred at the product level for now; the core architecture remains structured so additional targets can be introduced later without redesigning the analysis model.

## 📄 License

Duxo is licensed under the [MIT License](LICENSE).
