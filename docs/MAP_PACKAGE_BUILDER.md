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
