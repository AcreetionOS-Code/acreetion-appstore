use super::{Backend, BackendResult, PackageInfo};
use crate::settings::{AurHelper, Settings};
use crate::utils::helpers::{self, check_command};
use std::sync::{Arc, Mutex};

pub struct ParuBackend {
    settings: Arc<Mutex<Settings>>,
}

impl ParuBackend {
    pub fn new(settings: Arc<Mutex<Settings>>) -> Self {
        Self { settings }
    }

    fn build_flags(&self) -> Vec<String> {
        let s = self.settings.lock().unwrap();
        let mut flags = Vec::new();
        if !s.aur_build_dir.is_empty() {
            flags.push("--builddir".to_string());
            flags.push(s.aur_build_dir.clone());
        }
        if s.aur_clean_build {
            flags.push("--cleanbuild".to_string());
        }
        if s.aur_skip_review {
            flags.push("--skipreview".to_string());
        }
        flags
    }
}

impl Backend for ParuBackend {
    fn name(&self) -> &str { "AUR (paru)" }
    fn icon(&self) -> &str { "applications-engineering" }
    fn enabled(&self) -> bool {
        let s = self.settings.lock().unwrap();
        if !s.aur_enabled || !check_command("paru") { return false; }
        match s.aur_helper {
            AurHelper::Paru => true,
            AurHelper::Auto => true,
            AurHelper::Yay => false,
        }
    }
    fn set_enabled(&self, val: bool) {
        self.settings.lock().unwrap().aur_enabled = val;
    }

    fn refresh(&self) {
        let s = self.settings.lock().unwrap();
        if s.aur_refresh_on_sync {
            let _ = helpers::run_cmd(&["paru", "-Sy"], 120);
        }
    }

    fn search(&self, query: &str) -> Vec<PackageInfo> {
        if !self.enabled() { return vec![]; }
        let s = self.settings.lock().unwrap();
        let mut args = vec!["paru", "-Ss"];
        if s.aur_include_devel { args.push("--devel"); }
        if s.aur_sort_by_votes { args.push("--sortby"); args.push("votes"); }
        args.push(query);
        let r = helpers::run_cmd(&args, 180);
        drop(s);
        if r.code != 0 { return vec![]; }
        let mut results = Vec::new();
        let mut current: Option<PackageInfo> = None;
        for line in r.stdout.lines() {
            if line.starts_with(' ') && !line.trim().is_empty() {
                if let Some(ref mut p) = current { p.description = line.trim().to_string(); }
            } else if !line.is_empty() {
                if let Some(p) = current.take() { results.push(p); }
                if let Some((repo, rest)) = line.split_once('/') {
                    let parts: Vec<&str> = rest.splitn(2, ' ').collect();
                    if !parts.is_empty() {
                        current = Some(PackageInfo {
                            name: parts[0].to_string(),
                            version: if parts.len() > 1 { parts[1].trim().to_string() } else { String::new() },
                            repo: repo.to_string(), backend: "AUR (paru)".into(), backend_key: "paru".into(),
                            ..PackageInfo::new("")
                        });
                    }
                }
            }
        }
        if let Some(p) = current { results.push(p); }
        results
    }

    fn install(&self, name: &str, version: Option<&str>) -> BackendResult {
        let flags = self.build_flags();
        let mut args = vec!["paru", "-S", "--noconfirm"];
        for f in &flags {
            args.push(f.as_str());
        }
        match version {
            Some(v) => {
                let pkg = format!("{}={}", name, v);
                args.push(pkg.as_str());
                let r = helpers::run_cmd(&args, 300);
                (r.code, r.stdout, r.stderr)
            }
            None => {
                args.push(name);
                let r = helpers::run_cmd(&args, 300);
                (r.code, r.stdout, r.stderr)
            }
        }
    }

    fn remove(&self, name: &str) -> BackendResult {
        let r = helpers::run_cmd(&["paru", "-R", "--noconfirm", name], 120);
        (r.code, r.stdout, r.stderr)
    }

    fn list_installed(&self) -> Vec<PackageInfo> {
        if !self.enabled() { return vec![]; }
        let r = helpers::run_cmd(&["paru", "-Q"], 60);
        if r.code != 0 { return vec![]; }
        r.stdout.lines().filter_map(|line| {
            let mut parts = line.splitn(2, ' ');
            let name = parts.next()?.to_string();
            let ver = parts.next().unwrap_or("").to_string();
            Some(PackageInfo { name, version: ver.clone(), installed_version: ver,
                backend: "AUR (paru)".into(), backend_key: "paru".into(), ..PackageInfo::new("") })
        }).collect()
    }

    fn check_updates(&self) -> Vec<PackageInfo> {
        if !self.enabled() { return vec![]; }
        let s = self.settings.lock().unwrap();
        if !s.aur_check_updates { return vec![]; }
        let mut args = vec!["paru", "-Qu"];
        if s.aur_include_devel { args.push("--devel"); }
        let r = helpers::run_cmd(&args, 120);
        if r.code != 0 && r.code != 1 { return vec![]; }
        r.stdout.lines().filter_map(|line| {
            let parts: Vec<&str> = line.splitn(3, ' ').collect();
            if parts.len() >= 2 {
                Some(PackageInfo {
                    name: parts[0].to_string(), version: parts[1].to_string(),
                    installed_version: parts.get(2).unwrap_or(&"").trim().to_string(),
                    backend: "AUR (paru)".into(), backend_key: "paru".into(), ..PackageInfo::new("") })
            } else { None }
        }).collect()
    }

    fn get_versions(&self, name: &str) -> Vec<String> {
        let r = helpers::run_cmd(&["paru", "-Si", name], 30);
        if r.code != 0 { return vec![]; }
        for line in r.stdout.lines() {
            if let Some((key, val)) = line.split_once(':') {
                if key.trim() == "Version" { return vec![val.trim().to_string()]; }
            }
        }
        vec![]
    }

    fn get_info(&self, name: &str) -> PackageInfo {
        let mut pi = PackageInfo::new(name);
        pi.backend = "AUR (paru)".into(); pi.backend_key = "paru".into();
        let r = helpers::run_cmd(&["paru", "-Qi", name], 30);
        let stdout = if r.code == 0 { r.stdout } else {
            let r2 = helpers::run_cmd(&["paru", "-Si", name], 30);
            if r2.code == 0 { r2.stdout } else { return pi; }
        };
        for line in stdout.lines() {
            if let Some((key, val)) = line.split_once(':') {
                match key.trim() {
                    "Version" => pi.version = val.trim().to_string(),
                    "Installed Size" => pi.size = val.trim().to_string(),
                    "Description" => pi.description = val.trim().to_string(),
                    "URL" => pi.url = val.trim().to_string(),
                    "Licenses" => pi.license = val.trim().to_string(),
                    "Repository" => pi.repo = val.trim().to_string(),
                    _ => {}
                }
            }
        }
        pi
    }

    fn install_self(&self) -> BackendResult {
        let r = helpers::run_cmd(&["pacman", "-S", "--noconfirm", "paru"], 300);
        if r.code == 0 { self.settings.lock().unwrap().aur_enabled = true; }
        (r.code, r.stdout, r.stderr)
    }
}
