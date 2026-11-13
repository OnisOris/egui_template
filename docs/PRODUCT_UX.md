# Geographic editor UX (M5.4)

M5.4 changes presentation and UI-only preferences. It retains Bevy 0.20.1,
egui 0.36.2, egui_tiles, egui_ltreeview, egui_table/egui_plot and Walkers.
No new dependencies, crates, fonts, native application windows, event loops or
rendering systems are introduced.
ProjectDocument schema and geographic data formats remain unchanged.

## Main screens and tools

The menu holds project lifecycle, editing, panel visibility and settings. The
primary toolbar holds Add Object, Select, Focus and three explicit workspaces.
Project name and unsaved indication sit in the menu row. The bottom status holds
readiness/background loading, region dimensions and user object count.

Viewport has City / Terrain / Combined, layer toggles, Grid, Camera and Fit.
Imports and vector reload are in the labelled-tooltip overflow menu. Package IDs,
DEM statistics and tile counters are in Inspector or Diagnostics. Provider
attribution remains visible. Scene mode Editor/Demo/Real is independent of the
geographic view mode and of the workspace layout.

Map provides coordinate search, Pan/Select Region, Standard/Satellite and source
controls. Selected-region summary distinguishes Draft and Confirmed, with
Confirm Region and Load in 3D directly available. Bbox/center are expandable.
Region details are in More; primary confirm/fit/load actions share a wrapped row.
Region loading uses the existing automatic terrain/vector commands; no parallel
project or data state is created.

## Panels and contextual properties

Explorer retains Files and Scene and uses the installed tree widget. Layers have
type icons and text status; visibility is available on selection and in the
context menu. Building datasets stay one layer, never thousands of tree nodes.
File layers show cache availability instead of hashes. Resource Import/Refresh,
GLB placement, unused-asset deletion confirmation and file-manager actions remain.

Inspector uses native collapsible General, Location, Asset, Geographic Position,
Transform, Appearance and Loading Status groups as appropriate. Coordinate and
transform fields use aligned grids and explicit degrees/metres. The original
name-draft validator/commit handling remains in project_names. Inspector scrolls
independently. Selected-layer data and visibility are kept separate from user
model instances.

Empty Viewport offers Select a Region plus New/Open, Load Region in 3D for a
confirmed area, or Add Marker / Import Model after terrain is installed. These
route through the same application commands as menus. An empty project still
has an in-memory ProjectDocument; there is no extra "no project" data model.

## Workspaces and persistence

- Map Editing: central Map, adjacent Viewport, Explorer and Inspector.
- 3D Scene: central Viewport, adjacent Map, Explorer and Inspector.
- Analysis: Viewport/Map above Charts/Data Table, with side panels.

Only a user's explicit preset selection replaces a layout. Dragging, closing,
hiding and restoring tabs remains native egui_tiles behavior. Reset Layout
restores 3D Scene. Old layouts deserialize with no preset field; their panes are
not silently hidden or rearranged. New layouts omit Component Gallery until
Tools → Developer → Component Gallery is requested. Legacy layouts containing
that pane are honored.

layout.json remains the docking file. A one-time layout.pre-m54.json copy
preserves the pre-redesign layout on the first successful preference save.
appearance.json separately stores theme, UI zoom and density. Both use temporary
files and rename, with failures logged. No UI preference enters ProjectDocument.
The desktop eframe application retains its own storage and legacy default layout.

## Input and accessibility

Appearance offers Dark/Light, Compact/Comfortable and UI Scale 0.75–2.0; 1.0 uses
native monitor DPI. Shared defaults are 14 pt body, 16 pt headings, 12 pt supporting
text and 32 pt Comfortable controls. Native egui focus, disabled and hover states
remain; icon-only overflow controls have tooltips. Panel toggles and wrapped
controls remain available in narrow layouts.

Command/Ctrl+S saves, +O opens, +N creates. Delete acts on selection only when a
text edit does not own the keyboard. Escape cancels placement/region drawing.
Busy file operations and modal prompts block shortcuts. Unsaved/error prompts
use egui Modal. Camera input continues to be routed by the enabled viewport
response, not by global mouse deltas. Map source settings remain a modeless
window; pointer interaction with that window does not orbit the covered viewport.

## Developer access

Tools → Developer contains Component Gallery and Diagnostics. Diagnostics reports
frame interval, main CPU wall time, UI/sync/build CPU, projected marker entities,
shared tile cache statistics and detailed data statuses. It explicitly states
that GPU timing is unavailable. No FPS/debug strip is permanently overlaid on
the user's map. Charts remain a user-accessible analysis panel.

## Validation

All required checks passed locally on 2026-11-13: `cargo fmt --all --check`,
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
(**154 passed**, including **117** Bevy library tests),
`cargo build --workspace --release`, and `CARGO_BUILD_JOBS=1 ./scripts/check.sh`
(exit 0, including the egui version check). No hosted CI run is claimed.

Each of stages a, b and c ended with a successful workspace debug build; stage d
passed the full check script. Maximum Rust file length is **320 lines** (existing
desktop chain.rs); maximum changed Rust file is **258 lines** (desktop app.rs).
New modules remain below 300 lines.
OS mouse/keyboard automation is a smoke test, not human manual acceptance.
Before: [M5.3 city UI](screenshots/m53/city.png).


### Native Linux/Wayland smoke results

Release and debug executables ran on NVIDIA RTX 3050 6 GB / Vulkan, with a
separate temporary config directory. Mouse/keyboard input, native file dialogs
and actual window screenshots were used. The reference workspace was not edited.

| Scenario | Observed result |
|---|---|
| A — region to 3D | Selected and confirmed a fresh Kremlin area; Load in 3D installed DEM, live satellite, 395 buildings / 266 roads; City, Terrain and Combined switched; Ctrl+S saved. |
| B — GLB | Imported a renamed valid GLB through File → Import Asset; selected its Files entry; Place on Map created Model 4; Inspector changed yaw to 30°; Focus framed it; saved JSON contains the relative asset path and edited transform. |
| C — layers | Selected Buildings/Roads in Scene, toggled Visible in Inspector off/on and restored Combined. User GLBs remained visible. |
| D — Analysis | Opened Charts and Data Table in Analysis, selected the Model 4 row, returned to 3D Scene. Selection/document invariance also has a widget regression test. |
| E — restoration | Saved, closed and restarted; Analysis layout and 125% appearance were restored independently of the project; Ctrl+O reopened the saved schema-v3 project with three GLBs and one marker, DEM/satellite/vectors. |

Native Ctrl+S and Ctrl+O were exercised. Automated tests additionally gate
Command/Ctrl commands around dialogs and prevent Delete in a focused text field.
The original name-draft, model placement, Explorer/table selection, imagery,
project migration, resource lifecycle and geographic projection regressions pass.
No claim is made about native macOS/Windows keyboard or DPI testing.

An intentional duplicate-name GLB import displayed a blocking error modal,
retained the existing scene, and closed normally. Repeating import under a new
name succeeded. Error text and disabled background are visible in the screenshot.
Map Sources, light theme and 125% UI scaling were inspected in the native window.
The persisted appearance file contained Dark / scale 1.25 / Comfortable on exit;
restart applied it. Reset Appearance returned to the DPI-relative default.

All five requested native window sizes were exercised: **1100×700, 1366×768,
1600×900, 1920×1080 and 2560×1440**. Main controls remain accessible through wrapping
and menus; Inspector scrolls. At 1100 px, Explorer labels can truncate within its
narrow pane; resize/hide the pane for long names. On the 2560×1440 monitor, the
compositor's own top bar overlays the application menu in the full-screen capture;
that bar is outside the application UI. A different physical high-DPI monitor
was not available; 125% application zoom was tested on the native display.

### Before / after and screenshot index

| State | Evidence |
|---|---|
| Before | [M5.3 city](screenshots/m53/city.png) |
| Empty project | [Guided start](screenshots/m54/empty.png) |
| Map Editing | [Confirmed area](screenshots/m54/map-editing.png), [draft selection](screenshots/m54/selected-region.png) |
| Geographic modes | [City](screenshots/m54/city.png), [Terrain](screenshots/m54/terrain.png), [Combined](screenshots/m54/combined.png) |
| Selected imported GLB | [Inspector / transform](screenshots/m54/glb-inspector.png) |
| Analysis | [Charts](screenshots/m54/analysis.png), [Data Table](screenshots/m54/analysis-table.png) |
| Sources and error | [Map Sources](screenshots/m54/map-sources.png), [import error](screenshots/m54/load-error.png) |
| Layer control | [Buildings hidden](screenshots/m54/buildings-hidden.png), [Roads hidden](screenshots/m54/roads-hidden.png) |
| Appearance | [Light](screenshots/m54/light.png), [125% zoom](screenshots/m54/scale125.png) |
| Window sizes | [1100×700](screenshots/m54/1100x700.png), [1366×768](screenshots/m54/1366x768.png), [1600×900](screenshots/m54/1600x900.png), [2560×1440](screenshots/m54/2560x1440.png) |
| Reopened project | [Workspace and scene](screenshots/m54/restored-workspace.png) |

The screenshots document different stages of the same smoke session: the original
region has 624 buildings / 508 roads, the newly selected subset has 395 / 266;
the successful GLB placement increases user objects from three to four. Camera
zoom/focus also differs. This is a UI comparison, not a claim that every picture
shows an identical camera or dataset.

### Remaining bounds

This is automated native smoke plus source-backed regression testing, not human
manual acceptance. Long-duration editing and screen-reader acceptance were not
performed. Stock egui keyboard focus and widget accessibility are retained;
no external icon/font package was added. Geometric limitations of the prior
milestone (representative building bases, OSM heights, roads and DEM resolution)
remain documented in 3D_CITY.md and were not changed by the redesign.
