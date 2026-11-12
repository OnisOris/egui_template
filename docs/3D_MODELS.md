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
