# CLAUDE.md

This file provides guidance to Claude Code when working with this repository.

## Commands

```bash
# Development
pnpm tauri dev           # Start full app (Rust + Svelte, hot-reload)
pnpm dev                 # Start frontend only (no Tauri shell)

# Type checking
pnpm check               # Run svelte-check + TypeScript validation
pnpm check:watch         # Watch mode

# Build
pnpm tauri build         # Build distributable desktop app
pnpm build               # Build frontend only
```

There is no test runner configured yet.

## Architecture

**Tauri v2 desktop app** with **SvelteKit (Svelte 5) + TypeScript** frontend.

### Two-layer structure

**Frontend** (`src/`) — SvelteKit app compiled to static files:

- `src/routes/` — single-page app with tab-based navigation
- `src/lib/shortcuts.ts` — keyboard shortcut definitions
- `src/app.html` — HTML shell
- Uses `@sveltejs/adapter-static` — no SSR, pure static output
- Builds to `build/` (consumed by Tauri as `frontendDist`)

**Backend** (`src-tauri/`) — Rust Tauri application:

- `src-tauri/src/lib.rs` — all Tauri commands and app setup (~780 lines)
- `src-tauri/src/main.rs` — thin entry point, calls `lib::run()`
- `src-tauri/tauri.conf.json` — app config (window, bundle, CSP)
- `src-tauri/Cargo.toml` — Rust dependencies

### Key config

- Dev server: `http://localhost:1420`
- Window default: 800×600
- App identifier: `com.gabdcouto.mezame`

## Frontend Components

All components live in `src/routes/`:

| Component     | File                     | Purpose                                                         |
| ------------- | ------------------------ | --------------------------------------------------------------- |
| App shell     | `+page.svelte`           | Tab navigation, focus toggle, password modal, quit confirmation |
| Notes         | `Notes.svelte`           | Text editor with sidebar; create/delete/rename notes            |
| Pomodoro      | `Pomodoro.svelte`        | 25min/5min timer, task CRUD, time tracking per task             |
| Blocked Sites | `BlockedWebsites.svelte` | Domain blocklist management; add/remove domains                 |
| Reminders     | `Reminders.svelte`       | Simple todo list with completion                                |
| Shortcuts     | `ShortcutGuide.svelte`   | Modal showing keyboard shortcuts                                |

Layout config in `+layout.ts` sets `ssr = false` and `prerender = true`.

## Backend Commands (Rust)

All commands in `src-tauri/src/lib.rs`. Register with `#[tauri::command]` and add to `tauri::generate_handler![...]`.

### Notes (file-based)

- `list_notes()`, `load_note(name)`, `save_note(name, content)`
- `create_note(name)`, `delete_note(name)`, `rename_note(old, new)`

### Website Blocking

- `read_domains()`, `save_domains(domains)` — app-local domain list
- `get_blocking_status()`, `read_blocked()` — read `/etc/hosts` state
- `write_blocked(domains)` — modify `/etc/hosts` (requires sudo)
- `write_blocked_with_password(domains, password)` — sudo via osascript

### Pomodoro Timer

- `pomodoro_get_state()`, `pomodoro_toggle()`, `pomodoro_reset()`, `pomodoro_skip_break()`
- `set_active_task(id)` — select task for time tracking

### Tasks (SQLite)

- `get_tasks()`, `create_task(title)`, `update_task(id, title)`
- `delete_task(id)`, `reset_task_time(id)`

### Reminders (SQLite)

- `get_reminders()`, `create_reminder(title)`, `complete_reminder(id)`

### Misc

- `app_exit()` — close application

## State Management

### Frontend (Svelte 5 runes, no stores)

```ts
let activeTab = $state<"notes" | "pomodoro" | "blocked" | "reminders">("notes");
$effect(() => {
  /* reactive side effects */
});
```

All data fetched via `invoke()` calls. No Svelte stores — components use local `$state` variables.

### Backend → Frontend events

```ts
import { listen } from "@tauri-apps/api/event";
const unlisten = listen("pomodoro-tick", (event) => {
  /* PomodoroState payload */
});
```

Events emitted:

- `pomodoro-tick` — every second with full timer state
- `quit-requested` — from tray menu

### Backend state

- `PomodoroState` (`Arc<Mutex>`) — timer mode, remaining seconds, running flag, sessions, active task
- `DbShared` (`Arc<Mutex<Connection>>`) — SQLite connection shared across commands
- Background tokio task ticks every 1s: decrements timer, flushes task time, emits events, updates tray

## Data Persistence

| Data            | Storage      | Location                              |
| --------------- | ------------ | ------------------------------------- |
| Notes           | Text files   | `app_data_dir/notes/*.txt`            |
| Tasks           | SQLite       | `app_data_dir/` (tables: `tasks`)     |
| Reminders       | SQLite       | `app_data_dir/` (tables: `reminders`) |
| Blocked domains | Text file    | `app_data_dir/domains.txt`            |
| Active blocks   | `/etc/hosts` | System file (requires admin)          |

## Styling Conventions

- **No CSS framework** — scoped `<style>` blocks in each component
- **Dark theme**: bg `#1e1e1e`, text `#d4d4d4`, accent `#4ec9b0` (teal), borders `#3d3d3d`
- **Monospace fonts**: Menlo, Monaco, Courier New
- **Hover states**: `#2d2d2d` / `#4a4a4a`
- Keep new components consistent with this palette

## Frontend ↔ Backend Communication

```ts
import { invoke } from "@tauri-apps/api/core";
const result = await invoke("command_name", { arg: value });
```

### Adding a new Tauri command

1. Add `#[tauri::command]` function in `src-tauri/src/lib.rs`
2. Register in `tauri::generate_handler![...]` in the `run()` function
3. Call from Svelte via `invoke('command_name', { args })`

### Adding a new tab/feature

1. Create component in `src/routes/NewFeature.svelte`
2. Add tab option to `activeTab` type in `+page.svelte`
3. Add tab button and conditional render in `+page.svelte`
4. Add keyboard shortcut (⌘+N pattern) in `+page.svelte` keydown handler
5. Add shortcut entry in `src/lib/shortcuts.ts`

## Platform Notes

- **macOS only** for website blocking: uses `osascript` to run `sed` with admin privileges on `/etc/hosts`
- Tray icon uses `tauri::tray::TrayIconBuilder` with dynamic title showing timer
- Auto-blocks websites when pomodoro starts in work mode (focus toggle)
- Password cached in-memory for the session to avoid repeated prompts

## Dependencies

### Frontend

- `@tauri-apps/api` ^2, `@tauri-apps/plugin-dialog` ^2, `@tauri-apps/plugin-opener` ^2
- `lucide-svelte` ^0.577.0 (icons)
- SvelteKit ^2.55, Svelte ^5, Vite ^8, TypeScript ~5.6

### Backend

- `tauri` v2 (with tray-icon, image-png features)
- `rusqlite` 0.32 (bundled SQLite)
- `tokio` (async runtime for background timer)
- `serde` + `serde_json` (serialization)
