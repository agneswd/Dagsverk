use gpui::{Div, FontWeight, Hsla, ParentElement, SharedString, Styled, div, px};

use super::M3ColorScheme;

pub const MATERIAL_SYMBOLS_FAMILY: &str = "Material Symbols Outlined";

pub fn m3_icon(name: impl Into<SharedString>, size: f32, colors: M3ColorScheme) -> Div {
    m3_icon_colored(name, size, colors.on_surface_variant)
}

pub fn m3_icon_colored(name: impl Into<SharedString>, size: f32, color: Hsla) -> Div {
    div()
        .font_family(MATERIAL_SYMBOLS_FAMILY)
        .text_size(px(size))
        .line_height(px(size))
        .text_color(color)
        .child(name.into())
}

pub fn m3_icon_filled(name: impl Into<SharedString>, size: f32, color: Hsla) -> Div {
    m3_icon_colored(name, size, color).font_weight(FontWeight::BOLD)
}
