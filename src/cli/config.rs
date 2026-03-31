use crate::cli::commands::CommandList;
use std::env::{current_dir, home_dir};
use std::error::Error;
use std::path::PathBuf;
use std::{fs, process};

#[derive(Debug)]
pub struct ApplicationConfig {
    pub build_file_path: PathBuf,
    pub build_args: Vec<String>,
    pub binary_install_path: PathBuf,
}

impl ApplicationConfig {
    fn get_install_binary_path() -> PathBuf {
        #[cfg(target_os = "windows")]
        {
            PathBuf::from(std::env::var("LOCALAPPDATA").expect("Windows LOCALAPPDATA path."))
                .join("Microsoft")
                .join("WindowsApps")
        }
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            PathBuf::from(
                std::env::home_dir()
                    .expect("Linux or MacOS homedir/.local/bin path.")
                    .join(".local/bin"),
            )
        }
    }

    fn expand_path(path_str: &str) -> PathBuf {
        if path_str.starts_with('~') {
            if let Some(home) = home_dir() {
                return PathBuf::from(path_str.replacen('~', &home.to_string_lossy(), 1));
            }
        }
        PathBuf::from(path_str)
    }

    pub fn build(
        mut args: impl Iterator<Item = String>,
    ) -> Result<ApplicationConfig, Box<dyn Error>> {
        let mut build_file_path = current_dir()?;
        let mut build_args = Vec::new();
        let mut custom_install_base: Option<PathBuf> = None;

        while let Some(arg) = args.next() {
            match CommandList::from_str(&arg) {
                Some(CommandList::BuildPath) => {
                    let path_str = args.next().ok_or("Missing value for --build-path")?;
                    let path = Self::expand_path(&path_str);

                    if !path.is_dir() {
                        return Err(
                            format!("Input --build-path is not a directory: {:?}", path).into()
                        );
                    }
                    build_file_path = path;
                }
                Some(CommandList::BuildArgs) => {
                    let input_args = args.next().ok_or("Missing values for --build-args")?;
                    build_args.extend(input_args.split_whitespace().map(|s| s.to_string()));
                }
                Some(CommandList::CompiledPath) => {
                    let path_str = args.next().ok_or("Missing value for --compiled-path")?;
                    let path = Self::expand_path(&path_str);

                    if !path.is_dir() {
                        return Err(format!(
                            "Input --compiled-path is not a directory: {:?}",
                            path
                        )
                        .into());
                    }
                    custom_install_base = Some(path);
                }
                Some(CommandList::ListBinaries) => {
                    let binary_list = fs::read_dir(ApplicationConfig::get_install_binary_path())?;

                    let file_list: Vec<_> = binary_list
                        .filter_map(|entry| entry.ok())
                        .map(|entry| entry.path())
                        .collect();

                    println!("{:?}", file_list);
                    process::exit(0);
                }
                Some(CommandList::Version) => {
                    let version = env!("CARGO_PKG_VERSION");
                    println!("bti version {}", version);
                    process::exit(0);
                }
                Some(CommandList::Help) => {
                    println!("Commandlist\n{}", CommandList::to_string());
                    process::exit(0);
                }
                None => {
                    return Err(format!(
                        "Unknown argument '{}' \n\nCommand list: \n{}",
                        arg,
                        CommandList::to_string()
                    )
                    .into());
                }
            }
        }

        let project_name = build_file_path.file_name().ok_or("Invalid build path")?;

        let binary_install_path = match custom_install_base {
            Some(base) => base.join(project_name),
            None => Self::get_install_binary_path().join(project_name),
        };

        Ok(ApplicationConfig {
            build_file_path,
            build_args,
            binary_install_path,
        })
    }
}
