---
name: Notion Lite desktop app
overview: Turn the notion-lite axum server into an installable Windows desktop app with Tauri 2, keeping the standalone server binary working.
todos:
  - id: lib-split
    content: Move app/router into src/lib.rs with run(listener, Config); thin src/main.rs with HOST env (default 127.0.0.1)
    status: completed
  - id: workspace-desktop
    content: Root Cargo.toml becomes a workspace; add desktop/ Tauri 2 crate notion-lite-desktop
    status: completed
  - id: secret-auth
    content: Per-launch secret auth mode (k query to HttpOnly SameSite=Strict cookie)
    status: completed
  - id: token-setup
    content: Keyring token, /setup page, swappable token, Settings link
    status: completed
  - id: links-out
    content: External navigation and new windows open in default browser
    status: completed
  - id: icon-bundle
    content: Icon set and NSIS per-user bundle
    status: completed
  - id: gates
    content: Run gates, commit and push
    status: completed
isProject: false
---

# Notion Lite desktop app

Approved by Charles: a real Windows desktop app (own window, taskbar icon, Start menu entry, no terminal), installable per user on his other PCs. Windows only. The server binary keeps working (Docker included). Commit and push to `main` approved; no releases.

## Steps

### 1. Library split (least change)
- `src/lib.rs` holds the modules `notion`, `render`, `views` and the app/router code from `src/main.rs`.
- Public API: `Config { token, auth: Auth, save_token: Option<SaveFn> }`, `Auth::{None, Basic(String), Secret(String)}`, `pub async fn run(listener: tokio::net::TcpListener, cfg: Config) -> std::io::Result<()>`.
- `src/main.rs` becomes a thin server binary: reads `NOTION_TOKEN` (still `expect`), `DASH_USER`/`DASH_PASSWORD` (Basic auth), `PORT` (default 8080) and new `HOST` (default `127.0.0.1`).
- `Dockerfile`: `ENV HOST=0.0.0.0` and build only the server package (`-p notion-lite`) so the Linux image never compiles Tauri.

### 2. Workspace and desktop crate
- Root `Cargo.toml` gains `[workspace] members = [".", "desktop"]`; the release profile stays at the root.
- `desktop/`: Tauri 2 crate and binary `notion-lite-desktop`, product name "Notion Lite", identifier `com.jadepuma.notionlite`, depends on the root lib by path.
- `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`.

### 3. Local server inside the app
- In Tauri `setup`: bind `127.0.0.1:0` (std listener, so the port is known before the window opens), make a random 32-byte hex secret, spawn `run` on `tauri::async_runtime`.
- Main window "Notion Lite", about 1400x900, at `http://127.0.0.1:<port>/?k=<secret>`.
- Secret auth: a request with a matching `?k=` gets an HttpOnly, SameSite=Strict, Path=/ session cookie and a redirect to the same path without `k`; any other request without the cookie gets 401. Basic auth stays for the server binary. `/healthz` stays open.

### 4. Token per computer
- Desktop reads the token from Windows Credential Manager via `keyring` (service `notion-lite`, account `notion-token`). Missing token means an empty token, never a panic.
- Notion client token becomes swappable (`Arc<std::sync::RwLock<String>>`), plus `check_token` (GET `/v1/users/me` with a candidate token).
- `/setup` (only when `save_token` is set, i.e. desktop): password input for the `ntn_` secret, note that the Tasks and Clients databases must be shared with the integration, Save.
- POST `/setup`: validate with Notion; on success store in keyring, swap into the client, clear caches, kick a refresh of all boards, redirect to the board; on failure re-show the form with Notion's message.
- Boards redirect to `/setup` when the token is empty or Notion answers 401 (desktop only). Warm loop skips while the token is empty.
- Header gets a small "Settings" link to `/setup` in desktop mode only.

### 5. Links out
- `WebviewWindowBuilder::on_navigation`: allow only `http://127.0.0.1:<port>`; http/https/mailto/tel elsewhere open in the default browser via `tauri-plugin-opener` and are cancelled.
- `on_new_window`: open the URL in the default browser, return `NewWindowResponse::Deny`.
- Fallback for the reported WebView2 issue where `on_new_window` does not fire for `target=_blank` anchors (tauri issue 15872): a webview initialization script turns external `target=_blank` clicks and `window.open` calls into a top-level navigation, which `on_navigation` then sends to the browser.

### 6. Bind check
- Desktop binds only `127.0.0.1`. Verified with netstat by PID.

### 7. Icon
- Generate a 1024px PNG (dark rounded square, white N) with Python/Pillow, then `cargo tauri icon` into `desktop/icons/`.

### 8. Bundle
- `bundle.targets = ["nsis"]`, `nsis.installMode = "currentUser"`, `webviewInstallMode = downloadBootstrapper`.
- Install the CLI with `cargo install tauri-cli --version "^2" --locked`; build with `cargo tauri build`.

## Gates
1. `cargo build --release` (whole workspace): zero errors, zero warnings.
2. Server binary on `PORT=8091`: `/healthz` and `/v/priority` return 200; not reachable on the LAN IP; stop that PID only. Never touch the instance on port 8080.
3. `cargo tauri build` succeeds; report installer path/size and desktop exe size.
4. Launch the desktop exe, confirm it listens only on 127.0.0.1 (netstat by PID), close it by PID.
5. Commit and push to origin main.

## Don'ts
- No board UI changes beyond the Settings link and setup page; no refactors or renames outside this work.
- No token in any file, env default, installer or commit. No Notion writes during testing.

## Outcome (2026-11-24)
- Builds ran with `CARGO_TARGET_DIR=target/desktop-build` because another agent's server held a lock on `target/release/notion-lite.exe`.
- `build.frontendDist` is omitted; the window is created in `setup` with an external 127.0.0.1 URL, and the build was fine without it.
- All gates passed. The `?k=` cookie flow, the bogus-token setup error and the external link to the default browser were checked live through WebView2 remote debugging. A real token save was not exercised.
