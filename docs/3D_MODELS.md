# 3D models (M5.1 / M5.2)

## Native assets

Bevy remains pinned to 0.20.1. The `bevy_gltf` feature supplies the native GLB
loader and pulls in `bevy_world_serialization` (including its world spawner).
`mesh_picking` supplies Bevy's existing `MeshRayCast`; PNG/JPEG features support
embedded raster textures. The direct `gltf = 1.4.1` dependency is the same parser
already used by Bevy, with default importer features disabled.

Import checks `.glb`, the 12-byte GLB header, version 2, exact declared length and
full parser validation. External buffers/images and unknown required extensions
are rejected. The default file limit is 100 MiB; `STARTER_MODEL_MAX_MIB` accepts
1–1024 MiB. Imports run on the existing project I/O task, with no main-thread
file dialog or model decode. The source file is read once into a bounded buffer;
the validated bytes are published into `Assets/Models` without overwriting an
existing resource. Images keep the pre-existing import behavior.

A single `models://` source, registered before AssetPlugin, reads the app's
system cache `starter-bevy/models`. Project files are resolved with the existing
relative-path and canonical containment checks. A bounded background preparation
queue (four jobs) validates and hashes each GLB, repairs corrupt cache entries,
and atomically publishes `<blake3>.glb`. AssetServer receives only controlled
content filenames. The original project GLB remains authoritative; cache copies
can be rebuilt. No asset source is registered per file or project.

One strong GLTF/WorldAsset handle pair is held per project resource; AssetServer
shares content-identical resources. Hierarchies use `WorldAssetRoot` and the
native world spawner. Removing a root removes its children; releasing the last
project/root handles lets Bevy release dependent meshes/materials/images. GLB
cameras, lights and animation playback are disabled. Models use existing render
layer 3 and the existing Viewport camera.

`ModelGeometry` observes `WorldInstanceReady`, propagates RenderLayers to mesh
children and computes local bounds with each descendant transform. Vertices
are left untouched. A scene without finite mesh bounds is reported as unsupported.

## Reproducible native fixture check

The project-owned `tests/fixtures/box.glb` contains a box under two transformed
nodes. It has no third-party content or external files. Expected root-local
bounds are `(2, 5, -1)` to `(6, 11, 1)`, deliberately above the origin to expose
pivot/anchor mistakes. Tests use the real AssetServer, GLTF loader and world
spawner, without a GPU or network requirement.

Run the actual application Viewport with a fixed-position fixture:

```sh
cargo run -p starter-bevy --example model_smoke --release -- 1
cargo run -p starter-bevy --example model_smoke --release -- 10
cargo run -p starter-bevy --example model_smoke --release -- 100
```

The example activates the existing Viewport, keeps the user document empty and
prints `GLB_BENCH` when all scene bounds are ready. It does not modify a project.
Use an isolated `XDG_CONFIG_HOME` if the test layout should not persist into the
normal workspace.

## M5.1 verification

On Linux/Wayland with NVIDIA RTX 3050 6 GB / Vulkan, the native fixture was
visually confirmed inside the existing Viewport. These checks used OS input and
screenshot inspection; they are not a human manual UX acceptance test.

Initial debug-build observations (CPU readiness, including native world spawning):

| Instances | Readiness ms | Mesh entities | Unique mesh assets | Transform component writes µs |
| ---: | ---: | ---: | ---: | ---: |
| 1 | 214.329 | 2 | 2 | 10.356 |
| 10 | 190.496 | 11 | 2 | 15.309 |
| 100 | 326.884 | 101 | 2 | 23.578 |

Entity/asset totals include the application's pre-existing hidden editor ground
plane. The fixture contributes one shared mesh asset at every instance count.
The write measurement covers one direct ECS component-update pass; it excludes
transform propagation and GPU work. These short local observations are not a
rendering throughput or GPU upload benchmark.

The native-loader regression confirms shared handles, descendant bounds,
independent hierarchy deletion, and release of mesh assets after the final root
is deleted. Boundary/cache and import/Save As regressions pass. M5.1 workspace
suite: 102 tests; Clippy with warnings denied passes.

## Geographic instances (M5.2)

`starter-core::ModelInstance` stores a stable ObjectId, name, relative GLB path,
WGS84 coordinates, AGL, yaw, uniform scale and visibility. Schema 2 adds a
`models` list alongside the unchanged marker list. Opening schema 1 migrates it
in memory, validates the entire document and saves schema 2 next time. IDs share
one sequence across markers and models. Bevy handles/entities are never saved.
The desktop application's separate document and benchmark remain unchanged.

`ProjectDocument::object_rows()` is a borrowed view used by the existing Explorer,
virtualized table and Walkers overlay. Selection is one `ProjectSelection`.
Inspector name drafts remain transient, commit on Enter/blur/selection change,
and retain invalid text when returning to an object. Numeric fields use existing
DragValue editing and core validation: finite WGS84/AGL/yaw, scale 0.001–1000.

To place a model:

1. Save a project, then use File → Import Asset (or Files → Import).
2. Select the GLB under Assets → Models and click **Place on Map**.
3. Click the existing Walkers map; Escape cancels without creating an object.
4. Load DEM, select the instance and edit its position/AGL/yaw/scale in Inspector.
5. Save and reopen the JSON. Save As copies imported GLBs to the new project.

Without DEM coverage, the instance remains in the document, Explorer, table and
2D overlay; Inspector reports missing surface elevation and no 3D root is made.
With coverage, existing coordinate/elevation helpers compute east/south and
`terrain MSL - reference MSL + AGL`, using f64 geographic calculations. Bevy gets
local f32 Transform values, one unit per metre. The child-transformed minimum Y
is subtracted after uniform scaling, aligning the lowest geometry with AGL.
Yaw rotates around world Y; imported mesh coordinates are preserved.

`models/instances.rs` holds only derived ObjectId → root mappings and read-only
loading/bounds information. Transform/visibility edits update the corresponding
root without loading its GLB again. Region changes reproject objects; missing
coverage, deletion and project changes remove native hierarchies. Project changes
also drop old registry handles/tasks. File refresh retries failed/missing assets.
Removing a used GLB is disabled and checked again in storage; deleting an unused
resource requires an explicit confirmation. Pending placement is cancelled before removal,
and Place on Map is disabled during project I/O to prevent adding a reference
while an unused file is being deleted. Save As refuses to omit a missing
used model or overwrite an existing destination resource.

The existing viewport-local pointer ray feeds Bevy's MeshRayCast, restricted to
render layer 3. Mesh hits follow ChildOf to ModelRoot/ObjectId. Terrain occludes
objects behind it. Generation, region revision and terrain fingerprint reject
stale clicks. The existing perspective ray was corrected to keep off-centre ray
angles independent of camera distance. Selection uses Gizmos on the existing
camera/layer, without modifying imported materials or duplicating meshes.

Explorer actions occupy an existing egui bottom panel before the scroll area;
otherwise the tree could consume the available height and hide Place on Map.
A small widget regression checks that the action remains clickable at 280×360.

## M5.2 verification — 2026-11-12

All required checks passed locally: `cargo fmt --all --check`,
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`,
`cargo build --workspace --release`, and `./scripts/check.sh` (115 tests, including
82 Bevy library tests). Release examples also build. Maximum Rust file length
is 320 lines (unchanged desktop file); the largest changed/new file is 297.
M5.2 adds no dependencies. M5.1 adds only the shared gltf parser dependency and
Bevy features described above; Cargo.lock introduces seven transitive packages.

Offline tests cover the native loader and shared scene handles, final mesh release,
independent root transforms/deletion, repeated project-generation cleanup and
final mesh release, child bounds, DEM anchoring, no-DEM data
preservation, region reprojection, benchmark isolation, native child-mesh picking
and stale-click rejection, schema 1 migration/schema 2 round trip, shared IDs,
path containment, used-asset removal protection and Save As. Corrupt headers,
missing files, external references, unknown required extensions, empty selected
scenes, cycles and excessive node depth are rejected. Map placement, Inspector,
Explorer/table selection and clipped action regression use synthetic widget
input; none require network access or claim human UX acceptance.

A native Linux/Wayland smoke test used OS mouse/keyboard input, system file
pickers, the actual release application and screenshot inspection on RTX 3050
6 GB/Vulkan. It is an automated real-window check, not a human manual acceptance:

- Started an empty project, saved it, imported the owned GLB through rfd and
  selected it under Assets/Models.
- Placed it on Walkers before DEM; verified the 2D marker and missing-height warning.
- Selected/confirmed an Alps region and used Load in 3D. Actual Mapterhorn DEM
  (214×192 samples, approximately 24.9 m spacing) loaded independently of satellite.
- Focused and selected the rendered GLB by clicking its child geometry; verified
  yellow bounds and shared Inspector/Explorer selection.
- Set name `Survey GLB`, AGL 12.5 m, yaw 37°, scale 2.5; observed changed geometry
  and dimensions 10×15×5 m. Saved, closed, restarted and opened through rfd;
  verified the same ID, path, values and selectable rendered model on cached DEM.
- Placed a second independent instance of the same GLB; used table selection and
  Map/Viewport switching. Resized 1366×768 → 1920×1080 and resized Explorer;
  viewport-local picking still selected a model in the shifted viewport.
- The terrain installation log measured 0.036 ms CPU asset installation on reopen;
  GPU upload latency was not measured. No satellite provider was configured.

Release fixture observations, one local run per case with warm filesystem cache:

| Instances | GLBs | Loader ready ms | Native spawn CPU µs | Transform writes µs | Mesh entities / assets |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 1 | 15.090 | 533.107 | 3.181 | 2 / 2 |
| 10 | 1 | 5.396 | 364.605 | 2.553 | 11 / 2 |
| 100 | 1 | 22.401 | 3703.004 | 3.939 | 101 / 2 |
| 100 | 2 | 5.412 | 1445.399 | 3.083 | 101 / 3 |

`model_smoke 100 mixed` alternates the two owned fixtures. Both 100-instance
cases were visually confirmed in the real Viewport. Totals include the existing
hidden editor plane; a repeated GLB contributes one mesh asset, not one per root.
Loader readiness includes scheduling/frame latency. Spawn CPU measures the native
SpawnScene schedule; transform writes exclude propagation/GPU work. These are
small fixture observations, not representative production-model throughput.
`project_smoke <project.json>` also exercises the actual project/cache/DEM/model
integration and prints resolved root transforms for reproducible GPU diagnosis.

## Current limits

Only self-contained glTF 2.0 GLB is accepted. The native loader's supported
extensions/textures still apply; unsupported required extensions are rejected,
not repaired. Hierarchies are limited to 256 levels. Bounds/ground anchoring and
mesh picking use undeformed geometry; animated/skinned/morphed bounds and
alpha-texture pixel picking are not implemented. Animation playback, model lights
and cameras are disabled. A very large decoded scene can still stall during
native world spawning/GPU upload; the compressed-file size limit is not a decoded
memory budget. No custom batching, LOD, CAD geometry, roads or buildings are added.
