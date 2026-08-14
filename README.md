# AcreetionOS App Store

The official app store for AcreetionOS — a fast, native GTK4/Rust application for
discovering, installing, and managing packages from every major source.

Built with Rust, GTK4, and async-std. No web views, no Electron — just native
widgets, so it stays snappy even on old hardware.

## Features

- **Unified search across all sources** — one box, results from every enabled backend
- **Pacman** — native Arch/AcreetionOS repository packages (via `pacman`)
- **Flatpak** — sandboxed apps from Flatpak remotes
- **Snap** — Snap packages via `snapd`
- **AppImage** — portable AppImages discovered in your AppImage directories,
  plus URL download from the detail view
- **AUR (optional)** — tick the *"Enable AUR support"* box in the sidebar to turn
  the Arch User Repository on. Uses `yay` or `paru` (auto-detected) with your
  choice of build directory, clean builds, dev-package tracking, vote sorting,
  and PKGBUILD review skipping
- **Updates view** — shows available updates from every enabled backend,
  with optional parallel checking
- **GNOME-style UI that themes like Cinnamon** — header bar, sidebar, cards,
  list/grid toggle, source badges. No hardcoded colors: it follows your theme
- **Lots of settings** — Preferences dialog with 7 tabs and 30+ options

## Preferences (7 tabs, 30+ options)

| Tab | What you can play with |
|-----|------------------------|
| **General** | Check for updates on launch, remember window size, default view, notifications |
| **AUR** | Enable AUR, helper (auto/yay/paru), build directory, clean build, -git/dev packages, sort by votes, AUR update checks, refresh on sync, skip PKGBUILD review |
| **Backends** | Toggle pacman / flatpak / snap / appimage individually, auto-install flatpak or snapd when missing |
| **Updates** | Background check frequency (never/hourly/6h/daily), refresh DBs before checking, remove orphans on uninstall, parallel checks |
| **Appearance** | Theme (system/light/dark), list icon size, source badges, descriptions, compact rows, grid view |
| **Behaviour** | Confirm before install/remove, show command output, global command timeout cap |
| **AppImage** | Scan directories (colon-separated), register with appimaged/appimagelauncherd, scan on launch |

Settings persist to `~/.config/acreetion-appstore/settings.toml`.

## Build

```sh
cargo build --release
```

Requires: `gtk4`, `cargo`, and the backend tools you want to use
(`pacman`, `flatpak`, `snap`, `yay`/`paru` for AUR).

## Run

```sh
cargo run --release
# or install:
sudo install -Dm755 target/release/acreetion_appstore /usr/bin/acreetion_appstore
```

## Layout

```
src/
├── main.rs                 # bootstrap, settings load
├── settings.rs             # Settings struct + TOML persistence
├── backends/
│   ├── mod.rs              # Backend trait, PackageInfo, BackendManager
│   ├── pacman.rs           # native repos
│   ├── yay.rs / paru.rs    # AUR helpers (settings-driven flags)
│   ├── flatpak.rs
│   ├── snap.rs
│   └── appimage.rs         # local discovery + download dir
└── ui/
    ├── window.rs           # main window, sidebar w/ AUR toggle
    ├── package_list.rs     # list + grid views, badges
    ├── package_detail.rs   # detail page with version picker
    └── settings_dialog.rs  # the 7-tab preferences window
```

## License

MIT
