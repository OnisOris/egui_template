# Bevy + egui benchmarks

## Environment

- OS: CachyOS, kernel 7.3.1
- CPU: Intel Core 5 210H (12 threads)
- GPU: NVIDIA GeForce RTX 3050 6GB Laptop GPU
- Driver: NVIDIA 615.78.08, Vulkan backend
- Session: Wayland / Hyprland, 2560×1440 output
- Window: 1440×900 logical, one native winit window
- Rust: 1.97.1, release profile (`opt-level = 3`, debug assertions off)

These numbers are from one machine. They are not a claim that every machine
will reach 60 FPS with 10,000 objects.

## Method

The app accepts diagnostics environment variables:

```bash
STARTER_LOAD=10000 STARTER_BENCH=1 cargo run -p starter-bevy --release
```

- `STARTER_LOAD` loads 100, 1,000, or 10,000 demo objects before the first
  frame.
- `STARTER_BENCH=1` switches the window to `PresentMode::AutoNoVsync` and prints
  one `BENCH ...` line per second.
- `STARTER_BENCH_EDIT=1` also moves one object every second so the incremental
  document → ECS sync path is exercised instead of the steady state.

Reported values are the median of the last five or six one-second samples after
startup. `frame_ms` comes from Bevy's `Time::delta`, `ui_ms` is wall time spent
inside the egui pass, `build_ms` is the first structure sync, and `sync_ms` is
the most recent sync pass. GPU timestamp queries are not enabled, so GPU frame
time is not reported. The values are not derived from egui callback frequency.

The window was allowed to render normally. `AutoNoVsync` is used only for the
benchmark; the default app uses `AutoVsync`.

## Release results (no edits)

| Objects | Visual entities | FPS | Frame ms | UI ms | Build ms | Sync ms |
|---:|---:|---:|---:|---:|---:|---:|
| 100 | 100 | 156.8 | 6.38 | 0.94 | 0.05 | 0.05 |
| 1,000 | 1,000 | 156.2 | 6.40 | 0.77 | 0.41 | 0.41 |
| 10,000 | 10,000 | 99.3 | 10.08 | 2.46 | 4.12 | 4.12 |

The 10,000-object scene is GPU-bound or submission-bound by this point in the
default single-threaded render loop, not by the egui panels. 100 and 1,000
objects are dominated by fixed-frame overhead.

Repeated runs on the same machine produced these release FPS ranges:

| Objects | Observed FPS range |
|---:|---:|
| 100 | 100–157 |
| 1,000 | 156–206 |
| 10,000 | 99–146 |

The variance is caused by the uncapped present mode, compositor scheduling and
other desktop load. The initial build was 2.0–4.1 ms at 10,000 objects.

## Release results (one object moved per second)

| Objects | FPS | Frame ms | UI ms | Incremental sync ms | Initial build ms |
|---:|---:|---:|---:|---:|---:|
| 100 | 85.8 | 11.73 | 1.25 | 0.015 | 0.08 |
| 1,000 | 106.4 | 9.42 | 1.06 | 0.130 | 0.22 |
| 10,000 | 103.0 | 9.71 | 2.54 | 1.19 | 2.39 |

The per-edit sync is linear in the object count because the revision pass
compares every document object, but it does not respawn entities or recreate the
scene. At 10,000 objects the measured cost is about 1.2 ms in release.

FPS varies noticeably between runs because the benchmark is uncapped and the
compositor/machine load is not isolated. In repeated runs the 10,000-object
per-edit sync measured 1.2–1.6 ms and the initial build 2.0–4.1 ms. The sync
and UI timings are more stable and are the useful numbers for this template.

## Debug results (no edits)

| Objects | FPS | Frame ms | UI ms | Build ms |
|---:|---:|---:|---:|---:|
| 100 | 27.0 | 37.08 | 6.96 | 0.68 |
| 1,000 | 23.0 | 43.54 | 9.50 | 1.59 |
| 10,000 | 23.7 | 42.13 | 7.13 | 12.91 |

Debug numbers are included only to show the development cost. They are not a
target and should not be compared with the release table.

## Memory

Measured with `ps -o rss` after the scene settled, release build:

| Objects | RSS |
|---:|---:|
| 100 | about 531 MB |
| 10,000 | about 563 MB |

The base is dominated by Bevy, wgpu and the driver. The projected entities add
roughly 32 MB for 10,000 objects on this machine. This is a rough process RSS,
not a GPU allocation report.

## What was tested manually

On the Wayland/Hyprland session with the release and debug builds:

- one native window with docking works;
- the 3D viewport is inside the Viewport docking tab;
- grid, directional light, shared cube/sphere/cylinder meshes and materials
  render;
- loading 100, 1,000 and 10,000 objects works without a hang or panic;
- selection in the Explorer updates the Inspector;
- Inspector fields render for the selected object; edits return the same
  document commands as the desktop app (covered by starter-ui/desktop tests);
- Perspective/Orthographic switching changes the rendered projection and the
  status bar;
- resizing the native window and the docking layout changes the render target
  without panics;
- the app ran for an extended timeout and exited on termination without a
  panic in the log.

Mouse picking into the 3D viewport is covered by the ray/sphere unit test.
Injected pointer input in this headless test session was not reliable enough to
claim a real mouse-picking test; that remains a manual check for a normal
desktop session.

## Known limits

- GPU frame time and GPU memory counters are not collected.
- The app renders continuously; idle CPU/GPU use is the normal Bevy loop and was
  not measured with an external profiler.
- Editor, Simulation and Game modes share the document/viewport core, but only
  the Editor mode is implemented.
- No physical engine, networking, audio, AI or game framework is included.
