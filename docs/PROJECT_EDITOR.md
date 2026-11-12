# M4: Geographic Project Editor

## Components and source of truth

- `starter-core/geo.rs`: the existing geographic types, re-exported from the
  previous `map/region.rs` path. Coordinate conversion and projection algorithms
  are unchanged.
- `starter-core/project.rs`: versioned `ProjectDocument`, stable marker IDs,
  resource references and layer visibility. `ProjectState` holds selection,
  revision, dirty state and current marker-placement intent.
- `starter-bevy/project`: one project Resource; background rfd dialogs,
  import/save/open tasks; atomic JSON persistence; restoration and scene reset.
- `ui/project_explorer`: Files/Scene adapters for `starter-ui::ExplorerTree` /
  `egui_ltreeview`, without a second tree implementation. Scene nodes contain map
  layers and project markers, not tile IDs or internal ECS entities.
- `ui/project_table`: the existing `starter-ui::show_table` / `egui_table`
  virtualized table, with cached filtering/sorting of marker indices.
- `ui/project_inspector`: project, manifest, resource, terrain, imagery and marker
  properties. Marker edits are validated before changing the document.
- `map/objects`: derived marker ECS projection, sharing one sphere mesh.
  Per-marker material handles and entities are removed when deleted, hidden or
  outside the current DEM. ECS stores stable IDs and graphical resources, never
  a second editable marker document.
- `map/marker_overlay`: Walkers overlay for the same document markers. Selection
  uses the same `ProjectSelection` as Explorer, table, Inspector and 3D picking.

The legacy `DocumentState::default()` remains a 100-object demo for desktop.
Bevy's `DocumentResource::default()` and ordinary startup are empty. Tools →
Demo / Benchmark generates 100 / 1,000 / 10,000 objects in that separate legacy
resource. Return to Project restores the previous scene mode; project objects,
region and resources never become benchmark objects. The existing geographic
synthetic Demo remains an explicit additional Map action.

## Project format and files

Project JSON has `schema_version = 1`, `name`, optional `map_region`, exactly one
Terrain and one Satellite layer, `objects`, `assets`, and `next_id`.

Each Map Marker stores `id`, `name`, `latitude`, `longitude`, `altitude_agl`,
`visible` and RGBA `color`. IDs are independent of Bevy Entity IDs. Coordinate,
name, altitude, duplicate-ID, schema, count and path validation runs on load.
Selection, windows, docking, camera handles and GPU resources are not serialized.

File → New Project / Open Project / Save / Save As / Import Asset handles the
project. Save is also in the toolbar and on Ctrl+S. New/Open/Exit, including the
native close request, offer Save / Discard / Cancel when dirty. Cancelled dialogs
and failed writes preserve the current document. Editing is disabled while a
project file operation or unsaved-change decision is pending. Saving an older
snapshot does not clear dirty state if the project changed meanwhile.

The chosen JSON parent is the project root. Save creates `Assets/Models`,
`Assets/Textures`, and `Geodata`. The Files tab shows the actual JSON filename and
registered resources, not an unrestricted filesystem browser. Image assets go to
Textures, other files to Models. Imported files are copied through staging and
published without overwriting an existing resource. Model assets are registered,
not rendered as arbitrary imported 3D models in this milestone.

Asset paths are relative to the project root. Absolute/traversal paths and
symlinks escaping that root are rejected. Missing resources retain their JSON
references and have a missing status. Refresh rechecks registered paths only.
Save As copies registered available assets to the new root, preserving missing
references. Existing destination resources cause an error instead of overwrite.
Show in File Manager opens the selected resource's directory or project root.

JSON writes use a unique staging file, `sync_all`, then atomic rename. A failed
save never deletes the existing JSON. Save As can leave copied assets in a new
root if a later filesystem operation fails; retry into a clean destination.
There is no automatic recursive filesystem scan.

## Terrain, marker elevation and restoration

A confirmed region updates the document. Loaded terrain/satellite package IDs
are retained as references to the existing system cache. Large geographic files
are not recopied on every save. Relative `Geodata` assets can be registered for
future portable packaging; automatic portable geodata export is not implemented.

Open restores region, marker coordinates, layer visibility and package IDs.
Cached packages pass the existing importers, then feed the existing renderer.
If cache is missing/corrupt, the document keeps the references and markers; Map
→ Load in 3D rebuilds data for the restored region. The Map recenters/fits to the
restored region. New/Open cancels obsolete work and clears owned scene assets.

3D marker placement samples the existing regular DEM with bilinear
interpolation, using unchanged local east/south/up conversion:

`Bevy Y = DEM MSL - reference MSL + marker AGL`.

No elevation coverage means no 3D marker and an Inspector warning, not a marker
on an invented zero-height ground. Changing the active region immediately
invalidates old marker projections. 2D markers remain geographically preserved.
Focus Selected recenters Walkers and, when current DEM coverage exists, focuses
the 3D camera. Basic 3D selection uses the existing screen ray/sphere picker.
There are no placement gizmos, physics or arbitrary 3D-model editing.

## Satellite layers

Use the existing `STARTER_IMAGERY_CONFIG` described in
[MAP_PACKAGE_BUILDER.md](MAP_PACKAGE_BUILDER.md). Both 2D and 3D use that same
provider definition, authorization environment variable, attribution, declared
zoom range, tile size and permanent-cache permission. No imagery endpoint or
third-party licensing permission is bundled by default.

Standard uses the existing OSM Walkers source. Satellite uses a Walkers `Tiles`
adapter because Walkers 0.62 `HttpOptions` has no arbitrary Authorization-header
hook. The adapter delegates downloading, retries, response caps, cache writes
and image validation to the existing builder downloader/decoder. Bearer headers
are supplied, not bypassed or placed in URLs. Walkers still owns tile placement,
projection, cropping, pan and zoom. There is no local HTTP proxy/server.

The adapter uses one bounded worker, up to four HTTP tile requests per batch,
32 pending requests, eight completed replies and at most 64 MiB of retained
texture pixels (also capped at 128 tiles). It respects provider zoom limits,
including Walkers' tile-size zoom offset. Drop cancels new work; an in-flight
request can finish within the existing timeout. Disk tile caching shares the
provider namespace used by the existing 3D imagery builder. Reload source reads
changed configuration and Retry tiles retries the active source. Missing or
invalid Satellite displays a reason; OSM is never presented as satellite.

The existing imagery package builder, `Georeference::uv()`, JPEG importer,
`real_scene` and `satellite_scene` remain the 3D path. Satellite arrives after
terrain and changes material/UVs on existing terrain geometry. Errors preserve
DEM. Terrain/Satellite toggles do not rebuild the DEM mesh. Actual attribution
is visible in Map and in Satellite Inspector/viewport.

The viewport uses a neutral background and modest ambient/directional lighting.
A standard Bevy Gizmos group draws the existing grid vertices at 1.5-pixel line
width, with the original grid mesh retained for the existing package/UV layout.
The redundant thin mesh is hidden. New satellite installation defaults Grid off.

## Verification and limits

Normal tests use generated WebP/PNG/JPEG and local HTTP fixtures only. They cover
Bearer authorization, source zoom limits, tile reuse, package alignment/UV seams,
DEM-only behavior, stable marker projection, reference/AGL heights, missing DEM,
asset cleanup, project save/open, relative imports, missing files, symlink escape,
unsaved decisions, benchmark isolation, 10,000 demo entities and shared selection.
Existing workspace docking, builder cancellation/cache and desktop tests remain.

Run `./scripts/check.sh` for line budgets, rustfmt, workspace clippy with denied
warnings, workspace tests, release build and the single-egui-version check.

Known limits: one marker type, no undo/redo or grouping implementation, no
arbitrary model renderer, no portable geodata export, no geodata cache quota.
Resource metadata is a snapshot refreshed on open/import/Refresh. 3D ray picking
has a screen-sized tolerance and does not test terrain occlusion. Imported
packages outside the system cache must be reimported or rebuilt after reopening.
Source policy supports permanent-package permission and bearer authorization as
in M3; temporary-only licenses and other auth schemes need a later extension.

Manual/human GUI testing and automated widget/GPU execution are separate checks;
a synthetic event test is never reported as a successful manual smoke test.
Final measured results are appended below.

### Example project

[fixtures/m4-project.json](fixtures/m4-project.json) is a validated project-format
example with one Alps Survey Point, 12 m AGL, Terrain visible and Satellite hidden.
It is never loaded by ordinary startup. Open it explicitly through File → Open
Project; save to your own directory with Save As. Its terrain ID refers to the
real Alps package measured in M3. On another machine, missing cache is expected:
use Load in 3D to reconstruct the same selected area. No absolute paths or secrets
are embedded. Tests save/open this exact fixture and compare the document.

### Recorded results — 2026-11-12

- Full `scripts/check.sh` succeeded: line budgets, `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, workspace tests,
  `cargo build --workspace --release`, dependency/egui-version check.
- **96 tests passed**, including **66 starter-bevy library tests**. No public
  cartographic or paid-service HTTP requests occur in normal CI tests.
- A real native Bevy process launched on Linux/Hyprland with NVIDIA GeForce
  RTX 3050 6GB Laptop GPU / Vulkan (NVIDIA 615.78.08). Startup completed without
  logged rendering or resource-access errors; it was also closed gracefully.
- **Human/manual GUI smoke test is incomplete**: the Linux session was locked
  when visual inspection/input was attempted. No password/unlock bypass or input
  into the lock screen was attempted. Automated egui event tests are explicitly
  labeled synthetic, including actual Walkers placement and Explorer/table
  selection synchronization. Native startup is not claimed as a completed manual
  Save/Open, 2D/3D alignment, camera or docking workflow.
- Storage and ECS integration tests demonstrate save → open → restore region,
  visibility and markers → preserve coordinates without DEM → install validated
  terrain → display marker → New → release terrain/marker assets. They also
  exercise the exact documented JSON example.
- 10,000-object benchmark action and Return to Project preserve the original
  project document. One debug CPU ECS projection run measured **18.622 ms** for
  10,000 entities. This excludes GPU upload/rendering and is not an FPS guarantee.
- Live licensed satellite imagery was not configured. A local authenticated PNG
  fixture verified the 2D Bearer adapter; existing generated imagery tests cover
  3D package/UV seams and resource replacement. Live geographic alignment at
  strong camera zoom and human review remain unverified.
- New runtime dependencies: **none**. Existing `starter-ui`, `egui_ltreeview`,
  `egui_table`, Walkers, rfd, serde, image, reqwest and Bevy resources/tasks/gizmos
  are reused. `builder`, `real_scene` and `dem_mesh` are unchanged from baseline;
  satellite installation only gained cancellation and default grid-off behavior.
