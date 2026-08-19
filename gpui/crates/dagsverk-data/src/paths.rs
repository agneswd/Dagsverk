use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    Linux,
}

#[derive(Debug, Default)]
pub struct DataPathOptions<'a> {
    pub database: Option<&'a Path>,
    pub data_dir: Option<&'a Path>,
    pub environment_data_dir: Option<&'a Path>,
    pub app_data: Option<&'a Path>,
    pub xdg_config_home: Option<&'a Path>,
    pub home: Option<&'a Path>,
}

pub fn database_path(platform: Platform, options: &DataPathOptions<'_>) -> Option<PathBuf> {
    if let Some(path) = options.database {
        return Some(path.to_owned());
    }
    if let Some(path) = options.data_dir.or(options.environment_data_dir) {
        return Some(path.join("dagsverk.db"));
    }
    match platform {
        Platform::Windows => options
            .app_data
            .map(|path| path.join("Dagsverk").join("dagsverk.db")),
        Platform::Linux => options
            .xdg_config_home
            .map(Path::to_owned)
            .or_else(|| options.home.map(|path| path.join(".config")))
            .map(|path| path.join("Dagsverk").join("dagsverk.db")),
    }
}
