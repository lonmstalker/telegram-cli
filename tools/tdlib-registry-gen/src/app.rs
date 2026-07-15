use std::error::Error;
use std::ffi::OsString;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use crate::engine;

const MANIFEST_PATH: &str = "vendor/tdlib/manifest.json";
const SCHEMA_PATH: &str = "vendor/tdlib/td_api.tl";
const POLICY_PATH: &str = "policy/tdlib-feature-owners.json";
pub(crate) const OUTPUT_PATH: &str = "generated/tdlib-feature-owners.json";
pub(crate) const TEMP_SUFFIX: &str = ".tmp";

const MAX_MANIFEST_BYTES: usize = 64 * 1024;
const MAX_SCHEMA_BYTES: usize = 2 * 1024 * 1024;
const MAX_POLICY_BYTES: usize = 1024 * 1024;
const MAX_OUTPUT_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Command {
    Check,
    Generate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Outcome {
    Current,
    Updated,
    Unchanged,
}

pub(crate) fn parse_command(args: &[String]) -> Result<Command, AppError> {
    match args {
        [command] if command == "check" => Ok(Command::Check),
        [command] if command == "generate" => Ok(Command::Generate),
        _ => Err(AppError::new(
            AppErrorKind::Usage,
            "использование: tdlib-registry-gen <check|generate>",
        )),
    }
}

pub(crate) fn run(root: &Path, command: Command) -> Result<Outcome, AppError> {
    match command {
        Command::Check => check(root),
        Command::Generate => run_generate_with_hook(root, || {}),
    }
}

fn check(root: &Path) -> Result<Outcome, AppError> {
    let expected = build_expected(root)?;
    let output_path = root.join(OUTPUT_PATH);
    let actual = read_optional(&output_path, MAX_OUTPUT_BYTES)?;
    if actual.as_deref() == Some(expected.as_slice()) {
        Ok(Outcome::Current)
    } else {
        Err(AppError::new(
            AppErrorKind::OutOfDate,
            format!(
                "{} отсутствует или не совпадает с canonical generation",
                output_path.display()
            ),
        ))
    }
}

fn run_generate_with_hook(root: &Path, after_acquire: impl FnOnce()) -> Result<Outcome, AppError> {
    let output_path = root.join(OUTPUT_PATH);
    let mut publication = Publication::acquire(&output_path)?;
    after_acquire();
    let expected = build_expected(root)?;
    if read_optional(&output_path, MAX_OUTPUT_BYTES)?.as_deref() == Some(expected.as_slice()) {
        return Ok(Outcome::Unchanged);
    }
    publication.publish(&expected)?;
    Ok(Outcome::Updated)
}

fn build_expected(root: &Path) -> Result<Vec<u8>, AppError> {
    let manifest = read_required(&root.join(MANIFEST_PATH), MAX_MANIFEST_BYTES)?;
    let schema = read_required(&root.join(SCHEMA_PATH), MAX_SCHEMA_BYTES)?;
    let policy = read_required(&root.join(POLICY_PATH), MAX_POLICY_BYTES)?;
    engine::generate(&manifest, &schema, &policy).map_err(|error| {
        AppError::new(
            AppErrorKind::Generation,
            format!("owner generation failed ({:?}): {error}", error.kind()),
        )
    })
}

#[cfg(test)]
pub(crate) fn atomic_publish(output_path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    Publication::acquire(output_path)?.publish(bytes)
}

struct Publication {
    output_path: PathBuf,
    parent_path: PathBuf,
    temp_path: PathBuf,
    directory: File,
    file: Option<File>,
    armed: bool,
}

impl Publication {
    fn acquire(output_path: &Path) -> Result<Self, AppError> {
        let parent = output_path.parent().ok_or_else(|| {
            AppError::new(AppErrorKind::Io, "generated output has no parent directory")
        })?;
        let path_metadata =
            fs::symlink_metadata(parent).map_err(|error| io_error("inspect", parent, error))?;
        if !path_metadata.file_type().is_dir() {
            return Err(AppError::new(
                AppErrorKind::Io,
                format!("{} is not a real directory", parent.display()),
            ));
        }
        let directory = File::open(parent).map_err(|error| io_error("open", parent, error))?;
        let opened_metadata = directory
            .metadata()
            .map_err(|error| io_error("inspect", parent, error))?;
        if !same_file(&path_metadata, &opened_metadata) {
            return Err(path_identity_error(parent));
        }

        let temp_path = temp_path(output_path)?;
        let file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
        {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                return Err(AppError::new(
                    AppErrorKind::ConcurrentWriter,
                    format!(
                        "fixed publication temp {} already exists; another writer or a crashed run must be resolved",
                        temp_path.display()
                    ),
                ));
            }
            Err(error) => return Err(io_error("create", &temp_path, error)),
        };

        let publication = Self {
            output_path: output_path.to_owned(),
            parent_path: parent.to_owned(),
            temp_path,
            directory,
            file: Some(file),
            armed: true,
        };
        publication.verify_parent_identity()?;
        publication.verify_temp_identity()?;
        Ok(publication)
    }

    fn publish(&mut self, bytes: &[u8]) -> Result<(), AppError> {
        if bytes.len() > MAX_OUTPUT_BYTES {
            return Err(AppError::new(
                AppErrorKind::ResourceLimit,
                format!(
                    "publication payload is {} bytes, exceeding the {MAX_OUTPUT_BYTES}-byte cap",
                    bytes.len()
                ),
            ));
        }
        let file = self.file.as_mut().expect("publication owns an open temp");
        file.write_all(bytes)
            .map_err(|error| io_error("write", &self.temp_path, error))?;
        file.sync_all()
            .map_err(|error| io_error("sync", &self.temp_path, error))?;

        self.verify_parent_identity()?;
        self.verify_temp_identity()?;
        fs::rename(&self.temp_path, &self.output_path)
            .map_err(|error| io_error("rename", &self.temp_path, error))?;
        self.verify_output_identity()?;
        self.directory
            .sync_all()
            .map_err(|error| io_error("sync", &self.parent_path, error))?;
        self.armed = false;
        self.file.take();
        Ok(())
    }

    fn verify_parent_identity(&self) -> Result<(), AppError> {
        let path_metadata = fs::symlink_metadata(&self.parent_path)
            .map_err(|error| io_error("inspect", &self.parent_path, error))?;
        let opened_metadata = self
            .directory
            .metadata()
            .map_err(|error| io_error("inspect", &self.parent_path, error))?;
        if !path_metadata.file_type().is_dir() || !same_file(&path_metadata, &opened_metadata) {
            return Err(path_identity_error(&self.parent_path));
        }
        Ok(())
    }

    fn verify_temp_identity(&self) -> Result<(), AppError> {
        self.verify_file_identity(&self.temp_path)
    }

    fn verify_output_identity(&self) -> Result<(), AppError> {
        self.verify_file_identity(&self.output_path)
    }

    fn verify_file_identity(&self, path: &Path) -> Result<(), AppError> {
        let file = self.file.as_ref().expect("publication owns an open temp");
        let opened_metadata = file
            .metadata()
            .map_err(|error| io_error("inspect", path, error))?;
        let path_metadata =
            fs::symlink_metadata(path).map_err(|error| io_error("inspect", path, error))?;
        if !path_metadata.file_type().is_file() || !same_file(&path_metadata, &opened_metadata) {
            return Err(path_identity_error(path));
        }
        Ok(())
    }
}

impl Drop for Publication {
    fn drop(&mut self) {
        if self.armed {
            let owns_path = self.file.as_ref().is_some_and(|file| {
                let Ok(opened_metadata) = file.metadata() else {
                    return false;
                };
                let Ok(path_metadata) = fs::symlink_metadata(&self.temp_path) else {
                    return false;
                };
                path_metadata.file_type().is_file() && same_file(&path_metadata, &opened_metadata)
            });
            if owns_path {
                let _ = fs::remove_file(&self.temp_path);
            }
            self.file.take();
        }
    }
}

fn temp_path(output_path: &Path) -> Result<PathBuf, AppError> {
    let file_name = output_path
        .file_name()
        .ok_or_else(|| AppError::new(AppErrorKind::Io, "generated output has no file name"))?;
    let mut temp_name = OsString::from(file_name);
    temp_name.push(TEMP_SUFFIX);
    Ok(output_path.with_file_name(temp_name))
}

fn read_required(path: &Path, cap: usize) -> Result<Vec<u8>, AppError> {
    read_optional(path, cap)?.ok_or_else(|| {
        AppError::new(
            AppErrorKind::Io,
            format!("required input {} does not exist", path.display()),
        )
    })
}

fn read_optional(path: &Path, cap: usize) -> Result<Option<Vec<u8>>, AppError> {
    let path_metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(io_error("inspect", path, error)),
    };
    if !path_metadata.file_type().is_file() {
        return Err(AppError::new(
            AppErrorKind::Io,
            format!("{} is not a real regular file", path.display()),
        ));
    }
    let mut file = File::open(path).map_err(|error| io_error("open", path, error))?;
    let opened_metadata = file
        .metadata()
        .map_err(|error| io_error("inspect", path, error))?;
    if !same_file(&path_metadata, &opened_metadata) {
        return Err(path_identity_error(path));
    }
    if opened_metadata.len() > cap as u64 {
        return Err(AppError::new(
            AppErrorKind::ResourceLimit,
            format!(
                "{} is {} bytes, exceeding the {cap}-byte cap",
                path.display(),
                opened_metadata.len()
            ),
        ));
    }

    let initial_capacity = usize::try_from(opened_metadata.len())
        .unwrap_or(cap)
        .min(cap);
    let mut bytes = Vec::with_capacity(initial_capacity);
    Read::by_ref(&mut file)
        .take(cap as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| io_error("read", path, error))?;
    if bytes.len() > cap {
        return Err(AppError::new(
            AppErrorKind::ResourceLimit,
            format!("{} grew beyond the {cap}-byte cap", path.display()),
        ));
    }
    let final_path_metadata =
        fs::symlink_metadata(path).map_err(|error| io_error("revalidate", path, error))?;
    if !final_path_metadata.file_type().is_file()
        || !same_file(&final_path_metadata, &opened_metadata)
    {
        return Err(path_identity_error(path));
    }
    Ok(Some(bytes))
}

fn path_identity_error(path: &Path) -> AppError {
    AppError::new(
        AppErrorKind::Io,
        format!("{} changed identity or crosses a symlink", path.display()),
    )
}

#[cfg(unix)]
fn same_file(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;

    left.dev() == right.dev() && left.ino() == right.ino()
}

#[cfg(not(unix))]
fn same_file(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    left.file_type() == right.file_type()
        && left.len() == right.len()
        && left.modified().ok() == right.modified().ok()
}

fn io_error(operation: &str, path: &Path, error: io::Error) -> AppError {
    AppError::new(
        AppErrorKind::Io,
        format!("cannot {operation} {}: {error}", path.display()),
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AppErrorKind {
    Usage,
    Io,
    ResourceLimit,
    Generation,
    OutOfDate,
    ConcurrentWriter,
}

#[derive(Debug)]
pub(crate) struct AppError {
    kind: AppErrorKind,
    detail: String,
}

impl AppError {
    fn new(kind: AppErrorKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
        }
    }

    pub(crate) fn kind(&self) -> AppErrorKind {
        self.kind
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for AppError {}

#[cfg(test)]
mod tests;
