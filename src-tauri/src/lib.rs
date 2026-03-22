use std::process::Command;
use std::sync::{Arc, Mutex};
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};

const WORK_SECS: u32 = 25 * 60;
const BREAK_SECS: u32 = 5 * 60;

#[derive(serde::Serialize, serde::Deserialize)]
struct Reminder {
    id: i64,
    title: String,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Task {
    id: i64,
    title: String,
    total_seconds: i64,
}

type DbShared = Arc<Mutex<rusqlite::Connection>>;

#[derive(Clone, serde::Serialize)]
struct PomodoroState {
    mode: String,
    remaining: u32,
    running: bool,
    completed_sessions: u32,
    active_task_id: Option<i64>,
    active_task_elapsed: u32,
}

type PomodoroShared = Arc<Mutex<PomodoroState>>;

fn update_tray_from_state(app: &tauri::AppHandle, s: &PomodoroState) {
    let title = if !s.running && s.mode == "work" && s.remaining == WORK_SECS {
        String::new()
    } else if !s.running && s.mode == "break" && s.remaining == BREAK_SECS {
        String::new()
    } else {
        let m = s.remaining / 60;
        let sec = s.remaining % 60;
        if s.running {
            format!("{:02}:{:02}", m, sec)
        } else {
            format!("⏸ {:02}:{:02}", m, sec)
        }
    };
    if let Some(tray) = app.tray_by_id("main") {
        tray.set_title(Some(title.as_str())).ok();
    }
}

const BLOCK_START: &str = "### nelson";
const BLOCK_END: &str = "### nelson end";
const HOSTS_PATH: &str = "/etc/hosts";
const HOSTS_TMP: &str = "/tmp/nelson_hosts_tmp";

static CACHED_PASSWORD: Mutex<Option<String>> = Mutex::new(None);

fn notes_dir(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("notes");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn migrate_legacy_notes(app: &tauri::AppHandle) {
    let data_dir = match app.path().app_data_dir() {
        Ok(d) => d,
        Err(_) => return,
    };
    let legacy = data_dir.join("notes.txt");
    if !legacy.exists() {
        return;
    }
    let notes_subdir = data_dir.join("notes");
    if std::fs::create_dir_all(&notes_subdir).is_ok() {
        let dest = notes_subdir.join("Notes.txt");
        if !dest.exists() {
            let _ = std::fs::copy(&legacy, &dest);
        }
        let _ = std::fs::remove_file(&legacy);
    }
}

fn sanitize_name(name: &str) -> Result<String, String> {
    let s = name.trim().to_string();
    if s.is_empty() {
        return Err("Note name cannot be empty".into());
    }
    if s.contains('/') || s.contains('\\') || s.contains('\0') || s.starts_with('.') {
        return Err("Invalid note name".into());
    }
    Ok(s)
}

#[tauri::command]
fn list_notes(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    migrate_legacy_notes(&app);
    let dir = notes_dir(&app)?;
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name();
            let name_str = name.to_str()?.to_string();
            name_str.strip_suffix(".txt").map(|s| s.to_string())
        })
        .collect();
    names.sort();
    if names.is_empty() {
        std::fs::write(dir.join("Notes.txt"), "").map_err(|e| e.to_string())?;
        names.push("Notes".to_string());
    }
    Ok(names)
}

#[tauri::command]
fn load_note(app: tauri::AppHandle, name: String) -> Result<String, String> {
    let path = notes_dir(&app)?.join(format!("{}.txt", name));
    if path.exists() {
        std::fs::read_to_string(path).map_err(|e| e.to_string())
    } else {
        Ok(String::new())
    }
}

#[tauri::command]
fn save_note(app: tauri::AppHandle, name: String, content: String) -> Result<(), String> {
    let dir = notes_dir(&app)?;
    std::fs::write(dir.join(format!("{}.txt", name)), content).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_note(app: tauri::AppHandle, name: String) -> Result<String, String> {
    let dir = notes_dir(&app)?;
    let base = if name.trim().is_empty() {
        "Note".to_string()
    } else {
        sanitize_name(&name)?
    };
    let mut candidate = base.clone();
    let mut i = 2usize;
    loop {
        let path = dir.join(format!("{}.txt", candidate));
        if !path.exists() {
            std::fs::write(&path, "").map_err(|e| e.to_string())?;
            return Ok(candidate);
        }
        candidate = format!("{} {}", base, i);
        i += 1;
    }
}

#[tauri::command]
fn delete_note(app: tauri::AppHandle, name: String) -> Result<(), String> {
    let path = notes_dir(&app)?.join(format!("{}.txt", name));
    if path.exists() {
        std::fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn rename_note(app: tauri::AppHandle, old_name: String, new_name: String) -> Result<(), String> {
    let new_name = sanitize_name(&new_name)?;
    let dir = notes_dir(&app)?;
    let old_path = dir.join(format!("{}.txt", old_name));
    let new_path = dir.join(format!("{}.txt", new_name));
    if new_path.exists() {
        return Err("A note with that name already exists".into());
    }
    std::fs::rename(old_path, new_path).map_err(|e| e.to_string())
}

fn osascript_cp(password: &str) -> Result<(), String> {
    let escaped = password.replace('\\', "\\\\").replace('"', "\\\"");
    let script = format!(
        r#"do shell script "cp {} {}" with administrator privileges password "{}""#,
        HOSTS_TMP, HOSTS_PATH, escaped
    );
    let output = Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if stderr.contains("User canceled") || stderr.contains("(-128)") {
        Err("Cancelled".into())
    } else if stderr.contains("password") || stderr.contains("authorization") || stderr.contains("not authorized") {
        Err("WrongPassword".into())
    } else {
        Err(stderr)
    }
}

fn write_hosts_with_sudo(content: &str) -> Result<(), String> {
    std::fs::write(HOSTS_TMP, content).map_err(|e| e.to_string())?;

    let cached = CACHED_PASSWORD.lock().unwrap().clone();
    match cached {
        Some(ref password) => {
            match osascript_cp(password) {
                Ok(()) => Ok(()),
                Err(ref e) if e == "WrongPassword" => {
                    *CACHED_PASSWORD.lock().unwrap() = None;
                    Err("NeedPassword".into())
                }
                Err(e) => Err(e),
            }
        }
        None => Err("NeedPassword".into()),
    }
}

#[tauri::command]
fn write_blocked_with_password(domains: Vec<String>, password: String) -> Result<(), String> {
    let hosts = std::fs::read_to_string(HOSTS_PATH).map_err(|e| e.to_string())?;
    let new_hosts = rebuild_hosts(&hosts, &domains);
    std::fs::write(HOSTS_TMP, &new_hosts).map_err(|e| e.to_string())?;
    osascript_cp(&password)?;
    *CACHED_PASSWORD.lock().unwrap() = Some(password);
    Ok(())
}

fn parse_block(hosts: &str) -> Vec<String> {
    let mut inside = false;
    let mut domains = Vec::new();
    for line in hosts.lines() {
        if line.trim() == BLOCK_START {
            inside = true;
            continue;
        }
        if line.trim() == BLOCK_END {
            break;
        }
        if inside {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && !parts[1].starts_with("www.") {
                domains.push(parts[1].to_string());
            }
        }
    }
    domains
}

fn rebuild_hosts(hosts: &str, domains: &[String]) -> String {
    let block = build_block(domains);

    // Replace existing block if present
    if let Some(start) = hosts.find(BLOCK_START) {
        if let Some(end_offset) = hosts[start..].find(BLOCK_END) {
            let end = start + end_offset + BLOCK_END.len();
            // consume trailing newline if present
            let end = if hosts.as_bytes().get(end) == Some(&b'\n') {
                end + 1
            } else {
                end
            };
            return format!("{}{}{}", &hosts[..start], block, &hosts[end..]);
        }
    }

    // Append block if not found
    let sep = if hosts.ends_with('\n') { "" } else { "\n" };
    format!("{}{}{}", hosts, sep, block)
}

fn build_block(domains: &[String]) -> String {
    let mut s = String::from("### nelson\n");
    for domain in domains {
        s.push_str(&format!("0.0.0.0 {}\n", domain));
        if !domain.starts_with("www.") {
            s.push_str(&format!("0.0.0.0 www.{}\n", domain));
        }
    }
    s.push_str("### nelson end\n");
    s
}

#[tauri::command]
fn read_domains(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    let path = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("domains.txt");
    if path.exists() {
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let domains: Vec<String> = text
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();
        return Ok(domains);
    }
    // Migration: fall back to reading from /etc/hosts on first run
    let hosts = std::fs::read_to_string(HOSTS_PATH).unwrap_or_default();
    Ok(parse_block(&hosts))
}

#[tauri::command]
fn save_domains(app: tauri::AppHandle, domains: Vec<String>) -> Result<(), String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("domains.txt"), domains.join("\n")).map_err(|e| e.to_string())
}


#[tauri::command]
fn get_blocking_status() -> bool {
    let hosts = std::fs::read_to_string(HOSTS_PATH).unwrap_or_default();
    !parse_block(&hosts).is_empty()
}

#[tauri::command]
fn read_blocked() -> Result<Vec<String>, String> {
    let hosts = std::fs::read_to_string(HOSTS_PATH).map_err(|e| e.to_string())?;
    let domains = parse_block(&hosts);

    // If block doesn't exist yet, create it
    if !hosts.contains(BLOCK_START) {
        let new_hosts = rebuild_hosts(&hosts, &[]);
        write_hosts_with_sudo(&new_hosts)?;
    }

    Ok(domains)
}

#[tauri::command]
fn write_blocked(domains: Vec<String>) -> Result<(), String> {
    let hosts = std::fs::read_to_string(HOSTS_PATH).map_err(|e| e.to_string())?;
    let new_hosts = rebuild_hosts(&hosts, &domains);
    write_hosts_with_sudo(&new_hosts)
}

#[tauri::command]
fn get_reminders(db: tauri::State<DbShared>) -> Result<Vec<Reminder>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, title FROM reminders WHERE completed = 0 ORDER BY created_at ASC")
        .map_err(|e| e.to_string())?;
    let reminders = stmt
        .query_map([], |row| {
            Ok(Reminder {
                id: row.get(0)?,
                title: row.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(reminders)
}

#[tauri::command]
fn create_reminder(db: tauri::State<DbShared>, title: String) -> Result<(), String> {
    let title = title.trim().to_string();
    if title.is_empty() {
        return Err("Title cannot be empty".into());
    }
    let conn = db.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO reminders (title) VALUES (?1)",
        rusqlite::params![title],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn complete_reminder(db: tauri::State<DbShared>, id: i64) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE reminders SET completed = 1 WHERE id = ?1",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ── Task commands ──────────────────────────────────────────────────────────────

#[tauri::command]
fn get_tasks(db: tauri::State<DbShared>) -> Result<Vec<Task>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, title, total_seconds FROM tasks ORDER BY created_at ASC")
        .map_err(|e| e.to_string())?;
    let tasks = stmt
        .query_map([], |row| {
            Ok(Task {
                id: row.get(0)?,
                title: row.get(1)?,
                total_seconds: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(tasks)
}

#[tauri::command]
fn create_task(db: tauri::State<DbShared>, title: String) -> Result<Task, String> {
    let title = title.trim().to_string();
    if title.is_empty() {
        return Err("Title cannot be empty".into());
    }
    let conn = db.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO tasks (title) VALUES (?1)",
        rusqlite::params![title],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    Ok(Task { id, title, total_seconds: 0 })
}

#[tauri::command]
fn update_task(db: tauri::State<DbShared>, id: i64, title: String) -> Result<Task, String> {
    let title = title.trim().to_string();
    if title.is_empty() {
        return Err("Title cannot be empty".into());
    }
    let conn = db.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE tasks SET title = ?1 WHERE id = ?2",
        rusqlite::params![title, id],
    )
    .map_err(|e| e.to_string())?;
    let total_seconds: i64 = conn
        .query_row(
            "SELECT total_seconds FROM tasks WHERE id = ?1",
            rusqlite::params![id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(Task { id, title, total_seconds })
}

#[tauri::command]
fn delete_task(
    state: tauri::State<PomodoroShared>,
    db: tauri::State<DbShared>,
    id: i64,
) -> Result<(), String> {
    {
        let mut s = state.lock().unwrap();
        if s.active_task_id == Some(id) {
            s.active_task_id = None;
            s.active_task_elapsed = 0;
        }
    }
    let conn = db.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM tasks WHERE id = ?1", rusqlite::params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn reset_task_time(
    state: tauri::State<PomodoroShared>,
    db: tauri::State<DbShared>,
    id: i64,
) -> Result<(), String> {
    {
        let mut s = state.lock().unwrap();
        if s.active_task_id == Some(id) {
            s.active_task_elapsed = 0;
        }
    }
    let conn = db.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE tasks SET total_seconds = 0 WHERE id = ?1",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn set_active_task(
    app: tauri::AppHandle,
    state: tauri::State<PomodoroShared>,
    db: tauri::State<DbShared>,
    id: Option<i64>,
) {
    let (s, old_id, elapsed) = {
        let mut s = state.lock().unwrap();
        let old_id = s.active_task_id;
        let elapsed = s.active_task_elapsed;
        s.active_task_elapsed = 0;
        s.active_task_id = id;
        (s.clone(), old_id, elapsed)
    };
    // Flush accumulated time for the previous task
    if let Some(tid) = old_id {
        if elapsed > 0 {
            if let Ok(conn) = db.lock() {
                conn.execute(
                    "UPDATE tasks SET total_seconds = total_seconds + ?1 WHERE id = ?2",
                    rusqlite::params![elapsed as i64, tid],
                )
                .ok();
            }
        }
    }
    app.emit("pomodoro-tick", &s).ok();
}

// ── Pomodoro commands ──────────────────────────────────────────────────────────

#[tauri::command]
fn pomodoro_get_state(state: tauri::State<PomodoroShared>) -> PomodoroState {
    state.lock().unwrap().clone()
}

#[tauri::command]
fn pomodoro_toggle(
    app: tauri::AppHandle,
    state: tauri::State<PomodoroShared>,
    db: tauri::State<DbShared>,
) {
    let (s, flush_id, flush_elapsed) = {
        let mut s = state.lock().unwrap();
        let was_running = s.running;
        s.running = !s.running;
        let mut flush_id = None;
        let mut flush_elapsed = 0u32;
        // Flush on pause (not on resume)
        if was_running && s.mode == "work" {
            if let Some(tid) = s.active_task_id {
                flush_id = Some(tid);
                flush_elapsed = s.active_task_elapsed;
                s.active_task_elapsed = 0;
            }
        }
        (s.clone(), flush_id, flush_elapsed)
    };
    if let Some(tid) = flush_id {
        if flush_elapsed > 0 {
            if let Ok(conn) = db.lock() {
                conn.execute(
                    "UPDATE tasks SET total_seconds = total_seconds + ?1 WHERE id = ?2",
                    rusqlite::params![flush_elapsed as i64, tid],
                )
                .ok();
            }
        }
    }
    update_tray_from_state(&app, &s);
    app.emit("pomodoro-tick", &s).ok();
}

#[tauri::command]
fn pomodoro_reset(
    app: tauri::AppHandle,
    state: tauri::State<PomodoroShared>,
    db: tauri::State<DbShared>,
) {
    let (s, flush_id, flush_elapsed) = {
        let mut s = state.lock().unwrap();
        let mut flush_id = None;
        let mut flush_elapsed = 0u32;
        // Flush if we were running in work mode
        if s.running && s.mode == "work" {
            if let Some(tid) = s.active_task_id {
                flush_id = Some(tid);
                flush_elapsed = s.active_task_elapsed;
            }
        }
        s.active_task_elapsed = 0;
        s.running = false;
        s.remaining = if s.mode == "work" { WORK_SECS } else { BREAK_SECS };
        (s.clone(), flush_id, flush_elapsed)
    };
    if let Some(tid) = flush_id {
        if flush_elapsed > 0 {
            if let Ok(conn) = db.lock() {
                conn.execute(
                    "UPDATE tasks SET total_seconds = total_seconds + ?1 WHERE id = ?2",
                    rusqlite::params![flush_elapsed as i64, tid],
                )
                .ok();
            }
        }
    }
    update_tray_from_state(&app, &s);
    app.emit("pomodoro-tick", &s).ok();
}

#[tauri::command]
fn pomodoro_skip_break(app: tauri::AppHandle, state: tauri::State<PomodoroShared>) {
    let s = {
        let mut s = state.lock().unwrap();
        s.running = false;
        s.mode = "work".into();
        s.remaining = WORK_SECS;
        s.clone()
    };
    update_tray_from_state(&app, &s);
    app.emit("pomodoro-tick", &s).ok();
}

#[tauri::command]
fn app_exit(app: tauri::AppHandle) {
    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let pomodoro: PomodoroShared = Arc::new(Mutex::new(PomodoroState {
        mode: "work".into(),
        remaining: WORK_SECS,
        running: false,
        completed_sessions: 0,
        active_task_id: None,
        active_task_elapsed: 0,
    }));

    tauri::Builder::default()
        .manage(pomodoro)
        .setup(|app| {
            let show = MenuItemBuilder::with_id("show", "Show").build(app)?;
            let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
            let menu = MenuBuilder::new(app).items(&[&show, &quit]).build()?;

            let _tray = TrayIconBuilder::with_id("main")
                .menu(&menu)
                .icon(tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png")).unwrap())
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            window.show().unwrap();
                            window.set_focus().unwrap();
                        }
                    }
                    "quit" => {
                        if let Some(window) = app.get_webview_window("main") {
                            window.show().ok();
                            window.set_focus().ok();
                        }
                        app.emit("quit-requested", ()).ok();
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                window.hide().unwrap();
                            } else {
                                window.show().unwrap();
                                window.set_focus().unwrap();
                            }
                        }
                    }
                })
                .build(app)?;

            // Initialise SQLite
            let db_path = app.path().app_data_dir()?.join("tasks.db");
            let conn = rusqlite::Connection::open(&db_path)?;
            conn.execute_batch(
                "PRAGMA journal_mode=WAL;
                 CREATE TABLE IF NOT EXISTS tasks (
                     id           INTEGER PRIMARY KEY AUTOINCREMENT,
                     title        TEXT    NOT NULL,
                     total_seconds INTEGER NOT NULL DEFAULT 0,
                     created_at   INTEGER NOT NULL DEFAULT (strftime('%s','now'))
                 );
                 CREATE TABLE IF NOT EXISTS reminders (
                     id         INTEGER PRIMARY KEY AUTOINCREMENT,
                     title      TEXT    NOT NULL,
                     completed  INTEGER NOT NULL DEFAULT 0,
                     created_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
                 );",
            )?;
            let db: DbShared = Arc::new(Mutex::new(conn));
            app.manage(db);

            // Background timer loop
            let db_clone = app.state::<DbShared>().inner().clone();
            let pomodoro = app.state::<PomodoroShared>().inner().clone();
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    let (state, flush_id, flush_elapsed) = {
                        let mut s = pomodoro.lock().unwrap();
                        let mut flush_id = None;
                        let mut flush_elapsed = 0u32;
                        if s.running {
                            if s.remaining > 0 {
                                s.remaining -= 1;
                                // Accumulate elapsed in work mode
                                if s.mode == "work" && s.active_task_id.is_some() {
                                    s.active_task_elapsed += 1;
                                }
                            }
                            if s.remaining == 0 {
                                s.running = false;
                                if s.mode == "work" {
                                    s.completed_sessions += 1;
                                    // Flush accumulated time on session complete
                                    if let Some(tid) = s.active_task_id {
                                        flush_id = Some(tid);
                                        flush_elapsed = s.active_task_elapsed;
                                        s.active_task_elapsed = 0;
                                    }
                                    s.mode = "break".into();
                                    s.remaining = BREAK_SECS;
                                } else {
                                    s.mode = "work".into();
                                    s.remaining = WORK_SECS;
                                }
                            }
                        }
                        (s.clone(), flush_id, flush_elapsed)
                    };
                    if let Some(tid) = flush_id {
                        if flush_elapsed > 0 {
                            if let Ok(conn) = db_clone.lock() {
                                conn.execute(
                                    "UPDATE tasks SET total_seconds = total_seconds + ?1 WHERE id = ?2",
                                    rusqlite::params![flush_elapsed as i64, tid],
                                )
                                .ok();
                            }
                        }
                    }
                    update_tray_from_state(&app_handle, &state);
                    app_handle.emit("pomodoro-tick", &state).ok();
                }
            });

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            list_notes,
            load_note,
            save_note,
            create_note,
            delete_note,
            rename_note,
            read_blocked,
            write_blocked,
            write_blocked_with_password,
            read_domains,
            save_domains,
            get_blocking_status,
            pomodoro_get_state,
            pomodoro_toggle,
            pomodoro_reset,
            pomodoro_skip_break,
            get_reminders,
            create_reminder,
            complete_reminder,
            get_tasks,
            create_task,
            update_task,
            delete_task,
            reset_task_time,
            set_active_task,
            app_exit,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
