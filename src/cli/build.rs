use crate::cli::config::ApplicationConfig;
use std::ffi::OsStr;
use std::fs::{self};
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;

#[derive(Debug)]
enum BuildCommand {
    Cargo,
    Cmake,
    Npm,
    Bun,
    Zig,
    Maven,
    MavenW,
    Gradle,
    GradleW,
    Go,
}

impl AsRef<OsStr> for BuildCommand {
    fn as_ref(&self) -> &OsStr {
        match self {
            BuildCommand::Cargo => OsStr::new("cargo"),
            BuildCommand::Cmake => OsStr::new("cmake"),
            BuildCommand::Npm => OsStr::new("npm"),
            BuildCommand::Bun => OsStr::new("bun"),
            BuildCommand::Zig => OsStr::new("zig"),
            BuildCommand::Maven => OsStr::new("maven"),
            BuildCommand::MavenW => OsStr::new("mavenw"),
            BuildCommand::Gradle => OsStr::new("gradle"),
            BuildCommand::GradleW => OsStr::new("gradlew"),
            BuildCommand::Go => OsStr::new("go"),
        }
    }
}

#[derive(Debug)]
pub struct BuildConfig {
    command: BuildCommand,
    args: Vec<String>,
    pub target_release_path: PathBuf,
    pub target_binary_install_path: PathBuf,
}

#[derive(Debug)]
pub enum BuildError {
    InvalidPath,
    UnsupportedBuildSystem(PathBuf),
}

impl std::error::Error for BuildError {}

impl From<std::io::Error> for BuildError {
    fn from(_: std::io::Error) -> Self {
        Self::InvalidPath
    }
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPath => write!(f, "The provided path is not a valid directory."),
            Self::UnsupportedBuildSystem(p) => {
                write!(f, "No supported build system found in {}", p.display())
            }
        }
    }
}

impl BuildConfig {
    pub fn build(application_config: ApplicationConfig) -> Result<Self, BuildError> {
        let project_name = application_config
            .build_file_path
            .file_name()
            .ok_or(BuildError::InvalidPath)?;

        let command = fs::read_dir(&application_config.build_file_path)?
            .flatten()
            .find_map(|entry| {
                let name = entry.file_name().to_string_lossy().to_lowercase();

                match name.as_ref() {
                    "cargo.toml" => Some(BuildCommand::Cargo),
                    "cmakelists.txt" => Some(BuildCommand::Cmake),
                    "pom.xml" => Some(BuildCommand::Maven),
                    "mvnw" | "mvnw.cmd" => Some(BuildCommand::MavenW),
                    "build.gradle" | "build.gradle.kts" => Some(BuildCommand::Gradle),
                    "gradlew" | "gradlew.bat" => Some(BuildCommand::GradleW),
                    "package.json" => Some(BuildCommand::Npm),
                    "bun.lockb" => Some(BuildCommand::Bun),
                    "build.zig" => Some(BuildCommand::Zig),
                    "go.mod" => Some(BuildCommand::Go),
                    _ => None,
                }
            })
            .ok_or_else(|| {
                BuildError::UnsupportedBuildSystem(application_config.build_file_path.clone())
            })?;

        let num_cores = thread::available_parallelism()
            .map(|n| n.get())?
            .to_string();

        let (args, base_path) = match command {
            BuildCommand::Cargo => {
                let mut args: Vec<String> = vec![];
                if !application_config.build_args.is_empty() {
                    args.extend(application_config.build_args);
                } else {
                    args.extend([
                        "build".to_string(),
                        "--release".to_string(),
                        "-j".to_string(),
                        num_cores.clone(),
                    ]);
                }
                (args, PathBuf::from("target/release"))
            }
            BuildCommand::Cmake => {
                let mut args: Vec<String> = vec![];
                if !application_config.build_args.is_empty() {
                    args.extend(application_config.build_args);
                } else {
                    let _ = fs::create_dir_all("build");

                    let configure_status = std::process::Command::new("cmake")
                        .arg("-S")
                        .arg(".")
                        .arg("-B")
                        .arg("build")
                        .arg("-DCMAKE_BUILD_TYPE=Release")
                        .status();

                    if configure_status.is_ok_and(|s| s.success()) {
                        args.extend([
                            "--build".to_string(),
                            "build".to_string(),
                            "--config".to_string(),
                            "Release".to_string(),
                            "-j".to_string(),
                            num_cores.clone(),
                        ]);
                    }
                }
                (args, get_cmake_base_path())
            }
            BuildCommand::Zig => {
                let mut args: Vec<String> = vec![];
                if !application_config.build_args.is_empty() {
                    args.extend(application_config.build_args);
                } else {
                    args.extend([
                        "build".to_string(),
                        "-Doptimize=ReleaseSafe".to_string(),
                        "-j".to_string(),
                        num_cores.clone(),
                    ]);
                }
                (args, PathBuf::from("zig-out/bin"))
            }
            BuildCommand::Maven | BuildCommand::MavenW => {
                let mut args: Vec<String> = vec![];
                if !application_config.build_args.is_empty() {
                    args.extend(application_config.build_args);
                } else {
                    args.extend([
                        "clean".to_string(),
                        "verify".to_string(),
                        "-T".to_string(),
                        num_cores.clone(),
                        "-DskipTests".to_string(),
                        "-P".to_string(),
                        "release".to_string(),
                    ]);
                }
                (args, application_config.build_file_path.clone())
            }
            BuildCommand::Gradle | BuildCommand::GradleW => {
                let mut args: Vec<String> = vec![];
                if !application_config.build_args.is_empty() {
                    args.extend(application_config.build_args);
                } else {
                    args.extend([
                        "clean".to_string(),
                        "build".to_string(),
                        "--configuration-cache".to_string(),
                        "--parallel".to_string(),
                        format!("--max-workers={}", num_cores),
                        "-PbuildType=release".to_string(),
                    ]);
                }
                (args, application_config.build_file_path.clone())
            }
            BuildCommand::Npm | BuildCommand::Bun => {
                let mut args: Vec<String> = vec![];
                if !application_config.build_args.is_empty() {
                    args = application_config.build_args;
                } else {
                    args.extend(["run".to_string(), "build".to_string()]);
                }
                (args, PathBuf::from("dist"))
            }
            BuildCommand::Go => {
                let mut _binary_target = project_name.to_owned();
                #[cfg(target_os = "windows")]
                {
                    _binary_target.set_extension("exe");
                }

                let mut args: Vec<String> = vec![];
                if !application_config.build_args.is_empty() {
                    args.extend(application_config.build_args);
                } else {
                    args.extend([
                        "build".to_string(),
                        "-p".to_string(),
                        num_cores.clone(),
                        "-ldflags=-s -w".to_string(),
                        "-o".to_string(),
                        format!("bin/{}", _binary_target.display()),
                        ".".to_string(),
                    ]);
                }
                (args, PathBuf::from("bin"))
            }
        };

        let mut _binary_name = project_name.to_owned();
        #[cfg(target_os = "windows")]
        {
            // Check if it already has an extension, if not, add .exe
            if binary_name.extension().is_none() {
                let mut s = binary_name.into_os_string();
                s.push(".exe");
                _binary_name = PathBuf::from(s);
            }
        }

        let target_release_path = application_config
            .build_file_path
            .join(&base_path)
            .join(_binary_name);

        let target_binary_install_path = application_config.binary_install_path;

        Ok(BuildConfig {
            command,
            args,
            target_release_path,
            target_binary_install_path,
        })
    }
}

fn get_cmake_base_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        PathBuf::from("build").join("Release")
    }
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        PathBuf::from("build")
    }
}

pub fn run_build_command(build_config: &BuildConfig) -> bool {
    let mut stderr = io::stderr();

    writeln!(stderr, "\n--- Starting Build ---").expect("Failed to Write to stdout.");

    writeln!(
        stderr,
        "Running the following command with args: {:?} {:?}",
        &build_config.command, &build_config.args
    )
    .expect("Failed to Write to stdout.");

    stderr.flush().expect("Failed to Flush stdout.");

    let build_status = Command::new(&build_config.command)
        .args(&build_config.args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .expect("Failed to execute build command.");

    if build_status.success() {
        writeln!(stderr, "Build finished successfully!").expect("Failed to write to stdout.");
    } else {
        writeln!(stderr, "Result: Build failed. {}", build_status)
            .expect("Failed to Write to stdout.");
        writeln!(stderr, "----------------------").expect("Failed to Write to stdout.");
        stderr.flush().expect("Failed to flush stdout.");
        std::process::exit(1);
    }

    stderr.flush().expect("Failed to flush stdout.");

    return true;
}

pub fn install_compiled_binary(from: PathBuf, to: PathBuf) -> Result<(), io::Error> {
    if !from.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Source binary not found at {}", from.display()),
        ));
    }

    fs::copy(&from, &to)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&to)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&to, perms)?;
    }

    let mut stdout = io::stdout();

    writeln!(stdout, "Binary installed at path: {}", to.display()).expect("Write to stdout.");

    stdout.flush().expect("Flush stdout");
    Ok(())
}
