use super::{Backend, BackendResult, PackageInfo};
use crate::settings::Settings;
use crate::utils::helpers::{self, check_command, run_as_root};
use std::sync::{Arc, Mutex};

pub struct SnapBackend {
    settings: Arc<Mutex<Settings>>,
}

impl SnapBackend {
    pub fn new(settings: Arc<Mutex<Settings>>) -> Self {
        Self { settings }
    }
}

impl Backend for SnapBackend {
    fn name(&self) -> &str { "Snap" }
    fn icon(&self) -> &str { "package-x-generic" }
    fn enabled(&self) -> bool {
        self.settings.lock().unwrap().snap_enabled && check_command("snap")
    }
    fn set_enabled(&self, val: bool) {
        self.settings.lock().unwrap().snap_enabled = val;
    }
    fn refresh(&self) {}

    fn search(&self, query: &str) -> Vec<PackageInfo> {
        if !self.enabled() { return vec![]; }
        let r = helpers::run_cmd(&["snap", "find", query], 60);
        if r.code != 0 { return vec![]; }
        r.stdout.lines().skip(1).filter_map(|line| {
            let parts: Vec<&str> = line.splitn(4, ' ').collect();
            if parts.len() >= 3 {
                Some(PackageInfo {
                    name: parts[0].to_string(), version: parts[1].to_string(),
                    description: if parts.len() > 3 { parts[3..].join(" ") } else { String::new() },
                    repo: parts[2].to_string(), backend: "Snap".into(), backend_key: "snap".into(),
                    ..PackageInfo::new("")
                })
            } else { None }
        }).collect()
    }

    fn install(&self, name: &str, version: Option<&str>) -> BackendResult {
        match version {
            Some(v) => {
                let r = run_as_root(&["snap", "install", name, "--channel", v], 300);
                (r.code, r.stdout, r.stderr)
            }
            None => {
                let r = run_as_root(&["snap", "install", name], 300);
                (r.code, r.stdout, r.stderr)
            }
        }
    }

    fn remove(&self, name: &str) -> BackendResult {
        let r = run_as_root(&["snap", "remove", name], 120);
        (r.code, r.stdout, r.stderr)
    }

    fn list_installed(&self) -> Vec<PackageInfo> {
        if !self.enabled() { return vec![]; }
        let r = helpers::run_cmd(&["snap", "list"], 30);
        if r.code != 0 { return vec![]; }
        r.stdout.lines().skip(1).filter_map(|line| {
            let parts: Vec<&str> = line.splitn(3, ' ').collect();
            if parts.len() >= 2 {
                Some(PackageInfo { name: parts[0].to_string(), version: parts[1].to_string(),
                    installed_version: parts[1].to_string(),
                    backend: "Snap".into(), backend_key: "snap".into(), ..PackageInfo::new("") })
            } else { None }
        }).collect()
    }

    fn check_updates(&self) -> Vec<PackageInfo> {
        if !self.enabled() { return vec![]; }
        let r = helpers::run_cmd(&["snap", "refresh", "--list"], 30);
        if r.code != 0 { return vec![]; }
        r.stdout.lines().skip(1).filter_map(|line| {
            let parts: Vec<&str> = line.splitn(3, ' ').collect();
            if parts.len() >= 3 {
                Some(PackageInfo { name: parts[0].to_string(), version: parts[2].to_string(),
                    installed_version: parts[1].to_string(),
                    backend: "Snap".into(), backend_key: "snap".into(), ..PackageInfo::new("") })
            } else { None }
        }).collect()
    }

    fn get_versions(&self, name: &str) -> Vec<String> {
        let r = helpers::run_cmd(&["snap", "info", name], 30);
        if r.code != 0 { return vec!["stable".to_string()]; }
        let mut versions = Vec::new();
        let mut in_channels = false;
        for line in r.stdout.lines() {
            if line.to_lowercase().contains("channels:") { in_channels = true; continue; }
            if in_channels && !line.trim().is_empty() {
                if let Some(v) = line.split(':').next() {
                    let v = v.trim().to_string();
                    if !v.is_empty() && !versions.contains(&v) { versions.push(v); }
                }
            } else if in_channels && line.trim().is_empty() { break; }
        }
        if versions.is_empty() { versions.push("stable".to_string()); }
        versions
    }

    fn get_info(&self, name: &str) -> PackageInfo {
        let mut pi = PackageInfo::new(name);
        pi.backend = "Snap".into(); pi.backend_key = "snap".into();
        let r = helpers::run_cmd(&["snap", "info", name], 30);
        if r.code == 0 {
            for line in r.stdout.lines() {
                if let Some((key, val)) = line.split_once(':') {
                    match key.trim() {
                        "version" => pi.version = val.trim().to_string(),
                        "description" => pi.description = val.trim().to_string(),
                        "license" => pi.license = val.trim().to_string(),
                        "contact" => pi.url = val.trim().to_string(),
                        _ => {}
                    }
                }
            }
        }
        pi
    }

    fn install_self(&self) -> BackendResult {
        let r = run_as_root(&["pacman", "-S", "--noconfirm", "snapd"], 300);
        if r.code == 0 {
            self.settings.lock().unwrap().snap_enabled = true;
            let _ = run_as_root(&["systemctl", "enable", "--now", "snapd.socket"], 60);
        }
        (r.code, r.stdout, r.stderr)
    }
}
