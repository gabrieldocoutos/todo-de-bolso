use std::process::Command;
use tauri::Manager;

const BLOCK_START: &str = "### nelson";
const BLOCK_END: &str = "### nelson end";
const HOSTS_PATH: &str = "/etc/hosts";
const HOSTS_TMP: &str = "/tmp/nelson_hosts_tmp";

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

fn write_hosts_with_sudo(content: &str) -> Result<(), String> {
    std::fs::write(HOSTS_TMP, content).map_err(|e| e.to_string())?;
    let script = format!(
        r#"do shell script "cp {} {}" with administrator privileges"#,
        HOSTS_TMP, HOSTS_PATH
    );
    let output = Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        if stderr.contains("User canceled") || stderr.contains("(-128)") {
            return Err("Cancelled".into());
        }
        return Err(stderr);
    }
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
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            load_notes,
            save_notes,
            read_blocked,
            write_blocked
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
