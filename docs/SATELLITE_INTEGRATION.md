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
