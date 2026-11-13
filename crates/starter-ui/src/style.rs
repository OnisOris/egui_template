//! Shared desktop style. Sizes are logical egui points, independent of monitor DPI.
use crate::Theme;
use egui::{Color32, Context, FontId, Stroke, TextStyle, Visuals, vec2};
use serde::{Deserialize, Serialize};

pub const BACKGROUND: Color32 = Color32::from_rgb(23, 26, 32);
pub const SURFACE: Color32 = Color32::from_rgb(32, 36, 44);
pub const RAISED: Color32 = Color32::from_rgb(41, 46, 56);
pub const BORDER: Color32 = Color32::from_rgb(54, 60, 71);
pub const TEXT: Color32 = Color32::from_rgb(232, 235, 240);
pub const MUTED: Color32 = Color32::from_rgb(160, 168, 181);
pub const ACCENT: Color32 = Color32::from_rgb(78, 156, 245);
pub const SUCCESS: Color32 = Color32::from_rgb(69, 184, 140);
pub const WARNING: Color32 = Color32::from_rgb(226, 170, 84);
pub const ERROR: Color32 = Color32::from_rgb(220, 100, 100);

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum Density {
    Compact,
    #[default]
    Comfortable,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Appearance {
    pub scale: f32,
    pub density: Density,
}
impl Default for Appearance {
    fn default() -> Self {
        Self {
            scale: 1.0,
            density: Density::Comfortable,
        }
    }
}
impl Appearance {
    pub fn apply(self, ctx: &Context, theme: Theme) {
        let scale = if self.scale.is_finite() {
            self.scale.clamp(0.75, 2.0)
        } else {
            1.0
        };
        let key = egui::Id::new("starter_style");
        let current = (theme, self.density, scale);
        if ctx.data(|d| d.get_temp::<(Theme, Density, f32)>(key)) == Some(current) {
            return;
        }
        let mut style = egui::Style::default();
        style.visuals = match theme {
            Theme::Dark => Visuals::dark(),
            Theme::Light => Visuals::light(),
        };
        let v = &mut style.visuals;
        if theme == Theme::Dark {
            v.panel_fill = SURFACE;
            v.window_fill = SURFACE;
            v.extreme_bg_color = BACKGROUND;
            v.faint_bg_color = RAISED;
            v.override_text_color = Some(TEXT);
            v.weak_text_color = Some(MUTED);
            v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
            v.widgets.inactive.bg_fill = RAISED;
            v.widgets.inactive.weak_bg_fill = RAISED;
            v.widgets.hovered.bg_fill = Color32::from_rgb(52, 64, 82);
            v.widgets.hovered.weak_bg_fill = Color32::from_rgb(52, 64, 82);
            v.widgets.active.bg_fill = Color32::from_rgb(45, 77, 114);
            v.widgets.active.weak_bg_fill = v.widgets.active.bg_fill;
        }
        v.selection.bg_fill = if theme == Theme::Dark {
            Color32::from_rgb(38, 77, 122)
        } else {
            Color32::from_rgb(188, 216, 250)
        };
        v.selection.stroke = Stroke::new(
            1.0,
            if theme == Theme::Dark {
                TEXT
            } else {
                Color32::from_rgb(22, 54, 95)
            },
        );
        v.hyperlink_color = ACCENT;
        v.warn_fg_color = WARNING;
        v.error_fg_color = ERROR;
        for widget in [
            &mut v.widgets.inactive,
            &mut v.widgets.hovered,
            &mut v.widgets.active,
        ] {
            widget.corner_radius = 5.into();
        }
        let compact = self.density == Density::Compact;
        style.spacing.item_spacing = vec2(8., if compact { 5. } else { 8. });
        style.spacing.button_padding = vec2(10., if compact { 4. } else { 6. });
        style.spacing.interact_size = vec2(32., if compact { 28. } else { 32. });
        style.spacing.window_margin = 12.into();
        style.spacing.menu_margin = 8.into();
        style.spacing.indent = 18.;
        style.spacing.text_edit_width = 180.;
        for (kind, size) in [
            (TextStyle::Heading, 16.),
            (TextStyle::Body, 14.),
            (TextStyle::Button, 14.),
            (TextStyle::Small, 12.),
            (TextStyle::Monospace, 12.),
        ] {
            style.text_styles.insert(kind, FontId::proportional(size));
        }
        style
            .text_styles
            .insert(TextStyle::Monospace, FontId::monospace(12.));
        ctx.set_theme(if theme == Theme::Dark {
            egui::Theme::Dark
        } else {
            egui::Theme::Light
        });
        ctx.set_global_style(style);
        ctx.set_zoom_factor(scale);
        ctx.data_mut(|d| d.insert_temp(key, current));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn themes_density_and_scale_share_readable_sizes() {
        let ctx = Context::default();
        for theme in [Theme::Dark, Theme::Light] {
            Appearance::default().apply(&ctx, theme);
            assert_eq!(ctx.global_style().visuals.dark_mode, theme == Theme::Dark);
            assert_eq!(ctx.global_style().text_styles[&TextStyle::Body].size, 14.);
            assert_eq!(ctx.global_style().spacing.interact_size.y, 32.);
        }
        Appearance {
            scale: f32::NAN,
            density: Density::Compact,
        }
        .apply(&ctx, Theme::Dark);
        assert_eq!(ctx.global_style().spacing.interact_size.y, 28.);
        assert_eq!(ctx.zoom_factor(), 1.);
        let saved = serde_json::to_string(&Appearance::default()).unwrap();
        assert_eq!(
            serde_json::from_str::<Appearance>(&saved).unwrap(),
            Appearance::default()
        );
    }
}

/// A standard collapsible property group, shared by all Inspector selections.
pub fn section(ui: &mut egui::Ui, title: &str, draw: impl FnOnce(&mut egui::Ui)) {
    egui::CollapsingHeader::new(title)
        .default_open(true)
        .show(ui, |ui| {
            ui.set_min_width(0.);
            draw(ui);
        });
}
