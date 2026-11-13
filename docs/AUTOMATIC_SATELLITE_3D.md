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
