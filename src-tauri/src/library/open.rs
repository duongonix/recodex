use std::{path::Path, process::Command};

use crate::errors::IpcError;

pub fn open_item(path: &Path) -> Result<(), IpcError> {
    open_path(path, false)
}

pub fn reveal_item(path: &Path) -> Result<(), IpcError> {
    open_path(path, true)
}

pub fn open_output_folder(path: &Path) -> Result<(), IpcError> {
    if !path.is_dir() {
        return Err(IpcError::new(
            "library_folder_unavailable",
            "The selected output folder is not available. Choose an existing folder in Settings.",
            true,
        ));
    }
    open_path(path, false)
}

#[cfg(windows)]
fn open_path(path: &Path, reveal: bool) -> Result<(), IpcError> {
    let mut command = Command::new("explorer.exe");
    if reveal {
        command.arg(format!("/select,{}", path.display()));
    } else {
        command.arg(path);
    }
    command.spawn().map(|_| ()).map_err(open_error)
}

#[cfg(target_os = "macos")]
fn open_path(path: &Path, reveal: bool) -> Result<(), IpcError> {
    let mut command = Command::new("open");
    if reveal {
        command.arg("-R");
    }
    spawn_opener(command.arg(path))
}

#[cfg(target_os = "linux")]
fn open_path(path: &Path, reveal: bool) -> Result<(), IpcError> {
    let target = if reveal {
        path.parent().unwrap_or(path)
    } else {
        path
    };
    spawn_opener(Command::new("xdg-open").arg(target))
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
fn open_path(_path: &Path, _reveal: bool) -> Result<(), IpcError> {
    Err(IpcError::new(
        "library_open_unsupported",
        "Opening capture files is not supported on this platform.",
        true,
    ))
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn spawn_opener(command: &mut Command) -> Result<(), IpcError> {
    command.spawn().map(|_| ()).map_err(open_error)
}

fn open_error(error: std::io::Error) -> IpcError {
    IpcError::new(
        "library_open_failed",
        format!("Could not open the selected capture: {error}"),
        true,
    )
}
