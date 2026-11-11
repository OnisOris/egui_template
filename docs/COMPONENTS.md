# Components

All egui-facing crates are pinned to the egui 0.37 line and locked in `Cargo.lock`.

| Component | Library / version | License | Used API | Adapter / location | Limits |
|---|---|---|---|---|---|
| Native runner and widgets | `eframe` / `egui` 0.37.0 | MIT OR Apache-2.0 | `eframe::App`, standard egui widgets | desktop `app.rs`, feature panels | Callback timing is not GPU FPS |
| Docking | `egui_tiles` 0.18.0 | MIT OR Apache-2.0 | `Tree`, `Tiles`, `Behavior` | `starter-ui/src/workspace.rs` | App selects the default tile arrangement |
| Tree view | `egui_ltreeview` 0.10.0 | MIT | `TreeView`, `TreeViewState`, `Action` | `starter-ui/src/explorer.rs` | App supplies node IDs, hierarchy, and commands |
| Virtual table | `egui_table` 0.11.0 | MIT OR Apache-2.0 | `Table`, `TableDelegate`, resizable `Column`s | `starter-ui/src/table.rs` | Sorting/filtering are app-side ID projections |
| Plots | `egui_plot` 0.38.0 | MIT OR Apache-2.0 | `Plot`, `Line`, `PlotPoints` | `starter-ui/src/chart.rs` | Data production belongs to the app |
| UI tests | `egui_kittest` 0.37.0 | MIT OR Apache-2.0 | `Harness`, accessibility queries | `starter-ui/tests` | Tests do not replace native desktop interaction tests |
| Native file dialogs | `rfd` 0.18.0 | MIT | `AsyncFileDialog` | `starter-desktop/services/files.rs` | Linux relies on XDG desktop portal support |
| Future executor | `pollster` 1.0.1 | Apache-2.0 OR MIT | `block_on` inside worker threads | `services/files.rs` | Never blocks the UI thread |
| Serialization | `serde` 1.0.229, `serde_json` 1.0.154 | MIT OR Apache-2.0 | derives, JSON document format | `starter-core`, file service | Schema version currently starts at 1 |

`egui_extras` is intentionally omitted: the selected components and built-in egui widgets cover this demo without it. No alternate docking crate is included. `cargo tree -d` is part of the check script to catch duplicate dependency versions.
