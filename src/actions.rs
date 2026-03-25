use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;
use std::io;
use serde_json::Value;
use gtk::gdk;
use gtk::prelude::*;
use gtk::{ApplicationWindow, Button, Box, Label};
use std::rc::Rc;
use std::net::IpAddr;
use crate::wireguard_config::{WireguardConfig};

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct AppConfig {
    pub config_file: String,
}

impl ::std::default::Default for AppConfig {
    fn default() -> Self {
        Self {
            config_file: "".into(),
        }
    }
}

pub struct AppState {
    pub window: ApplicationWindow,
    pub vbox: Box,
    pub button: Button,
}

pub struct DialogState {
    pub window: gtk::Window,
    pub file_label: gtk::Label,
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

fn extract_domain(endpoint: &str) -> Option<String> {
    // First split host:port safely (IPv6 included)
    let host = endpoint
        .rsplit_once(':')
        .map(|(h, _)| h.trim_matches(['[', ']'].as_ref()))?;

    // Reject if it's an IP address
    if host.parse::<IpAddr>().is_ok() {
        return None;
    }

    if host.contains('.') && !host.starts_with('.') && !host.ends_with('.') {
        Some(host.to_string())
    } else {
        None
    }
}

fn get_autoguard_url(path: &str) -> Result<String,String> {
    let cfg = WireguardConfig::load(path)
        .map_err(|e| format!("Failed to read config: {e}"))?;

    let endpoint = cfg.get_peer_endpoint().unwrap();
    let domain = extract_domain(endpoint).unwrap();

    Ok(format!("https://auto.{}/routes.json",domain))
}

pub fn update_allowed_ips(state: &AppState) {

    // show busy cursor immediately
    set_busy_cursor(&state.window, true);

    let cfg: AppConfig = confy::load("autoguard", None).unwrap();
    let path_str = expand_home(&cfg.config_file).to_string_lossy().to_string();

    let autoguard_url = get_autoguard_url(&path_str).unwrap();

    // run blocking work later, after GTK repaints
    glib::idle_add_local({
        // clone widgets again for this closure
        let window_idle = state.window.clone();
        let vbox_idle = state.vbox.clone();
        let button_idle = state.button.clone();

        move || {
            let result = match fetch_allowed_ips(&autoguard_url) {
                Ok(allowed_ips) => update_peer_allowed_ips(&path_str, &allowed_ips),
                Err(e) => Err(format!("Fetch failed: {}", e)),
            };


            update_config(&path_str);

            set_busy_cursor(&window_idle, false);

            match result {
                Ok(_) => show_success(&vbox_idle, &button_idle),
                Err(e) => eprintln!("Error: {}", e),
            }

            glib::ControlFlow::Break
        }
    });


}

#[cfg(target_os = "macos")]
fn update_config(config_path: &str) {
    use std::process::Command;

    // Load config
    let cfg = WireguardConfig::load(config_path)
        .expect("Failed to load WireGuard config");
    let peer_pubkey = cfg.get_peer_public_key().expect("No peer in config");

    // Detect interface that has our peer
    let iface_output = Command::new("wg")
        .arg("show")
        .arg("interfaces")
        .output()
        .expect("Failed to run wg show interfaces");
    let iface_str = String::from_utf8_lossy(&iface_output.stdout);

    let interfaces: Vec<&str> = iface_str.lines().collect();
    let mut interface = None;
    for iface in interfaces {
        let dump = Command::new("wg")
            .args(["show", iface, "dump"])
            .output()
            .expect("Failed to run wg show dump");
        let dump_str = String::from_utf8_lossy(&dump.stdout);
        if dump_str.contains(&peer_pubkey) {
            interface = Some(iface);
            break;
        }
    }
    let interface = interface.expect("Could not find matching interface");

    // Fetch latest AllowedIPs
    let autoguard_url = get_autoguard_url(config_path)
        .expect("Failed to get AutoGuard URL");
    let allowed_ips = fetch_allowed_ips(&autoguard_url)
        .expect("Failed to fetch allowed IPs");

    // Update local config
    update_peer_allowed_ips(config_path, &allowed_ips)
        .expect("Failed to update local config");

    // Apply update to running interface
    let output = Command::new("wg")
        .args(["set", interface, "peer", &peer_pubkey, "allowed-ips", &allowed_ips])
        .output()
        .expect("Failed to run wg set");

    if !output.status.success() {
        eprintln!("wg set failed: {}", String::from_utf8_lossy(&output.stderr));
    }
}

#[cfg(target_os = "windows")]
fn update_config(config_path: &str) {

    Command::new("cmd")
    .args(&["/C", "wireguard", "/uninstalltunnelservice", "PW6"])
    .output()
    .unwrap();

    let _ = wait_for_service_stopped("Wireguard");

    Command::new("cmd")
    .args(&["/C", "wireguard", "/installtunnelservice", &config_path])
    .output()
    .unwrap();
}

#[cfg(target_os = "windows")]
fn wait_for_service_stopped(service_name: &str) -> io::Result<()> {
    let sleep_duration = Duration::from_millis(20);

    loop {
        let output = Command::new("sc")
            .arg("query")
            .arg(service_name)
            .output()?;

        if !output.status.success() {
            eprintln!("Error running 'sc query'");
            return Err(io::Error::new(io::ErrorKind::Other, "sc query failed"));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        for line in stdout.lines() {
            if line.trim_start().starts_with("STATE") {
                if let Some(pos) = line.find(':') {
                    let status_part = &line[pos+1..].trim();

                    if status_part.starts_with("1") && status_part.contains("STOPPED") {
                        return Ok(());
                    }
                }
            }
        }

        thread::sleep(sleep_duration);
    }
}

#[cfg(target_os = "linux")]
fn update_config(config_path: &str) {
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
            "nmcli connection import failed (exit code {:?}): {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let output = Command::new("nmcli")
        .arg("connection")
        .arg("modify")
        .arg(name)
        .arg("autoconnect")
        .arg("no")
        .output()
        .unwrap();

    if !output.status.success() {
        eprintln!(
            "nmcli connection modify autoconnect failed (exit code {:?}): {}",
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

pub fn show_settings_dialog(state: &AppState) {
    let builder = gtk::Builder::from_resource(
        "/com/autoguard/autoguard/ui/settings_dialog.ui"
    );

    let window: gtk::Window = builder.object("settings_window").unwrap();

    window.set_transient_for(Some(&state.window));
    window.set_modal(true);

    let choose_button: gtk::Button = builder.object("choose_file_button").unwrap();
    let file_label: gtk::Label = builder.object("file_label").unwrap();

    let dialog_state = Rc::new(DialogState {
        window: window.clone(),
        file_label: file_label.clone()
    });

    let cfg: AppConfig = confy::load("autoguard", None).unwrap();
    let saved_path = cfg.config_file.clone();

    if !saved_path.is_empty() {
        file_label.set_label(&saved_path);
    }

    choose_button.connect_clicked({
        let dialog_state = dialog_state.clone();
        move |_| choose_file(&dialog_state)
    });

    window.present();
}

pub fn choose_file(state: &DialogState) {
    let dialog = gtk::FileChooserNative::new(
        Some("Select a file"),
        Some(&state.window),
        gtk::FileChooserAction::Open,
        Some("Open"),
        Some("Cancel"),
    );

    dialog.connect_response({
        let label = state.file_label.clone();
        move |dialog, response| {
            if response == gtk::ResponseType::Accept {
                if let Some(file) = dialog.file() {
                    if let Some(path) = file.path() {
                        let path_str = path.to_string_lossy().to_string();
                        label.set_label(&path_str);

                        // Save to confy
                        let mut cfg: AppConfig = confy::load("autoguard", None).unwrap();
                        cfg.config_file = path_str.clone();
                        confy::store("autoguard", None, cfg).unwrap();
                    }
                }
            }
        }
    });

    dialog.show();
}
