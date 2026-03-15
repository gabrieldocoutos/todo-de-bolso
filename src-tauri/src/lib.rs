use std::process::Command;
use std::sync::Mutex;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

const BLOCK_START: &str = "### nelson";
const BLOCK_END: &str = "### nelson end";
const HOSTS_PATH: &str = "/etc/hosts";
const HOSTS_TMP: &str = "/tmp/nelson_hosts_tmp";

static CACHED_PASSWORD: Mutex<Option<String>> = Mutex::new(None);

#[tauri::command]
fn load_notes(app: tauri::AppHandle) -> Result<String, String> {
    let path = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("notes.txt");
    if path.exists() {
        std::fs::read_to_string(path).map_err(|e| e.to_string())
    } else {
        Ok(String::new())
    }
}

#[tauri::command]
fn save_notes(app: tauri::AppHandle, content: String) -> Result<(), String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("notes.txt"), content).map_err(|e| e.to_string())
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
fn update_tray_title(app: tauri::AppHandle, title: String) -> Result<(), String> {
    if let Some(tray) = app.tray_by_id("main") {
        tray.set_title(Some(title.as_str())).map_err(|e| e.to_string())?;
    }
    Ok(())
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
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

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            load_notes,
            save_notes,
            read_blocked,
            write_blocked,
            write_blocked_with_password,
            read_domains,
            save_domains,
            get_blocking_status,
            update_tray_title
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
