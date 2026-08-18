use gpui::{Hsla, rgb};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedTheme {
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct M3ColorScheme {
    pub background: Hsla,
    pub surface: Hsla,
    pub surface_container_lowest: Hsla,
    pub surface_container_low: Hsla,
    pub surface_container: Hsla,
    pub surface_container_high: Hsla,
    pub surface_container_highest: Hsla,
    pub primary: Hsla,
    pub on_primary: Hsla,
    pub primary_container: Hsla,
    pub on_primary_container: Hsla,
    pub secondary_container: Hsla,
    pub on_secondary_container: Hsla,
    pub on_surface: Hsla,
    pub on_surface_variant: Hsla,
    pub outline: Hsla,
    pub outline_variant: Hsla,
    pub grid_line: Hsla,
    pub success: Hsla,
    pub success_container: Hsla,
    pub on_success_container: Hsla,
    pub warning: Hsla,
    pub warning_container: Hsla,
    pub on_warning_container: Hsla,
    pub error: Hsla,
    pub error_container: Hsla,
    pub on_error_container: Hsla,
}

impl M3ColorScheme {
    pub fn resolve(theme: ResolvedTheme) -> Self {
        match theme {
            ResolvedTheme::Light => Self::light(),
            ResolvedTheme::Dark => Self::dark(),
        }
    }

    pub fn light() -> Self {
        Self {
            background: color(0xffffff),
            surface: color(0xf0f4f9),
            surface_container_lowest: color(0xffffff),
            surface_container_low: color(0xf0f4f9),
            surface_container: color(0xe8eef5),
            surface_container_high: color(0xdee6ef),
            surface_container_highest: color(0xd4deea),
            primary: color(0x5f875f),
            on_primary: color(0xffffff),
            primary_container: color(0xd9ead6),
            on_primary_container: color(0x19351c),
            secondary_container: color(0xd9ead6),
            on_secondary_container: color(0x19351c),
            on_surface: color(0x1f1f1f),
            on_surface_variant: color(0x444746),
            outline: color(0x747775),
            outline_variant: color(0xc4c7c5),
            grid_line: color(0xdadce0),
            success: color(0x137333),
            success_container: color(0xceead6),
            on_success_container: color(0x0c5223),
            warning: color(0xb06000),
            warning_container: color(0xffe0b2),
            on_warning_container: color(0x4e2600),
            error: color(0xba1a1a),
            error_container: color(0xffdad6),
            on_error_container: color(0x410002),
        }
    }

    pub fn dark() -> Self {
        Self {
            background: color(0x131314),
            surface: color(0x1b1b1b),
            surface_container_lowest: color(0x131314),
            surface_container_low: color(0x1b1b1b),
            surface_container: color(0x232426),
            surface_container_high: color(0x2b2c2f),
            surface_container_highest: color(0x37393b),
            primary: color(0xacd4a8),
            on_primary: color(0x16351a),
            primary_container: color(0x345238),
            on_primary_container: color(0xd9ead6),
            secondary_container: color(0x345238),
            on_secondary_container: color(0xd9ead6),
            on_surface: color(0xe3e3e3),
            on_surface_variant: color(0xc4c7c5),
            outline: color(0x8e918f),
            outline_variant: color(0x444746),
            grid_line: color(0x333537),
            success: color(0x81c995),
            success_container: color(0x28412e),
            on_success_container: color(0xceead6),
            warning: color(0xfdd663),
            warning_container: color(0x44391f),
            on_warning_container: color(0xffe0b2),
            error: color(0xf2b8b5),
            error_container: color(0x8c1d18),
            on_error_container: color(0xf9dedc),
        }
    }
}

fn color(hex: u32) -> Hsla {
    rgb(hex).into()
}

#[cfg(test)]
mod tests {
    use super::{M3ColorScheme, ResolvedTheme};
    use gpui::{Hsla, rgb};

    fn expected(hex: u32) -> Hsla {
        rgb(hex).into()
    }

    #[test]
    fn light_and_dark_tokens_match_the_electron_theme() {
        let light = M3ColorScheme::resolve(ResolvedTheme::Light);
        assert_eq!(light.background, expected(0xffffff));
        assert_eq!(light.surface_container, expected(0xe8eef5));
        assert_eq!(light.primary, expected(0x5f875f));
        assert_eq!(light.error, expected(0xba1a1a));

        let dark = M3ColorScheme::resolve(ResolvedTheme::Dark);
        assert_eq!(dark.background, expected(0x131314));
        assert_eq!(dark.surface_container, expected(0x232426));
        assert_eq!(dark.primary, expected(0xacd4a8));
        assert_eq!(dark.error, expected(0xf2b8b5));
    }
}
