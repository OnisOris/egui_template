# Map Package Builder

The builder prepares packages for the existing `load_terrain()` and
`load_satellite()` adapters. It has no dependency on panels, cameras or docking.
The existing Bevy scene owns GPU assets and releases them when replacing terrain.

## Terrain source and coordinates

Mapterhorn: https://mapterhorn.com/data-access/ and
https://mapterhorn.com/attribution . Terrain tiles are 512px Terrarium WebP.
TileJSON is fetched and validated, then cached. Its template, encoding,
attribution, tile size, coverage bounds and zoom limits drive requests. When
zoom limits are absent (as in the live November 2026 response), use documented
global zooms 0–12. Higher regional PMTiles zooms are deliberately not inferred.
Missing coverage at a selected zoom triggers a lower zoom; other HTTP failures
are reported. Delete the metadata cache to refresh changed provider metadata.
Terrarium encoding precision is not the accuracy of the original elevation survey.

Projection uses `walkers::mercator::project`, scaled from Walkers' 256px world to
the actual tile size. Raster coordinates refer to pixel edges; interpolation
subtracts 0.5 to locate pixel centers. DEM requests include the neighboring
samples, including across tile boundaries. Exact east/south coverage edges do
not add tiles for imagery. At world edges samples clamp to the last pixel center.
No antimeridian crossing is supported.

The regular output grid includes the exact bbox edges, north first, west first,
row major. Local coordinates use the existing east/south/up conversion and the
selected independent origin. Grid dimensions obey the configured sample/edge
limits; source resolution limits the actual grid spacing. Heights are decoded as
`256*R + G + B/256 - 32768` and interpolated in f64, stored as f32 MSL.
Origin elevation is sampled from source tiles, fetching extra tiles if needed.
If unavailable, the manifest records `fallback-mean`; this is never silently
reported as an origin measurement.

## Packages and cache

`terrain/manifest.json` is terrain v1; `heights.f32` contains little-endian f32
MSL heights. Every newly written package passes the existing full importer before
publication. IDs hash region, origin, format, source metadata, zoom and settings.

System cache: `starter-bevy/geo/{metadata,tiles,packages,locks}`. Tile namespaces
hash source identity. Tile writes use unique staging paths and atomic rename.
Packages use OS file locks (released after process exit), staging directories,
validation and directory rename. Corrupt packages are rebuilt; corrupt decoded
tiles are removed and reported so the next attempt can fetch them again.
DEM and imagery have independent identities and locks.

HTTP requests have connection/overall timeouts, an 8 MiB response cap, three
attempts with bounded backoff, and at most the configured 1–6 simultaneous tile
requests (default four). HTTP errors never include credential-bearing URLs.
Missing/transparent/corrupt elevation data is never replaced with zero.

## Background execution and UX

`Load in 3D` creates a request ID, snapshots revision/bbox/origin, and replaces the
single pending request. One lazy background worker owns one reusable blocking
reqwest client (and its internal HTTP runtime); scoped tile workers obey the
parallelism limit. No network `block_on()` occurs in Bevy Update. Result queue is
bounded to two events. The Bevy bridge prepares CPU meshes off-thread, checks the
request identity, hands terrain to the existing scene, then installs satellite
only against the same terrain fingerprint. A newer region or manual DEM import
cancels the builder. In-flight blocking HTTP may take up to its 25s timeout to
finish; cancellation immediately prevents scene installation, stops further
requests and interrupts retry backoff. No runtime is created per request.

The existing manual imports, Demo, material toggles and camera controls remain.
Map's `More…` contains manual DEM and Demo. Import a terrain package, confirm its
area, then use `Load in 3D` to open that imported package. Satellite manual import
remains in the viewport toolbar. Progress reports tile counts, never invented
percentages. Actual DEM spacing and reference source appear beside terrain info.
`Partial · Satellite source not configured` is an expected DEM-only result.

## Configuration

Optional `STARTER_MAP_BUILDER_CONFIG=/absolute/path/builder.json` accepts a JSON
object with any of these fields (omitted fields use defaults):

```json
{
  "step_m": 25.0,
  "max_edge": 1025,
  "max_samples": 2097152,
  "max_tiles": 256,
  "http_parallelism": 4,
  "imagery_step_m": 3.0,
  "imagery_max_edge": 4096,
  "imagery_max_pixels": 16777216,
  "jpeg_quality": 90
}
```

Satellite imagery is opt-in. Obtain permission for downloading, assembling and
permanently retaining imagery from your provider. Configure an XYZ endpoint via
`STARTER_IMAGERY_CONFIG=/absolute/path/imagery.json`, outside the repository:

```json
{
  "name": "Your licensed satellite source",
  "url": "https://your-authorized-host.example/{z}/{x}/{y}.jpg",
  "min_zoom": 0,
  "max_zoom": 18,
  "tile_size": 256,
  "attribution": "Required copyright / license attribution",
  "allow_permanent_packages": true,
  "bearer_env": "STARTER_IMAGERY_TOKEN"
}
```

Set `STARTER_IMAGERY_TOKEN` in the launching environment. Do not put API keys in
Git or URLs; the built-in authorization mechanism supports bearer tokens via
environment variables. Sources allowing only temporary caches are rejected;
no permanent package is created. `tile.openstreetmap.org` is rejected. Providers
such as Google, Esri or MapTiler have no bundled permission or default endpoint.
Use only an endpoint licensed for your use. HTTPS is required except loopback
HTTP for local fixtures/services. The app itself requires no local server.

Satellite zoom obeys the declared provider limits and output limits. Pixel crop
edges round outward to integer pixel edges (the existing v2 validator requires
integers). Thus JPEG size exactly equals crop width/height, while bbox stays
unchanged. Existing `Georeference::uv()` accounts for the subpixel bbox offset;
there is no synthetic 0..1 rectangular unwrap. Output JPEG uses RGB/sRGB colors.

DEM and imagery packages are stored separately under their own hashed IDs;
missing imagery never invalidates a valid DEM. There is no cache eviction quota.
Remove `geo` while the application is closed to clear cached data. Old staging
folders are cleaned on access after a 60s grace period; active writers hold OS
locks, so concurrent processes cannot publish over an active package build.

## Validation

Offline tests use generated lossless Terrarium WebP and JPEG images and an
in-process loopback HTTP server. Normal tests never contact public tile servers.
Run `./scripts/check.sh` for the repository's required checks. GPU/interactive
results and measured live timings are recorded below after the final smoke run.

### Recorded QA (2026-11-12)

- `scripts/check.sh`: rust line budget, rustfmt, workspace clippy with warnings
  denied, workspace tests, release build, and single egui-version check passed.
- 51 starter-bevy library tests, including 13 builder tests: projection, exact
  tile edges and interpolation seams, sample budgets, Terrarium, missing/corrupt
  data, package roundtrip, origin outside bbox, HTTP retry/timeout/cancellation,
  cache reuse/repair (including offline lower-zoom reuse), explicit origin fallback,
  file locks/staging cleanup, stale request/revision rejection,
  imagery stitching/crop/UV/JPEG roundtrip and DEM preservation on imagery failure.
  Other workspace tests passed too. HTTP fixtures run entirely on loopback.
- Real native GUI on NVIDIA GeForce RTX 3050 6GB Laptop GPU, Vulkan/NVIDIA
  615.78.08: Map selection/confirmation, Load in 3D, replacement of Moscow terrain
  with another region, Alps loading, cache reload, orbit/pan/wheel, Map tab close
  and restoration through View, and viewport resizing exercised. Existing asset
  replacement tests also verify mesh/material/image release across three packages.
- Live Alps bbox W 7.837405… E 8.043459… S 46.548079… N 46.661002…,
  approximately 15.754 × 12.577 km: 632 × 505 samples, ~25 m grid, elevations
  587.6–4094.7 m MSL, `origin-sample` reference. Package ID
  `9050e7829692c7e26a8963236bec3ca0b5f3a32391e029766dcb289b59fcfbe7`.
  Initial network + preparation + CPU mesh build: **3.074 s**; package cache +
  CPU mesh build: **0.074 s**. CPU scene asset installation: **0.110 ms** initial,
  **3.091 ms** on reuse (includes discarding unused newly prepared CPU geometry).
  These are one debug-build run on this machine, not performance guarantees.
- GPU upload/completion latency is **not measured**. The scene installation log
  explicitly labels its CPU-only measurement. No artificial FPS cap was added.
- A licensed live satellite endpoint was not configured: real GUI verified
  DEM-only/Partial behavior. Satellite assembly and installation are covered by
  local generated image tests, not by a live licensed satellite download.

### Remaining limits

No antimeridian, regional PMTiles discovery, persistent quota/eviction, automatic
metadata refresh, temporary-only imagery licenses, or non-bearer authentication.
Cancelling an already running HTTP read can leave the single worker waiting for
its bounded timeout; stale results are discarded immediately. Imported formats
and the existing renderer are unchanged. The source's actual survey resolution
and coverage constrain quality; lower zooms deliberately produce coarser grids.
