#![allow(deprecated)]

use crate::settings::{Settings, Theme};
use crate::utils::helpers;
use gtk4::prelude::*;
use gtk4::{
    self, Application, ApplicationWindow, Button, ButtonsType, CheckButton, Dialog, Entry,
    Expander, HeaderBar, Label, ListBox, ListBoxRow, MessageDialog, MessageType, Notebook,
    Paned, PasswordEntry, ProgressBar, ResponseType, ScrolledWindow, SearchEntry, Separator,
    TextBuffer, TextView,
};
use glib::{self, ControlFlow, Object as GtkObject};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::backends::BackendManager;
use super::package_detail::PackageDetail;
use super::package_list::PackageList;
use super::settings_dialog::SettingsDialog;

const BACKEND_ORDER: [&str; 6] = ["pacman", "yay", "paru", "flatpak", "snap", "appimage"];

const GRID_CSS: &str = "
.appstore-card {
    background-color: alpha(@theme_bg_color, 0.65);
    border: 1px solid alpha(@theme_fg_color, 0.12);
    border-radius: 12px;
    padding: 10px;
}
.appstore-card:hover {
    background-color: alpha(@theme_selected_bg_color, 0.18);
}
";

#[derive(Clone)]
struct AppState {
    manager: Arc<BackendManager>,
    settings: Arc<Mutex<Settings>>,
    package_list: Arc<Mutex<PackageList>>,
    detail_view: Arc<PackageDetail>,
    status_label: Label,
    progress_bar: ProgressBar,
    output_buffer: TextBuffer,
    output_expander: Expander,
    notebook: Notebook,
    sidebar_map: Arc<Mutex<HashMap<usize, String>>>,
    current_category: Arc<Mutex<String>>,
}

impl AppState {
    fn set_progress(&self, active: bool) {
        self.progress_bar.set_visible(active);
        if active {
            self.progress_bar.pulse();
        }
    }

    fn append_output(&self, text: &str) {
        let mut end = self.output_buffer.end_iter();
        self.output_buffer.insert(&mut end, text);
        let mut end = self.output_buffer.end_iter();
        self.output_buffer.insert(&mut end, "\n");
        self.output_expander.set_visible(true);
        if self.settings.lock().unwrap().show_output_expander {
            self.output_expander.set_expanded(true);
        }
    }

    fn clear_output(&self) {
        self.output_buffer.set_text("");
        self.output_expander.set_visible(false);
    }

    fn show_error(&self, title: &str, msg: &str) {
        let dialog = MessageDialog::builder()
            .message_type(MessageType::Error)
            .text(title)
            .secondary_text(msg)
            .buttons(ButtonsType::Ok)
            .modal(true)
            .build();
        dialog.connect_response(|d, _| d.close());
        dialog.present();
    }
}

pub struct AppWindow;

impl AppWindow {
    pub fn new(app: &Application, settings: Arc<Mutex<Settings>>) {
        Self::ensure_css();

        let window = ApplicationWindow::builder()
            .application(app)
            .title("AcreetionOS AppStore")
            .default_width(1024)
            .default_height(768)
            .build();

        // Restore remembered window size
        {
            let s = settings.lock().unwrap();
            if s.remember_window_size && s.window_width >= 640 && s.window_height >= 480 {
                window.set_default_size(s.window_width, s.window_height);
            }
            helpers::set_command_timeout_cap(s.command_timeout);
            Self::apply_theme(&s);
        }

        // Persist window size on close
        let settings_c = settings.clone();
        window.connect_close_request(move |w| {
            let s = settings_c.lock().unwrap();
            if s.remember_window_size {
                let (w_, h_) = w.default_size();
                drop(s);
                let mut s2 = settings_c.lock().unwrap();
                s2.window_width = w_;
                s2.window_height = h_;
                s2.save();
            }
            glib::Propagation::Proceed
        });

        let header = HeaderBar::new();
        let title = Label::builder()
            .label("AcreetionOS AppStore")
            .css_classes(["title-2"])
            .build();
        header.set_title_widget(Some(&title));

        let search_entry = SearchEntry::builder()
            .placeholder_text("Search...")
            .build();
        header.pack_start(&search_entry);

        let refresh_btn = Button::from_icon_name("view-refresh");
        let gear_btn = Button::from_icon_name("preferences-system");
        header.pack_end(&gear_btn);
        header.pack_end(&refresh_btn);
        window.set_titlebar(Some(&header));

        let main_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        let paned = Paned::new(gtk4::Orientation::Horizontal);
        let sidebar_sw = ScrolledWindow::builder().width_request(220).build();
        let sidebar_list = ListBox::new();
        sidebar_sw.set_child(Some(&sidebar_list));
        paned.set_start_child(Some(&sidebar_sw));
        paned.set_resize_start_child(false);

        let right_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        let notebook = Notebook::builder()
            .show_tabs(false)
            .vexpand(true)
            .build();
        let package_list = Arc::new(Mutex::new(PackageList::new()));
        notebook.append_page(
            &package_list.lock().unwrap().scrolled,
            Some(&Label::new(Some("List"))),
        );

        let detail_view = Arc::new(PackageDetail::new());
        notebook.append_page(
            &detail_view.scrolled,
            Some(&Label::new(Some("Detail"))),
        );

        right_box.append(&notebook);
        paned.set_end_child(Some(&right_box));
        main_box.append(&paned);

        let op_box = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
        let progress_bar = ProgressBar::builder().visible(false).build();
        op_box.append(&progress_bar);

        let output_expander = Expander::builder()
            .label("Output")
            .visible(false)
            .build();
        let out_sw = ScrolledWindow::builder().min_content_height(100).build();
        let output_buffer = TextBuffer::new(None);
        let output_view = TextView::builder()
            .editable(false)
            .buffer(&output_buffer)
            .build();
        out_sw.set_child(Some(&output_view));
        output_expander.set_child(Some(&out_sw));
        op_box.append(&output_expander);
        main_box.append(&op_box);

        let status_bar = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        status_bar.set_css_classes(&["statusbar"]);
        let status_label = Label::new(None);
        status_bar.append(&status_label);
        main_box.append(&status_bar);

        window.set_child(Some(&main_box));

        let default_cat = state_default_category(&settings);

        let state = AppState {
            manager: Arc::new(BackendManager::new(settings.clone())),
            settings,
            package_list,
            detail_view,
            status_label,
            progress_bar,
            output_buffer,
            output_expander,
            notebook,
            sidebar_map: Arc::new(Mutex::new(HashMap::new())),
            current_category: Arc::new(Mutex::new(default_cat)),
        };

        Self::rebuild_sidebar(&sidebar_list, &state);
        Self::connect_signals(
            &window,
            &sidebar_list,
            &search_entry,
            &refresh_btn,
            &gear_btn,
            state.clone(),
        );

        // Initial load
        let s = state.clone();
        glib::spawn_future_local(async move {
            Self::load_packages(s).await;
        });

        // Launch update check
        let s = state.clone();
        glib::spawn_future_local(async move {
            if s.settings.lock().unwrap().check_updates_on_launch {
                Self::check_for_updates(s).await;
            }
        });

        // Background update-check timer
        Self::schedule_update_timer(state);

        window.present();
    }

    fn ensure_css() {
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| {
            if let Some(display) = gtk4::gdk::Display::default() {
                let provider = gtk4::CssProvider::new();
                provider.load_from_data(GRID_CSS);
                gtk4::StyleContext::add_provider_for_display(
                    &display,
                    &provider,
                    gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
                );
            }
        });
    }

    fn apply_theme(s: &Settings) {
        if let Some(gtk_settings) = gtk4::Settings::default() {
            gtk_settings.set_gtk_application_prefer_dark_theme(s.theme == Theme::Dark);
        }
    }

    fn schedule_update_timer(state: AppState) {
        let secs = match state.settings.lock().unwrap().update_check_frequency {
            crate::settings::UpdateFrequency::Never => return,
            crate::settings::UpdateFrequency::EveryHour => 3600,
            crate::settings::UpdateFrequency::Every6Hours => 6 * 3600,
            crate::settings::UpdateFrequency::Daily => 24 * 3600,
        };
        glib::timeout_add_local(std::time::Duration::from_secs(secs), move || {
            let s = state.clone();
            glib::spawn_future_local(async move {
                Self::check_for_updates(s).await;
            });
            ControlFlow::Continue
        });
    }

    fn rebuild_sidebar(sidebar: &ListBox, state: &AppState) {
        while let Some(child) = sidebar.first_child() {
            sidebar.remove(&child);
        }
        let mut map = HashMap::new();

        let add_row = |sidebar: &ListBox,
                           map: &mut HashMap<usize, String>,
                           id: &str,
                           label: &str,
                           icon: &str| {
            let row = ListBoxRow::new();
            let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
            hbox.set_margin_start(8);
            hbox.set_margin_end(8);
            hbox.set_margin_top(6);
            hbox.set_margin_bottom(6);
            hbox.append(&gtk4::Image::from_icon_name(icon));
            hbox.append(&Label::builder().label(label).xalign(0.0).build());
            row.set_child(Some(&hbox));
            map.insert(row.entity_id(), id.to_string());
            sidebar.append(&row);
        };

        add_row(sidebar, &mut map, "installed", "Installed", "emblem-system");
        add_row(sidebar, &mut map, "updates", "Updates Available", "software-update-available");
        sidebar.append(&Separator::new(gtk4::Orientation::Horizontal));

        for key in BACKEND_ORDER {
            if let Some(backend) = state.manager.backends.get(key) {
                if backend.enabled() {
                    add_row(sidebar, &mut map, key, backend.name(), backend.icon());
                }
            }
        }

        sidebar.append(&Separator::new(gtk4::Orientation::Horizontal));

        // Settings entry
        {
            let row = ListBoxRow::new();
            let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
            hbox.set_margin_start(8);
            hbox.set_margin_end(8);
            hbox.set_margin_top(6);
            hbox.set_margin_bottom(6);
            hbox.append(&gtk4::Image::from_icon_name("preferences-system"));
            hbox.append(&Label::builder().label("Preferences").xalign(0.0).build());
            row.set_child(Some(&hbox));
            map.insert(row.entity_id(), "settings".to_string());
            sidebar.append(&row);
        }

        // AUR toggle
        sidebar.append(&Self::aur_toggle_row(state));

        *state.sidebar_map.lock().unwrap() = map;
    }

    fn aur_toggle_row(state: &AppState) -> ListBoxRow {
        let row = ListBoxRow::new();
        row.set_activatable(false);
        row.set_selectable(false);
        let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        hbox.set_margin_start(8);
        hbox.set_margin_end(8);
        hbox.set_margin_top(6);
        hbox.set_margin_bottom(6);

        let check = CheckButton::builder()
            .label("Enable AUR support")
            .active(state.settings.lock().unwrap().aur_enabled)
            .tooltip_text("Search and install packages from the Arch User Repository")
            .build();

        let s = state.clone();
        check.connect_toggled(move |c| {
            let on = c.is_active();
            {
                let mut settings = s.settings.lock().unwrap();
                settings.aur_enabled = on;
                settings.save();
            }
            s.manager.refresh_aur_helper();
            let s_clone = s.clone();
            glib::spawn_future_local(async move {
                Self::load_packages(s_clone).await;
            });
        });

        hbox.append(&check);
        row.set_child(Some(&hbox));
        row
    }

    fn open_settings_dialog(sidebar: &ListBox, window: &ApplicationWindow, state: AppState) {
        let sidebar_ref = sidebar.clone();
        let window_ref = window.clone();
        let st = state.clone();
        let on_saved = Box::new(move || {
            {
                let s = st.settings.lock().unwrap();
                Self::apply_theme(&s);
                helpers::set_command_timeout_cap(s.command_timeout);
            }
            st.manager.refresh_aur_helper();
            Self::rebuild_sidebar(&sidebar_ref, &st);
            let st2 = st.clone();
            glib::spawn_future_local(async move {
                Self::load_packages(st2).await;
            });
        });
        let parent: Option<gtk4::Window> = Some(window_ref.clone().upcast());
        SettingsDialog::show(parent.as_ref(), state.settings.clone(), on_saved);
    }

    fn connect_signals(
        window: &ApplicationWindow,
        sidebar: &ListBox,
        search: &SearchEntry,
        refresh: &Button,
        gear: &Button,
        state: AppState,
    ) {
        // Sidebar category / settings row activation
        let s = state.clone();
        let sidebar_clone = sidebar.clone();
        let window_clone = window.clone();
        sidebar.connect_row_activated(move |_, row| {
            let id = s
                .sidebar_map
                .lock()
                .unwrap()
                .get(&row.entity_id())
                .cloned();
            if let Some(id) = id {
                if id == "settings" {
                    Self::open_settings_dialog(&sidebar_clone, &window_clone, s.clone());
                    return;
                }
                *s.current_category.lock().unwrap() = id;
                let s_clone = s.clone();
                glib::spawn_future_local(async move {
                    Self::load_packages(s_clone).await;
                });
            }
        });

        let s = state.clone();
        search.connect_search_changed(move |entry| {
            let query = entry.text().to_string();
            let s_clone = s.clone();
            glib::spawn_future_local(async move {
                if !query.is_empty() {
                    Self::show_search_results(s_clone, &query).await;
                } else {
                    Self::load_packages(s_clone).await;
                }
            });
        });

        let s = state.clone();
        refresh.connect_clicked(move |_| {
            let s_clone = s.clone();
            glib::spawn_future_local(async move {
                s_clone.status_label.set_text("Refreshing...");
                let manager = s_clone.manager.clone();
                let s2 = s_clone.settings.clone();
                let refresh_db = s2.lock().unwrap().refresh_databases_before_update;
                let fut = async_std::task::spawn_blocking(move || {
                    if refresh_db {
                        manager.refresh_all();
                    } else {
                        for (_, b) in manager.enabled_backends() {
                            if b.name() != "pacman" && !b.name().starts_with("AUR") {
                                b.refresh();
                            }
                        }
                    }
                });
                fut.await;
                s_clone.status_label.set_text("Refresh complete.");
                Self::load_packages(s_clone).await;
            });
        });

        // Gear button → settings
        let s = state.clone();
        let sidebar_g = sidebar.clone();
        let window_g = window.clone();
        gear.connect_clicked(move |_| {
            Self::open_settings_dialog(&sidebar_g, &window_g, s.clone());
        });

        // List + grid row activation → detail view
        let s = state.clone();
        state
            .package_list
            .lock()
            .unwrap()
            .listbox
            .connect_row_activated(move |_, row| {
                let s_clone = s.clone();
                let idx = row.index();
                if idx >= 0 {
                    Self::open_package_detail(s_clone, idx as usize);
                }
            });

        let s = state.clone();
        state
            .package_list
            .lock()
            .unwrap()
            .flowbox
            .connect_child_activated(move |_, child| {
                let s_clone = s.clone();
                let idx = child.index();
                if idx >= 0 {
                    Self::open_package_detail(s_clone, idx as usize);
                }
            });

        let s = state.clone();
        state.detail_view.back_btn.connect_clicked(move |_| {
            s.notebook.set_current_page(Some(0));
        });

        let s = state.clone();
        state.detail_view.install_btn.connect_clicked(move |_| {
            let s_clone = s.clone();
            let pkg_opt = s_clone.detail_view.current_pkg.lock().unwrap().clone();
            if let Some(pkg) = pkg_opt {
                if s_clone.settings.lock().unwrap().confirm_before_install {
                    Self::confirm_then_install(s_clone, pkg);
                } else {
                    glib::spawn_future_local(Self::request_install(s_clone, pkg));
                }
            }
        });

        let s = state.clone();
        state.detail_view.remove_btn.connect_clicked(move |_| {
            let s_clone = s.clone();
            let pkg_opt = s_clone.detail_view.current_pkg.lock().unwrap().clone();
            if let Some(pkg) = pkg_opt {
                glib::spawn_future_local(Self::request_remove(s_clone, pkg));
            }
        });
    }

    fn open_package_detail(state: AppState, idx: usize) {
        if let Some(pkg) = state.package_list.lock().unwrap().get_package(idx) {
            let manager = state.manager.clone();
            let detail = state.detail_view.clone();
            let notebook = state.notebook.clone();
            let pkg_name = pkg.name.clone();
            let backend_key = pkg.backend_key.clone();
            glib::spawn_future_local(async move {
                let versions = async_std::task::spawn_blocking(move || {
                    manager
                        .backends
                        .get(&backend_key)
                        .map(|b| b.get_versions(&pkg_name))
                        .unwrap_or_default()
                })
                .await;
                detail.show_package(pkg, versions);
                notebook.set_current_page(Some(1));
            });
        }
    }

    fn confirm_then_install(state: AppState, pkg: crate::backends::PackageInfo) {
        let confirm = MessageDialog::builder()
            .message_type(MessageType::Question)
            .text("Install Package")
            .secondary_text(&format!("Are you sure you want to install {}?", pkg.name))
            .buttons(ButtonsType::OkCancel)
            .modal(true)
            .build();

        let s = state.clone();
        let pkg_clone = pkg.clone();
        confirm.connect_response(move |d, resp| {
            d.close();
            if resp == ResponseType::Ok {
                let s_clone = s.clone();
                glib::spawn_future_local(Self::request_install(s_clone, pkg_clone.clone()));
            }
        });

        confirm.present();
    }

    async fn request_install(state: AppState, pkg: crate::backends::PackageInfo) {
        if pkg.backend_key == "appimage" {
            Self::request_appimage_install(state);
            return;
        }

        let needs_sudo = matches!(pkg.backend_key.as_str(), "pacman" | "flatpak" | "snap");

        if needs_sudo && !helpers::has_sudo_password() {
            Self::run_install_with_password(state, pkg);
        } else {
            Self::run_install(state, pkg).await;
        }
    }

    fn request_appimage_install(state: AppState) {
        let dialog = Dialog::builder()
            .title("Download AppImage")
            .modal(true)
            .build();

        let content = dialog.content_area();
        content.set_margin_start(12);
        content.set_margin_end(12);
        content.set_margin_top(12);
        content.set_margin_bottom(12);

        let label = Label::new(Some("Enter the download URL for the AppImage:"));
        content.append(&label);

        let entry = Entry::builder()
            .placeholder_text("https://example.com/app.AppImage")
            .margin_top(8)
            .build();
        content.append(&entry);

        dialog.add_button("Cancel", ResponseType::Cancel);
        dialog.add_button("Download", ResponseType::Ok);

        let s = state.clone();
        dialog.connect_response(move |d, resp| {
            if resp == ResponseType::Ok {
                let url = entry.text().to_string();
                if !url.is_empty() {
                    let s_clone = s.clone();
                    glib::spawn_future_local(Self::download_appimage_file(s_clone, url));
                }
            }
            d.close();
        });

        dialog.present();
    }

    async fn download_appimage_file(state: AppState, url: String) {
        state.clear_output();
        state.set_progress(true);
        state.append_output(&format!("Downloading AppImage from {}", url));
        state.status_label.set_text("Downloading AppImage...");

        let manager = state.manager.clone();
        let app_url = url.clone();
        let result = async_std::task::spawn_blocking(move || {
            let apps_dir = manager
                .backends
                .get("appimage")
                .map(|b| b.primary_download_dir())
                .filter(|d| !d.is_empty())
                .unwrap_or_else(|| {
                    format!("{}/Applications", std::env::var("HOME").unwrap_or_default())
                });
            let _ = std::fs::create_dir_all(&apps_dir);

            let filename = std::path::Path::new(&app_url)
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("downloaded.AppImage");
            let dest = format!("{}/{}", apps_dir, filename);

            let r = helpers::run_cmd(&["curl", "-L", "-o", &dest, "--progress-bar", &app_url], 600);

            if r.code != 0 {
                let r2 = helpers::run_cmd(&["wget", "-O", &dest, &app_url], 600);
                if r2.code != 0 {
                    return (1, String::new(), format!("Download failed: {}", r2.stderr));
                }
            }

            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = std::fs::metadata(&dest) {
                let mut perms = meta.permissions();
                perms.set_mode(0o755);
                let _ = std::fs::set_permissions(&dest, perms);
            }

            // Ask the AppImage backend to register the new file with the daemon
            if let Some(b) = manager.backends.get("appimage") {
                b.refresh();
            }

            (0, format!("Downloaded {} to {}", filename, dest), String::new())
        }).await;

        state.set_progress(false);

        if result.0 == 0 {
            state.append_output(&result.1);
            state.status_label.set_text("AppImage installed successfully");
        } else {
            state.append_output(&format!("Error: {}", result.2));
            state.status_label.set_text("Failed to download AppImage");
            state.show_error("Download Failed", &result.2);
        }

        let s = state.clone();
        Self::load_packages(s).await;
    }

    fn run_install_with_password(state: AppState, pkg: crate::backends::PackageInfo) {
        let dialog = Dialog::builder()
            .title("Authentication Required")
            .modal(true)
            .build();

        let content = dialog.content_area();
        content.set_margin_start(12);
        content.set_margin_end(12);
        content.set_margin_top(12);
        content.set_margin_bottom(12);

        let label = Label::new(Some("Enter your password to install packages:"));
        content.append(&label);

        let entry = PasswordEntry::builder()
            .show_peek_icon(true)
            .margin_top(8)
            .build();
        content.append(&entry);

        dialog.add_button("Cancel", ResponseType::Cancel);
        dialog.add_button("Authenticate", ResponseType::Ok);

        let s = state.clone();
        dialog.connect_response(move |d, resp| {
            if resp == ResponseType::Ok {
                let pw = entry.text().to_string();
                if !pw.is_empty() {
                    helpers::set_sudo_password(pw);
                    let s_clone = s.clone();
                    glib::spawn_future_local(Self::run_install(s_clone, pkg.clone()));
                }
            }
            d.close();
        });

        dialog.present();
    }

    async fn run_install(state: AppState, pkg: crate::backends::PackageInfo) {
        state.clear_output();
        state.set_progress(true);
        state.append_output(&format!("Installing {}...", pkg.name));
        state.status_label.set_text(&format!("Installing {}...", pkg.name));

        let manager = state.manager.clone();
        let settings = state.settings.clone();
        let name = pkg.name.clone();
        let backend_key = pkg.backend_key.clone();
        let version = state.detail_view.selected_version();

        let result = async_std::task::spawn_blocking(move || {
            // Auto-install flatpak/snapd if they're missing and the user asked for it
            let auto = {
                let s = settings.lock().unwrap();
                (backend_key == "flatpak" && s.flatpak_auto_install)
                    || (backend_key == "snap" && s.snap_auto_install)
            };
            if auto {
                let bin = if backend_key == "flatpak" { "flatpak" } else { "snap" };
                if !helpers::check_command(bin) {
                    if let Some(b) = manager.backends.get(&backend_key) {
                        b.install_self();
                    }
                }
            }
            if let Some(backend) = manager.backends.get(&backend_key) {
                backend.install(&name, version.as_deref())
            } else {
                (-1, String::new(), format!("Backend '{}' not found", backend_key))
            }
        }).await;

        state.set_progress(false);

        if result.0 == 0 {
            state.append_output(&format!("Successfully installed {}", pkg.name));
            state.status_label.set_text(&format!("{} installed successfully", pkg.name));
        } else {
            let err = if !result.2.is_empty() { result.2.clone() } else { result.1.clone() };
            state.append_output(&format!("Error: {}", err));
            state.status_label.set_text(&format!("Failed to install {}", pkg.name));
            state.show_error("Installation Failed", &err);
        }

        let s = state.clone();
        Self::load_packages(s).await;
    }

    async fn request_remove(state: AppState, pkg: crate::backends::PackageInfo) {
        let confirm = state.settings.lock().unwrap().confirm_before_remove;

        if !confirm {
            Self::run_remove(state, pkg).await;
            return;
        }

        let dialog = MessageDialog::builder()
            .message_type(MessageType::Question)
            .text("Remove Package")
            .secondary_text(&format!("Are you sure you want to remove {}?", pkg.name))
            .buttons(ButtonsType::OkCancel)
            .modal(true)
            .build();

        let s = state.clone();
        let pkg_clone = pkg.clone();
        dialog.connect_response(move |d, resp| {
            d.close();
            if resp == ResponseType::Ok {
                let s_clone = s.clone();
                glib::spawn_future_local(Self::run_remove(s_clone, pkg_clone.clone()));
            }
        });

        dialog.present();
    }

    async fn run_remove(state: AppState, pkg: crate::backends::PackageInfo) {
        let needs_sudo = matches!(pkg.backend_key.as_str(), "pacman" | "flatpak" | "snap");

        if needs_sudo && !helpers::has_sudo_password() {
            let dialog = Dialog::builder()
                .title("Authentication Required")
                .modal(true)
                .build();

            let content = dialog.content_area();
            content.set_margin_start(12);
            content.set_margin_end(12);
            content.set_margin_top(12);
            content.set_margin_bottom(12);

            let label = Label::new(Some("Enter your password to remove packages:"));
            content.append(&label);

            let entry = PasswordEntry::builder()
                .show_peek_icon(true)
                .margin_top(8)
                .build();
            content.append(&entry);

            dialog.add_button("Cancel", ResponseType::Cancel);
            dialog.add_button("Authenticate", ResponseType::Ok);

            let s = state.clone();
            dialog.connect_response(move |d, resp| {
                if resp == ResponseType::Ok {
                    let pw = entry.text().to_string();
                    if !pw.is_empty() {
                        helpers::set_sudo_password(pw);
                        let s_clone = s.clone();
                        glib::spawn_future_local(Self::do_remove(s_clone, pkg.clone()));
                    }
                }
                d.close();
            });

            dialog.present();
        } else {
            Self::do_remove(state, pkg).await;
        }
    }

    async fn do_remove(state: AppState, pkg: crate::backends::PackageInfo) {
        state.clear_output();
        state.set_progress(true);
        state.append_output(&format!("Removing {}...", pkg.name));
        state.status_label.set_text(&format!("Removing {}...", pkg.name));

        let manager = state.manager.clone();
        let settings = state.settings.clone();
        let name = pkg.name.clone();
        let backend_key = pkg.backend_key.clone();

        let result = async_std::task::spawn_blocking(move || {
            let backend_result = manager
                .backends
                .get(&backend_key)
                .map(|b| b.remove(&name))
                .unwrap_or_else(|| (-1, String::new(), format!("Backend '{}' not found", backend_key)));

            // Optional orphan cleanup after successful pacman removal
            if backend_result.0 == 0
                && backend_key == "pacman"
                && settings.lock().unwrap().remove_orphans_on_uninstall
            {
                let r = helpers::run_cmd(&["pacman", "-Qtdq"], 30);
                let orphans: Vec<String> = r
                    .stdout
                    .lines()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                    .collect();
                if !orphans.is_empty() {
                    let mut args: Vec<&str> = vec!["pacman", "-Rns", "--noconfirm"];
                    let refs: Vec<&str> = orphans.iter().map(|s| s.as_str()).collect();
                    args.extend(refs);
                    let _ = helpers::run_as_root(&args, 300);
                    return (
                        backend_result.0,
                        format!(
                            "{}\nRemoved {} orphaned package(s)",
                            backend_result.1,
                            orphans.len()
                        ),
                        backend_result.2,
                    );
                }
            }

            backend_result
        }).await;

        state.set_progress(false);

        if result.0 == 0 {
            state.append_output(&format!("Successfully removed {}", pkg.name));
            state.status_label.set_text(&format!("{} removed successfully", pkg.name));
        } else {
            let err = if !result.2.is_empty() { result.2.clone() } else { result.1.clone() };
            state.append_output(&format!("Error: {}", err));
            state.status_label.set_text(&format!("Failed to remove {}", pkg.name));
            state.show_error("Removal Failed", &err);
        }

        let s = state.clone();
        Self::load_packages(s).await;
    }

    async fn load_packages(state: AppState) {
        state.status_label.set_text("Loading packages...");
        let cat = state.current_category.lock().unwrap().clone();
        let manager = state.manager.clone();
        let parallel = state.settings.lock().unwrap().download_updates_in_parallel;

        let results = async_std::task::spawn_blocking(move || {
            match cat.as_str() {
                "installed" => manager
                    .enabled_backends()
                    .into_iter()
                    .flat_map(|(_, b)| b.list_installed())
                    .collect(),
                "updates" => {
                    let backends = manager.enabled_backends();
                    if parallel {
                        std::thread::scope(|scope| {
                            let handles: Vec<_> = backends
                                .into_iter()
                                .map(|(_, b)| scope.spawn(move || b.check_updates()))
                                .collect();
                            handles
                                .into_iter()
                                .flat_map(|h| h.join().unwrap_or_default())
                                .collect()
                        })
                    } else {
                        backends.into_iter().flat_map(|(_, b)| b.check_updates()).collect()
                    }
                }
                key => manager
                    .backends
                    .get(key)
                    .map(|b| b.list_installed())
                    .unwrap_or_default(),
            }
        })
        .await;

        let count = results.len();
        let msg = format!("{} packages found", count);
        let settings = state.settings.lock().unwrap().clone();
        state.package_list.lock().unwrap().set_packages(results, &settings);
        state.status_label.set_text(&msg);
    }

    async fn show_search_results(state: AppState, query: &str) {
        state.status_label.set_text(&format!("Searching for '{}'...", query));
        let q = query.to_string();
        let manager = state.manager.clone();

        let results = async_std::task::spawn_blocking(move || manager.search_all(&q)).await;

        let count = results.len();
        let settings = state.settings.lock().unwrap().clone();
        state.package_list.lock().unwrap().set_packages(results, &settings);
        state.notebook.set_current_page(Some(0));
        state.status_label
            .set_text(&format!("Found {} results for '{}'", count, query));
    }

    async fn check_for_updates(state: AppState) {
        state.status_label.set_text("Checking for updates...");
        let manager = state.manager.clone();
        let refresh_db = state.settings.lock().unwrap().refresh_databases_before_update;

        let count = async_std::task::spawn_blocking(move || {
            if refresh_db {
                manager.refresh_all();
            }
            manager
                .enabled_backends()
                .into_iter()
                .flat_map(|(_, b)| b.check_updates())
                .count()
        })
        .await;

        state
            .status_label
            .set_text(&format!("{} update(s) available", count));
        if *state.current_category.lock().unwrap() == "updates" {
            Self::load_packages(state).await;
        }
    }
}

fn state_default_category(settings: &Arc<Mutex<Settings>>) -> String {
    let s = settings.lock().unwrap();
    match s.default_category.as_str() {
        "updates" => "updates".to_string(),
        _ => "installed".to_string(),
    }
}

trait EntityId {
    fn entity_id(&self) -> usize;
}

impl<T: IsA<GtkObject>> EntityId for T {
    fn entity_id(&self) -> usize {
        self.as_ptr() as usize
    }
}
