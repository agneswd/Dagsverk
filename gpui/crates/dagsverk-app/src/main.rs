use std::borrow::Cow;
use std::sync::Arc;

use dagsverk_app::{
    platform::{NativeFileDialogService, NativeShellService},
    shell::{AppShell, AppShellServices},
    startup::StartupOptions,
    state::{AppModel, Language},
};
use dagsverk_core::{clock::SystemClock, tax::TaxEngine};
use dagsverk_data::Database;
use dagsverk_ui::component_gallery::ComponentGallery;
use gpui::{
    App, AppContext, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions, px, size,
};

const ROBOTO: &[u8] = include_bytes!("../../../assets/fonts/Roboto-Variable.ttf");
const MATERIAL_SYMBOLS: &[u8] = include_bytes!("../../../assets/fonts/MaterialSymbolsOutlined.ttf");

fn main() {
    let options = match StartupOptions::parse(std::env::args_os().skip(1)) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("failed to parse startup options: {error}");
            std::process::exit(2);
        }
    };
    let mut runtime = if options.component_gallery {
        None
    } else {
        match create_runtime(&options) {
            Ok(runtime) => Some(runtime),
            Err(error) => {
                eprintln!("failed to initialize Dagsverk: {error}");
                std::process::exit(1);
            }
        }
    };

    Application::new().run(move |cx: &mut App| {
        if let Err(error) = cx
            .text_system()
            .add_fonts(vec![Cow::Borrowed(ROBOTO), Cow::Borrowed(MATERIAL_SYMBOLS)])
        {
            eprintln!("failed to load bundled fonts: {error}");
        }

        ComponentGallery::register_key_bindings(cx);
        AppShell::register_key_bindings(cx);
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        let bounds = Bounds::centered(None, size(px(1366.), px(850.)), cx);
        let window_options = || WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("Dagsverk GPUI Preview".into()),
                ..Default::default()
            }),
            app_id: Some("dev.agneswd.dagsverk-gpui-preview".into()),
            window_min_size: Some(size(px(960.), px(640.))),
            ..Default::default()
        };
        let result = if let Some((model, services)) = runtime.take() {
            cx.open_window(window_options(), |window, cx| {
                cx.new(|cx| AppShell::new(model, services, window, cx))
            })
            .map(|_| ())
        } else {
            cx.open_window(window_options(), |window, cx| {
                cx.new(|cx| ComponentGallery::new(window, cx))
            })
            .map(|_| ())
        };

        if let Err(error) = result {
            eprintln!("failed to open Dagsverk GPUI Preview: {error}");
            cx.quit();
            return;
        }
        cx.activate(true);
    });
}

fn create_runtime(
    options: &StartupOptions,
) -> Result<(AppModel, AppShellServices), Box<dyn std::error::Error>> {
    let clock = Arc::new(SystemClock);
    let repository = Arc::new(Database::open(options.database_path()?, SystemClock)?);
    let mut tax = TaxEngine::default();
    tax.register_json(include_str!("../../../../public/tax-data/tax-2026.json"))?;
    let mut model = AppModel::new_with_system_language(
        repository.clone(),
        clock,
        tax,
        false,
        system_language(),
    );
    model.initialize()?;
    Ok((
        model,
        AppShellServices {
            data: repository,
            file_dialog: Arc::new(NativeFileDialogService),
            shell: Arc::new(NativeShellService),
        },
    ))
}

fn system_language() -> Language {
    ["LANGUAGE", "LC_ALL", "LC_MESSAGES", "LANG"]
        .into_iter()
        .filter_map(|name| std::env::var(name).ok())
        .find(|value| !value.trim().is_empty())
        .map_or(Language::English, |value| {
            if value.to_ascii_lowercase().starts_with("sv") {
                Language::Swedish
            } else {
                Language::English
            }
        })
}
