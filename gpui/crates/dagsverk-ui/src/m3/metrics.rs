use gpui::{Pixels, px};

pub const SUPPORTED_SCALE_PERCENTAGES: [u16; 6] = [80, 90, 100, 110, 125, 150];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiScale(f32);

impl UiScale {
    pub fn from_percent(percent: u16) -> Option<Self> {
        SUPPORTED_SCALE_PERCENTAGES
            .contains(&percent)
            .then_some(Self(percent as f32 / 100.0))
    }

    pub fn factor(self) -> f32 {
        self.0
    }

    pub fn px(self, logical: f32) -> Pixels {
        px(logical * self.0)
    }
}

impl Default for UiScale {
    fn default() -> Self {
        Self(1.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct M3Metrics {
    pub control_height: Pixels,
    pub icon_button_size: Pixels,
    pub minimum_hit_target: Pixels,
    pub text_field_height: Pixels,
    pub chip_height: Pixels,
    pub sidebar_expanded: Pixels,
    pub sidebar_collapsed: Pixels,
    pub top_bar_height: Pixels,
    pub ledger_row_height: Pixels,
    pub editor_width: Pixels,
}

impl M3Metrics {
    pub fn resolve(scale: UiScale) -> Self {
        Self {
            control_height: scale.px(40.0),
            icon_button_size: scale.px(40.0),
            minimum_hit_target: scale.px(48.0),
            text_field_height: scale.px(56.0),
            chip_height: scale.px(32.0),
            sidebar_expanded: scale.px(256.0),
            sidebar_collapsed: scale.px(80.0),
            top_bar_height: scale.px(64.0),
            ledger_row_height: scale.px(52.0),
            editor_width: scale.px(416.0),
        }
    }
}

impl Default for M3Metrics {
    fn default() -> Self {
        Self::resolve(UiScale::default())
    }
}

#[cfg(test)]
mod tests {
    use super::{M3Metrics, UiScale};
    use gpui::px;

    #[test]
    fn every_supported_scale_changes_all_material_metrics() {
        let scale = UiScale::from_percent(150).expect("150 is a supported interface scale");
        let metrics = M3Metrics::resolve(scale);
        assert_eq!(metrics.control_height, px(60.0));
        assert_eq!(metrics.sidebar_expanded, px(384.0));
        assert_eq!(metrics.editor_width, px(624.0));
        assert!(UiScale::from_percent(95).is_none());
    }
}
