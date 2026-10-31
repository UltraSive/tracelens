# MTR GUI

Cross-platform MTR-style traceroute/ping GUI. The probing engine is
[`trippy-core`](https://crates.io/crates/trippy-core) (ICMP/UDP/TCP, IPv4/IPv6,
Linux/macOS/Windows), wrapped by a small shared crate so multiple frontends can
be compared.

```
crates/engine/   shared engine: TraceSession, snapshots, report export (no UI deps)
src-tauri/       Tauri 2 backend: commands + 2 Hz snapshot events
src/             SvelteKit (SPA) + Tailwind 4 + shadcn-svelte frontend
```

The `native-egui` branch adds `crates/native/`, a utilitarian egui frontend
built on the same engine, for comparison.

## Prerequisites

- Rust (stable) and Node 20+
- Linux: `sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev libayatana-appindicator3-dev libdbus-1-dev libssl-dev build-essential pkg-config`
- See https://tauri.app/start/prerequisites/ for macOS/Windows.

## Run

```sh
npm install
npm run tauri dev       # dev
npm run tauri build     # installers in target/release/bundle
```

Headless engine smoke test: `cargo run -p mtr-engine --example cli -- 1.1.1.1`

## Privileges (the part that differs per OS)

Tracing needs raw sockets.

| OS | What to do |
|---|---|
| Linux | `sudo setcap cap_net_raw+ep <binary>` (redo after every rebuild), **or** allow unprivileged ICMP for everyone: `sudo sysctl -w net.ipv4.ping_group_range="0 2147483647"` (ICMP/UDP only). TCP mode needs the capability. |
| macOS | ICMP works unprivileged. UDP/TCP may need `sudo`. |
| Windows | Run as Administrator (a `requireAdministrator` manifest is the packaged answer). |

For dev on Linux, the sysctl route is easiest since `tauri dev` rebuilds the binary
(dropping file capabilities). For distribution, set the capability in the deb/rpm
post-install script, or move probing into a small privileged helper process.

## Notes

- `trippy-core` has no cancel API; `TraceSession::stop` sets a flag checked in the
  per-round callback and unwinds the tracer thread with `resume_unwind` (no panic
  output). Stop takes effect at the end of the current round.
- The UI follows the OS light/dark theme and suppresses the most obvious web tells
  (text selection everywhere, the webview context menu in release builds, overscroll).
