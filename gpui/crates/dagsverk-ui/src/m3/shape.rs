use gpui::Pixels;

use super::UiScale;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct M3Shape {
    pub extra_small: Pixels,
    pub small: Pixels,
    pub medium: Pixels,
    pub large: Pixels,
    pub extra_large: Pixels,
    pub full: Pixels,
}

impl M3Shape {
    pub fn resolve(scale: UiScale) -> Self {
        Self {
            extra_small: scale.px(4.0),
            small: scale.px(8.0),
            medium: scale.px(12.0),
            large: scale.px(16.0),
            extra_large: scale.px(28.0),
            full: scale.px(999.0),
        }
    }
}

impl Default for M3Shape {
    fn default() -> Self {
        Self::resolve(UiScale::default())
    }
}
