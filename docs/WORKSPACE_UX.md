# M4.5 — Workspace UX

Changes are confined to `starter-bevy` UI. No new dependencies, project schema,
coordinate math, package builder, renderer, ECS synchronization, or desktop
behavior changes.

- Data Table keeps readable minimum column widths and uses `egui_table`'s
  existing horizontal scrolling and sticky ID column. The filter field fits
  narrow panes. Sorting, filtering, virtualization, and selection are retained.
- Project and marker names use persistent `InspectorState` drafts, following the
  existing Inspector pattern. Enter or blur commits a trimmed, validated name.
  Empty/invalid drafts remain editable; changing selection initializes the
  corresponding name. Marker coordinate/style changes remain independent.
- The unimplemented Groups node is hidden.
- Builder status appears once per Map/Viewport view and is no longer repeated
  in Satellite Inspector. Known partial/failure messages remain visible.

## Layout verification

The existing implementation passed; `layout.rs`, `Workspace`, and Reset Layout
were not modified. No migration or layout schema change was introduced.

Tests used a separate `XDG_CONFIG_HOME` under `/tmp`; the user's layout was not
removed, replaced, or reset. A separate Bevy instance was used throughout.

| Scenario | Observed result |
| --- | --- |
| No saved layout | Explorer left, Map active in center beside Viewport, Inspector right |
| Reset Layout | Standard left/center/right arrangement restored |
| Normal close and restart after Reset | Arrangement and active central Viewport persisted |
| Drag Map into right column above Inspector; close and restart | Custom Map placement persisted |
| Reset again after custom placement | Saved JSON semantically equals the first reset JSON |
| Deliberately truncated `layout.json` | App started without panic with standard layout and Map active; normal exit wrote valid JSON |

Loading real DEM activated Viewport. A valid saved layout was used on restart;
it was not overridden by the new-launch Map default.

## Native GUI smoke — Linux, 2026-11-12

Actual Bevy window, Hyprland/Wayland, NVIDIA RTX 3050 6 GB, Vulkan. Checks used
native mouse/keyboard automation and screenshot inspection, **not a human manual
acceptance test**. This is separate from the synthetic egui widget regressions.
Window client sizes tested: **1366 × 768** and **1920 × 1080**.

1. Started an empty project with no settings: zero objects, no Groups.
2. Searched Alps (`46.6, 7.95`), drew and confirmed a geographic rectangle,
   clicked Load in 3D; real terrain rendered, Viewport activated automatically.
3. Switched Map/Viewport; added Marker by clicking Map and selected it in Explorer.
4. Cleared the marker name and pressed Enter: empty input and validation error
   remained visible. Entered `M4.5 Survey Point`, committed with Enter, changed
   latitude to `46.605` in Inspector.
5. Saved through the native dialog, closed the app, restarted, opened saved JSON
   through the native dialog. Region, marker name/coordinates and package ID
   survived; real DEM was restored from cache.
6. Resized to both required sizes. Narrowed Data Table with the splitter: headers
   stayed separate; horizontal wheel input revealed later columns.
7. Dragged Map into the right column, restarted, verified custom placement;
   reset again and compared the saved docking tree. Tested corrupt JSON startup.

The DEM was 387 × 387 samples, 25 m grid. Builder logged 0.369 s to readiness;
CPU asset installation logged 0.264 ms, and 0.099 ms on project restoration.
These are observed local timings, not a GPU benchmark; GPU upload was not timed.

No authorized Satellite provider was configured: DEM-only worked with the
explicit source-not-configured message. Satellite rendering was not changed or
visually revalidated in this task. The comprehensive lifecycle was exercised
across both window sizes, rather than repeated twice from scratch.

## Automated verification

Two targeted regressions cover Inspector draft/commit/selection behavior and
minimum table widths plus horizontal scrolling. The existing Explorer widget
regression also checks the absent Groups node. Existing project, layout,
selection, DEM and Satellite tests remain in the workspace suite.

All required checks passed locally:

| Command | Result |
| --- | --- |
| `cargo fmt --all --check` | Pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass |
| `cargo test --workspace` | Pass, 99 tests (69 Bevy) |
| `cargo build --workspace --release` | Pass |
| `./scripts/check.sh` | Pass, including file-size and single-egui checks |

No Rust file exceeds 400 lines. No new dependency was introduced.
