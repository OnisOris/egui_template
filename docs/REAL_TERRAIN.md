# Real terrain packages (M1)

The template is a read-only viewer, not a geographic data downloader. It has no
UAV workspace/path dependency and never modifies the UAV project or its cache.

## Use

1. Map: select and Confirm Region, then Import DEM… and choose a package folder.
2. If bbox/origin differs, review Use package area and Confirm Region explicitly.
   This preserves fixed package home origins, including those outside the bbox.
3. Open in 3D installs Real DEM. Demo remains a separately labelled button.
4. Viewport: Reset Camera, Fit Terrain and Grid. A pending/new confirmed region
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
removes old entities, meshes and materials. Camera fitting uses actual 3D bounds
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
