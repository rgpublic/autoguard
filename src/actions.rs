use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use serde_json::Value;
use gtk::gdk;
use gtk::prelude::*;
use gtk::{ApplicationWindow, Button, Box, Label};
use crate::wireguard_config::{WireguardConfig};

pub struct AppState {
    pub window: ApplicationWindow,
    pub vbox: Box,
    pub button: Button,
    pub config_path: String
}

fn set_busy_cursor(window: &ApplicationWindow, busy: bool) {
    if let Some(surface) = window.surface() {
        if busy {
            if let Some(cursor) = gdk::Cursor::from_name("wait", None) {
                surface.set_cursor(Some(&cursor));
            }
        } else {
            surface.set_cursor(None);
        }
    }
}

fn expand_home(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped);
        }
    }
    PathBuf::from(path)
}

fn update_peer_allowed_ips(path: &str, new_ips: &str) -> Result<String, String> {
    let mut cfg = WireguardConfig::load(path)
        .map_err(|e| format!("Failed to read config: {e}"))?;

    cfg.set_peer_allowed_ips(new_ips);

    cfg.save(path)
        .map_err(|e| format!("Failed to save config: {e}"))?;

    Ok(format!("Updated AllowedIPs to {new_ips} in {path}"))
}

fn fetch_allowed_ips(url: &str) -> Result<String, String> {
    let resp = reqwest::blocking::get(url)
        .map_err(|e| format!("Failed to fetch {url}: {e}"))?;

    let json: Value = resp.json()
        .map_err(|e| format!("Failed to parse JSON: {e}"))?;

    // Expect an object, collect all values
    let obj = json.as_object()
        .ok_or("JSON is not an object".to_string())?;

    let ips: Vec<String> = obj.values()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();

    Ok(ips.join(","))
}

pub fn update_allowed_ips(state: &AppState) {

    // show busy cursor immediately
    set_busy_cursor(&state.window, true);

    let path_str = expand_home(&state.config_path).to_string_lossy().to_string();

    // run blocking work later, after GTK repaints
    glib::idle_add_local({
        // clone widgets again for this closure
        let window_idle = state.window.clone();
        let vbox_idle = state.vbox.clone();
        let button_idle = state.button.clone();
        let config_path = state.config_path.clone();

        move || {
            let result = match fetch_allowed_ips("https://routing.pw6.de/routes.json") {
                Ok(allowed_ips) => update_peer_allowed_ips(&path_str, &allowed_ips),
                Err(e) => Err(format!("Fetch failed: {}", e)),
            };

            restart_network_manager(&config_path);

            set_busy_cursor(&window_idle, false);

            match result {
                Ok(_) => show_success(&vbox_idle, &button_idle),
                Err(e) => eprintln!("Error: {}", e),
            }

            glib::ControlFlow::Break
        }
    });


}

#[cfg(target_os = "linux")]
fn restart_network_manager(config_path: &str) {
    let name = Path::new(config_path)
        .file_stem()
        .unwrap();

    let _output = Command::new("nmcli")
        .arg("connection")
        .arg("delete")
        .arg(name)
        .output();

    let path_str = expand_home(config_path).to_string_lossy().to_string();

    let output = Command::new("nmcli")
        .arg("connection")
        .arg("import")
        .arg("type")
        .arg("wireguard")
        .arg("file")
        .arg(path_str)
        .output()
        .unwrap();

    if !output.status.success() {
        eprintln!(
            "nmcli failed (exit code {:?}): {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        );
    }

}

fn show_success(vbox: &Box, button: &Button) {
    let label = Label::new(Some("✅ WireGuard config updated successfully."));
    vbox.remove(button);   // remove the button
    vbox.append(&label);   // add the label
}
