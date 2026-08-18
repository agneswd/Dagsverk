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

    use super::{StartupError, StartupOptions};

    #[test]
    fn parses_overrides_and_rejects_unknown_options() {
        let options = StartupOptions::parse([
            OsString::from("--database"),
            OsString::from("/tmp/copy.db"),
            OsString::from("--component-gallery"),
            OsString::from("--today"),
            OsString::from("2026-08-18"),
        ])
        .expect("valid startup options");
        assert_eq!(options.database.as_deref(), Some(Path::new("/tmp/copy.db")));
        assert!(options.component_gallery);
        assert_eq!(
            options.today,
            NaiveDate::from_ymd_opt(2026, 8, 18),
            "fixed date must be deterministic"
        );
        assert_eq!(
            StartupOptions::parse([OsString::from("--wat")]),
            Err(StartupError::UnknownOption("--wat".to_owned()))
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
