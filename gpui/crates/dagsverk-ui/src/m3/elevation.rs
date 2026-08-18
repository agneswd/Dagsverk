use gpui::{BoxShadow, Hsla, point, px};

fn shadow(x: f32, y: f32, blur: f32, opacity: f32) -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: Hsla::black().opacity(opacity),
        offset: point(px(x), px(y)),
        blur_radius: px(blur),
        spread_radius: px(0.0),
    }]
}

pub fn menu_elevation() -> Vec<BoxShadow> {
    shadow(0.0, 4.0, 12.0, 0.18)
}

pub fn workspace_menu_elevation() -> Vec<BoxShadow> {
    shadow(0.0, 8.0, 24.0, 0.22)
}

pub fn side_sheet_elevation() -> Vec<BoxShadow> {
    shadow(-4.0, 0.0, 16.0, 0.12)
}

pub fn dialog_elevation() -> Vec<BoxShadow> {
    shadow(0.0, 8.0, 24.0, 0.24)
}
