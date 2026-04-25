use crate::cli::commands::CommandList;
use crate::cli::error::CliError;
use std::env::{current_dir, home_dir};
use std::ffi::OsStr;
use std::fs;
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
pub struct ApplicationConfig {
    pub build_file_path: PathBuf,
    pub build_args: Vec<String>,
    pub binary_install_path: PathBuf,
    pub binary_name: Option<String>,
}

#[derive(Debug)]
pub enum ApplicationAction {
    Build(ApplicationConfig),
    ListBinaries(Vec<PathBuf>),
    PrintMessage(String),
}

impl ApplicationConfig {
    fn get_install_binary_path() -> Result<PathBuf, std::io::Error> {
        #[cfg(target_os = "windows")]
        {
            let local_app_data = std::env::var("LOCALAPPDATA").map_err(|error| {
                Error::new(
                    ErrorKind::NotFound,
                    format!("Windows LOCALAPPDATA path is not available: {error}"),
                )
            })?;

            Ok(PathBuf::from(local_app_data)
                .join("Microsoft")
                .join("WindowsApps"))
        }
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            let home = home_dir().ok_or_else(|| {
                Error::new(
                    ErrorKind::NotFound,
                    "Linux or MacOS homedir/.local/bin path is not available.",
                )
            })?;

            Ok(home.join(".local/bin"))
        }
    }

    pub fn build(mut args: impl Iterator<Item = String>) -> Result<ApplicationAction, CliError> {
        let mut build_file_path = current_dir()?;
        let mut build_args = Vec::new();
        let mut custom_install_base: Option<PathBuf> = None;
        let mut binary_name: Option<String> = None;

        while let Some(arg) = args.next() {
            match CommandList::from_str(&arg) {
                Some(CommandList::BuildPath) => {
                    let path_str = args.next().ok_or(CliError::MissingFlagValue {
                        flag: "--build-path",
                    })?;
                    let path = Self::generate_build_file_path(&path_str)?;

                    if !path.is_dir() {
                        return Err(CliError::PathNotADirectory {
                            path: path.clone(),
                            flag: "--build-path",
                        });
                    }
                    build_file_path = path;
                }
                Some(CommandList::BuildArgs) => {
                    let input_args = args.next().ok_or(CliError::MissingFlagValue {
                        flag: "--build-args",
                    })?;
                    build_args.extend(input_args.split_whitespace().map(|s| s.to_string()));
                }
                Some(CommandList::CompiledPath) => {
                    let path_str = args.next().ok_or(CliError::MissingFlagValue {
                        flag: "--compiled-path",
                    })?;
                    let path = Self::generate_compiled_file_path(&path_str);

                    if !path.is_dir() {
                        return Err(CliError::PathNotADirectory {
                            path: path.clone(),
                            flag: "--compiled-path",
                        });
                    }
                    custom_install_base = Some(path);
                }
                Some(CommandList::BinaryName) => {
                    let value = args.next().ok_or(CliError::MissingFlagValue {
                        flag: "--binary-name",
                    })?;
                    if value.trim().is_empty() {
                        return Err(CliError::UnknownArgument { arg: value });
                    }
                    binary_name = Some(value);
                }
                Some(CommandList::ListBinaries) => {
                    let binary_list = fs::read_dir(ApplicationConfig::get_install_binary_path()?)?;

                    let file_list: Vec<_> = binary_list
                        .map(|entry| entry.map(|entry| entry.path()))
                        .collect::<Result<_, _>>()?;

                    return Ok(ApplicationAction::ListBinaries(file_list));
                }
                Some(CommandList::Version) => {
                    let version = env!("CARGO_PKG_VERSION");
                    return Ok(ApplicationAction::PrintMessage(format!(
                        "bti version {}",
                        version
                    )));
                }
                Some(CommandList::Help) => {
                    return Ok(ApplicationAction::PrintMessage(format!(
                        "Commandlist\n{}",
                        CommandList::to_string()
                    )));
                }
                None => {
                    return Err(CliError::UnknownArgument { arg });
                }
            }
        }

        let project_name = build_file_path
            .file_name()
            .ok_or(CliError::InvalidBuildPath)?;

        let install_file_name = binary_name
            .as_deref()
            .map(OsStr::new)
            .unwrap_or(project_name);

        let binary_install_path = match custom_install_base {
            Some(base) => base.join(install_file_name),
            None => Self::get_install_binary_path()?.join(install_file_name),
        };

        Ok(ApplicationAction::Build(ApplicationConfig {
            build_file_path,
            build_args,
            binary_install_path,
            binary_name,
        }))
    }

    fn generate_build_file_path(path_str: &str) -> Result<PathBuf, std::io::Error> {
        if path_str.starts_with('~') {
            if let Some(home) = home_dir() {
                return Ok(PathBuf::from(path_str.replacen(
                    '~',
                    &home.to_string_lossy(),
                    1,
                )));
            }
        }

        let temp_dir = std::env::temp_dir();
        let file_name = Path::new(path_str).file_stem().ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidInput,
                format!("Could not determine filename from path: {path_str}"),
            )
        })?;

        let final_path = temp_dir.join(file_name);

        if final_path.exists() {
            std::fs::remove_dir_all(&final_path)?;
        }

        if path_str.starts_with("git") || path_str.ends_with(".git") {
            eprintln!("Downloading Git repository from {}", &path_str);
            let status = Command::new("git")
                .arg("clone")
                .arg("--depth")
                .arg("1")
                .arg("--recurse-submodules")
                .arg("--shallow-submodules")
                .arg(path_str)
                .arg(&final_path)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()?;
            if status.success() {
                return Ok(final_path);
            } else {
                return Err(Error::new(ErrorKind::Other, "Git exited with an error"));
            }
        }

        Ok(PathBuf::from(path_str))
    }

    fn generate_compiled_file_path(path_str: &str) -> PathBuf {
        if path_str.starts_with('~') {
            if let Some(home) = home_dir() {
                return PathBuf::from(path_str.replacen('~', &home.to_string_lossy(), 1));
            }
        }

        PathBuf::from(path_str)
    }
}
