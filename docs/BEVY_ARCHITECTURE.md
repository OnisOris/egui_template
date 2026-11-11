# Bevy + egui architecture

## Version matrix

| Component | Version | Why |
|---|---|---|
| Bevy | 0.20.1 | Latest stable engine. |
| bevy_egui | 0.43.1 | Latest stable integration; requires egui 0.36. |
| egui / eframe / egui_kittest | 0.36.2 | Last compatible UI line. |
| egui_tiles | 0.17.1 | Docking compatible with egui 0.36. |
| egui_ltreeview | 0.9.0 | Explorer tree compatible with egui 0.36. |
| egui_table | 0.10.0 | Data table compatible with egui 0.36. |
| egui_plot | 0.37.0 | Charts compatible with egui 0.36. |
| Walkers | 0.62.0 | Raster map widget compatible with egui 0.36. |

Versions were checked on 2026-11-11. The egui 0.37 stack cannot yet be used
with released bevy_egui without incompatible duplicate egui types or a fork.
See [MAP_PICKER.md](MAP_PICKER.md) for the complete compatibility matrix.
`scripts/check.sh` enforces that the workspace resolves only `egui v0.36.2`.

## Crates

```text
apps/starter-desktop/   existing eframe application, unchanged in behaviour
apps/starter-bevy/      Bevy application
crates/starter-core/    document model and commands, no Bevy or egui
crates/starter-ui/      docking, Explorer, Inspector, table, charts, theme
```

`starter-ui` now also owns the Explorer, Inspector and Data Table panels so
both applications render the same widgets instead of keeping two copies.

## Bevy plugins

```text
BevyEditorPlugin composition (src/lib.rs)
├── ScenePlugin
│   ├── SceneAssets          shared meshes + material cache
│   ├── sync_scene           DocumentState -> ECS projection
│   ├── draw_grid            ground grid gizmos
│   └── draw_selection       selected-object gizmos
├── ViewportPlugin
│   ├── ViewportState        camera, image handle, texture id, resize request
│   ├── setup_viewport       render-to-texture camera + primary egui camera
│   ├── apply_camera         orbit state -> Transform/Projection
│   └── target               image allocation and Egui texture bookkeeping
├── MapPlugin
│   ├── MapPicker            Walkers view + draft selection (non-send)
│   ├── ActiveMapRegion      sole confirmed bbox/origin + revision
│   └── demo::open_region    explicit request -> replace demo meshes
└── EguiPrimaryContextPass
    └── editor_ui            menu, toolbar, docking, panels, status bar
```

`ViewportPlugin` owns the shared viewport camera and render target. `ScenePlugin`
owns the document-to-ECS projection; `MapPlugin` owns the optional demo terrain. `editor_ui` never creates a second window
or a nested eframe loop.

## One native window

Two cameras are used:

1. **Viewport camera** — order -1, `RenderTarget::Image`, `RenderLayers::layer(1)`
   for the editor or layer 2 for demo terrain. It renders the chosen scene and
   grid into the viewport texture.
2. **Primary egui camera** — order 0, default render layer, carries
   `PrimaryEguiContext`. It clears the window and renders egui on top. No 3D
   content is on its render layer.

`EguiGlobalSettings::auto_create_primary_context` is disabled so the primary
context cannot accidentally attach to the texture camera.

## Render target lifecycle

- `ViewportState::from_world` creates a 512×512 `Image` and registers it with
  `EguiUserTextures`.
- The viewport panel allocates its egui rect, draws the current texture and
  records the requested physical size (`rect * pixels_per_point`).
- After the UI pass, `target::apply_pending_resize` runs only when the size
  changed. It creates the new image, removes the old handle from
  `EguiUserTextures`, registers the new one, and updates the camera
  `RenderTarget`.
- The old `Handle<Image>` has no remaining references, so Bevy releases the GPU
  texture. Resizing never accumulates textures, and a hidden viewport keeps at
  most one texture.
- The viewport is clipped by painting the image into its own egui rect, not by
  rendering the scene behind the UI.

## ECS projection and single source of truth

`starter_core::DocumentState` remains the source of truth. Bevy stores only a
projection:

```text
ProjectedObject(ObjectId)           marker component on each visual entity
SceneSync.entries: ObjectId -> ProjectionEntry { Entity, ... }
```

`ProjectionEntry` remembers the last position, size, colour and visibility.
`sync_scene` reacts to the revisions exposed by `DocumentState`
(structure/position/size/colour/visibility):

- structure change: entities for removed IDs are despawned, new IDs are spawned,
  unchanged IDs keep their entity;
- position/size: only matching transforms are updated;
- colour: a cached material is reused; materials are shared by colour, not per
  object;
- visibility: only the `Visibility` component changes.

Nothing is copied when no revision changed, and an edit to one object does not
recreate the scene. `ObjectId` is stable across frames; `Entity` is never used
as a document identifier.

Selection is read directly from `DocumentState`; selected objects are drawn with
grid gizmos, so selection changes do not respawn meshes.

## Input routing

Map input and selection are described in [MAP_PICKER.md](MAP_PICKER.md).
In the demo scene LMB always orbits, so it cannot pick hidden editor objects.
Camera control runs only from egui `Response` methods on the viewport widget:

- LMB on an object: select and drag it in the horizontal plane;
- LMB on empty space: orbit;
- RMB: pan;
- wheel while the viewport is hovered: zoom;
- double click: focus the picked object.

Because the response belongs to the viewport panel, dragging the docking
splitter, scrolling the Explorer or using the Inspector cannot move the 3D
camera.

Picking uses the orbit camera state and a ray/sphere test against the document's
object positions and sizes. Bevy's built-in mesh picking was evaluated but it is
driven by window pointer coordinates; mapping it into an egui-hosted render
texture would require a second input path. The template keeps a small analytic
picker instead of a full raycasting engine.

## Camera

`OrbitCamera` stores yaw, pitch, distance, target and projection mode. It is
independent of Bevy and unit-tested. `apply_camera` writes the resulting
`Transform` and `Projection` to the viewport camera every frame. The resource
model makes Reset View and Focus work without touching render internals.
Perspective and Orthographic use the same orbit distance; orthographic scale is
derived from the render-target height.

## Application modes

Only the Editor mode is implemented. `ScenePlugin` and `ViewportPlugin` do not
depend on the editor chrome, so a Simulation or Game mode can reuse the same
document, scene projection and 3D viewport while replacing `editor_ui` with its
own input and HUD plugin.

## Adding a new 3D object type

1. Add a mesh handle in `SceneAssets::from_world`.
2. Extend `mesh_index`/`SceneAssets::mesh` with the mapping from document data to
   that handle.
3. If the shape needs a different radius, adjust `picking.rs` (the current
   approximation is a sphere around the object).
4. Spawn it in `sync_scene` through the same `ProjectedObject` marker and
   `RenderLayers::layer(1)`.

No new plugin or render pipeline is needed for a normal mesh.

## Current limitations

- The Bevy application is an editor demo; file open/save and persistence are
  only implemented in `starter-desktop`.
- Mesh picking is analytic; deforming or concave meshes are not handled.
- Objects share materials by colour and meshes by group. Per-object material
  instances are intentionally avoided.
- The ground plane is fixed at 20,000 × 20,000 units and the grid is drawn with
  gizmos, not a textured mesh.
- No transform gizmo handles, physics, undo history, or CAD runtime modules.
- GPU frame time is not exposed by the current diagnostic setup; the status bar
  reports CPU/UI/sync timings.
- Bevy renders continuously. The reported FPS is the real frame rate, not the
  egui callback rate.
