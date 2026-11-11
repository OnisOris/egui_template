# Real terrain and satellite packages (M1 / M2)

The template is a read-only viewer, not a geographic data downloader. It has no
UAV workspace/path dependency and never modifies the UAV project or its cache.

## Use

1. Map: select and Confirm Region, then Import DEM… and choose a package folder.
2. If bbox/origin differs, review Use package area and Confirm Region explicitly.
   This preserves fixed package home origins, including those outside the bbox.
3. Open in 3D installs Real DEM. Demo remains a separately labelled button.
4. Viewport: Import Satellite… optionally selects a matching map-data folder.
   Terrain / Satellite switches material without rebuilding geometry.
5. Viewport: Reset Camera, Fit Terrain and Grid. A pending/new confirmed region
   does not silently replace the visible scene; a previous-region warning remains.

The optional `STARTER_TERRAIN_PACKAGE=/path/to/package` environment variable
stages the same asynchronous import on startup; it never confirms a region.

## Contracts

Supported terrain package: UAV `formatVersion: 1`, `manifest.json` plus
`heights.f32`, `encoding: little-endian-f32-row-major`, MSL samples, source
`origin-sample` or `fallback-mean` for the reference elevation. The read-only
adapter checks bounded files, sample count, finite metadata/samples, elevation
range, grid spacing and grid extent against bbox/origin. It does not copy the
network client, Mapterhorn TileJSON logic, stores, runtime or full UAV validator.

The viewer caps a DEM at 2,097,152 samples, metadata at 1 MiB, local coordinates
at ±1000 km and MSL values at ±100 km. Minimum spacing is 1 cm, additionally
constrained by local float precision. Unsupported packages produce an error.
Source bbox and origin must match the confirmed region within 1e-9 degrees;
there is no implicit clipping, resampling or silent origin substitution.

`TerrainPayload` owns one sample array. Manifest metadata is source provenance,
not a second mutable selected region. `ActiveMapRegion` remains authoritative.
`TerrainImport` owns one cancellable Bevy Task and at most one incoming package;
`RealScene` owns the visible package and its derived graphics resources. Keeping
an old visible package while a new one loads is intentional. Request revisions
are checked before accepting a task result, and bbox/origin again before install.

```text
MapRegion + revision → TerrainImport (Bevy Task)
    → source::load_terrain (read-only, bounded files)
    → TerrainPayload + content fingerprint → dem_mesh (CPU worker)
    → revision check → Open in 3D → RealScene → existing Bevy viewport
```

## Coordinates and rendering

`coordinates.rs` now matches UAV `uav-geo` v1: 111320 metres/latitude-degree,
longitude scaled by cos(origin latitude). It replaces the old radius-based
approximation for **all** local geometry. Shared known points verify both
conversion directions. Domain east/south/up maps to Bevy east/up/south; Bevy Y
is sample MSL minus reference MSL, computed in f64 before conversion to f32.

Fixed chunks contain at most 128×128 cells, as in the UAV strategy renderer.
This implementation is small and independent of the UAV runtime. Positions at
shared edges use identical full-grid indices; finite-difference normals sample
across chunk boundaries, avoiding lighting seams. The optional draped grid uses
the actual sample edges. There is no LOD, streaming, collision or vertical
exaggeration. Geometry preparation and file access happen off the main thread.

Bevy uses its existing camera, image RenderTarget, Mesh and StandardMaterial.
Editor, Demo and Real DEM use separate RenderLayers. Loading does not touch the
test-object document. A BLAKE3 content identity prevents unnecessary geometry
replacement on reopening an unchanged package. Replacing a package explicitly
removes old entities, meshes, materials and satellite images. Camera fitting uses actual 3D bounds
including relative elevations and the current dock aspect ratio.

## Verification and future source

Tests include damaged/version/size metadata, nonfinite samples, MSL mapping,
known UAV geographic points, matching chunk vertices/normals, repeated swaps
between three packages, mismatched-region rejection and obsolete task discard.
For local authoritative packages additionally run:

```bash
STARTER_TEST_TERRAIN_ROOT=/path/to/terrain-cache/packages cargo test --workspace
```

Normal CI uses generated format fixtures and needs no private files or network.
A future SetMapArea source should replace the file-reading task and return the
same validated payload with the request revision; the renderer stays unchanged.
It must preserve the runtime's fixed home origin, package identity and reference
MSL, and discard any response whose request revision is no longer current.

M1 verification: all 60 workspace tests passed, including the original 52 and
three locally prepared real packages. Strict clippy and release build passed.
Native Linux GUI automation exercised package area → Confirm → Open in 3D;
the 1025×562 real DEM rendered with the draped grid in the docked viewport.
The isolated X11 smoke test used Xvfb + Mesa lavapipe after the interactive
Wayland session ended. Synthetic mouse input is not a human UX review.

## Satellite package and georeference

Supported map-data format: v2. Only `manifest.json` and `satellite.jpg` are read;
roads/buildings are ignored and need not exist. The bbox and fixed origin must
match the loaded DEM and current confirmed revision. JPEG decoding and per-vertex
UV preparation use a Bevy Task. Results are discarded if either the region
revision or the DEM content fingerprint changed. An error preserves the last
valid scene; a DEM works without satellite data.

`SatellitePayload` contains decoded RGBA, provenance and UV attributes, no second
height model. Walkers' public Mercator projection supplies global pixel positions;
tileSize scales its 256-pixel convention. UV = (global pixel − cropPixelBounds
left/top) / crop extent. V increases south/down, matching JPEG rows. Rounded crop
bounds may include pixels outside the DEM bbox. Mapping the DEM to a plain 0..1
rectangle would shift the image and is deliberately avoided.

Limits: JPEG file 64 MiB, metadata 1 MiB, at most 16,777,216 decoded pixels and
8192 per edge. Integer crop bounds must agree with decoded dimensions, zoom,
tile size and cover the DEM. Malformed/mismatched packages are rejected. This
fixed-resolution viewer does not resample oversized images or stream textures.

Bevy stores one linear-filtered sRGB image. Initial satellite installation adds
UV attributes to the existing meshes; positions, normals, indices, entities and
handles stay unchanged. Terrain/Satellite only changes the existing material's
texture and tint. Reimporting identical content reuses the image. Replacing a DEM
releases its satellite image, and replacing satellite content releases the old
image. Viewport resizing only changes the existing render target.

The provider attribution from the package appears below the viewport controls.
The viewer grants no new imagery rights: use packages permitted by the original
provider's terms; no downloader, API keys or bulk tile cache is included.

Optional reproducible startup: `STARTER_SATELLITE_PACKAGE=/path/to/map-data`
imports once after a DEM is opened, through the same validated background path.
For local real DEM/satellite lifecycle checks:

```bash
STARTER_TEST_TERRAIN_ROOT=/path/to/terrain-cache/packages \
STARTER_TEST_MAP_ROOT=/path/to/map-data-cache/packages cargo test --workspace
```

These checks cycle three matched real packages three times and repeatedly toggle
materials, asserting stable mesh/entity/material counts and image release.
The independent fixtures exercise offset crop UVs, shared edges, V direction,
JPEG dimensions/corruption, mismatched origins, unsupported versions and stale
requests. CI requires no private project, cache, imagery or network.

Libraries remain Bevy 0.20.1 / bevy_egui 0.43.1 / egui 0.36.2 / Walkers 0.62.0;
JPEG uses image 0.25.10 (already present transitively), dialogs rfd 0.18.0 and
content identity BLAKE3 1.8.7. No fork or additional GIS engine is introduced.

M2 verification: all 66 workspace tests pass (the original 52 plus 14 new tests),
including three real DEM/satellite pairs cycled three times. `cargo fmt --all
--check`, strict workspace clippy, tests, workspace release build and the full
`scripts/check.sh` pass. Native Linux X11/Vulkan automation rendered the real
1025×562 DEM with a matching 3305×1809 Esri package and provider attribution.
The original starter-desktop also starts. GUI evidence uses isolated Xvfb and
Mesa lavapipe, not a human UX or hardware-performance review.
