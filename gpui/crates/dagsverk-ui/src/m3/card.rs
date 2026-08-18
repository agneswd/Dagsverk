use gpui::{Div, Styled, div, px};

use super::M3ColorScheme;

pub fn m3_card(colors: M3ColorScheme) -> Div {
    div()
        .rounded(px(16.0))
        .bg(colors.surface_container_lowest)
        .border_1()
        .border_color(colors.outline_variant)
}
