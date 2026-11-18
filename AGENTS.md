# AGENTS.md

Instructions for AI coding agents (and humans) working on this repository.
Read this whole file before making changes. When this file and your instincts disagree, this file wins.

---

## 1. Project summary

A WiFi manager for the terminal, written in **pure Rust**, rendered with **Ratatui**, talking to **NetworkManager over D-Bus via `zbus`**.

Goals, in priority order:

1. **The UI is remarkable.** It should look and feel better than any other WiFi TUI. This is the product's identity, not a finishing touch.
2. **It is correct and never blocks.** Rendering never waits on D-Bus. The terminal is always restored.
3. **It is small and dependency-light.** One `cargo build`, one binary, no C toolchain.

### Scope (what this project does)

- List WiFi access points with live-updating signal data.
- Scan on demand.
- Connect to open, WPA2-PSK and WPA3-SAE networks; connect to saved networks.
- Disconnect, forget saved networks, connect to hidden SSIDs.
- Show details and signal history for the selected network.
- Show a channel/spectrum view built from the data NetworkManager already gives us.
- Configuration file (theme, keybinds, scan interval).

### Non-goals (do NOT build these)

- No C/C++, no FFI, no `unsafe` (see section 6).
- No `iwd`, `wpa_supplicant`, or raw `nl80211` backends. NetworkManager only.
- No Ethernet, VPN, bluetooth, hotspot/AP-mode, or general network management.
- No WPA-Enterprise (802.1X) in v1. Detect it and show it as "unsupported", do not half-implement it.
- No packet capture, monitor mode, or security-testing features.
- No GUI, web UI, or daemon mode.
- No telemetry or network calls other than D-Bus to the local system bus.

If a task seems to need something in this list, stop and ask instead of expanding scope.

---

## 2. Tech stack

| Layer | Crate | Notes |
|---|---|---|
| Runtime | `tokio` | Single runtime for everything |
| TUI | `ratatui`, `crossterm` | crossterm `event-stream` feature enabled |
| Streams | `futures`, `tokio-stream` | `StreamExt` for key events and D-Bus signals |
| D-Bus | `zbus` | `tokio` feature, so it shares our runtime |
| Errors | `thiserror` (library-ish modules), `color-eyre` (top level) | |
| Logging | `tracing`, `tracing-appender` | **File only.** stdout belongs to the TUI |
| Config | `serde`, `toml`, `directories` | |
| Text input | `tui-input` or `tui-textarea` | Password and hidden-SSID prompts |
| Spinners | `throbber-widgets-tui` (optional) | Custom widgets are fine if prettier |
| Secrets | `secrecy`, `zeroize` | PSKs never linger in memory or logs |
| Testing | `insta`, ratatui `TestBackend` | Snapshot-test every screen |

Rules:

- Pin versions in `Cargo.toml`. Ratatui's API changes between minor versions, so **check the docs for the pinned version** rather than relying on memory.
- Adding a dependency requires a one-line justification in the PR/commit message. Prefer writing 30 lines over adding a crate.
- Set `rust-version` (MSRV) in `Cargo.toml` and keep it honest.

---

## 3. Commands

```bash
make help                          # list all helper commands
make run                           # run the app (logs go to file)
make demo                          # run with fake backend for UI testing
make check                         # fmt-check + clippy + test
make fmt                           # format code
make lint                          # run clippy with -D warnings
make test                          # unit + snapshot tests
make build-release                 # release build
cargo insta review                 # review changed snapshots
```

Before finishing any task, `fmt`, `clippy -D warnings`, and `test` must all pass.

Debugging tip: run with `RUST_LOG=debug` and `tail -f` the log file in a second terminal. Never `println!` or `eprintln!`; it corrupts the screen.

A fake backend (`--demo` flag, see Phase 8) lets UI work proceed without a WiFi card or NetworkManager. Prefer it for UI development.

---

## 4. Architecture

Unidirectional, Elm-style. State changes in exactly one place.

```
crossterm events ─┐
tick timer ───────┼─► Event ─► Action ─► update(&mut State) ─► view(&State) ─► draw
NM signals ───────┘                │
                                   └─► Command ─► NM service task ─► D-Bus
```

- **UI task** owns `State`. It runs a `tokio::select!` over key events, a tick interval, and backend events.
- **NM service task** owns the `zbus::Connection`. It receives `Command`s and emits `NmEvent`s over `mpsc` channels.
- **The UI never calls D-Bus directly.** If a function in `ui/` or `app.rs` touches `zbus`, that is a bug.
- **`view()` is pure.** It reads `&State` and draws. It does not mutate state, spawn tasks, or do I/O.
- **`update()` is the only mutator.** It takes an `Action` or `NmEvent` and changes `State`. It is synchronous and fast.
- **Redraw only when dirty.** Set a dirty flag in `update()`. Run a fast tick (about 30 fps) only while an animation is active; otherwise idle at a slow tick (about 1 Hz) so the app costs almost no CPU.

### Message types

```rust
// nm/mod.rs
pub enum Command {
    Scan,
    Connect { ap: ApId, psk: Option<SecretString> },
    ConnectHidden { ssid: Vec<u8>, psk: Option<SecretString> },
    Disconnect,
    Forget(ConnId),
}

pub enum NmEvent {
    AccessPoints(Vec<AccessPoint>), // full snapshot; simplest to reduce
    State(ConnState),
    ScanStarted,
    ScanFinished,
    ConnectResult(Result<(), NmError>),
    Fatal(NmError),                 // backend cannot continue
}
```

### Core model

```rust
pub struct AccessPoint {
    pub id: ApId,            // D-Bus object path
    pub ssid: Vec<u8>,       // NOT guaranteed UTF-8; lossy-convert for display only
    pub bssids: Vec<[u8; 6]>,
    pub strength: u8,        // 0-100 from NM
    pub freq_mhz: u32,
    pub security: Security,  // Open / Wpa2 / Wpa3 / Enterprise / Unsupported
    pub known: bool,
    pub active: bool,
}
```

---

## 5. Project structure

```
src/
├── main.rs              # runtime bootstrap, wiring, CLI flags
├── tui.rs               # terminal init/restore, panic hook, capability detection
├── event.rs             # Event enum + event loop (keys, ticks, backend)
├── action.rs            # Action enum (UI intents)
├── app.rs               # State + update() reducer
├── config.rs            # config + theme loading
├── nm/
│   ├── mod.rs           # public API: Command, NmEvent, spawn()
│   ├── proxies.rs       # zbus #[proxy] traits
│   ├── service.rs       # backend task loop
│   ├── model.rs         # AccessPoint, Device, ConnState (plain Rust types)
│   ├── settings.rs      # builds connection dicts (a{sa{sv}})
│   ├── error.rs         # NmError (typed, user-presentable)
│   └── demo.rs          # fake backend for --demo and tests
└── ui/
    ├── mod.rs           # top-level layout + view()
    ├── theme.rs         # palettes, color ramps, color-depth fallback
    ├── anim.rs          # easing, tweens, spinner/radar frames
    ├── nwlist.rs        # access point list view
    ├── detail.rs        # selected network details
    ├── schart.rs        # per-network history sparkline
    ├── spectrum.rs      # channel/band view
    ├── password.rs      # input popup
    ├── confirm.rs       # modal confirmations (e.g. forget network)
    ├── toast.rs         # transient notifications
    ├── bar.rs           # bottom status bar
    └── help.rs          # keyboard shortcuts overlay
tests/                   # snapshot tests per screen/size
```

Layer rule: `nm/` knows nothing about Ratatui. `ui/` knows nothing about `zbus`. They meet only through the types in `nm/model.rs` and the two channels.

---

## 6. Coding rules

- **No `unsafe`.** Add `#![forbid(unsafe_code)]` in `main.rs`.
- **No `unwrap()` / `expect()` / indexing that can panic** outside tests. Use `?`, `get()`, and typed errors. The one allowed exception is a documented invariant with a comment.
- **No blocking calls on the UI task.** No `std::thread::sleep`, no sync I/O, no `block_on`.
- **Terminal safety:** `tui.rs` installs a panic hook and a drop guard that always leave the alternate screen, disable raw mode, disable mouse capture and show the cursor. This is the first thing built and must never regress.
- **Secrets:** PSKs are `SecretString`, are zeroized after use, are never logged, never `Debug`-printed, never written to config. Use `secrecy`'s redaction.
- **SSIDs are bytes.** Store as `Vec<u8>`. Display with lossy conversion. Strip or escape control characters before rendering, since an SSID can contain escape sequences that corrupt the terminal.
- **Errors reach the user in plain language.** `NmError` variants map to a short human message ("Wrong password", "Permission denied by polkit", "Scan rate-limited, try again in a few seconds"). Never show raw D-Bus error strings as the only output.
- **Selection is pinned to identity, not index.** The list reorders as signals fluctuate; the cursor must follow the SSID.
- **Dedupe by SSID.** Multiple APs for one network collapse into one row (strongest signal shown, BSSID count available in detail).
- **Layout and sizing:** never assume a terminal size. Every screen must render without panic at 1x1 up to very large, and degrade gracefully.
- Keep functions small and modules cohesive. Doc comments on every public item.
- Commit style: small commits, imperative subject line, one logical change each.

---

## 7. UI specification

The UI is the main deliverable. "Works" is not enough. Every screen must look intentional.

### 7.1 Design principles

1. **Calm by default, alive where it matters.** Static elements stay still; only live data (signal, scan, connect) moves.
2. **Hierarchy through color and weight, not clutter.** One accent color, restrained neutrals, semantic colors for signal/status.
3. **Information density with breathing room.** Consistent padding, aligned columns, no cramped borders.
4. **Everything is reachable by keyboard; mouse is a bonus.**
5. **Graceful degradation.** Looks great in truecolor with Unicode; still looks clean in 256-color, 16-color, `NO_COLOR`, and ASCII-only terminals.

### 7.2 Layout

Wide terminals (about 100+ columns): three regions.

```
╭─ Networks ──────────────────────╮ ╭─ Home-5G ───────────────────────╮
│ ● Home-5G        ▂▄▆█  WPA3  5G │ │  Signal   ▁▂▃▅▆▇█▇▆▅  -48 dBm    │
│   CoffeeShop     ▂▄▆_  WPA2  2G │ │  Band     5 GHz  ch 44  80 MHz  │
│   Neighbor_2G    ▂▄__  WPA2  2G │ │  Security WPA3-SAE              │
│   Guest          ▂___  Open  2G │ │  BSSIDs   2                     │
│                                 │ │  ─────────────────────────────  │
│                                 │ │  history  ▁▂▂▃▅▅▆▆▇▇▆▆▅▅▆▇      │
╰─────────────────────────────────╯ ╰─────────────────────────────────╯
╭─ Spectrum ───────────────────────────────────────────────────────────╮
│  channel/band view with APs plotted by frequency and strength         │
╰──────────────────────────────────────────────────────────────────────╯
 ● Connected to Home-5G   ·  ⏎ connect  s scan  d disconnect  ? help
```

- **Wide:** list + detail side by side, spectrum below.
- **Medium (about 60-99 cols):** list on top, detail below; spectrum behind a toggle key.
- **Narrow (under 60 cols):** list only; Enter opens detail as a full-screen view.
- **Tiny:** a single-line "terminal too small" message, never a panic.

Breakpoints live in one place (`ui/mod.rs`) and are covered by snapshot tests.

### 7.3 Visual language

- **Rounded borders** (`╭╮╰╯`), with a plain-border fallback when Unicode is limited.
- **Palette system:** `theme.rs` defines named themes with semantic slots (`bg`, `surface`, `border`, `text`, `muted`, `accent`, `good`, `warn`, `bad`, `selection`). Ship at least four themes: a dark default, a light theme, and two distinctive ones (for example a muted pastel and a high-contrast).
- **Signal color ramp:** signal strength maps smoothly through a gradient (poor to excellent), interpolated in truecolor, snapped to a 6-step palette in 256-color, and to three colors in 16-color.
- **Signal glyphs:** the bar glyph (`▂▄▆█`) is colored by the ramp; dBm value is shown alongside, with the NM `0-100` strength converted only for display and labeled clearly if it is an estimate.
- **Selection:** a full-row highlight with a subtle accent bar on the left edge, not a harsh inverted block.
- **Badges:** small consistent tags for security (`WPA3`, `WPA2`, `Open`, `Enterprise`), band (`2.4G`, `5G`, `6G`), and state (`known`, `active`). Open networks get a warning tint.
- **Typography:** use bold/dim/italic sparingly and consistently; muted text for secondary info.

### 7.4 Signature features (the "remarkable" part)

Each of these is a required deliverable, not an optional idea:

1. **Live signal history per network.** A ring buffer of recent samples per SSID, drawn as a smooth sparkline or braille line chart in the detail pane. Updates visibly as signals change.
2. **Spectrum / channel view.** Using each AP's frequency and strength (already provided by NM), plot networks as overlapping bell curves across the 2.4 GHz band (showing channel overlap, with channels 1/6/11 marked) and across 5 GHz and 6 GHz when present. The user should instantly see which channel is crowded. A "least crowded channel" hint is a nice touch.
3. **Animated scan indicator.** While scanning, show a tasteful animation (radar sweep, pulsing rings, or a flowing spinner) in the header or list. Results fade/slide into the list rather than popping in.
4. **Smooth transitions.** Signal bars animate to new values (short easing) instead of jumping. Rows entering or leaving the list briefly highlight or dim. Popups ease in.
5. **Connection flow feedback.** Connecting shows stages (`Authenticating`, `Obtaining IP`, `Connected`) with progress feedback; success and failure use toasts and a clear status-bar change. Wrong passwords produce a clear message and reopen the prompt.
6. **Password popup done well.** Centered modal, masked input with a reveal toggle, security info shown, paste supported, clear focus styling, `Esc` cancels cleanly, input zeroized on close.
7. **Toasts.** Non-blocking, auto-dismissing notifications stacked in a corner, colored by severity.
8. **Status bar.** Always shows connection state, current SSID, IP if available, and the keys relevant to the *current* context.
9. **Help overlay.** `?` opens a clean, grouped keybinding reference generated from the same keymap the app actually uses, so it cannot go stale.
10. **Empty and error states are designed.** "No WiFi device found", "NetworkManager not running", "WiFi is turned off", and "Permission denied" each get a clear, friendly, centered screen with the next step, not a stack trace.

### 7.5 Interaction

- Vim keys (`j/k`, `g/G`) and arrows both work; mouse click and scroll optional.
- Incremental filter: `/` opens a search that filters the list live.
- Sort toggle: by signal (default), by name, known-first.
- Keybinds are configurable; defaults are listed in the help overlay.
- No destructive action (forget network) without a confirmation prompt.
- Responsive to terminal resize with no flicker or layout glitches.

### 7.6 Performance and rendering

- Target smooth animation at 30 fps while animating, near-zero CPU while idle.
- Avoid allocating large strings every frame; precompute what can be cached.
- Never render a frame whose `State` has not changed and has no running animation.
- Detect color depth (truecolor via `COLORTERM`, 256-color, 16-color, `NO_COLOR`) once at startup in `tui.rs`, and have `theme.rs` adapt.

### 7.7 UI quality bar (acceptance for any UI change)

- Snapshot tests at three sizes (for example 120x40, 80x24, 50x15) plus a too-small size.
- Looks right in all four color modes.
- No visible flicker, no layout jumps when data refreshes.
- Text never overflows its container; long SSIDs truncate with an ellipsis.
- Reviewed in a real terminal, not only in tests.

---

## 8. NetworkManager reference

Service: `org.freedesktop.NetworkManager` on the **system bus**.

| Interface | Path | Use |
|---|---|---|
| `.NetworkManager` | `/org/freedesktop/NetworkManager` | `GetDevices`, `ActivateConnection`, `AddAndActivateConnection`, `DeactivateConnection`, `State`, `WirelessEnabled` |
| `.Device` | per-device | filter `DeviceType == 2` (WiFi); `ActiveConnection`, `State` |
| `.Device.Wireless` | per-device | `RequestScan`, `GetAllAccessPoints`, `AccessPointAdded` / `AccessPointRemoved`, `LastScan`, `ActiveAccessPoint` |
| `.AccessPoint` | per-AP | `Ssid` (`ay`), `Strength`, `Frequency`, `Flags`, `WpaFlags`, `RsnFlags`, `HwAddress` |
| `.Settings` | `/org/freedesktop/NetworkManager/Settings` | `ListConnections`, `AddConnection` |
| `.Settings.Connection` | per-connection | `GetSettings`, `Delete` |
| `.Connection.Active` | per-active-conn | connection `State`, IP config paths |

Logic:

- **Known network:** find the saved connection whose SSID matches, then `ActivateConnection`.
- **New secured network:** `AddAndActivateConnection` with a settings dict containing `connection`, `802-11-wireless`, and `802-11-wireless-security` (`key-mgmt` = `wpa-psk` or `sae`, plus `psk`).
- **Open network:** same call, without the security section.
- **Hidden network:** set `hidden = true` in `802-11-wireless` and supply the SSID bytes.
- **Live updates:** subscribe to `AccessPointAdded`/`Removed` and `PropertiesChanged` rather than polling. Re-snapshot and send `NmEvent::AccessPoints`.
- **Security classification:** derive `Security` from `Flags`/`WpaFlags`/`RsnFlags` in one tested function. Unknown combinations map to `Unsupported`, never to `Open`.

Known hazards:

- `RequestScan` is rate-limited by NM and can return an error. Treat it as normal, not fatal.
- Polkit can deny actions (common over SSH or in odd sessions). Surface it clearly.
- Devices and APs appear and disappear; handle stale object paths without panicking.
- The WiFi device may not exist, WiFi may be rfkill-disabled, or NM may not be running. Each is a designed UI state (see 7.4 item 10).

---

## 9. Roadmap

Work phase by phase. **Do not start a phase until the previous one meets its acceptance criteria.** Check boxes as work completes.

### Phase 0: Project skeleton
- [ ] `cargo new`, set edition, `rust-version`, `#![forbid(unsafe_code)]`
- [ ] Dependencies added and pinned (justify each)
- [ ] `rustfmt.toml`, clippy config, CI workflow running fmt, clippy, test
- [ ] Logging to a file via `tracing-appender`; `--help`/`--version` CLI flags
- [ ] README stub and LICENSE

**Accept:** `cargo run` starts and exits cleanly; CI is green.

### Phase 1: Terminal + event loop
- [ ] `tui.rs`: raw mode, alternate screen, panic hook, drop guard
- [ ] Color-depth and Unicode capability detection
- [ ] `event.rs`: key events, resize, tick, backend channel merged in `tokio::select!`
- [ ] `app.rs` with `State`, `Action`, and `update()`; dirty-flag redraw
- [ ] Empty layout renders; `q` / `Ctrl-C` quits cleanly

**Accept:** forced panic leaves the shell usable; resize never crashes; idle CPU is near zero.

### Phase 2: UI foundation (against the demo backend)
- [ ] `nm/demo.rs`: fake backend producing fluctuating APs (varied bands, security, duplicates, hidden, odd SSIDs)
- [ ] `theme.rs`: semantic palette slots, 4 themes, color-depth fallback, signal ramp
- [ ] Responsive layout with the three breakpoints and the too-small state
- [ ] Network list with badges, signal glyphs, dedupe, pinned selection, sorting, filter
- [ ] Detail pane (static data)
- [ ] Status bar and context-aware key hints
- [ ] Snapshot tests for each layout size and each theme

**Accept:** `cargo run -- --demo` already looks polished; all snapshots reviewed.

### Phase 3: NM read-only backend
- [ ] `proxies.rs`: `zbus` proxies for the interfaces in section 8
- [ ] Locate the WiFi device; handle none / rfkill / NM-not-running
- [ ] `GetAllAccessPoints` to `AccessPoint` model; security classification with unit tests
- [ ] Marks `known` (saved connections) and `active`
- [ ] Service task and channel wiring to the UI
- [ ] Designed error/empty states for each failure

**Accept:** real networks appear in the UI; each failure mode shows its designed screen.

### Phase 4: Live updates and scanning
- [ ] Subscribe to `AccessPointAdded` / `Removed` / `PropertiesChanged`
- [ ] `RequestScan` on keybind, with rate-limit handling
- [ ] Animated scan indicator and entering/leaving row transitions
- [ ] Animated signal bar easing
- [ ] Debounce rapid signal changes so rows do not flicker

**Accept:** list updates without user input; scanning looks great; no cursor jumping.

### Phase 5: Connecting
- [ ] Connect to known networks via `ActivateConnection`
- [ ] Password popup (masked, reveal toggle, paste, zeroize on close)
- [ ] `AddAndActivateConnection` for WPA2-PSK, WPA3-SAE, and open networks
- [ ] Connection stage tracking (authenticating, obtaining IP, connected) shown in UI
- [ ] Toasts for success/failure; wrong password reopens the prompt
- [ ] Disconnect
- [ ] Polkit-denied and timeout errors presented clearly

**Accept:** can connect, fail, retry, and disconnect on real hardware without leaving stale UI state.

### Phase 6: Signature visuals
- [ ] Per-network signal history ring buffer + sparkline/braille chart
- [ ] Spectrum view: 2.4 GHz bell curves with channel overlap and 1/6/11 markers
- [ ] 5 GHz and 6 GHz views when those APs exist
- [ ] "Least crowded channel" hint
- [ ] Popup ease-in, toast system, polished selection styling
- [ ] Performance pass: frame cost measured, allocation hot spots removed

**Accept:** the demo and the real app both show all signature features at 30 fps while animating.

### Phase 7: Management features
- [ ] Forget saved network (with confirmation)
- [ ] Hidden SSID connect flow
- [ ] Detail view extras: BSSID list, IP info for the active connection
- [ ] Filter/sort polish; remembered sort choice in config
- [ ] Enterprise networks detected and labeled unsupported
- [ ] WiFi on/off toggle (if NM exposes it to the user via polkit; otherwise show a clear message)

**Accept:** every scope item in section 1 is reachable from the UI.

### Phase 8: Config, help, and polish
- [ ] `config.rs`: TOML file in the XDG config dir; theme selection, keybinds, scan interval, Unicode/color overrides
- [ ] Config errors are reported clearly and fall back to defaults
- [ ] Help overlay generated from the live keymap
- [ ] `--demo` flag stable and documented
- [ ] Mouse support (click select, scroll)
- [ ] `NO_COLOR` and ASCII-only modes verified
- [ ] Accessibility pass: contrast in every theme, no color-only meaning

**Accept:** a new user can discover every feature from `?` alone.

### Phase 9: Hardening
- [ ] Soak test: run for hours with the demo backend; no memory growth
- [ ] Handle NM restarting while the app runs (reconnect to the bus)
- [ ] Handle device removal/addition (USB WiFi adapters)
- [ ] Fuzz or property-test SSID sanitizing and security classification
- [ ] Fuzz terminal-size changes
- [ ] Audit: no secrets in logs, no `unsafe`, no stray `println!`

**Accept:** survives NM restarts and adapter hot-plug without a crash or a corrupted terminal.

### Phase 10: Release
- [ ] README with screenshots/GIF, install and usage docs, keybinding table
- [ ] `cargo install` works; release binaries built in CI
- [ ] Changelog and version tag
- [ ] Packaging notes (for example AUR / Nix) only as documentation, not as scope creep

**Accept:** a stranger can install, run, and be impressed within two minutes.

---

## 10. Testing strategy

- **Unit tests:** security classification, SSID sanitizing, dedupe/sort, selection pinning, reducer transitions, settings-dict construction.
- **Snapshot tests:** every screen at three sizes plus the too-small size, in every theme, using `TestBackend` and `insta`. A UI change without updated snapshots is incomplete.
- **Backend tests:** the demo backend doubles as a test fixture. Test the service loop against it, including error injection (rate-limit, polkit denial, device removal).
- **Manual checklist** (run before each release): real connect/disconnect, wrong password, hidden SSID, forget, NM restart, SSH session, a tiny terminal, a huge terminal, `NO_COLOR`, a 16-color terminal.

---

## 11. Rules for agents

1. **Stay in scope.** Check section 1 before adding anything. If something is not listed, ask.
2. **Follow the roadmap order.** Do not jump ahead or build Phase 6 visuals before Phase 3 works.
3. **Make the smallest change that completes the task.** Do not refactor unrelated code.
4. **Never break the terminal.** Any change touching `tui.rs`, event handling, or panics requires manual verification that the shell is restored.
5. **Do not invent APIs.** For `ratatui`, `zbus`, and NetworkManager, check the docs for the pinned version. If unsure, say so rather than guessing.
6. **UI changes need snapshots** and must meet the quality bar in 7.7.
7. **Keep the layers separate** (`nm/` vs `ui/`), and keep `view()` pure.
8. **Handle errors, do not hide them.** No `unwrap()`, no silent `let _ =` on fallible operations that matter.
9. **Protect secrets.** No PSKs in logs, snapshots, test fixtures, or error messages.
10. **Run `fmt`, `clippy -D warnings`, and `test` before declaring a task done,** and report anything you could not verify (for example, behavior that needs real WiFi hardware).
11. **When requirements are ambiguous, ask one precise question** instead of making a large assumption.

---

## 12. Definition of done (per task)

- [ ] Meets the phase acceptance criteria it belongs to
- [ ] `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test` all pass
- [ ] Snapshots added/updated and reviewed (for UI work)
- [ ] No new `unsafe`, `unwrap`, `println!`, or secret leakage
- [ ] Terminal restore verified (for anything touching the event loop or panics)
- [ ] Docs/help text updated if behavior or keybindings changed
