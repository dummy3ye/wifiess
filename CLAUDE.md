# CLAUDE.md

Guidelines and architecture instructions for Claude and AI coding agents working on `wifiess`.

> **Note**: For complete project guidelines, architectural specifications, tech stack details, coding standards, and roadmap, please see [AGENTS.md](AGENTS.md).

---

## Quick Reference

### Core Commands
```bash
make help          # Show all available commands
make run           # Run wifiess (logs go to file)
make demo          # Run wifiess in demo mode (simulated WiFi backend)
make check         # Run fmt-check, clippy (-D warnings), and test (mirrors CI)
make fmt           # Auto-format all code
make lint          # Run strict clippy checks
make test          # Run all unit and UI snapshot tests
make build-release # Build optimized release binary
```

### Essential Rules
1. **Source of Truth**: Always adhere to [AGENTS.md](AGENTS.md). When this file or any instructions differ, `AGENTS.md` wins.
2. **Pure Elm-style Architecture**:
   - `crossterm` events + backend signals -> `Action` -> `update(&mut State)` -> `view(&State)` -> `draw`.
   - `view()` is strictly pure (no I/O, no mutation).
   - Only `update()` mutates state.
3. **Strict Separation of Concerns**:
   - `nm/` knows nothing about Ratatui or terminal UI.
   - `ui/` knows nothing about `zbus` or D-Bus.
   - They communicate exclusively through typed commands, channels, and models defined in `nm/model.rs`.
4. **Safety & Zero Panic Outside Tests**:
   - `#![forbid(unsafe_code)]` is strictly enforced.
   - No `unwrap()` or `expect()` outside of tests.
   - Never leak passwords/secrets (PSKs must be zeroized using `secrecy`).
5. **Terminal Protection**:
   - stdout is owned by Ratatui. Never use `println!` or `eprintln!`. Use structured `tracing` which logs to file only.
   - Panic hooks and drop guards in `tui.rs` must always restore the terminal cleanly.
