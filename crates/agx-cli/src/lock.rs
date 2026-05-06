use std::fs::{self, File, OpenOptions};
use std::path::PathBuf;

use crate::errors::{AgxError, AgxErrorCode};

#[derive(Debug)]
pub struct ResourceLock {
    path: PathBuf,
    _file: File,
}

impl Drop for ResourceLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub fn with_resource_lock<T>(
    resource: &'static str,
    operation: impl FnOnce() -> Result<T, AgxError>,
) -> Result<T, AgxError> {
    let _lock = acquire_resource_lock(resource)?;
    operation()
}

pub fn acquire_resource_lock(resource: &'static str) -> Result<ResourceLock, AgxError> {
    let path = lock_path(resource);
    let file = acquire_lock_file(resource, &path)?;
    Ok(ResourceLock { path, _file: file })
}

fn acquire_lock_file(resource: &'static str, path: &PathBuf) -> Result<File, AgxError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            AgxError::new(
                AgxErrorCode::InvalidArgument,
                format!("Failed to create lock directory: {error}"),
            )
        })?;
    }

    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            let code = if error.kind() == std::io::ErrorKind::AlreadyExists {
                AgxErrorCode::ResourceLocked
            } else {
                AgxErrorCode::InvalidArgument
            };
            AgxError::new(code, format!("Resource {resource} is locked: {error}"))
        })
}

fn lock_path(resource: &str) -> PathBuf {
    home_dir()
        .join(".quantex")
        .join(format!("{}.lock", resource.replace(' ', "-")))
}

fn home_dir() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
}
