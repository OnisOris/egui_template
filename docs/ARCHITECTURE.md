# Architecture

## Workspace and dependencies

```text
apps/starter-desktop -> starter-core
                     -> starter-ui -> egui component crates
                     -> eframe / serde_json / rfd
starter-core         -> serde
```

`starter-core` owns versioned document data, object IDs, selection, validation, and commands. It has no GUI types. `starter-ui` owns only reusable egui building blocks and docking composition; it does not know about demo objects. `starter-desktop` composes the UI and routes commands to the core.

The dependency direction is application → reusable UI/core. The UI and core crates do not depend on the application.

## State ownership

- **DocumentState** (`starter-core`): document objects, the single source of selection, and structure/name/position/size revisions used to invalidate only affected table projections.
- **UiState** (`apps/starter-desktop/src/state.rs`): workspace tree, theme, repaint mode, table projection controls, and feature-local UI state.
- **ViewportState** (`features/scene`): camera and transient drag target. Screen coordinates stay out of the document.
- **MetricsState** (`metrics.rs`): measured callback frequency, callback CPU time, and a bounded history of measurements. It reports no GPU presentation metric.

## Action flow

Widgets create a `starter_core::Command`; `DocumentState::dispatch` validates and applies it. Explorer, table, inspector, and viewport all read the same document and selection on the next egui pass. The table stores only an ID projection and an ID-to-vector-index lookup for sorting/filtering and virtual rows; it does not hold a second object model. Field revisions avoid rebuilding a 10k-row projection for unrelated edits.

```text
Inspector edit -> SetPosition(id, value) -> DocumentState -> Explorer/Table/Viewport
```

Menu, toolbar, and keyboard shortcuts produce the same `AppAction` and call the same application handler. Document JSON is validated before `ReplaceDocument` is dispatched.

## Lifecycle and persistence

`eframe::App::ui` handles pending file results, shortcuts, top-level UI, dock panes, then records callback timing. It requests another repaint only when continuous mode is enabled. `eframe` storage persists theme, layout, table controls, load selection, and selected IDs; the potentially large document is not autosaved. Open/Save uses `rfd::AsyncFileDialog` on a worker thread and requests a repaint when complete.

The dock tree is `egui_tiles::Tree<PanelId>` and is serialized by `egui_tiles`. Its pane identifiers are stable enum values, not titles. Hiding a tab marks its tile invisible, retaining its UI state so the View menu can restore it.

## Rerun Viewer notes

The Rerun architecture separates panels, views, viewer support, and data/model crates into dependency layers. This starter follows that separation at a much smaller scale; it deliberately does not import Rerun's renderer, store, networking, or plugin infrastructure. See the [Rerun architecture overview](https://github.com/rerun-io/rerun/blob/main/ARCHITECTURE.md).
