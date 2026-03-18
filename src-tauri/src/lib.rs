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

const NOTES_FOLDER: &str = "Produtividade de Bolso";

#[derive(serde::Serialize, serde::Deserialize)]
struct NoteInfo {
    id: String,
    name: String,
}

async fn run_applescript(script: String) -> Result<String, String> {
    let output = tauri::async_runtime::spawn_blocking(move || {
        Command::new("osascript").arg("-e").arg(&script).output()
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn write_tmp(name: &str, content: &str) -> Result<String, String> {
    let path = std::env::temp_dir().join(name);
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(path.to_str().unwrap().to_string())
}

async fn notes_ensure_folder() -> Result<(), String> {
    let script = format!(
        r#"tell application "Notes"
    tell account "iCloud"
        if not (exists folder "{}") then
            make new folder with properties {{name: "{}"}}
        end if
    end tell
end tell"#,
        NOTES_FOLDER, NOTES_FOLDER
    );
    run_applescript(script).await?;
    Ok(())
}

async fn notes_list() -> Result<Vec<NoteInfo>, String> {
    let script = format!(
        r#"tell application "Notes"
    set output to ""
    try
        set theFolder to folder "{}" of account "iCloud"
        set noteList to notes of theFolder
        repeat with n in noteList
            set noteName to name of n as string
            if noteName is not "" then
                set output to output & (id of n as string) & (ASCII character 0) & noteName & (ASCII character 1)
            end if
        end repeat
    end try
    return output
end tell"#,
        NOTES_FOLDER
    );
    let raw = run_applescript(script).await?;
    let mut notes = Vec::new();
    if !raw.is_empty() {
        for record in raw.split('\u{01}') {
            if record.is_empty() {
                continue;
            }
            if let Some(sep) = record.find('\u{00}') {
                notes.push(NoteInfo {
                    id: record[..sep].to_string(),
                    name: record[sep + 1..].to_string(),
                });
            }
        }
    }
    Ok(notes)
}

async fn notes_get_raw(id: &str) -> Result<String, String> {
    let script = format!(
        r#"tell application "Notes"
    return plaintext of (note id "{}")
end tell"#,
        id
    );
    run_applescript(script).await
}

async fn notes_create(name: &str) -> Result<String, String> {
    let tmp = write_tmp("notes_create_title.txt", name)?;
    let script = format!(
        r#"tell application "Notes"
    set titleStr to read POSIX file "{}" as «class utf8»
    set newNote to make new note at folder "{}" of account "iCloud" with properties {{body: titleStr}}
    return id of newNote as string
end tell"#,
        tmp, NOTES_FOLDER
    );
    run_applescript(script).await
}

async fn notes_update(id: &str, body: &str) -> Result<(), String> {
    let tmp = write_tmp("notes_update_body.txt", body)?;
    let script = format!(
        r#"tell application "Notes"
    set n to note id "{}"
    set fileContent to read POSIX file "{}" as «class utf8»
    set body of n to fileContent
end tell"#,
        id, tmp
    );
    run_applescript(script).await?;
    Ok(())
}

async fn notes_delete(id: &str) -> Result<(), String> {
    let script = format!(
        r#"tell application "Notes"
    delete (note id "{}")
end tell"#,
        id
    );
    run_applescript(script).await?;
    Ok(())
}

fn strip_title_line(full_text: &str) -> String {
    if let Some(pos) = full_text.find("\n\n") {
        full_text[pos + 2..].to_string()
    } else if let Some(pos) = full_text.find('\n') {
        full_text[pos + 1..].to_string()
    } else {
        String::new()
    }
}

async fn migrate_notes_to_mac(app: &tauri::AppHandle) {
    let data_dir = match app.path().app_data_dir() {
        Ok(d) => d,
        Err(_) => return,
    };
    let notes_dir = data_dir.join("notes");
    if !notes_dir.exists() || data_dir.join("notes_migrated").exists() {
        return;
    }
    let entries = match std::fs::read_dir(&notes_dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("txt") {
            continue;
        }
        let name = match path.file_stem().and_then(|s| s.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let id = match notes_create(&name).await {
            Ok(id) => id,
            Err(_) => continue,
        };
        if !content.is_empty() {
            let _ = notes_update(&id, &format!("{}\n\n{}", name, content)).await;
        }
    }
    let _ = std::fs::rename(&notes_dir, data_dir.join("notes_migrated"));
}

#[tauri::command]
async fn list_notes(app: tauri::AppHandle) -> Result<Vec<NoteInfo>, String> {
    notes_ensure_folder().await?;
    migrate_notes_to_mac(&app).await;
    let mut notes = notes_list().await?;
    if notes.is_empty() {
        let id = notes_create("Notes").await?;
        notes = vec![NoteInfo {
            id,
            name: "Notes".to_string(),
        }];
    }
    Ok(notes)
}

#[tauri::command]
async fn load_note(id: String) -> Result<String, String> {
    let full_text = notes_get_raw(&id).await?;
    Ok(strip_title_line(&full_text))
}

#[tauri::command]
async fn save_note(id: String, name: String, content: String) -> Result<(), String> {
    notes_update(&id, &format!("{}\n\n{}", name, content)).await
}

#[tauri::command]
async fn create_note(name: String) -> Result<NoteInfo, String> {
    let note_name = if name.trim().is_empty() {
        "Note".to_string()
    } else {
        name
    };
    let id = notes_create(&note_name).await?;
    Ok(NoteInfo {
        id,
        name: note_name,
    })
}

#[tauri::command]
async fn delete_note(id: String) -> Result<(), String> {
    notes_delete(&id).await
}

#[tauri::command]
async fn rename_note(id: String, new_name: String) -> Result<(), String> {
    let full_text = notes_get_raw(&id).await?;
    let content = strip_title_line(&full_text);
    notes_update(&id, &format!("{}\n\n{}", new_name, content)).await
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
