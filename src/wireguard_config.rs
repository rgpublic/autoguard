use std::collections::HashMap;
use std::io::{self};
use std::fs;

#[derive(Debug)]
struct Section {
    name: String,
    kv: HashMap<String, String>,
}

#[derive(Debug)]
pub struct WireguardConfig {
    sections: Vec<Section>,
}

impl WireguardConfig {
    pub fn load(path: &str) -> io::Result<Self> {
        let contents = fs::read_to_string(path)?;
        let mut sections = Vec::new();
        let mut current: Option<Section> = None;

        for line in contents.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                if let Some(sec) = current.take() {
                    sections.push(sec);
                }
                current = Some(Section {
                    name: trimmed.trim_matches(&['[', ']'][..]).to_string(),
                    kv: HashMap::new(),
                });
            } else if let Some(sec) = current.as_mut() {
                if let Some((k, v)) = trimmed.split_once('=') {
                    sec.kv.insert(k.trim().to_string(), v.trim().to_string());
                }
            }
        }
        if let Some(sec) = current.take() {
            sections.push(sec);
        }

        Ok(Self { sections })
    }

    pub fn save(&self, path: &str) -> io::Result<()> {
        let mut out = String::new();
        for sec in &self.sections {
            out.push_str(&format!("[{}]\n", sec.name));
            for (k, v) in &sec.kv {
                out.push_str(&format!("{} = {}\n", k, v));
            }
            out.push('\n');
        }
        fs::write(path, out)
    }

    pub fn set_peer_allowed_ips(&mut self, ips: &str) {
        for sec in &mut self.sections {
            if sec.name == "Peer" {
                sec.kv.insert("AllowedIPs".to_string(), ips.to_string());
            }
        }
    }
}
