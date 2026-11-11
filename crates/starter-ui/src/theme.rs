use egui::{Context, Visuals};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum Theme {
    Light,
    #[default]
    Dark,
}

impl Theme {
    pub fn apply(self, context: &Context) {
        context.set_visuals(match self {
            Self::Light => Visuals::light(),
            Self::Dark => Visuals::dark(),
        });
    }
}
