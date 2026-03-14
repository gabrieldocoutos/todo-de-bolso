# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Development
npm run tauri dev        # Start full app (Rust + Svelte, hot-reload)
npm run dev              # Start frontend only (no Tauri shell)

# Type checking
npm run check            # Run svelte-check + TypeScript validation
npm run check:watch      # Watch mode

# Build
npm run tauri build      # Build distributable desktop app
npm run build            # Build frontend only
```

There is no test runner configured yet.

## Architecture

This is a **Tauri v2 desktop app** with a **SvelteKit (Svelte 5) + TypeScript** frontend.

### Two-layer structure

**Frontend** (`src/`) — SvelteKit app compiled to static files:
- `src/routes/` — SvelteKit file-based routing (`+page.svelte`, `+layout.ts`)
- `src/app.html` — HTML shell
- Builds to `build/` (consumed by Tauri as `frontendDist`)
- Uses `@sveltejs/adapter-static` — no SSR, pure static output

**Backend** (`src-tauri/`) — Rust Tauri application:
- `src-tauri/src/lib.rs` — Tauri commands and app setup (`run()` entry point)
- `src-tauri/src/main.rs` — thin entry point, calls `lib::run()`
- `src-tauri/tauri.conf.json` — app config (window size, bundle targets, CSP, etc.)
- `src-tauri/Cargo.toml` — Rust dependencies

### Frontend ↔ Backend communication

Use `@tauri-apps/api` to call Rust commands from Svelte:

```ts
import { invoke } from '@tauri-apps/api/core';
const result = await invoke('command_name', { arg: value });
```

Register commands in `src-tauri/src/lib.rs` with `#[tauri::command]` and add them to `.invoke_handler(tauri::generate_handler![...])`.

### Key config

- Dev server runs on `http://localhost:1420`
- Window default: 800×600
- App identifier: `com.gabdcouto.produtividade-de-bolso`
