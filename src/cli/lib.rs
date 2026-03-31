use crate::cli::build::{self, BuildConfig};
use crate::cli::config::ApplicationConfig;
use std::env;
use std::error::Error;

pub fn run() -> Result<(), Box<dyn Error + 'static>> {
    let cli_args = env::args();

    let application_config = ApplicationConfig::build(cli_args.skip(1))?;

    let build_config = BuildConfig::build(application_config)?;

    let build_completed = build::run_build_command(&build_config);

    if build_completed == false {
        eprintln!("Build failed, exiting...");
        std::process::exit(1);
    }

    build::install_compiled_binary(
        build_config.target_release_path,
        build_config.target_binary_install_path,
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::current_dir;
    use std::path::PathBuf;

    #[test]
    fn valid_application_config_builds() {
        let test_args: [String; 0] = [];
        let result = ApplicationConfig::build(test_args.into_iter());
        assert!(
            result.is_ok(),
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
        let build_file_path = current_dir().unwrap();
        let build_args = Vec::new();
        let binary_install_path: PathBuf = build_file_path.clone();

        let test_application_config = ApplicationConfig {
            build_file_path,
            build_args,
            binary_install_path,
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
        };
        let result = BuildConfig::build(test_application_config);
        assert!(
            result.is_err(),
            "BuildConfig should not build with invalid args."
        );
    }

    #[test]
    fn valid_run_build_command() {
        let build_file_path = current_dir().unwrap();
        let build_args = Vec::new();
        let binary_install_path: PathBuf = PathBuf::from(build_file_path.file_name().unwrap());

        let test_application_config = ApplicationConfig {
            build_file_path,
            build_args,
            binary_install_path,
        };

        let build_config = BuildConfig::build(test_application_config).unwrap();

        let result = build::run_build_command(&build_config);
        assert!(
            result,
            "run_build_command should return true with valid build config."
        );
    }

    #[test]
    #[should_panic(expected = "InvalidPath")]
    fn invalid_run_build_command() {
        let build_file_path = PathBuf::new();
        let build_args = vec![String::from("this makes no sense.")];
        let binary_install_path: PathBuf = PathBuf::new();

        let test_application_config = ApplicationConfig {
            build_file_path,
            build_args,
            binary_install_path,
        };
        let build_config = BuildConfig::build(test_application_config).unwrap();

        build::run_build_command(&build_config);
    }
}
