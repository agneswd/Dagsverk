use gpui::{Div, Styled, div, prelude::FluentBuilder, px};

use super::{M3ColorScheme, dialog_elevation};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum M3CardVariant {
    FilledLow,
    Filled,
    Outlined,
    Elevated,
}

pub fn m3_card(colors: M3ColorScheme) -> Div {
    m3_card_variant(colors, M3CardVariant::FilledLow)
}

pub fn m3_card_variant(colors: M3ColorScheme, variant: M3CardVariant) -> Div {
    div()
        .rounded(px(match variant {
            M3CardVariant::Elevated => 28.0,
            _ => 16.0,
        }))
        .bg(match variant {
            M3CardVariant::FilledLow => colors.surface_container_low,
            M3CardVariant::Filled => colors.surface_container,
            M3CardVariant::Outlined => colors.surface_container_lowest,
            M3CardVariant::Elevated => colors.surface_container_high,
        })
        .when(variant == M3CardVariant::Outlined, |card| {
            card.border_1().border_color(colors.outline_variant)
        })
        .when(variant == M3CardVariant::Elevated, |card| {
            card.shadow(dialog_elevation())
        })
}
