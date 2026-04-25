use crate::cli::build::{self, BuildConfig};
use crate::cli::config::{ApplicationAction, ApplicationConfig};
use crate::cli::error::AppError;
use std::env;

fn print_and_short_circuit(
    action: ApplicationAction,
) -> Result<Option<ApplicationConfig>, AppError> {
    match action {
        ApplicationAction::Build(config) => Ok(Some(config)),
        ApplicationAction::ListBinaries(file_list) => {
            println!("{file_list:?}");
            Ok(None)
        }
        ApplicationAction::PrintMessage(message) => {
            println!("{message}");
            Ok(None)
        }
    }
}

pub fn run() -> Result<(), AppError> {
    let cli_args = env::args();

    let application_action = ApplicationConfig::build(cli_args.skip(1))?;

    let Some(application_config) = print_and_short_circuit(application_action)? else {
        return Ok(());
    };

    let build_config = BuildConfig::build(application_config)?;
    build::run_build_command(&build_config)?;

    build::install_compiled_binary_with_fallback(&build_config)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::current_dir;
    use std::path::PathBuf;

    fn test_current_dir() -> PathBuf {
        match current_dir() {
            Ok(path) => path,
            Err(error) => panic!("current_dir failed: {error}"),
        }
    }

    #[test]
    fn valid_application_config_builds() {
        let test_args: [String; 0] = [];
        let result = ApplicationConfig::build(test_args.into_iter());
        assert!(
            matches!(result, Ok(ApplicationAction::Build(_))),
            "ApplicationConfig should build with valid args."
        );
    }

    #[test]
    fn invalid_application_config_build() {
        let test_args: [String; 2] = [String::from("--bti"), String::from("--isbest")];
        let result = ApplicationConfig::build(test_args.into_iter());
        assert!(
            result.is_err(),
            "ApplicationConfig should not build with invalid args."
        );
    }

    #[test]
    fn valid_build_config_builds() {
        let build_file_path = test_current_dir();
        let build_args = Vec::new();
        let binary_install_path: PathBuf = build_file_path.clone();

        let test_application_config = ApplicationConfig {
            build_file_path,
            build_args,
            binary_install_path,
            binary_name: None,
        };
        let result = BuildConfig::build(test_application_config);
        assert!(result.is_ok(), "BuildConfig should build with valid args.");
    }

    #[test]
    fn invalid_build_config_build() {
        let build_file_path = PathBuf::new();
        let build_args = Vec::new();
        let binary_install_path: PathBuf = PathBuf::new();

        let test_application_config = ApplicationConfig {
            build_file_path,
            build_args,
            binary_install_path,
            binary_name: None,
        };
        let result = BuildConfig::build(test_application_config);
        assert!(
            result.is_err(),
            "BuildConfig should not build with invalid args."
        );
    }
}
