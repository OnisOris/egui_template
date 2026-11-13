# Desktop design system (M5.4)

The shared `starter-ui::style` module configures standard egui Style, Visuals,
Spacing and TextStyle; it does not implement a widget toolkit or add dependencies.

| Token | Dark value | Purpose |
|---|---|---|
| Background | #171A20 | Recessed surface |
| Surface | #20242C | Panels and windows |
| Raised | #292E38 | Controls and subtle groups |
| Border | #363C47 | Dividers |
| Text | #E8EBF0 | Primary text |
| Muted | #A0A8B5 | Supporting text |
| Accent | #4E9CF5 | Selection and links |
| Success | #45B88C | Ready |
| Warning | #E2AA54 | Missing data or attention |
| Error | #DC6464 | Failed action |

Light mode starts from egui's light palette, with matching typography, spacing and
selection semantics. Color is accompanied by a word or control state.

Body/buttons: 14 pt; headings: 16 pt; small/technical/monospace: 12 pt. All are
logical points. Comfortable controls are at least 32 pt tall; Compact uses 28 pt.
Spacing is 8 pt horizontally and 8/5 pt vertically, padding 10×6/4 pt, corner radius
5 pt. Dock panes have 10 pt insets and a 6 pt resize target between surfaces.
Default fonts are the bundled egui fonts. Hover, pressed, disabled and keyboard
focus behavior remains native egui behavior.

Appearance scale multiplies the monitor's native pixels-per-point through egui's
zoom factor; 1.0 preserves system DPI. A bounded 0.75–2.0 range and Compact /
Comfortable density avoid a collection of independent size settings.

Native widgets provide interaction states. Source/geometry diagnostics stay in
contextual properties or Tools → Developer; color never replaces a status word.
No tab-specific decorative palette is used. `style::section` is a thin native
CollapsingHeader configuration for property groups, not a separate Inspector.

Module responsibilities:

- starter-ui/style.rs: palette, typography, density, DPI-relative zoom, sections.
- starter-ui/theme.rs: compatibility entry point for both desktop shells.
- starter-ui/workspace.rs: existing docking plus the three explicit presets.
- ui/chrome.rs: menus, primary navigation and short status.
- ui/actions.rs / shortcuts.rs: shared command dispatch and keyboard gating.
- ui/appearance.rs / diagnostics.rs / empty_states.rs: optional windows and guidance.
- ui/project_explorer and *_inspector.rs: contextual structure and properties.
- map/controls.rs / ui.rs / picker.rs / imagery_view.rs: geographic tool presentation.
- layout.rs: UI-only appearance and layout storage, preserving legacy layouts.


Validation: all required local checks passed, including 154 tests and workspace
release builds. Native Dark/Light, 100%/125% zoom and all five requested window
sizes were inspected. See [PRODUCT_UX](PRODUCT_UX.md) for the smoke matrix and
[before](screenshots/m53/city.png) / [after](screenshots/m54/combined.png) evidence.
Native glyph testing found missing Unicode symbols; main actions use words and
layer icons use small egui painter shapes, with tooltips and text labels.
