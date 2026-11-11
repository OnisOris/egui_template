# Rust Desktop UI Starter Kit

A native desktop starter assembled from a GUI-independent demo core, reusable egui components, and two applications — an eframe editor and a Bevy + egui 3D editor. It is intended as a small base for engineering tools, visualizers and simulators—not as a CAD implementation or an integration with another application.

## Requirements and run (CachyOS / Arch Linux)

```bash
sudo pacman -S --needed rustup base-devel pkgconf libxkbcommon libxcb xdg-desktop-portal vulkan-icd-loader
rustup default stable

# Existing egui/eframe template
cargo run -p starter-desktop --release

# New Bevy + egui template
cargo run -p starter-bevy --release
```

Use an XDG portal backend provided by your desktop environment for native file dialogs (for example, the KDE or GNOME portal backend). The desktop app enables X11 and Wayland through eframe/glow. The Bevy app uses winit with X11 and Wayland and renders through wgpu/Vulkan. Cargo uses the pinned versions in `Cargo.lock`.

## Build and checks

```bash
cargo build --workspace --release
./scripts/check.sh
```

The same checks run in GitHub Actions on every push and pull request. They include both applications, clippy, tests, the release build, file-size limits, and the single-egui-version pin.

`cargo run --release` still starts the default member, `starter-desktop`.

## Included UI

- Dockable Explorer, Viewport, Inspector, Data Table, Charts, and Components panels; panel visibility and layout are persisted in the desktop app.
- Versioned JSON Open/Save using asynchronous native dialogs (desktop app).
- Demo object loads of 100, 1,000, and 10,000; synchronized explorer/table/inspector selection and edits.
- Bevy 3D viewport rendered to a texture and embedded in the docking panel: orbit, pan, zoom, focus, reset, perspective/orthographic, picking and horizontal object dragging.
- Synthetic plots and measured UI callback CPU-time plot. Callback frequency is not GPU presentation FPS.
- Event-driven repaint by default in the desktop app; Bevy renders continuously and reports real frame time.

## Workspace crates

- `starter-core`: document, stable IDs, validation, selection, and commands. No egui or Bevy dependency.
- `starter-ui`: reusable docking, explorer, inspector, table, plot, and theme helpers.
- `starter-desktop`: eframe panel composition, UI viewport, demo features, settings, and file service.
- `starter-bevy`: Bevy ECS scene projection, embedded 3D viewport, camera, picking, and egui editor shell.

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), [docs/BEVY_ARCHITECTURE.md](docs/BEVY_ARCHITECTURE.md), [docs/COMPONENTS.md](docs/COMPONENTS.md), and [docs/EXTENDING.md](docs/EXTENDING.md) before extending the template.
