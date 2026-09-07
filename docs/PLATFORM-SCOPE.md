# Duxo Platform Scope

## Current focus

Duxo is currently developed and validated with macOS as the primary platform and Windows as the secondary platform.

- **macOS:** primary development, UI, packaging, and end-user validation target.
- **Windows:** retained as a secondary validation target because Duxo analyzes and adapts Windows software and needs Windows-side behavioral evidence.
- **Linux:** deferred for now.

## Reversible decision

Linux support is intentionally deferred rather than removed from the architecture. Re-enable Linux CI and packaging when cross-platform validation provides enough value to justify the additional build time and maintenance surface.

## Why Windows remains

Duxo's conversion pipeline ultimately needs to understand Windows executables, APIs, runtimes, graphics interfaces, installers, and filesystem/process behavior. A Windows CI target gives us a real Windows environment for regression tests and future conversion validation.

The Windows target is secondary: Mac-first implementation and testing takes priority unless a Windows result is required to validate a Windows-specific behavior.
