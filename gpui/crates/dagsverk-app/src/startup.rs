use std::{ffi::OsString, path::PathBuf};

use chrono::NaiveDate;
use dagsverk_data::paths::{DataPathOptions, Platform, database_path};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct StartupOptions {
    pub database: Option<PathBuf>,
    pub data_dir: Option<PathBuf>,
    pub compatibility_mode: bool,
    pub component_gallery: bool,
    pub today: Option<NaiveDate>,
    pub visual_state: Option<VisualState>,
    pub window_size: Option<(u32, u32)>,
    pub interface_scale_percent: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualState {
    Ledger,
    Calendar,
    Editor,
    Projects,
    SettingsGeneral,
    SettingsOvertime,
    Backups,
    Workspaces,
    MonthMenu,
    ColorPicker,
    LedgerDark,
    CalendarDark,
    EditorDark,
    SettingsDark,
    WorkspacesDark,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum StartupError {
    #[error("{0} requires a path")]
    MissingValue(&'static str),
    #[error("unknown option: {0}")]
    UnknownOption(String),
    #[error("the operating system data directory is unavailable")]
    DataDirectoryUnavailable,
    #[error("invalid --today date: {0}")]
    InvalidToday(String),
    #[error("invalid --visual-state value: {0}")]
    InvalidVisualState(String),
    #[error("invalid --window-size value: {0}")]
    InvalidWindowSize(String),
    #[error("invalid --interface-scale value: {0}")]
    InvalidInterfaceScale(String),
}

impl StartupOptions {
    pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Self, StartupError> {
        let mut options = Self::default();
        let mut args = args.into_iter();
        while let Some(argument) = args.next() {
            match argument.to_string_lossy().as_ref() {
                "--database" => {
                    options.database = Some(PathBuf::from(
                        args.next()
                            .ok_or(StartupError::MissingValue("--database"))?,
                    ));
                }
                "--data-dir" => {
                    options.data_dir = Some(PathBuf::from(
                        args.next()
                            .ok_or(StartupError::MissingValue("--data-dir"))?,
                    ));
                }
                "--compatibility-mode" => options.compatibility_mode = true,
                "--component-gallery" => options.component_gallery = true,
                "--today" => {
                    let value = args.next().ok_or(StartupError::MissingValue("--today"))?;
                    let value = value.to_string_lossy();
                    options.today = Some(
                        NaiveDate::parse_from_str(&value, "%Y-%m-%d")
                            .map_err(|_| StartupError::InvalidToday(value.into_owned()))?,
                    );
                }
                "--visual-state" => {
                    let value = args
                        .next()
                        .ok_or(StartupError::MissingValue("--visual-state"))?;
                    let value = value.to_string_lossy();
                    options.visual_state = Some(
                        VisualState::parse(&value)
                            .ok_or_else(|| StartupError::InvalidVisualState(value.into_owned()))?,
                    );
                }
                "--window-size" => {
                    let value = args
                        .next()
                        .ok_or(StartupError::MissingValue("--window-size"))?;
                    let value = value.to_string_lossy();
                    let (width, height) = value
                        .split_once('x')
                        .and_then(|(width, height)| {
                            Some((width.parse::<u32>().ok()?, height.parse::<u32>().ok()?))
                        })
                        .filter(|(width, height)| *width >= 960 && *height >= 640)
                        .ok_or_else(|| StartupError::InvalidWindowSize(value.to_string()))?;
                    options.window_size = Some((width, height));
                }
                "--interface-scale" => {
                    let value = args
                        .next()
                        .ok_or(StartupError::MissingValue("--interface-scale"))?;
                    let value = value.to_string_lossy();
                    let scale = value
                        .parse::<u16>()
                        .ok()
                        .filter(|scale| [80, 90, 100, 110, 125, 150].contains(scale))
                        .ok_or_else(|| StartupError::InvalidInterfaceScale(value.to_string()))?;
                    options.interface_scale_percent = Some(scale);
                }
                unknown => return Err(StartupError::UnknownOption(unknown.to_owned())),
            }
        }
        Ok(options)
    }

    pub fn database_path(&self) -> Result<PathBuf, StartupError> {
        let environment_data_dir = std::env::var_os("DAGSVERK_DATA_DIR").map(PathBuf::from);
        let app_data = std::env::var_os("APPDATA").map(PathBuf::from);
        let xdg_config_home = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from);
        let home = std::env::var_os("HOME").map(PathBuf::from);
        self.database_path_for(
            current_platform(),
            environment_data_dir.as_deref(),
            app_data.as_deref(),
            xdg_config_home.as_deref(),
            home.as_deref(),
        )
    }

    fn database_path_for(
        &self,
        platform: Platform,
        environment_data_dir: Option<&std::path::Path>,
        app_data: Option<&std::path::Path>,
        xdg_config_home: Option<&std::path::Path>,
        home: Option<&std::path::Path>,
    ) -> Result<PathBuf, StartupError> {
        let explicit = DataPathOptions {
            database: self.database.as_deref(),
            data_dir: self.data_dir.as_deref(),
            environment_data_dir,
            ..Default::default()
        };
        if self.database.is_some() || self.data_dir.is_some() || environment_data_dir.is_some() {
            return database_path(platform, &explicit)
                .ok_or(StartupError::DataDirectoryUnavailable);
        }

        let base = DataPathOptions {
            app_data,
            xdg_config_home,
            home,
            ..Default::default()
        };
        let stable =
            database_path(platform, &base).ok_or(StartupError::DataDirectoryUnavailable)?;
        if self.compatibility_mode {
            Ok(stable)
        } else {
            Ok(stable
                .parent()
                .ok_or(StartupError::DataDirectoryUnavailable)?
                .with_file_name("Dagsverk GPUI Preview")
                .join("dagsverk.db"))
        }
    }
}

impl VisualState {
    fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "ledger" => Self::Ledger,
            "calendar" => Self::Calendar,
            "editor" => Self::Editor,
            "projects" => Self::Projects,
            "settings-general" => Self::SettingsGeneral,
            "settings-overtime" => Self::SettingsOvertime,
            "backups" => Self::Backups,
            "workspaces" => Self::Workspaces,
            "month-menu" => Self::MonthMenu,
            "color-picker" => Self::ColorPicker,
            "ledger-dark" => Self::LedgerDark,
            "calendar-dark" => Self::CalendarDark,
            "editor-dark" => Self::EditorDark,
            "settings-dark" => Self::SettingsDark,
            "workspaces-dark" => Self::WorkspacesDark,
            _ => return None,
        })
    }
}

#[cfg(target_os = "windows")]
fn current_platform() -> Platform {
    Platform::Windows
}

#[cfg(not(target_os = "windows"))]
fn current_platform() -> Platform {
    Platform::Linux
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsString, path::Path};

    use chrono::NaiveDate;
    use dagsverk_data::paths::Platform;

    use super::{StartupError, StartupOptions, VisualState};

    #[test]
    fn parses_overrides_and_rejects_unknown_options() {
        let options = StartupOptions::parse([
            OsString::from("--database"),
            OsString::from("/tmp/copy.db"),
            OsString::from("--component-gallery"),
            OsString::from("--today"),
            OsString::from("2026-08-18"),
            OsString::from("--visual-state"),
            OsString::from("editor-dark"),
            OsString::from("--window-size"),
            OsString::from("1366x820"),
            OsString::from("--interface-scale"),
            OsString::from("125"),
        ])
        .expect("valid startup options");
        assert_eq!(options.database.as_deref(), Some(Path::new("/tmp/copy.db")));
        assert!(options.component_gallery);
        assert_eq!(
            options.today,
            NaiveDate::from_ymd_opt(2026, 8, 18),
            "fixed date must be deterministic"
        );
        assert_eq!(options.visual_state, Some(VisualState::EditorDark));
        assert_eq!(options.window_size, Some((1366, 820)));
        assert_eq!(options.interface_scale_percent, Some(125));
        assert_eq!(
            StartupOptions::parse([OsString::from("--wat")]),
            Err(StartupError::UnknownOption("--wat".to_owned()))
        );
        assert_eq!(
            StartupOptions::parse([OsString::from("--interface-scale"), OsString::from("175")]),
            Err(StartupError::InvalidInterfaceScale("175".to_owned()))
        );
        assert_eq!(
            StartupOptions::parse([OsString::from("--window-size"), OsString::from("800x600")]),
            Err(StartupError::InvalidWindowSize("800x600".to_owned()))
        );
        assert_eq!(
            StartupOptions::parse([OsString::from("--visual-state"), OsString::from("unknown")]),
            Err(StartupError::InvalidVisualState("unknown".to_owned()))
        );
    }

    #[test]
    fn preview_and_compatibility_paths_are_deliberately_distinct() {
        let preview = StartupOptions::default()
            .database_path_for(
                Platform::Linux,
                None,
                None,
                Some(Path::new("/config")),
                None,
            )
            .expect("preview path");
        assert_eq!(
            preview,
            Path::new("/config/Dagsverk GPUI Preview/dagsverk.db")
        );

        let compatibility = StartupOptions {
            compatibility_mode: true,
            ..Default::default()
        }
        .database_path_for(
            Platform::Linux,
            None,
            None,
            Some(Path::new("/config")),
            None,
        )
        .expect("compatibility path");
        assert_eq!(compatibility, Path::new("/config/Dagsverk/dagsverk.db"));
    }

    #[test]
    fn explicit_database_has_highest_precedence() {
        let options = StartupOptions {
            database: Some("/tmp/fixture.db".into()),
            data_dir: Some("/ignored".into()),
            compatibility_mode: true,
            component_gallery: false,
            today: None,
            visual_state: None,
            window_size: None,
            interface_scale_percent: None,
        };
        assert_eq!(
            options
                .database_path_for(
                    Platform::Windows,
                    Some(Path::new("/also-ignored")),
                    Some(Path::new("C:/Users/test/AppData/Roaming")),
                    None,
                    None,
                )
                .expect("explicit database path"),
            Path::new("/tmp/fixture.db")
        );
    }
}
