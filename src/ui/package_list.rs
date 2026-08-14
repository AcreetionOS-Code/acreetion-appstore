use crate::backends::PackageInfo;
use crate::settings::Settings;
use gtk4::prelude::*;
use gtk4::{
    self, FlowBox, FlowBoxChild, Label, ListBox, ListBoxRow, ScrolledWindow, Widget,
};
use std::sync::Arc;

pub struct PackageList {
    pub scrolled: ScrolledWindow,
    pub listbox: ListBox,
    pub flowbox: FlowBox,
    packages: Arc<Vec<PackageInfo>>,
    grid_mode: bool,
}

impl PackageList {
    pub fn new() -> Self {
        let scrolled = ScrolledWindow::builder().vexpand(true).build();
        let listbox = ListBox::new();
        let flowbox = FlowBox::builder()
            .max_children_per_line(30)
            .min_children_per_line(2)
            .row_spacing(8)
            .column_spacing(8)
            .margin_top(10)
            .margin_bottom(10)
            .margin_start(10)
            .margin_end(10)
            .build();
        scrolled.set_child(Some(&listbox));
        Self {
            scrolled,
            listbox,
            flowbox,
            packages: Arc::new(Vec::new()),
            grid_mode: false,
        }
    }

    pub fn set_packages(&mut self, packages: Vec<PackageInfo>, settings: &Settings) {
        self.packages = Arc::new(packages);
        let grid = settings.use_grid_view;

        if grid != self.grid_mode {
            self.grid_mode = grid;
            let new_child: Widget = if grid {
                self.flowbox.clone().upcast()
            } else {
                self.listbox.clone().upcast()
            };
            self.scrolled.set_child(Some(&new_child));
        }

        if grid {
            while let Some(child) = self.flowbox.first_child() {
                self.flowbox.remove(&child);
            }
        } else {
            while let Some(child) = self.listbox.first_child() {
                self.listbox.remove(&child);
            }
        }

        for pkg in self.packages.iter() {
            let content = PackageRow::content(pkg.clone(), settings, grid);
            if grid {
                let child = FlowBoxChild::new();
                child.set_child(Some(&content));
                self.flowbox.append(&child);
            } else {
                let row = ListBoxRow::new();
                row.set_child(Some(&content));
                self.listbox.append(&row);
            }
        }
    }

    pub fn get_package(&self, index: usize) -> Option<PackageInfo> {
        self.packages.get(index).cloned()
    }
}

pub struct PackageRow;

impl PackageRow {
    pub fn content(pkg: PackageInfo, settings: &Settings, grid: bool) -> gtk4::Box {
        if grid {
            Self::grid_card(pkg, settings)
        } else {
            Self::list_row(pkg, settings)
        }
    }

    fn list_row(pkg: PackageInfo, settings: &Settings) -> gtk4::Box {
        let compact = settings.compact_rows;
        let pad = if compact { 3 } else { 6 };

        let hbox = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(12)
            .margin_start(12)
            .margin_end(12)
            .margin_top(pad)
            .margin_bottom(pad)
            .build();

        let icon = gtk4::Image::from_icon_name(pkg_icon(&pkg.backend_key));
        icon.set_pixel_size(settings.list_icon_size);
        icon.set_valign(gtk4::Align::Center);
        hbox.append(&icon);

        let text_vbox = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(2)
            .hexpand(true)
            .build();

        let name_hbox = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(6)
            .build();

        let name_label = Label::builder()
            .xalign(0.0)
            .css_classes(["title-4"])
            .build();
        name_label.set_markup(&format!("<b>{}</b>", escape(&pkg.name)));
        name_hbox.append(&name_label);

        let ver_label = Label::builder()
            .xalign(0.0)
            .label(&pkg.version)
            .css_classes(["caption"])
            .opacity(0.7)
            .build();
        name_hbox.append(&ver_label);

        text_vbox.append(&name_hbox);

        if settings.show_descriptions {
            let desc = if pkg.description.chars().count() > 120 {
                format!("{}...", pkg.description.chars().take(117).collect::<String>())
            } else {
                pkg.description.clone()
            };
            let desc_label = Label::builder()
                .xalign(0.0)
                .label(&desc)
                .wrap(true)
                .max_width_chars(60)
                .build();
            text_vbox.append(&desc_label);
        }

        hbox.append(&text_vbox);

        if settings.show_source_badges {
            let badge = Label::builder()
                .label(badge_label(&pkg.backend))
                .css_classes(["badge", "accent"])
                .halign(gtk4::Align::End)
                .valign(gtk4::Align::Center)
                .build();
            hbox.append(&badge);
        }

        hbox
    }

    fn grid_card(pkg: PackageInfo, settings: &Settings) -> gtk4::Box {
        let vbox = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(6)
            .width_request(180)
            .build();
        vbox.set_css_classes(&["appstore-card"]);

        let icon = gtk4::Image::from_icon_name(pkg_icon(&pkg.backend_key));
        icon.set_pixel_size(48);
        icon.set_halign(gtk4::Align::Center);
        vbox.append(&icon);

        let name_label = Label::builder()
            .halign(gtk4::Align::Center)
            .wrap(true)
            .lines(2)
            .build();
        name_label.set_markup(&format!("<b>{}</b>", escape(&pkg.name)));
        vbox.append(&name_label);

        if settings.show_descriptions {
            let desc = if pkg.description.chars().count() > 60 {
                format!("{}...", pkg.description.chars().take(57).collect::<String>())
            } else {
                pkg.description.clone()
            };
            let desc_label = Label::builder()
                .halign(gtk4::Align::Center)
                .wrap(true)
                .lines(2)
                .css_classes(["dim-label"])
                .build();
            desc_label.set_label(&desc);
            vbox.append(&desc_label);
        }

        if settings.show_source_badges {
            let badge = Label::builder()
                .label(badge_label(&pkg.backend))
                .css_classes(["badge", "accent"])
                .halign(gtk4::Align::Center)
                .build();
            vbox.append(&badge);
        }

        vbox
    }
}

pub fn pkg_icon(backend_key: &str) -> &'static str {
    match backend_key {
        "pacman" => "system-software-install",
        "yay" | "paru" => "applications-engineering",
        "flatpak" => "package-x-generic",
        "snap" => "package-x-generic",
        "appimage" => "application-x-executable",
        _ => "package-x-generic",
    }
}

pub fn badge_label(backend: &str) -> &str {
    match backend {
        "pacman" => "Repo",
        "AUR (yay)" | "AUR (paru)" => "AUR",
        "Flatpak" => "Flatpak",
        "Snap" => "Snap",
        "AppImage" => "AppImage",
        _ => backend,
    }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}
