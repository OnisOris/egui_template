# Rust Desktop UI Starter Kit

A native desktop starter assembled from three crates: a GUI-independent demo core, reusable egui components, and an eframe application. It is intended as a small base for engineering tools and visualizers—not as a CAD implementation or an integration with another application.

## Requirements and run (CachyOS / Arch Linux)

```bash
sudo pacman -S --needed rustup base-devel pkgconf libxkbcommon libxcb xdg-desktop-portal
rustup default stable
cargo run --release
```

Use an XDG portal backend provided by your desktop environment for native file dialogs (for example, the KDE or GNOME portal backend). The app enables X11 and Wayland through eframe/glow. Cargo uses the pinned versions in `Cargo.lock`.

## Build and checks

```bash
cargo build --workspace --release
./scripts/check.sh
```

The app can also be run from the workspace root with `cargo run --release` (the desktop app is the default member).

## Included UI

- Dockable Explorer, Viewport, Inspector, Data Table, Charts, and Components panels; panel visibility and layout are persisted.
- Versioned JSON Open/Save using asynchronous native dialogs.
- Demo object loads of 100, 1,000, and 10,000; synchronized explorer/table/inspector/viewport selection and edits.
- Synthetic plots and a measured UI callback CPU-time plot. Callback frequency is not GPU presentation FPS.
- Event-driven repaint by default; continuous repaint is an explicit load-test option.

In event-driven mode, callback-rate samples may remain unchanged while the app is idle; values update when callbacks resume.

## Workspace crates

- `starter-core`: document, stable IDs, validation, selection, and commands. No egui dependency.
- `starter-ui`: reusable docking, explorer, table, plot, and theme helpers.
- `starter-desktop`: panel composition, viewport, demo features, settings, and file service.

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), [docs/COMPONENTS.md](docs/COMPONENTS.md), and [docs/EXTENDING.md](docs/EXTENDING.md) before extending the template.
