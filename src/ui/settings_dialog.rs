#![allow(deprecated)]

use crate::settings::{AurHelper, Settings, Theme, UpdateFrequency};
use gtk4::prelude::*;
use gtk4::{
    self, Button, DropDown, Entry, HeaderBar, Label, Notebook, Orientation,
    ScrolledWindow, SpinButton, StringList, Switch, Window,
};
use std::sync::{Arc, Mutex};

pub struct SettingsDialog;

impl SettingsDialog {
    /// Open the preferences window. `on_saved` is called (on the main thread)
    /// after the user hits Save so the caller can rebuild the sidebar/reload.
    pub fn show(
        parent: Option<&Window>,
        settings: Arc<Mutex<Settings>>,
        on_saved: Box<dyn Fn() + 'static>,
    ) {
        let current = settings.lock().unwrap().clone();
        let on_saved = std::sync::Arc::new(on_saved);

        let dialog = Window::builder()
            .title("Preferences")
            .modal(true)
            .default_width(760)
            .default_height(600)
            .build();
        if let Some(p) = parent {
            dialog.set_transient_for(Some(p));
        }

        let header = HeaderBar::new();
        let reset_btn = Button::with_label("Reset to Defaults");
        let cancel_btn = Button::with_label("Cancel");
        let save_btn = Button::with_label("Save");
        save_btn.add_css_class("suggested-action");
        header.pack_start(&reset_btn);
        header.pack_end(&save_btn);
        header.pack_end(&cancel_btn);
        dialog.set_titlebar(Some(&header));

        let notebook = Notebook::new();

        // ── General ────────────────────────────────────────────────────────
        let general = tab_page();
        let gen_launch_check = switch_row(&general, "Check for updates on launch",
            "Run an update check when the app starts", current.check_updates_on_launch);
        let gen_remember_size = switch_row(&general, "Remember window size",
            "Restore the window size from last session", current.remember_window_size);
        let gen_default = dropdown_row(&general, "Default view",
            "Category selected when the app opens", &["Installed", "Updates"],
            if current.default_category == "updates" { 1 } else { 0 });
        let gen_notify = switch_row(&general, "Show notifications",
            "Display desktop notifications for long operations", current.show_notifications);
        notebook.append_page(&general, Some(&Label::new(Some("General"))));

        // ── AUR ────────────────────────────────────────────────────────────
        let aur = tab_page();
        let aur_enable = switch_row(&aur, "Enable AUR support",
            "Search and install packages from the Arch User Repository", current.aur_enabled);
        let aur_helper = dropdown_row(&aur, "AUR helper",
            "Auto prefers paru, falls back to yay", &["Auto", "yay", "paru"],
            match current.aur_helper {
                AurHelper::Auto => 0,
                AurHelper::Yay => 1,
                AurHelper::Paru => 2,
            });
        let aur_builddir = entry_row(&aur, "Build directory",
            "Where PKGBUILDs are built (empty = helper default)", &current.aur_build_dir);
        let aur_cleanbuild = switch_row(&aur, "Clean build",
            "Rebuild from a clean source directory", current.aur_clean_build);
        let aur_devel = switch_row(&aur, "Include -git / development packages",
            "Show and track VCS packages in search and updates", current.aur_include_devel);
        let aur_votes = switch_row(&aur, "Sort search by votes",
            "Order AUR search results by community votes", current.aur_sort_by_votes);
        let aur_checkupd = switch_row(&aur, "Check AUR for updates",
            "Include AUR packages in the Updates view", current.aur_check_updates);
        let aur_sync = switch_row(&aur, "Refresh databases on sync",
            "Run -Sy when refreshing", current.aur_refresh_on_sync);
        let aur_review = switch_row(&aur, "Skip PKGBUILD review prompt",
            "Install without showing the PKGBUILD diff (dangerous!)", current.aur_skip_review);
        notebook.append_page(&aur, Some(&Label::new(Some("AUR"))));

        // ── Backends ───────────────────────────────────────────────────────
        let backends = tab_page();
        let be_pacman = switch_row(&backends, "Pacman (repositories)",
            "Native packages from official repos", current.pacman_enabled);
        let be_flatpak = switch_row(&backends, "Flatpak",
            "Sandboxed apps from Flatpak remotes", current.flatpak_enabled);
        let be_snap = switch_row(&backends, "Snap",
            "Snap packages via snapd", current.snap_enabled);
        let be_appimage = switch_row(&backends, "AppImage",
            "Portable AppImages in your AppImage directories", current.appimage_enabled);
        let be_flatpak_auto = switch_row(&backends, "Auto-install Flatpak runtime",
            "Install flatpak automatically if missing when needed", current.flatpak_auto_install);
        let be_snap_auto = switch_row(&backends, "Auto-install snapd",
            "Install snapd automatically if missing when needed", current.snap_auto_install);
        notebook.append_page(&backends, Some(&Label::new(Some("Backends"))));

        // ── Updates ────────────────────────────────────────────────────────
        let updates = tab_page();
        let up_freq = dropdown_row(&updates, "Automatic update checks",
            "How often to re-check for updates in the background", &["Never", "Every hour", "Every 6 hours", "Daily"],
            match current.update_check_frequency {
                UpdateFrequency::Never => 0,
                UpdateFrequency::EveryHour => 1,
                UpdateFrequency::Every6Hours => 2,
                UpdateFrequency::Daily => 3,
            });
        let up_refresh = switch_row(&updates, "Refresh databases before update check",
            "Run pacman -Sy first so the list is current", current.refresh_databases_before_update);
        let up_orphans = switch_row(&updates, "Remove orphaned dependencies on uninstall",
            "Clean up packages no longer required after removing one", current.remove_orphans_on_uninstall);
        let up_parallel = switch_row(&updates, "Check backends in parallel",
            "Query all sources at once (faster, more CPU churn)", current.download_updates_in_parallel);
        notebook.append_page(&updates, Some(&Label::new(Some("Updates"))));

        // ── Appearance ─────────────────────────────────────────────────────
        let appearance = tab_page();
        let ap_theme = dropdown_row(&appearance, "Theme",
            "Overrides the system theme preference", &["System", "Light", "Dark"],
            match current.theme {
                Theme::System => 0,
                Theme::Light => 1,
                Theme::Dark => 2,
            });
        let ap_iconsize = spin_row(&appearance, "List icon size", 16.0, 48.0, 4.0,
            current.list_icon_size as f64);
        let ap_badges = switch_row(&appearance, "Show source badges",
            "Display a Repo/AUR/Flatpak badge on each row", current.show_source_badges);
        let ap_desc = switch_row(&appearance, "Show descriptions",
            "Show the package description under the name", current.show_descriptions);
        let ap_compact = switch_row(&appearance, "Compact rows",
            "Tighter list rows", current.compact_rows);
        let ap_grid = switch_row(&appearance, "Grid view",
            "Show packages as a card grid instead of a list", current.use_grid_view);
        notebook.append_page(&appearance, Some(&Label::new(Some("Appearance"))));

        // ── Behaviour ──────────────────────────────────────────────────────
        let behaviour = tab_page();
        let bh_confirm_install = switch_row(&behaviour, "Confirm before installing",
            "Ask for confirmation before every install", current.confirm_before_install);
        let bh_confirm_remove = switch_row(&behaviour, "Confirm before removing",
            "Ask for confirmation before every removal", current.confirm_before_remove);
        let bh_output = switch_row(&behaviour, "Show command output",
            "Expand the output panel during installs and removals", current.show_output_expander);
        let bh_timeout = spin_row(&behaviour, "Command timeout (seconds)",
            0.0, 600.0, 5.0, current.command_timeout as f64);
        note_row(&behaviour, "Timeout of 0 uses per-operation defaults.");
        notebook.append_page(&behaviour, Some(&Label::new(Some("Behaviour"))));

        // ── AppImage ───────────────────────────────────────────────────────
        let appimage = tab_page();
        let ai_dirs = entry_row(&appimage, "AppImage directories",
            "Colon-separated folders to scan for AppImages", &current.appimage_dirs.join(":"));
        let ai_daemon = switch_row(&appimage, "Register with AppImage daemon",
            "Spawn appimaged / appimagelauncherd when refreshing", current.appimage_register_appimaged);
        let ai_scan = switch_row(&appimage, "Scan directories on launch",
            "Discover AppImages at startup", current.appimage_scan_on_launch);
        notebook.append_page(&appimage, Some(&Label::new(Some("AppImage"))));

        let content = gtk4::Box::new(Orientation::Vertical, 0);
        content.append(&notebook);
        dialog.set_child(Some(&content));

        let settings_c = settings.clone();
        let on_saved_reset = on_saved.clone();
        let dialog_reset = dialog.clone();
        reset_btn.connect_clicked(move |_| {
            let defaults = Settings::default();
            *settings_c.lock().unwrap() = defaults.clone();
            settings_c.lock().unwrap().save();
            dialog_reset.close();
            on_saved_reset();
        });

        let dialog_cancel = dialog.clone();
        cancel_btn.connect_clicked(move |_| dialog_cancel.close());

        let settings_save = settings.clone();
        let dialog_save = dialog.clone();
        save_btn.connect_clicked(move |_| {
            let mut s = settings_save.lock().unwrap().clone();

            // General
            s.check_updates_on_launch = gen_launch_check.is_active();
            s.remember_window_size = gen_remember_size.is_active();
            s.default_category = if gen_default.selected() == 1 { "updates".into() } else { "installed".into() };
            s.show_notifications = gen_notify.is_active();

            // AUR
            s.aur_enabled = aur_enable.is_active();
            s.aur_helper = match aur_helper.selected() {
                1 => AurHelper::Yay,
                2 => AurHelper::Paru,
                _ => AurHelper::Auto,
            };
            s.aur_build_dir = aur_builddir.text().to_string();
            s.aur_clean_build = aur_cleanbuild.is_active();
            s.aur_include_devel = aur_devel.is_active();
            s.aur_sort_by_votes = aur_votes.is_active();
            s.aur_check_updates = aur_checkupd.is_active();
            s.aur_refresh_on_sync = aur_sync.is_active();
            s.aur_skip_review = aur_review.is_active();

            // Backends
            s.pacman_enabled = be_pacman.is_active();
            s.flatpak_enabled = be_flatpak.is_active();
            s.snap_enabled = be_snap.is_active();
            s.appimage_enabled = be_appimage.is_active();
            s.flatpak_auto_install = be_flatpak_auto.is_active();
            s.snap_auto_install = be_snap_auto.is_active();

            // Updates
            s.update_check_frequency = match up_freq.selected() {
                1 => UpdateFrequency::EveryHour,
                2 => UpdateFrequency::Every6Hours,
                3 => UpdateFrequency::Daily,
                _ => UpdateFrequency::Never,
            };
            s.refresh_databases_before_update = up_refresh.is_active();
            s.remove_orphans_on_uninstall = up_orphans.is_active();
            s.download_updates_in_parallel = up_parallel.is_active();

            // Appearance
            s.theme = match ap_theme.selected() {
                1 => Theme::Light,
                2 => Theme::Dark,
                _ => Theme::System,
            };
            s.list_icon_size = ap_iconsize.value() as i32;
            s.show_source_badges = ap_badges.is_active();
            s.show_descriptions = ap_desc.is_active();
            s.compact_rows = ap_compact.is_active();
            s.use_grid_view = ap_grid.is_active();

            // Behaviour
            s.confirm_before_install = bh_confirm_install.is_active();
            s.confirm_before_remove = bh_confirm_remove.is_active();
            s.show_output_expander = bh_output.is_active();
            s.command_timeout = bh_timeout.value() as u64;

            // AppImage
            s.appimage_dirs = ai_dirs
                .text()
                .split(':')
                .map(|d| d.trim().to_string())
                .filter(|d| !d.is_empty())
                .collect();

            *settings_save.lock().unwrap() = s.clone();
            settings_save.lock().unwrap().save();
            dialog_save.close();
            on_saved();
        });

        dialog.present();
    }
}

// ── widget builders ──────────────────────────────────────────────────────────

fn tab_page() -> ScrolledWindow {
    let sw = ScrolledWindow::new();
    let vbox = gtk4::Box::new(Orientation::Vertical, 6);
    vbox.set_margin_start(18);
    vbox.set_margin_end(18);
    vbox.set_margin_top(14);
    vbox.set_margin_bottom(14);
    sw.set_child(Some(&vbox));
    sw
}

fn row(label: &str, sub: &str, widget: &impl IsA<gtk4::Widget>) -> gtk4::Box {
    let hbox = gtk4::Box::new(Orientation::Horizontal, 12);
    hbox.set_margin_top(4);
    hbox.set_margin_bottom(4);

    let text_box = gtk4::Box::new(Orientation::Vertical, 0);
    text_box.set_hexpand(true);
    let l = Label::builder()
        .label(label)
        .halign(gtk4::Align::Start)
        .build();
    text_box.append(&l);
    if !sub.is_empty() {
        let s = Label::builder()
            .label(sub)
            .halign(gtk4::Align::Start)
            .wrap(true)
            .css_classes(["dim-label"])
            .build();
        text_box.append(&s);
    }
    hbox.append(&text_box);
    hbox.append(widget);
    hbox
}

fn note_row(parent: &ScrolledWindow, text: &str) {
    if let Some(vbox) = parent.child().and_then(|c| c.downcast::<gtk4::Box>().ok()) {
        let l = Label::builder()
            .label(text)
            .halign(gtk4::Align::Start)
            .css_classes(["dim-label"])
            .margin_top(2)
            .build();
        vbox.append(&l);
    }
}

fn switch_row(parent: &ScrolledWindow, label: &str, sub: &str, active: bool) -> Switch {
    let sw = Switch::builder().active(active).valign(gtk4::Align::Center).build();
    if let Some(vbox) = parent.child().and_then(|c| c.downcast::<gtk4::Box>().ok()) {
        vbox.append(&row(label, sub, &sw));
    }
    sw
}

fn dropdown_row(parent: &ScrolledWindow, label: &str, sub: &str, options: &[&str], selected: u32) -> DropDown {
    let model = StringList::new(options);
    let dd = DropDown::builder()
        .model(&model)
        .selected(selected)
        .valign(gtk4::Align::Center)
        .build();
    if let Some(vbox) = parent.child().and_then(|c| c.downcast::<gtk4::Box>().ok()) {
        vbox.append(&row(label, sub, &dd));
    }
    dd
}

fn entry_row(parent: &ScrolledWindow, label: &str, sub: &str, value: &str) -> Entry {
    let entry = Entry::builder()
        .text(value)
        .width_request(220)
        .valign(gtk4::Align::Center)
        .build();
    if let Some(vbox) = parent.child().and_then(|c| c.downcast::<gtk4::Box>().ok()) {
        vbox.append(&row(label, sub, &entry));
    }
    entry
}

fn spin_row(parent: &ScrolledWindow, label: &str, min: f64, max: f64, step: f64, value: f64) -> SpinButton {
    let adj = gtk4::Adjustment::new(value, min, max, step, step * 5.0, 0.0);
    let spin = SpinButton::builder()
        .adjustment(&adj)
        .valign(gtk4::Align::Center)
        .build();
    if let Some(vbox) = parent.child().and_then(|c| c.downcast::<gtk4::Box>().ok()) {
        vbox.append(&row(label, "", &spin));
    }
    spin
}
