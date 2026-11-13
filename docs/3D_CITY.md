# Geographic city layers (M5.3)

M5.3a–c implement packages, automatic loading and scene modes. M5.3d adds Explorer
groups, diagnostics, lifecycle regressions and the validation evidence below.

## Sources and geometry

Vector data is independent of the DEM, satellite TileService, markers and imported
GLB assets. UAV map-data v2 vectors use the original field/enum order and bincode
v2 serde standard configuration. Imports validate the entire consumed binary,
manifest version, exact bbox/origin, count/byte/coordinate limits, finite heights,
closed valid polygons and holes. Satellite import is independent; reading vectors
does not replace an installed satellite image.

The local reference workspace is read only. No UAV runtime/crate is linked.
`bincode 2.0.1`, `mvt-reader 2.5.0`, `geo 0.31.0` and `earcutr 0.5.0` are used for
compatibility, MVT decoding, polygon/line clipping and roof triangulation. The
existing dependencies do not implement these formats/operations. HTTP continues
to use the existing bounded `builder::Download` client, timeout and retry policy.

Local coordinates use f64 east/south metres relative to the package origin;
rendering uses Bevy f32 east/up/south. Buildings have roofs, courtyard holes,
MultiPolygon components, explicit outward wall normals and upward roof normals.
Combined geometry samples representative DEM ground per polygon and adds
min_height/height relative to the DEM reference elevation. Missing ground skips
the geometry. Roads use category widths, at most 20 m between height samples,
and a 0.15 m surface offset. Unknown terrain never gets bridged.

Triangles are batched in 512 m spatial cells by layer/material. No wall, roof or
road segment gets its own entity. Region changes remove owned entity and asset
handles without deleting project objects. CPU mesh preparation and imports run
on background tasks; GPU asset installation happens on the Bevy main world.

## Validation

Stage a tests use authored bincode and DEM fixtures, including courtyard roof
area, MultiPolygon, clipped boundary crossings, mismatched origin, corruption,
trailing binary data and elevation draping. No public server is used by tests.

GPU upload completion is not inferred from CPU asset insertion time. Native GUI
validation, source comparisons and measured timings will be recorded separately.

Stage b reads OpenFreeMap TileJSON before planning z12–14 coverage (at most 64
source tiles). It resolves metadata templates, extracts only building and
transportation, excludes tunnels, clips to the selected bbox, removes duplicate
clipped geometry and validates the result. The processed cache is keyed by source
and exact region; binary content hashes detect corruption. The existing downloader
limits parallel HTTP requests to four and each response to 8 MiB.

Vector downloads are independent background tasks and can finish before DEM.
Geometry waits for a matching terrain fingerprint; obsolete region/task handles
are discarded and cancellation stops subsequent requests. An error in vectors
does not clear the DEM, satellite imagery, GLB assets or project markers.
An authored 125-byte MVT fixture and local HTTP TileJSON server test automatic
loading, real MVT decoding, source height, cache reuse, cancellation and coverage
limits without contacting OpenFreeMap in CI.

## Presentation and persistence

`GeoViewMode` is separate from Editor/Demo/Real scene loading:

| Mode | Ground | Buildings/roads | User objects |
|---|---|---|---|
| City | Flat, unlit satellite ground | Extruded / flat ribbons | Flat ground + AGL |
| Terrain | Original DEM and satellite | Hidden | Sampled DEM + AGL |
| Combined | Original DEM and satellite | Representative DEM bases / draped roads | Sampled DEM + AGL |

City uses one four-vertex geographic ground patch per existing DEM chunk, sharing
its material and geographic imagery coverage. Corner UVs use the same f64 local
→ WGS84 → WebMercator crop mapping. Adjacent patches share exact UVs. The original
DEM meshes are retained unchanged; changing modes swaps mesh handles, updates
bounds and visibility, and never requests new vectors or changes stored heights.
Within a flat patch the GPU interpolates Mercator UVs between geographic corners.

Vectors prepare both City and Combined geometry off-thread once per dataset and
terrain fingerprint. Two shared StandardMaterials serve all chunks. Terrain mode
hides those entities; returning to City/Combined reuses them. Flat ground and
vector asset handles are explicitly released on region/project replacement.

New projects use schema v3 and City view. Legacy v1 projects migrate through v2;
v2 migrates to v3 with Terrain view, retaining Terrain/Satellite layer IDs,
visibility, bbox/origin, marker/model coordinates, asset paths and stable IDs.
Roads/Buildings store validated cache IDs, visibility and the vector source enum.
No ECS/GPU handles, temporary satellite package IDs or secret URLs are serialized.
Missing vector cache is diagnosed with Reload vectors / Import Vectors actions.
Restoring a valid cache does not dirty the project or redownload source tiles.

## Package compatibility and limits

The imported manifest is the UAV map-data **v2** camelCase manifest: `formatVersion`,
`mapDataId`, `bbox`, `origin`, `satellite` metadata and a `vectors` object. The
satellite file itself is not required for vector import. `vectors` contains
`provider`, `zoom`, `tileCount`, `roadCount`, `buildingCount`, `roadsFile`,
`buildingsFile`, `binaryFormat` and `attribution`. Filenames must be `roads.bin`
and `buildings.bin`; binaryFormat must be `bincode-v2-serde`.

Binary structs follow the reference field order:

```rust
LocalPoint2 { x: f64, y: f64 }
RoadFeature { id: String, class: RoadClass, points: Vec<LocalPoint2> }
BuildingPolygon { rings: Vec<Vec<LocalPoint2>> }
BuildingFeature {
    id: String, polygons: Vec<BuildingPolygon>, height_m: f32, min_height_m: f32
}
```

RoadClass variant order is Motorway, Trunk, Primary, Secondary, Tertiary, Minor,
Service. Exterior rings precede holes. Closed rings, finite coordinates and valid
polygons are required. Each binary is bounded to 64 MiB, combined feature counts
to 100,000, input points to 2,000,000 and estimated mesh vertices to 8,000,000.
Coordinates outside the confirmed bbox are rejected on import; online geometry
is clipped first. A shared 1e-9 degree tolerance accepts JSON round-trip noise
(under 0.2 mm), not a different geographic region.

Non-finite heights reject an offline package. Finite but inconsistent heights
(negative base, top below base, or top above 2000 m) skip that building and report
the count. Online height priority is render_height → height → **approximate 12 m**;
base priority is render_min_height → min_height → 0. OSM heights are not surveyed
measurements. A missing tag is not evidence that the default height is accurate.

Imported packages get content-derived cache IDs, independent of their external
mapDataId. The processed vector cache retains region/source/fingerprint and
hashes of both binaries; corrupt entries are rejected and automatic sources can
rebuild them from bounded tile downloads. HTTP attempts, processed-cache reuse,
tile-cache hits, decoding and loading times are exposed by `vector_probe`.

## Native window evidence

The screenshots below are from the running Linux/Wayland Bevy application on an
NVIDIA RTX 3050 6 GB (Vulkan), with real Moscow/Kremlin geography and live satellite
imagery. They show **624 buildings, 508 roads, one marker and two GLB instances**.
These are OS-input smoke checks, not human manual acceptance.

| City: flat satellite and extrusions | Terrain: original DEM | Combined: DEM and extrusions |
|---|---|---|
| [City](screenshots/m53/city.png) | [Terrain](screenshots/m53/terrain.png) | [Combined](screenshots/m53/combined.png) |

[The running MapLibre reference](screenshots/m53/maplibre-reference.png) was only
observed; no source files or projects in the reference repository were edited.
The Kremlin outline, courtyard buildings, river embankment and bridge occupy the
corresponding locations on the satellite imagery. Bevy uses the same source-layer
and height precedence as ensureCity3dVectorLayers, with independent lit roofs and
walls. MapLibre has different camera/projection, line joins/casings, opacity and
lighting. This is a qualitative landmark comparison, not a pixel match or a
surveyed positional-error measurement.

Switching City → Terrain → Combined retained GLBs/marker and hid vectors only in
Terrain. Save wrote schema v3 with Combined and stable geographic object records.
Restarting the latest `project_smoke` executable restored Combined, both GLB
roots, marker, DEM, live satellite and the cached 56 vector chunks; the project
remained clean. [Restored project](screenshots/m53/restored.png).
The prior native run also exercised Select Region → Confirm Region → Load in 3D;
the screenshots in this continuation start from its loaded project. This
continuation also selected and confirmed a new, overlapping area in the Map
panel and pressed Load in 3D: **481 buildings / 303 roads**, matching new DEM and
satellite, replaced the previous scene automatically (16 vector entities/meshes,
two materials, 46.442 ms CPU mesh preparation). Both model records and the marker
survived. [Region replacement](screenshots/m53/region-replacement.png).
File → Open Project through the native dialog then restored the original saved
region, all layers and all three objects, with Inspector showing Saved. The
saved JSON was byte-for-byte unchanged by the replacement/reopen checks.
Automated World tests independently cover replacement, stale results and asset
cleanup.

## Performance evidence

The native debug run prepared both modes for 624 buildings / 508 roads in
**78.235 ms**, then inserted **56 entities, 56 meshes and 2 shared materials** in
**0.322 ms**. Entity/mesh counts include both flat and draped variants; only the
selected variant is visible. CPU insertion is not GPU upload completion.

Repeatable authored-data debug measurements on this machine, using
`cargo test -p starter-bevy bounded_batches -- --nocapture`:

| Buildings | Chunks per mode | Vertices per mode | City CPU mesh | Combined CPU mesh |
|---:|---:|---:|---:|---:|
| 32 | 2 | 960 | 0.436 ms | 0.318 ms |
| 1,000 | 4 | 30,000 | 5.878 ms | 8.056 ms |
| 10,000 | 16 | 300,000 | 56.670 ms | 65.987 ms |

The fixture checks exact vertex counts and bounded batches. Its simple boxes are
not a claim about complex OSM footprints or 10,000 real buildings in the chosen
region. Both modes retain their mesh assets to avoid rebuilding on mode changes.
The application logs per-frame main schedule CPU wall time separately from frame
interval; neither number measures GPU execution. Concurrent compilation and
other desktop applications make these observations unsuitable for an FPS target.

On the restarted debug application, 30 one-second observations after startup gave
median main CPU **12.50 ms** (9.36–24.74), UI **3.27 ms** (2.53–9.57), and frame
interval **23.76 ms** (21.78–66.88). The model-focus probe uses a close camera view;
these are whole-application main schedule wall times, not isolated vector costs.
Mesh preparation on that restart was 65.812 ms and CPU insertion was 0.212 ms,
with the same 56 entities/meshes and two materials.

A fresh real OpenFreeMap load for the pictured region used **four z14 tiles** and
**five HTTP requests** (TileJSON plus tiles): 624 buildings / 508 roads, 2103.952 ms
total, 448.374 ms decode/clip/validation. Five inconsistent building heights were
skipped. City/Combined meshes each contained 28 chunks / 148,830 vertices and
took 42.192 / 31.419 ms. An immediate repeat used the processed cache, **zero HTTP
requests**, the identical fingerprint and 93.606 ms total. These are one debug
run with a public source, not CI fixtures or guaranteed provider timings.

## Required checks (2026-11-13)

All passed on `bevy` after the lifecycle fix:

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace` — **147 passed**, including 112 Bevy library tests
- `cargo build --workspace --release`
- `CARGO_BUILD_JOBS=1 ./scripts/check.sh` — exit 0, including egui version check

Checks ran locally; no hosted CI run is claimed. Serial compilation avoids
simultaneously linking several large debug Bevy examples on this 16 GiB host.
Maximum Rust file length is 320 lines; vector runtime is 312 (above the 300-line
target, below the 400-line hard limit). No new crates were created.

Regressions include bincode corruption/trailing bytes, mismatched origins, holes
and MultiPolygon, clipping/crossing roads, both road-edge DEM samples, City height
invariance, marker/GLB projection, mode/layer visibility, schema migrations,
stale region tasks and asset removal, failed reload retaining DEM/vectors,
JSON round-trip noise installing only once without dirtying a restored project,
and source changes cancelling a download while retaining current mesh preparation.
Local MVT/HTTP fixtures also assert cold/warm HTTP metrics and corrupt-cache repair.

## Remaining limitations

- Representative DEM bases can intersect steep ground. Diagnostics report ground
  spans over 3 m and missing coverage; the native scene reported 267 polygons
  over that threshold, maximum 15.14 m. Buildings are rigid, not DEM deformation.
- OSM coverage, building heights and detail vary by source zoom/date. MVT tile
  boundary fragments can remain separate features; exact duplicate clipped
  geometry is removed, rather than reconstructing whole cadastral buildings.
- Roads are category-width ribbons with a 0.15 m offset, without MapLibre's round
  joins/casings or bridge engineering. DEM is sampled on both ribbon edges at
  intervals no longer than 20 m; unknown terrain segments are skipped.
- GPU upload latency, long-duration allocator profiling and a native scene with
  10,000 real source buildings remain unmeasured. No new renderer, shaders,
  quadtree, GIS framework or UAV workspace dependency was introduced.
