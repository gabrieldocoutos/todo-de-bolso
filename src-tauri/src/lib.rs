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
    id: String,
    title: String,
}

#[derive(Clone, serde::Serialize)]
struct PomodoroState {
    mode: String,
    remaining: u32,
    running: bool,
    completed_sessions: u32,
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

async fn run_sidecar(app: &tauri::AppHandle, args: &[&str]) -> Result<String, String> {
    use tauri_plugin_shell::ShellExt;
    let output = app
        .shell()
        .sidecar("reminders-helper")
        .map_err(|e| e.to_string())?
        .args(args)
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[tauri::command]
async fn get_reminders(app: tauri::AppHandle) -> Result<Vec<Reminder>, String> {
    let json = run_sidecar(&app, &["list"]).await?;
    serde_json::from_str::<Vec<Reminder>>(&json).map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_reminder(app: tauri::AppHandle, title: String) -> Result<(), String> {
    run_sidecar(&app, &["create", &title]).await?;
    Ok(())
}

#[tauri::command]
async fn complete_reminder(app: tauri::AppHandle, id: String) -> Result<(), String> {
    run_sidecar(&app, &["complete", &id]).await?;
    Ok(())
}

#[tauri::command]
fn pomodoro_get_state(state: tauri::State<PomodoroShared>) -> PomodoroState {
    state.lock().unwrap().clone()
}

#[tauri::command]
fn pomodoro_toggle(app: tauri::AppHandle, state: tauri::State<PomodoroShared>) {
    let s = {
        let mut s = state.lock().unwrap();
        s.running = !s.running;
        s.clone()
    };
    update_tray_from_state(&app, &s);
    app.emit("pomodoro-tick", &s).ok();
}

#[tauri::command]
fn pomodoro_reset(app: tauri::AppHandle, state: tauri::State<PomodoroShared>) {
    let s = {
        let mut s = state.lock().unwrap();
        s.running = false;
        s.remaining = if s.mode == "work" { WORK_SECS } else { BREAK_SECS };
        s.clone()
    };
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let pomodoro: PomodoroShared = Arc::new(Mutex::new(PomodoroState {
        mode: "work".into(),
        remaining: WORK_SECS,
        running: false,
        completed_sessions: 0,
    }));

    tauri::Builder::default()
        .manage(pomodoro)
        .setup(|app| {
            let show = MenuItemBuilder::with_id("show", "Show").build(app)?;
            let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
            let menu = MenuBuilder::new(app).items(&[&show, &quit]).build()?;

            let _tray = TrayIconBuilder::with_id("main")
                .menu(&menu)
                .icon(app.default_window_icon().unwrap().clone())
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            window.show().unwrap();
                            window.set_focus().unwrap();
                        }
                    }
                    "quit" => {
                        app.exit(0);
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

            let pomodoro = app.state::<PomodoroShared>().inner().clone();
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    let state = {
                        let mut s = pomodoro.lock().unwrap();
                        if s.running {
                            if s.remaining > 0 {
                                s.remaining -= 1;
                            }
                            if s.remaining == 0 {
                                s.running = false;
                                if s.mode == "work" {
                                    s.completed_sessions += 1;
                                    s.mode = "break".into();
                                    s.remaining = BREAK_SECS;
                                } else {
                                    s.mode = "work".into();
                                    s.remaining = WORK_SECS;
                                }
                            }
                        }
                        s.clone()
                    };
                    update_tray_from_state(&app_handle, &state);
                    app_handle.emit("pomodoro-tick", &state).ok();
                }
            });

            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
