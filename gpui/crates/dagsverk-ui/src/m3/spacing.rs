use gpui::Pixels;

use super::UiScale;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct M3Spacing {
    pub xxs: Pixels,
    pub xs: Pixels,
    pub sm: Pixels,
    pub md: Pixels,
    pub lg: Pixels,
    pub xl: Pixels,
    pub xxl: Pixels,
}

impl M3Spacing {
    pub fn resolve(scale: UiScale) -> Self {
        Self {
            xxs: scale.px(4.0),
            xs: scale.px(8.0),
            sm: scale.px(12.0),
            md: scale.px(16.0),
            lg: scale.px(20.0),
            xl: scale.px(24.0),
            xxl: scale.px(32.0),
        }
    }
}

impl Default for M3Spacing {
    fn default() -> Self {
        Self::resolve(UiScale::default())
    }
}
