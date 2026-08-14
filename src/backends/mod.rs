pub mod pacman;
pub mod yay;
pub mod paru;
pub mod flatpak;
pub mod snap;
pub mod appimage;

use crate::settings::Settings;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub type BackendResult = (i32, String, String);

#[derive(Clone, Debug)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub repo: String,
    pub backend: String,
    pub backend_key: String,
    pub size: String,
    pub url: String,
    pub license: String,
    pub installed_version: String,
    pub available_versions: Vec<String>,
}

impl PackageInfo {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            version: String::new(),
            description: String::new(),
            repo: String::new(),
            backend: String::new(),
            backend_key: String::new(),
            size: String::new(),
            url: String::new(),
            license: String::new(),
            installed_version: String::new(),
            available_versions: Vec::new(),
        }
    }

    pub fn is_installed(&self) -> bool {
        !self.installed_version.is_empty()
    }

    pub fn has_update(&self) -> bool {
        self.is_installed() && self.installed_version != self.version
    }
}

pub trait Backend: Send + Sync {
    fn name(&self) -> &str;
    fn icon(&self) -> &str;
    fn enabled(&self) -> bool;
    fn set_enabled(&self, val: bool);
    fn refresh(&self);
    fn search(&self, query: &str) -> Vec<PackageInfo>;
    fn install(&self, name: &str, version: Option<&str>) -> BackendResult;
    fn remove(&self, name: &str) -> BackendResult;
    fn list_installed(&self) -> Vec<PackageInfo>;
    fn check_updates(&self) -> Vec<PackageInfo>;
    fn get_versions(&self, name: &str) -> Vec<String>;
    fn get_info(&self, name: &str) -> PackageInfo;
    fn install_self(&self) -> BackendResult;
    /// Directory AppImage downloads land in (empty string = not applicable).
    fn primary_download_dir(&self) -> String { String::new() }
}

pub struct BackendManager {
    pub backends: HashMap<String, Box<dyn Backend>>,
    pub settings: Arc<Mutex<Settings>>,
    /// The AUR helper resolved from settings + what's installed.
    pub aur_helper: Mutex<Option<crate::settings::AurHelper>>,
}

impl BackendManager {
    pub fn new(settings: Arc<Mutex<Settings>>) -> Self {
        let mut bm = Self {
            backends: HashMap::new(),
            aur_helper: Mutex::new(None),
            settings: settings.clone(),
        };
        bm.backends.insert("pacman".into(), Box::new(pacman::PacmanBackend::new(settings.clone())));
        bm.backends.insert("yay".into(), Box::new(yay::YayBackend::new(settings.clone())));
        bm.backends.insert("paru".into(), Box::new(paru::ParuBackend::new(settings.clone())));
        bm.backends.insert("flatpak".into(), Box::new(flatpak::FlatpakBackend::new(settings.clone())));
        bm.backends.insert("snap".into(), Box::new(snap::SnapBackend::new(settings.clone())));
        bm.backends.insert("appimage".into(), Box::new(appimage::AppImageBackend::new(settings.clone())));
        bm.refresh_aur_helper();
        bm
    }

    /// Recompute which AUR helper (if any) should be active, from settings + binaries.
    pub fn refresh_aur_helper(&self) {
        let s = self.settings.lock().unwrap();
        let helper = s.resolved_aur_helper(
            crate::utils::helpers::check_command("yay"),
            crate::utils::helpers::check_command("paru"),
        );
        *self.aur_helper.lock().unwrap() = helper;
    }

    pub fn enabled_backends(&self) -> Vec<(&str, &dyn Backend)> {
        self.backends.iter()
            .filter(|(_, b)| b.enabled())
            .map(|(k, b)| (k.as_str(), b.as_ref()))
            .collect()
    }

    pub fn find_backend(&self, pkg: &PackageInfo) -> Option<&dyn Backend> {
        for (key, backend) in &self.backends {
            if backend.enabled() && (backend.name() == pkg.backend || key == &pkg.backend_key) {
                return Some(backend.as_ref());
            }
        }
        self.backends.get(&pkg.backend_key).map(|b| b.as_ref())
    }

    pub fn search_all(&self, query: &str) -> Vec<PackageInfo> {
        let mut results = Vec::new();
        for (_, backend) in &self.backends {
            if backend.enabled() {
                results.extend(backend.search(query));
            }
        }
        results
    }

    pub fn refresh_all(&self) {
        for (_, backend) in &self.backends {
            if backend.enabled() {
                backend.refresh();
            }
        }
    }
}
