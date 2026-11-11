# Map Picker → Bevy viewport

Run `cargo run -p starter-bevy --release`. Map is an optional `starter-ui`
docking pane, installed by the Bevy application. The desktop default layout
and its saved layouts keep the original six panes. Bevy restores its own layout
from the OS config directory (`~/.config/starter-bevy/layout.json` on Linux) and
saves it atomically on a clean exit. Missing/corrupt files fall back to defaults;
older layouts acquire a single Map pane without losing their other panes.

## Libraries and compatibility

Checked against crates.io on 2026-11-11. Direct dependencies use exact versions;
`Cargo.lock` also updates transitive dependencies.

| Package | Selected | Latest stable | Reason |
|---|---|---|---|
| Bevy | 0.20.1 | 0.20.1 | Updated engine |
| bevy_egui | 0.43.1 | 0.43.1 | Updated integration, requires egui `^0.36` |
| egui / eframe / egui_kittest | 0.36.2 | 0.37.0 | Last compatible UI line |
| egui_tiles | 0.17.1 | 0.18.0 | egui 0.36 docking |
| egui_ltreeview | 0.9.0 | 0.10.0 | egui 0.36 tree |
| egui_table | 0.10.0 | 0.11.0 | egui 0.36 table |
| egui_plot | 0.37.0 | 0.38.0 | egui 0.36 chart |
| Walkers | 0.62.0 | 0.65.0 | Last release supporting egui 0.36; 0.63+ requires 0.37 |
| directories | 6.0.0 | 6.0.0 | OS cache path |
| rfd / pollster | 0.18.0 / 1.0.1 | same | Desktop dialogs |
| serde / serde_json | 1.0.229 / 1.0.154 | same | Existing persistence |

Sources: [bevy_egui manifest](https://docs.rs/crate/bevy_egui/0.43.1/source/Cargo.toml),
[Walkers manifest](https://docs.rs/crate/walkers/0.62.0/source/Cargo.toml).
Updating the complete UI stack to egui 0.37 currently needs a new upstream
bevy_egui release. No forks or incompatible duplicate egui versions are used.
`scripts/check.sh` enforces one egui version across the workspace.

## State and data flow

```text
Walkers MapMemory + tiles         pointer → Walkers Projector
       │                                      │
 MapViewState                          RegionSelectionState
 (center, zoom, tool)                  (draft bbox, cursor, drag)
                                               │ Confirm Region
                                               ▼
                                      ActiveMapRegion
                                      (MapRegion + revision)
                                               │ Open in 3D request
                                               ▼
                                      demo::open_region
                                      ├─ standard Bevy Mesh/Material
                                      ├─ existing viewport camera + RenderTarget
                                      └─ activate Viewport docking pane
```

`ActiveMapRegion` is the sole confirmed geographic region. Confirm validates
and commits an immutable value; map navigation and draft editing cannot change
it. Open reads the confirmed revision, never the draft. Clear Selection and
Escape discard only the draft. The confirmed outline stays green; drafts are
blue. A new drag replaces the draft. Confirming does not rebuild the scene.
The demo keeps derived scene bounds and its revision, not another WGS84 region.
Thus even camera reset after confirming another region frames the scene that
was actually opened. The Editor scene button restores the existing editor view.

## Contract and measurements

`region.rs` contains `GeoBounds`, `GeoOrigin`, `MapRegion`, and `ActiveMapRegion`.
All geographic coordinates are `f64`. Validation rejects NaN/infinity, reversed
or zero bounds, longitude outside ±180°, latitude outside WebMercator's
±85.0511287798066°, and selections crossing the world edge. Longitudes are never
silently wrapped. Width follows the center parallel, height the meridian;
approximate area uses the spherical latitude-band area (radius 6378137 m).
These are UI estimates, not cadastral/geodesic survey measurements.

Origin defaults to the bbox center on confirmation. It is independent of the
bbox in the contract and may later be supplied by a fixed home outside it.
`coordinates.rs` is the only local conversion: domain X east, Y south, Z up;
Bevy X east, Y up, Z south. The local tangent approximation is deliberately
limited to demonstration use. Screen projection/unprojection and zoom are
entirely Walkers' responsibility.

## Input, tiles and failure handling

- Go accepts `latitude, longitude`; address lookup is not implemented.
- Pan uses left drag; the wheel zooms around the cursor without Ctrl.
- Select Region disables map drag/pan. Zoom remains available before/after a
  selection and is suspended during the drag. The pointer is clamped to the map
  rectangle on release; geographic coordinates remain stored across zooms.
- Fit projects bbox corners through Walkers and fits their pixel extent to the
  available pane. No XYZ coverage or Mercator implementation is duplicated.
- Input is scoped to the relevant pane. Hiding a pane cancels its active drag;
  the demo viewport uses orbit/pan and cannot pick hidden editor objects.
- HTTP work is asynchronous inside Walkers. `MapPicker` is a Bevy non-send
  resource because Walkers owns a channel receiver. It is not global state.

OpenStreetMap is the single raster source. Attribution is always shown below
the map. HTTP requests identify starter-bevy and use Walkers' HTTP disk cache
under the OS cache directory (`~/.cache/starter-bevy/map-tiles` on Linux).
The app requests visible tiles only; there is no bulk download or prefetch UI.
Follow the [OSMF tile policy](https://operations.osmfoundation.org/policies/tiles/),
including attribution, cache headers and provider capacity limits. For another
provider, replace the `TileSource` passed to `HttpTiles` and its attribution;
credentials must be supplied outside source control.

Walkers 0.62 does not expose individual HTTP failures. The small `Tiles` adapter
observes missing visible tiles and shows an availability warning after 12 seconds,
with Retry tiles. This is an availability timeout, not an exact HTTP diagnosis;
partial coverage can be visible during failures. Existing UI and selection stay
usable. Retry recreates the Walkers loader and retains its disk cache.

## Demo lifecycle and limits

A fixed 64×64-cell deterministic heightfield, draped grid with highlighted
perimeter, and three landmarks use standard Bevy meshes and materials on a
separate render layer. The existing viewport camera switches layers. No new
camera is created when opening a region. Both camera projections remain usable.
The first visible viewport pass fits the scene using its actual dock dimensions.
Opening a new revision despawns five old entities and explicitly removes three
meshes and three materials. Reopening the same revision only refocuses it.
Nothing rebuilds each frame, on selection, or on Confirm.

Demo sides must be 1 m–100 km; the demo origin must be within 1000 km of its
center. Geographic selection itself is not restricted to that size. Heights are
synthetic and must never be interpreted as DEM. No satellite imagery, buildings,
roads, LOS, LOD, simulation, or UAV dependencies are included.

## Future UAV adapter

Read-only inspection of the local UAV project's public Rust types established:
`uav-core::MapDefinition` contains `id`, `bbox`, `origin`, terrain source and
height range; `uav-protocol::Command::SetMapArea { bbox, origin }` keeps origin
separate. `uav-geo` maps to east/south local metres. `uav-map-data` and
`uav-terrain` supply packages with bbox/origin metadata and their own stores.
No implementations or source files were copied; that repository was not changed.

The next step is an adapter replacing `demo::open_region`: convert the confirmed
bbox/origin to Runtime `SetMapArea`, keep the revision as a request token, and
install the returned terrain/map package only if its token and origin still
match. Connect a small real DEM package first; then a licensed satellite package
through `uav-map-data`. Keep loading errors and package ownership in that adapter,
with the picker and its region contract unchanged.

## Validation

`cargo fmt --all --check`, strict workspace Clippy and 52 workspace tests pass.
The tests cover geographic validation/measurements, coordinate axes, selection
cancel/zoom/pan, cursor-anchored wheel zoom, fit near projection edges, scene
replacement and asset counts, and layout migration/serialization/splitter drag.
Existing Explorer, Inspector and desktop tests remain included. CI also builds
the entire workspace in release mode and checks the single-egui-version pin.

Native smoke checks used an Intel Vulkan device and automated OS input: OSM
tiles displayed, a rectangle was confirmed and opened in 3D, both camera
projections rendered, another region replaced the scene, Map/Viewport switched,
window resizing worked, and Explorer/Inspector remained responsive. With a
fresh cache and an unreachable HTTPS proxy, the tile warning appeared while
Inspector still worked. Both native applications started; Bevy wrote its layout
on clean close. This automation is **not** a full human mouse/wheel UX review;
that manual review has not been performed.
