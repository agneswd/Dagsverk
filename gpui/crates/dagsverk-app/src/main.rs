use std::borrow::Cow;

use dagsverk_ui::component_gallery::ComponentGallery;
use gpui::{
    App, AppContext, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions, px, size,
};

const ROBOTO: &[u8] = include_bytes!("../../../assets/fonts/Roboto-Variable.ttf");

fn main() {
    Application::new().run(|cx: &mut App| {
        if let Err(error) = cx.text_system().add_fonts(vec![Cow::Borrowed(ROBOTO)]) {
            eprintln!("failed to load bundled Roboto font: {error}");
        }

        ComponentGallery::register_key_bindings(cx);
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        let bounds = Bounds::centered(None, size(px(800.), px(600.)), cx);
        let result = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Dagsverk GPUI Preview".into()),
                    ..Default::default()
                }),
                app_id: Some("dev.agneswd.dagsverk-gpui-preview".into()),
                window_min_size: Some(size(px(640.), px(480.))),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| ComponentGallery::new(window, cx)),
        );

        if let Err(error) = result {
            eprintln!("failed to open Dagsverk GPUI Preview: {error}");
            cx.quit();
            return;
        }
        cx.activate(true);
    });
}
