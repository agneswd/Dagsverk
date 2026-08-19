use std::{
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    process::Command,
};

use dagsverk_core::models::{UpdateState, UpdateStatus};

pub type PlatformFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + 'a>>;

#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("failed to open {target}: {source}")]
    Open {
        target: String,
        source: std::io::Error,
    },
    #[error("updates are unavailable in this build")]
    UpdatesUnavailable,
}

pub type Result<T> = std::result::Result<T, PlatformError>;

#[derive(Debug, Clone, Default)]
pub struct OpenFileRequest {
    pub title: String,
    pub filters: Vec<(String, Vec<String>)>,
    pub directory: Option<PathBuf>,
}

#[derive(Debug, Clone, Default)]
pub struct SaveFileRequest {
    pub title: String,
    pub file_name: Option<String>,
    pub filters: Vec<(String, Vec<String>)>,
    pub directory: Option<PathBuf>,
}

pub trait FileDialogService: Send + Sync {
    fn choose_open_file(&self, request: OpenFileRequest) -> PlatformFuture<'_, Option<PathBuf>>;
    fn choose_save_file(&self, request: SaveFileRequest) -> PlatformFuture<'_, Option<PathBuf>>;
}

pub struct NativeFileDialogService;

impl FileDialogService for NativeFileDialogService {
    fn choose_open_file(&self, request: OpenFileRequest) -> PlatformFuture<'_, Option<PathBuf>> {
        Box::pin(async move {
            let mut dialog = rfd::AsyncFileDialog::new().set_title(request.title);
            if let Some(directory) = request.directory {
                dialog = dialog.set_directory(directory);
            }
            for (name, extensions) in request.filters {
                dialog = dialog.add_filter(name, &extensions);
            }
            Ok(dialog.pick_file().await.map(|file| file.path().to_owned()))
        })
    }

    fn choose_save_file(&self, request: SaveFileRequest) -> PlatformFuture<'_, Option<PathBuf>> {
        Box::pin(async move {
            let mut dialog = rfd::AsyncFileDialog::new().set_title(request.title);
            if let Some(directory) = request.directory {
                dialog = dialog.set_directory(directory);
            }
            if let Some(file_name) = request.file_name {
                dialog = dialog.set_file_name(file_name);
            }
            for (name, extensions) in request.filters {
                dialog = dialog.add_filter(name, &extensions);
            }
            Ok(dialog.save_file().await.map(|file| file.path().to_owned()))
        })
    }
}

pub trait ShellService: Send + Sync {
    fn open_folder(&self, path: &Path) -> Result<()>;
    fn open_external(&self, url: &str) -> Result<()>;
}

pub struct NativeShellService;

impl ShellService for NativeShellService {
    fn open_folder(&self, path: &Path) -> Result<()> {
        open_target(path.as_os_str(), &path.display().to_string())
    }

    fn open_external(&self, url: &str) -> Result<()> {
        open_target(std::ffi::OsStr::new(url), url)
    }
}

pub trait UpdateService {
    fn state(&self) -> PlatformFuture<'_, UpdateState>;
    fn check(&self) -> PlatformFuture<'_, UpdateState>;
    fn restart_and_apply(&self) -> PlatformFuture<'_, ()>;
}

pub struct UnavailableUpdateService {
    pub current_version: String,
}

impl UpdateService for UnavailableUpdateService {
    fn state(&self) -> PlatformFuture<'_, UpdateState> {
        Box::pin(async move { Ok(self.unavailable_state()) })
    }

    fn check(&self) -> PlatformFuture<'_, UpdateState> {
        Box::pin(async move { Ok(self.unavailable_state()) })
    }

    fn restart_and_apply(&self) -> PlatformFuture<'_, ()> {
        Box::pin(async { Err(PlatformError::UpdatesUnavailable) })
    }
}

impl UnavailableUpdateService {
    fn unavailable_state(&self) -> UpdateState {
        UpdateState {
            status: UpdateStatus::Unavailable,
            current_version: self.current_version.clone(),
            available_version: None,
            progress: None,
            message: Some("Updates are unavailable in this build.".to_owned()),
        }
    }
}

#[cfg(target_os = "windows")]
fn open_target(target: &std::ffi::OsStr, display: &str) -> Result<()> {
    Command::new("cmd")
        .args(["/C", "start", ""])
        .arg(target)
        .spawn()
        .map(|_| ())
        .map_err(|source| PlatformError::Open {
            target: display.to_owned(),
            source,
        })
}

#[cfg(not(target_os = "windows"))]
fn open_target(target: &std::ffi::OsStr, display: &str) -> Result<()> {
    Command::new("xdg-open")
        .arg(target)
        .spawn()
        .map(|_| ())
        .map_err(|source| PlatformError::Open {
            target: display.to_owned(),
            source,
        })
}

#[cfg(test)]
mod tests {
    use super::{UnavailableUpdateService, UpdateService};

    #[gpui::test]
    async fn development_updater_reports_unavailable(_cx: &gpui::TestAppContext) {
        let updater = UnavailableUpdateService {
            current_version: "0.1.0".to_owned(),
        };
        let state = updater.state().await.expect("state");
        assert_eq!(
            state.status,
            dagsverk_core::models::UpdateStatus::Unavailable
        );
        assert!(updater.restart_and_apply().await.is_err());
    }
}
