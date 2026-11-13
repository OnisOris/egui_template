# Geographic city layers (M5.3)

Work in progress: the acceptance results below are filled in after each stage.

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
