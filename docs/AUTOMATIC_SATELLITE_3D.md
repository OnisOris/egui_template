# Automatic satellite terrain imagery (M5.2.2)

## Shared tiles

`TileService` owns an immutable XYZ provider configuration, authorization snapshot and
unique configuration generation. Tile coordinates and size are scoped to that source
instance: changing configuration or credentials creates a new generation. Walkers and
3D clone the same service handle. Pending requests are coalesced before enqueueing.
Four workers share one existing `Download` client; the waiting queue holds 64 tiles.
Decoded RGB8 buffers use `Arc<[u8]>`. The LRU reserves at most 128 MiB, including
queued tiles, and does not evict pinned buffers. The service never writes to disk,
even when a source independently permits package export.

The downloader retains authorization, timeouts, response-size validation and retries.
429 integer Retry-After values up to 120 seconds are respected with cancellable waits.
Longer or HTTP-date Retry-After values stop automatic retries rather than retry early.
Errors are cached for five seconds. Dropping the last service handle cancels workers.

Walkers owns a separate bounded cache of egui textures (at most 64 MiB / 128 tiles).
It converts shared RGB tiles to GPU textures; source buffers outlive adapter textures
when another consumer holds them. No egui texture is passed to Bevy.

Online 2D authorization, `allow_online_3d` and `allow_permanent_packages` are independent.
The service can run for authorized 3D even when the 2D satellite adapter is disabled.
Existing XYZ configurations deserialize with Online 3D disabled. Esri's preset remains
preview only. Enabling a capability is the application's operator's explicit assertion
of permission, not a determination of a provider's licensing terms.

## Stage a validation

Local HTTP fixtures cover bearer authorization, two consumers sharing one HTTP request
and the same decoded allocation, memory-only behavior, source/configuration generation
isolation, source zoom bounds, network failures and retries. No public imagery server
is used by tests. `cargo test -p starter-bevy --lib`: 91 passed.

## Automatic 3D and persistence

Once a matching DEM is installed, `imagery_3d` obtains the service held by
`ImageryView`; Walkers and Bevy independently clone that same service. It independently requests the confirmed region's tile coverage.
A background thread crops an RGB mosaic, converts it to RGBA and prepares UVs using
local DEM coordinates → WGS84 → `Georeference::uv()` → WebMercator crop coordinates.
Adjacent chunk vertices use the same coordinate chain. The mosaic is limited to
4096 per edge, 16 Mpixels and 256 tiles (also at most 96 MiB of pinned RGB tiles).
HTTP, decode and mosaic buffers are never produced in Bevy Update.

The request identity includes region revision, terrain fingerprint and immutable source
configuration generation. Replacing any of them cancels preparation and rejects late
results. Dropping the Map panel does not cancel active 3D imagery. Compatible installed
local packages take precedence and keep the existing import and restoration paths.

`ImagerySource` distinguishes `Package(id)` from `Live { generation }`. An absent
installed texture is the no-imagery state. Live metadata contains no filename or package
ID. Project synchronization only saves package IDs from the Package variant. Project
schema v2 is unchanged. Reopening a project restores DEM and persistent packages as
before; live imagery is requested again from authorized application settings.
Secrets and GPU handles are never serialized into projects.

`MapSources.build_packages` defaults to false and explicitly selects persistent building.
Online 3D and offline export are independent: enabling one never implicitly enables the
other. Application settings contain the source configuration; project files need no
additional fields. Legacy sources load with Online 3D disabled.

Stage b: `cargo test -p starter-bevy --lib`: 93 passed; `cargo build -p starter-bevy`
passed. Added autonomous Bevy World tests for automatic live installation, shared chunk
UVs, permission rejection, cancellation and region-change Image removal. These are not
manual GUI tests.

## Progressive detail

`imagery_lod` selects up to four existing DEM chunks intersecting a conservative camera
frustum, ordered by distance to their bounding spheres. Camera distance, viewport size,
FOV, local source resolution and max zoom determine texel density. A 0.65–1.6 density
hysteresis and 300 ms coverage debounce avoid repeated boundary switching. Each patch
is capped at 2048 per edge / 64 source tiles. The base mosaic is never rebuilt by camera
movement. Source maximum zoom is respected; no synthetic extra detail is promised.

`imagery_detail` prepares one patch at a time through the shared service. Each chunk
uses a StandardMaterial UV transform from base-crop UV → global pixels at the requested
zoom → patch-crop UV. It never mutates a DEM mesh or creates another terrain surface.
The base and installed patches remain visible while higher detail is pending or fails.
A failed patch is retried after five seconds. Moving away releases unneeded patch
materials and Images; changing source, region, base texture or hiding imagery cancels
pending patches and frees their GPU assets. Bevy Images use RENDER_WORLD ownership so
uploaded CPU image copies need not remain in the main world. Four retained patch
Images and one pending patch bound detail texture buffers; the whole-region base is
retained for fallback. Material changes occur only on patch installation/removal.

Stage c: 95 library tests passed and the application built. Local multi-zoom fixtures
prove higher zoom URLs and actual new patch installation, unchanged mesh attributes,
edge UV transform equivalence, old-texture retention after HTTP 403, and Image/material
release on zoom-out and region changes. These checks do not measure GPU upload time.

## Source configuration

`STARTER_IMAGERY_CONFIG` accepts the existing provider JSON format with the added,
backward-compatible `allow_online_3d` boolean. XYZ URL placeholders are `{z}`, `{x}`,
`{y}` (including providers with a different placeholder order). Tiles are square
256/512/1024-pixel JPEG, PNG or WebP, decoded to RGB8. Optional `bearer_env` names an
environment variable; the authorization value is read into the service's immutable
snapshot. Reload after changing credentials. Production URLs require HTTPS; local
127.0.0.1 HTTP is supported for autonomous fixtures. No new dependencies were added.

```json
{
  "name": "Your authorized XYZ source",
  "url": "https://your-provider.example/tiles/{z}/{x}/{y}",
  "min_zoom": 0,
  "max_zoom": 19,
  "tile_size": 256,
  "attribution": "Your source attribution",
  "allow_online_3d": true,
  "allow_permanent_packages": false,
  "bearer_env": "YOUR_IMAGERY_TOKEN"
}
```

The example is a schema illustration, not an available provider. Settings → Map Sources
shows Online 2D, Online 3D and Offline Package Export independently. Preview permission
or the Esri preset alone never authorizes 3D or export. A permitted existing package
remains usable without online access. The former builder message “Satellite source not
configured” no longer overrides live status; the builder reports terrain readiness.

## Final validation, 2026-11-13

All required commands passed on branch `bevy`, including the existing desktop app:

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace` — 132 tests, including 99 Bevy library tests
- `cargo build --workspace --release`
- `./scripts/check.sh`

The final script also ran with `STARTER_TEST_TERRAIN_ROOT` and `STARTER_TEST_MAP_ROOT`
pointing at the installed UAV caches. This enabled the real-data tests rather than
merely returning early: three matching DEM/satellite package pairs were installed and
replaced repeatedly. Autonomous tests also cover shared HTTP requests and RGB allocation,
independent sources and renewed authorization, bounded Walkers pending requests,
2D/3D permission independence, no preview disk writes/export, exact chunk-edge UVs,
zoom transitions without mesh changes, stale revisions, source reset, HTTP failure
fallback, Image/material removal, project Save/Open without a live package ID, markers
and native GLB placement. Tests never contact public geographic servers.

Maximum Rust source length: **320 lines**, the existing desktop `chain.rs`.
Maximum changed/new Rust source length: **270 lines**, `imagery_3d.rs`.

### Repeatable local measurements

`imagery_measurements::bounded_one_five_and_fifteen_km_regions_reuse_all_warm_tiles`
uses an in-process local HTTP fixture and distinguishable tiles at multiple zooms.
The figures below are one **debug-build CPU/local-network** run, not a provider SLA or
GPU benchmark. “Reused” counts distinct warm tiles, whereas service cache-hit counters
count ready API lookups (the queue and blocking adapter can both look up one tile).

| Region | Zoom / image | HTTP cold / warm | Reused warm tiles | Decoded RAM | Decode total | Mosaic preparation | Cold / warm total |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1×1 km | z14 / 142×141 | 2 / 0 | 2 (100%) | 0.375 MiB | 6.84 ms | 3.42 ms | 31.07 / 2.97 ms |
| 5×5 km | z14 / 705×705 | 12 / 0 | 12 (100%) | 2.25 MiB | 31.39 ms | 71.48 ms | 157.38 / 67.47 ms |
| 15×15 km | z14 / 2113×2111 | 81 / 0 | 81 (100%) | 15.19 MiB | 219.45 ms | 857.26 ms | 1595.68 / 881.08 ms |

Preparation includes RGBA conversion. The service counts actual HTTP attempts, including
retries, and reports decoded bytes and decode time. Patch installation logs retained
patch count and material update count. The maximum is one base imagery Image plus four
detail Images; the viewport and unrelated engine/model resources are additional.

### Real-window smoke and hardware observations

Automated **native Linux/Wayland OS input and screenshot inspection** used the release
application and the existing `project_smoke` entry point, on NVIDIA RTX 3050 6 GB/Vulkan.
This is a real-window automated check, **not human manual acceptance**, and is separate
from the autonomous egui/Bevy World tests.

- At 1366×768: Map → Satellite → Select Region → Confirm Region → Load in 3D loaded a
  real Mapterhorn DEM and automatically applied local XYZ imagery without importing a
  satellite package. A second selected region replaced the previous terrain/imagery.
- At 1920×1080: camera wheel zoom changed the displayed imagery from base z14 to z15,
  with two installed chunk textures and new HTTP tiles. Orbit and Pan did not rebuild
  the base mosaic. Screenshots showed the new fixture raster density on the same DEM.
- The local XYZ source deliberately serves generated test patterns at different zooms.
  This validates live streaming and texture replacement; it is not real satellite
  photography and does not validate any public provider's 3D licensing or coverage.
- An existing real Moscow UAV map-data v2 package (z15, 700×829) restored on its matching
  77×91 real DEM, alongside one marker and two GLBs. Both 1366×768 and 1920×1080 rendered
  the package correctly; Terrain/Satellite switching retained DEM and models.
- An Alps project restored its 387×387 real DEM, marker and two GLBs with Live Imagery.
  Four detail Images were retained. Perspective and Orthographic modes rendered the
  textured terrain and placed GLBs. Disabling Online 3D and applying settings removed
  live imagery, left DEM/models intact and displayed “Satellite: 3D use not enabled”.
  Reloading the authorized source started a new generation and restored Live Imagery.
- A native Save As wrote `/tmp/m522-saved-live/project.json`: terrain package ID remained
  present, satellite package ID was null, and both model records and the marker remained.
  Open Project through the native dialog restored that saved file: DEM and both GLBs
  reappeared and the 42-tile live base was prepared again from the shared warm cache.

Representative release observations (local fixture, concurrent QA activity):

- Small selected DEM: 4 base tiles in 156 ms and CPU asset installation about 0.01 ms. Another selected DEM used 12 base tiles in 340 ms.
- Alps base: 42 tiles in 1.367 s, 60.42 ms preparation, 0.02 ms CPU asset installation;
  a z16 chunk used 64 tiles in 2.182 s and 77.90 ms preparation. These timings include
  a deliberately simple local Python fixture, not an optimized remote tile server.
- One four-patch snapshot reported 164 HTTP attempts, 30.75 MiB decoded tile RAM and
  40.07 ms cumulative decode time; the base and four patches needed five material
  installations (one base assignment, four chunk replacements).
- For a 30-sample Live/GLB run: median main schedule CPU wall time **3.93 ms** (range
  1.25–5.98), median UI CPU **1.07 ms**, median frame interval **7.30 ms**. `STARTER_BENCH`
  reports `main_cpu_ms` measured from Bevy First to Last; this excludes separate render
  scheduling and is not a GPU time measurement. No fixed FPS acceptance target is used.
- That whole application process used about **616 MiB RSS** and **487 MiB GPU memory**,
  including Bevy/PBR, viewport, GLBs and imagery; these are not isolated tile costs.
  GPU upload latency and long-duration GPU allocator profiling were not measured.

## Remaining bounds

Detail is limited by source maximum zoom, four visible chunk patches and 2048-pixel
patch edges. Conservative sphere/frustum selection can retain nearby partially visible
chunks. Camera movement never increases these limits. Live first-load failure has a
specific status; use the existing Reload source / Retry tiles action to retry after
access is restored. Patch failures retry automatically while keeping the old texture.
There is no persistent preview cache, global quadtree, new mesh, GIS engine, external
UAV runtime or renderer. No public Online 3D provider was exercised: tests used the
local multi-zoom source and existing permitted satellite packages.
