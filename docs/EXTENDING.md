# Extending the starter

## Add a panel

1. Add a stable variant and title to `starter_ui::PanelId`.
2. Add its pane to `Workspace::standard` in `starter-ui/src/workspace.rs`.
3. Add a `PanelId` arm to the composition in `starter-desktop/src/app.rs` and implement the view under `features/`.
4. If it needs document changes, dispatch a `starter_core::Command`; do not mutate a second panel-local copy.

## Add a data type or action

Put serializable domain data and validation in `starter-core`. Add a command variant and handle it in `DocumentState::dispatch`. Keep egui types, camera transforms, and screen-space values out of core. Add a round-trip and command test in `crates/starter-core/tests`.

## Add a UI dependency

Check its egui version, license, and API first. Add it only to `starter-ui` if it is a reusable component; keep app-specific dependencies in `starter-desktop`. Add the smallest adapter that gives the app a stable, useful API. Do not wrap ordinary egui widgets individually.

## Add a menu item and shortcut

Add an `AppAction`, emit it from the menu/toolbar, and handle it in `StarterApp::action`. Add shortcuts in `StarterApp::shortcuts`, respecting `egui_wants_keyboard_input()` so text editing keeps its keys.

## Replace the demo viewport

Replace `features/scene` and its `ViewportState`; retain `DocumentState` commands and selection synchronization. A different renderer should consume core document data without introducing renderer types into `starter-core`.

## Reuse in another app

Use `starter-core` for the data/command pattern and `starter-ui` for docking, tree, table, and chart helpers. `starter-desktop` is the integration example: copy or depend on only the features your app needs, then replace its demo model, panels, and JSON workflow.
