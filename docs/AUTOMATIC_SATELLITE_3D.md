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
Existing XYZ configurations deserialize with Online 3D disabled. Esri's preset remains
preview only. Enabling a capability is the application's operator's explicit assertion
of permission, not a determination of a provider's licensing terms.

## Stage a validation

Local HTTP fixtures cover bearer authorization, two consumers sharing one HTTP request
and the same decoded allocation, memory-only behavior, source/configuration generation
isolation, source zoom bounds, network failures and retries. No public imagery server
is used by tests. `cargo test -p starter-bevy --lib`: 91 passed.

## Automatic 3D and persistence

Once a matching DEM is installed, `imagery_3d` obtains the service from the existing
Walkers adapter. It independently requests the confirmed region's tile coverage.
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
