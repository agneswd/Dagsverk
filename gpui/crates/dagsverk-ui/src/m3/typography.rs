use std::sync::Arc;

use gpui::{FontFeatures, FontWeight, Styled, font};

use super::UiScale;

pub const ROBOTO_FAMILY: &str = "Roboto";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypographyRole {
    HeadlineSmall,
    TitleLarge,
    TitleMedium,
    TitleSmall,
    BodyLarge,
    BodyMedium,
    BodySmall,
    LabelLarge,
    LabelMedium,
    LabelSmall,
    Numeric,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct M3Typography {
    pub size: f32,
    pub line_height: Option<f32>,
    pub weight: FontWeight,
    pub tabular_numbers: bool,
}

impl M3Typography {
    pub const fn for_role(role: TypographyRole) -> Self {
        match role {
            TypographyRole::HeadlineSmall => Self::new(24.0, 32.0, FontWeight::NORMAL),
            TypographyRole::TitleLarge => Self::new(22.0, 28.0, FontWeight::NORMAL),
            TypographyRole::TitleMedium => Self::new(16.0, 24.0, FontWeight::MEDIUM),
            TypographyRole::TitleSmall | TypographyRole::LabelLarge => {
                Self::new(14.0, 20.0, FontWeight::MEDIUM)
            }
            TypographyRole::LabelMedium => Self::new(12.0, 16.0, FontWeight::MEDIUM),
            TypographyRole::BodyLarge => Self::new(16.0, 24.0, FontWeight::NORMAL),
            TypographyRole::BodyMedium => Self::new(14.0, 20.0, FontWeight::NORMAL),
            TypographyRole::BodySmall => Self::new(12.0, 16.0, FontWeight::NORMAL),
            TypographyRole::LabelSmall => Self::new(11.0, 16.0, FontWeight::MEDIUM),
            TypographyRole::Numeric => Self {
                size: 16.0,
                line_height: None,
                weight: FontWeight::NORMAL,
                tabular_numbers: true,
            },
        }
    }

    const fn new(size: f32, line_height: f32, weight: FontWeight) -> Self {
        Self {
            size,
            line_height: Some(line_height),
            weight,
            tabular_numbers: false,
        }
    }
}

pub trait M3TypographyExt: Styled + Sized {
    fn m3_typography(self, role: TypographyRole, scale: UiScale) -> Self {
        let token = M3Typography::for_role(role);
        let mut material_font = font(ROBOTO_FAMILY);
        if token.tabular_numbers {
            material_font.features = FontFeatures(Arc::new(vec![("tnum".into(), 1)]));
        }
        let element = self
            .font(material_font)
            .text_size(scale.px(token.size))
            .font_weight(token.weight);
        match token.line_height {
            Some(line_height) => element.line_height(scale.px(line_height)),
            None => element,
        }
    }
}

impl<T: Styled> M3TypographyExt for T {}

#[cfg(test)]
mod tests {
    use super::{M3Typography, TypographyRole};
    use gpui::FontWeight;

    #[test]
    fn typography_matches_the_electron_css_helpers() {
        let headline = M3Typography::for_role(TypographyRole::HeadlineSmall);
        assert_eq!((headline.size, headline.line_height), (24.0, Some(32.0)));
        assert_eq!(headline.weight, FontWeight::NORMAL);

        let label = M3Typography::for_role(TypographyRole::LabelSmall);
        assert_eq!((label.size, label.line_height), (11.0, Some(16.0)));
        assert_eq!(label.weight, FontWeight::MEDIUM);
        let numeric = M3Typography::for_role(TypographyRole::Numeric);
        assert!(numeric.tabular_numbers);
        assert_eq!(numeric.line_height, None);
    }
}
