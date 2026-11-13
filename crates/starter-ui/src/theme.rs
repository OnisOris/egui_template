use egui::Context;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum Theme {
    Light,
    #[default]
    Dark,
}

impl Theme {
    pub fn apply(self, context: &Context) {
        crate::style::Appearance::default().apply(context, self);
    }
}
