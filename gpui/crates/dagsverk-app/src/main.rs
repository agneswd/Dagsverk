use std::borrow::Cow;
use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};
use dagsverk_app::{
    logging,
    platform::{NativeFileDialogService, NativeShellService},
    shell::{AppShell, AppShellServices},
    startup::StartupOptions,
    state::{AppModel, Language},
};
use dagsverk_core::{
    clock::{Clock, FixedClock, SystemClock},
    tax::TaxEngine,
};
use dagsverk_data::Database;
use dagsverk_ui::{component_gallery::ComponentGallery, m3::UiScale};
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
    let database_path = if options.component_gallery {
        None
    } else {
        match options.database_path() {
            Ok(path) => {
                logging::initialize(&path);
                Some(path)
            }
            Err(error) => {
                eprintln!("failed to resolve the Dagsverk data path: {error}");
                std::process::exit(1);
            }
        }
    };
    let visual_state = options.visual_state;
    let interface_scale_percent = options.interface_scale_percent;
    let window_size = options.window_size.unwrap_or((1366, 850));
    logging::info("Dagsverk GPUI Preview starting.");
    let mut runtime = if let Some(database_path) = database_path {
        match create_runtime(database_path, options.today) {
            Ok(runtime) => Some(runtime),
            Err(error) => {
                logging::error("Dagsverk initialization failed.", error.as_ref());
                eprintln!("failed to initialize Dagsverk: {error}");
                std::process::exit(1);
            }
        }
    } else {
        None
    };

    Application::new().run(move |cx: &mut App| {
        if let Err(error) = cx
            .text_system()
            .add_fonts(vec![Cow::Borrowed(ROBOTO), Cow::Borrowed(MATERIAL_SYMBOLS)])
        {
            logging::error("Bundled font loading failed.", &error);
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

        let bounds = Bounds::centered(
            None,
            size(px(window_size.0 as f32), px(window_size.1 as f32)),
            cx,
        );
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
                let shell = cx.new(|cx| AppShell::new(model, services, window, cx));
                if let Some(state) = visual_state {
                    shell.update(cx, |shell, cx| shell.apply_visual_state(state, cx));
                }
                if let Some(scale) = interface_scale_percent {
                    shell.update(cx, |shell, cx| shell.apply_visual_scale(scale, cx));
                }
                shell
            })
            .map(|_| ())
        } else {
            cx.open_window(window_options(), |window, cx| {
                let scale = interface_scale_percent
                    .and_then(UiScale::from_percent)
                    .unwrap_or_default();
                cx.new(|cx| ComponentGallery::new_with_scale(window, scale, cx))
            })
            .map(|_| ())
        };

        if let Err(error) = result {
            logging::error("The preview window failed to open.", &error);
            eprintln!("failed to open Dagsverk GPUI Preview: {error}");
            cx.quit();
            return;
        }
        cx.activate(true);
    });
}

fn create_runtime(
    database_path: std::path::PathBuf,
    today: Option<NaiveDate>,
) -> Result<(AppModel, AppShellServices), Box<dyn std::error::Error>> {
    let clock = RuntimeClock::new(today);
    let repository = Arc::new(Database::open(database_path, clock)?);
    let mut tax = TaxEngine::default();
    tax.register_json(include_str!("../../../../public/tax-data/tax-2026.json"))?;
    let mut model = AppModel::new_with_system_language(
        repository.clone(),
        Arc::new(clock),
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

#[derive(Clone, Copy)]
enum RuntimeClock {
    System(SystemClock),
    Fixed(FixedClock),
}

impl RuntimeClock {
    fn new(today: Option<NaiveDate>) -> Self {
        today.map_or(Self::System(SystemClock), |date| {
            Self::Fixed(FixedClock::new(DateTime::from_naive_utc_and_offset(
                date.and_hms_opt(12, 0, 0).unwrap_or_else(|| unreachable!()),
                Utc,
            )))
        })
    }
}

impl Clock for RuntimeClock {
    fn today(&self) -> NaiveDate {
        match self {
            Self::System(clock) => clock.today(),
            Self::Fixed(clock) => clock.today(),
        }
    }

    fn now_utc(&self) -> DateTime<Utc> {
        match self {
            Self::System(clock) => clock.now_utc(),
            Self::Fixed(clock) => clock.now_utc(),
        }
    }
}

fn system_language() -> Language {
    if sys_locale::get_locale().is_some_and(|value| value.to_ascii_lowercase().starts_with("sv")) {
        Language::Swedish
    } else {
        Language::English
    }
}
