# Satellite integration (M5.2.1)

## Existing packages

The UAV reference was inspected read-only: `uav-map-data/{lib,imagery}.rs`,
`uav-strategy/{map_data,terrain}.rs`, and `docs/map-data-runtime.md`.
Its map-data v2 fields match the existing MapManifest/SatelliteMetadata/Georeference.
Extra vectors, counts and ground-resolution metadata are ignored by serde; roads
and buildings are not read. Old v1 packages are intentionally rejected.

Both implementations choose zoom from latitude-dependent ground resolution,
reduce detail to fit limits, round west/north crop outward with floor and
south/east with ceil, and store actual integer global WebMercator pixel edges.
The existing builder already targets 3 m/px, 4096 per edge, 256 tiles and JPEG 90.
The Bevy decoder's existing 16 Mpixel/8192-edge import limits remain enforced.
UVs use local → WGS84 → Walkers WebMercator → crop-relative coordinates; north is
V=0 and south is V=1. No copied UAV runtime, projection or terrain mesh is added.

Map → Satellite Packages → Import Existing Package opens a folder dialog.
Viewport → Import Existing Package opens the package browser, which also offers
the dialog. Packages can be decoded before DEM and wait for a matching surface.
Both bbox AND origin must match. A mismatch displays both coordinate systems;
Use Package Region explicitly confirms the package bbox and original origin.
The user then loads a matching DEM. Nothing is stretched onto incompatible DEM.

The optional directory defaults to `$UAV_MAP_DATA_CACHE_DIR/packages`, or
`~/.cache/uav-hit-estimator/map-data-cache/packages`. Scan examines at most 64
immediate entries, validates each JPEG off-thread using the existing decoder,
and ignores invalid packages. It never recursively searches the home directory
or writes, removes or repairs files in the source cache. Any package folder can
be selected through rfd. External cache paths are not part of project JSON.

SatelliteImport retains one pending payload and bounded task. Obsolete results
are discarded by region revision/terrain fingerprint; project reset clears the
pending package. Existing satellite_scene installs one sRGB Image, updates the
existing mesh UV attribute and changes the material. Reimport uses the fingerprint;
replacement explicitly releases the old Image. Terrain/Satellite changes the
material only. The DEM and GLB/marker projection remain unchanged.

## Source permissions

Online Preview and persistent Licensed Package export are separate capabilities.
Esri export is not enabled automatically. See official guidance:
- https://support.esri.com/en-us/knowledge-base/problem-unable-to-export-a-tile-package-from-the-world--000014855
- https://developers.arcgis.com/documentation/mapping-and-location-services/mapping/basemaps/introduction-basemap-styles-service/

Online 3D tile mosaics are not enabled for Esri without confirmed permission for
that mode. The supported 3D paths are existing authorized packages and an explicitly
licensed ImageryProvider. Public tile availability does not grant export rights.

## Online Preview settings

Settings → Map Sources offers an Esri World Imagery preset, explicit online-access
confirmation, optional bearer-token environment variable name and package directory.
Apply and Save writes application `map-sources.json` beside layout.json, never into
project JSON. No token value is persisted. Existing STARTER_IMAGERY_CONFIG remains
supported and overrides the saved source on restart; both Walkers and the licensed
builder use the same provider configuration. Disabling authorization disables use.

Esri preset: 256 px XYZ, z0–19, URL `/tile/{z}/{y}/{x}`. Coordinate x remains eastward
and y southward. Preview validates URL/dimensions independently of export permission.
The preset always sets allow_permanent_packages=false. The automatic imagery builder
skips preview-only sources; it continues preparing DEM without satellite export.

ImageryTiles still implements Walkers Tiles, reusing the existing authorized HTTP
loader (4 simultaneous requests, bounded queue/replies, 25 s timeout, at most three
attempts). Preview explicitly bypasses persistent tile reads AND writes. Only the
bounded texture LRU (at most 64 MiB / 128 textures) retains visible-map tiles. No
area prefetch, package assembly or 3D mosaic occurs in Preview. Dropping the adapter
cancels its job and releases texture handles. Licensed providers retain their existing
persistent-cache/package path. Failed tiles have a five-second retry cooldown;
Retry tiles / Reload source also recover. HTTP status diagnostics remain visible.

The user explicitly confirmed online-preview rights without a token in this session.
That confirmation does not authorize Esri package export or a temporary 3D mosaic.

## Restore and failure handling

Project JSON still stores geographic package IDs, not external absolute paths.
When its owned cached map-data directory is missing, SatelliteImport can resolve
`<catalog>/<package-id>` in the configured Existing Map Package Directory (or the
UAV default/environment root). The loaded manifest ID must match the saved ID;
bbox/origin and JPEG are then checked by the same decoder. Only this known cache
reference is eligible for lookup; arbitrary paths are not redirected. This
restores UAV packages after project reopen without copying their imagery into
persistent application storage. One-off folders with different names can still
be imported manually; automatic restore requires the ID-named catalog layout.

Packages can be inspected while a previous region's DEM remains visible. They
wait until the current confirmed DEM matches. Project reset cancels pending work;
manual imports supersede automatic imagery preparation after DEM is ready.
Successful import closes the browser and selects Satellite. Mismatch detail is
shown once, in the browser while it is open, with both bbox/origin values.
A DEM-only viewport reports `DEM Ready · Satellite source not configured`.

## Verification — 2026-11-12

Each stage ran the required fmt, Clippy (warnings denied), workspace tests, release
build and `scripts/check.sh`. Stage a: 116 tests; stage b: 119 tests. Final stage: 123 tests (including doctests), with compatibility, asynchronous
import, retry and catalog-resolution coverage.
No new dependencies or crates were added. Maximum Rust length remains 320 lines
(the existing desktop file); changed files remain at most 300 lines.

CI uses owned generated RGB/JPEG/PNG pixels and local HTTP servers only. Tests cover:
- A real UAV v2 manifest with extra vector fields and a non-central origin, valid
  JPEG dimensions, malformed JPEG/version/crop/IDs and origin mismatch.
- Packages decoded before DEM, deferred UV preparation, stale requests/cancellation,
  importing while previous DEM is visible, and preservation of marker/model data.
- Repeat import/fingerprint reuse, replacing Images, three-region reload loops,
  fixed Mesh/Material/entity counts, and Terrain/Satellite material switching.
- Known WebMercator anchors and different latitudes, crop-relative UVs and shared
  chunk boundaries, source zoom/tile-size constraints and Esri URL x/y order.
- HTTP 401/403/404/429, bounded retries, recovery after a failed preview, bearer auth,
  memory cache reuse, and bypass/preservation of an existing persistent tile file.
- Preview-only provider rejection by export validation and automatic-builder skip.

Actual Linux/Wayland release GUI checks used OS mouse/keyboard input, rfd dialogs
and screenshot inspection on NVIDIA RTX 3050/Vulkan; this is not human manual UX
acceptance or merely synthetic widget input:
- Configured Esri through Settings → Map Sources, confirmed the user's authorized
  online preview without a token, applied settings, and received real Esri tiles.
  Connected status, attribution, pan/zoom and Standard/Satellite switching worked.
  The preset's persistent-package permission remained false.
- Loaded the existing Moscow UAV v2 package (700×829, z15) over its matching actual
  Mapterhorn DEM (77×91, approximately 24.7 m spacing). Origin longitude 37.62 is
  distinct from bbox centre 37.625. Satellite/Terrain switched on the same surface.
- A deliberately altered origin (37.621) with identical bbox was rejected. The DEM
  remained visible and both origins were shown; importing the compatible package
  through the folder dialog restored Satellite automatically.
- Opened a test project with one marker and two native GLB instances; all three
  remained present/selectable on the textured terrain, including close camera zoom.
  Verified Viewport resize from 1366×768 to 1920×1080.
- Saved and reopened that project without STARTER_TERRAIN_PACKAGE or
  STARTER_SATELLITE_PACKAGE; its external satellite was restored by saved ID from
  the read-only UAV catalog, with marker/model IDs 1/2/3 preserved.
- In a new empty project, imported the package before DEM, adopted its exact
  region/origin and clicked Load in 3D. The existing MapPackageBuilder prepared
  a real 77×91 DEM from cache in 0.018 s; the pending package applied automatically.
  This was a cache-hit observation, not a cold-network or GPU-upload benchmark.
- Scanned the selected UAV package directory in the real browser; valid cards
  included ID, bbox/origin, dimensions, zoom, source and compatibility, while the
  UI remained available during decoding. No satellite.jpg was published into the
  application's persistent package cache during these preview/import checks.

## Limits

No Esri tile-package export or online 3D mosaic is implemented/authorized by the
preview setting. 3D uses an existing permitted package or an explicitly licensed
provider. There is no new GIS/DEM/GLB renderer, camera, server or UAV-workspace
runtime dependency. Source packages and the reference repository remain read-only.
Satellite detail remains limited by the source JPEG; zooming cannot add resolution.
GPU upload latency and long-running GPU-memory profiling were not measured.
