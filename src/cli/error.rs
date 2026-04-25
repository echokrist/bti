use crate::cli::commands::CommandList;
use std::fmt;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub enum CliError {
    Io(io::Error),
    MissingFlagValue { flag: &'static str },
    PathNotADirectory { path: PathBuf, flag: &'static str },
    UnknownArgument { arg: String },
    InvalidBuildPath,
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "{err}"),
            Self::MissingFlagValue { flag } => write!(f, "Missing value for {flag}"),
            Self::PathNotADirectory { path, flag } => {
                write!(f, "Input {flag} is not a directory: {}", path.display())
            }
            Self::UnknownArgument { arg } => write!(
                f,
                "Unknown argument '{arg}'\n\nCommand list: \n{}",
                CommandList::to_string()
            ),
            Self::InvalidBuildPath => write!(f, "Invalid build path"),
        }
    }
}

impl std::error::Error for CliError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for CliError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

#[derive(Debug)]
pub enum AppError {
    Cli(CliError),
    Build(crate::cli::build::BuildError),
    Install(io::Error),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cli(e) => e.fmt(f),
            Self::Build(e) => e.fmt(f),
            Self::Install(e) => write!(f, "Install I/O error: {e}"),
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Cli(e) => Some(e),
            Self::Build(e) => Some(e),
            Self::Install(e) => Some(e),
        }
    }
}

impl From<CliError> for AppError {
    fn from(e: CliError) -> Self {
        Self::Cli(e)
    }
}

impl From<crate::cli::build::BuildError> for AppError {
    fn from(e: crate::cli::build::BuildError) -> Self {
        Self::Build(e)
    }
}

impl From<io::Error> for AppError {
    fn from(e: io::Error) -> Self {
        Self::Install(e)
    }
}
