use gpui::{Div, ParentElement, SharedString, Styled, div, px};

use super::M3ColorScheme;

pub const MATERIAL_SYMBOLS_FAMILY: &str = "Material Symbols Outlined";

pub fn m3_icon(name: impl Into<SharedString>, size: f32, colors: M3ColorScheme) -> Div {
    div()
        .font_family(MATERIAL_SYMBOLS_FAMILY)
        .text_size(px(size))
        .line_height(px(size))
        .text_color(colors.on_surface_variant)
        .child(name.into())
}
