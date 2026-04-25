use crate::cli::config::ApplicationConfig;
use std::ffi::OsStr;
use std::fs::{self};
use std::io::{self};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::SystemTime;

#[derive(Debug)]
pub(crate) enum BuildCommand {
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
            BuildCommand::Maven => OsStr::new("mvn"),
            BuildCommand::MavenW => OsStr::new("mvnw"),
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
    configure_args: Option<Vec<String>>,
    pub build_file_path: PathBuf,
    pub target_release_path: PathBuf,
    pub target_binary_install_path: PathBuf,
    pub binary_name: PathBuf,
}

#[derive(Debug)]
pub enum BuildError {
    InvalidPath,
    Io(io::Error),
    BuildFailed { command: String, status: ExitStatus },
    UnsupportedBuildSystem(PathBuf),
}

impl std::error::Error for BuildError {}

impl From<io::Error> for BuildError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPath => write!(f, "The provided path is not a valid directory."),
            Self::Io(error) => write!(f, "Build I/O error: {error}"),
            Self::BuildFailed { command, status } => {
                write!(f, "Build command '{command}' failed with {status}")
            }
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

        let mut configure_args = None;

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
                    fs::create_dir_all(application_config.build_file_path.join("build"))?;
                    configure_args = Some(vec![
                        "-S".to_string(),
                        ".".to_string(),
                        "-B".to_string(),
                        "build".to_string(),
                    ]);
                    args.extend([
                        "--build".to_string(),
                        "build".to_string(),
                        "--config".to_string(),
                        "Release".to_string(),
                        "-j".to_string(),
                        num_cores.clone(),
                    ]);
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
                let binary_target = platform_binary_name(project_name);

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
                        format!("bin/{}", binary_target.display()),
                        ".".to_string(),
                    ]);
                }
                (args, PathBuf::from("bin"))
            }
        };

        let binary_name = application_config
            .binary_name
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| platform_binary_name(project_name));

        let target_release_path = application_config
            .build_file_path
            .join(&base_path)
            .join(&binary_name);

        let target_binary_install_path = application_config.binary_install_path;

        Ok(BuildConfig {
            command,
            args,
            configure_args,
            build_file_path: application_config.build_file_path,
            target_release_path,
            target_binary_install_path,
            binary_name,
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

fn platform_binary_name(project_name: &OsStr) -> PathBuf {
    let binary_name = PathBuf::from(project_name);

    #[cfg(target_os = "windows")]
    {
        if binary_name.extension().is_some() {
            binary_name
        } else {
            let mut s = binary_name.into_os_string();
            s.push(".exe");
            PathBuf::from(s)
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        binary_name
    }
}

pub fn run_build_command(build_config: &BuildConfig) -> Result<(), BuildError> {
    run_build_command_with(build_config, |command, args, current_dir| {
        run_command(command, args, current_dir)
    })
}

pub(crate) fn run_build_command_with<F>(
    build_config: &BuildConfig,
    mut exec: F,
) -> Result<(), BuildError>
where
    F: FnMut(&BuildCommand, &[String], &Path) -> Result<(), BuildError>,
{
    eprintln!("\n--- Starting Build ---");

    if let Some(configure_args) = &build_config.configure_args {
        eprintln!(
            "Configuring {} using {:?} with args: {:?}",
            &build_config.build_file_path.display(),
            BuildCommand::Cmake,
            configure_args
        );

        exec(
            &BuildCommand::Cmake,
            configure_args,
            &build_config.build_file_path,
        )?;
    }

    eprintln!(
        "Compiling {} using {:?} with args: {:?}",
        &build_config.target_release_path.display(),
        &build_config.command,
        &build_config.args
    );

    exec(
        &build_config.command,
        &build_config.args,
        &build_config.build_file_path,
    )?;

    println!("Build finished successfully!");

    Ok(())
}

fn run_command(
    command: &BuildCommand,
    args: &[String],
    current_dir: &Path,
) -> Result<(), BuildError> {
    let status = Command::new(command)
        .args(args)
        .current_dir(current_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(BuildError::BuildFailed {
            command: command.as_ref().to_string_lossy().into_owned(),
            status,
        })
    }
}

pub fn install_compiled_binary(from: &Path, to: &Path) -> Result<(), io::Error> {
    if !from.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Source binary not found at {}", from.display()),
        ));
    }

    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!(
                    "Failed to create install directory {}: {e}",
                    parent.display()
                ),
            )
        })?;
    }

    fs::copy(from, to).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!(
                "Failed to copy binary from {} to {}: {e}",
                from.display(),
                to.display()
            ),
        )
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(to)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(to, perms)?;
    }

    eprintln!("Binary installed at path: {}", to.display());

    Ok(())
}

pub fn install_compiled_binary_with_fallback(build_config: &BuildConfig) -> Result<(), io::Error> {
    if build_config.target_release_path.is_file() {
        return install_compiled_binary(
            &build_config.target_release_path,
            &build_config.target_binary_install_path,
        );
    }

    if !matches!(build_config.command, BuildCommand::Cmake) {
        return install_compiled_binary(
            &build_config.target_release_path,
            &build_config.target_binary_install_path,
        );
    }

    if let Ok(prefix) = cmake_install_prefix(build_config) {
        if run_cmake_install(build_config, &prefix).is_ok() {
            return Ok(());
        }
    }

    let build_dir = build_config.build_file_path.join("build");
    let discovered = discover_cmake_executable(&build_dir)?;

    let install_target = if build_config.binary_name
        != platform_binary_name(
            build_config
                .build_file_path
                .file_name()
                .unwrap_or_else(|| OsStr::new("")),
        ) {
        build_config.target_binary_install_path.clone()
    } else {
        let parent = build_config
            .target_binary_install_path
            .parent()
            .unwrap_or_else(|| Path::new("."));
        parent.join(
            discovered
                .file_name()
                .unwrap_or_else(|| build_config.target_binary_install_path.as_os_str()),
        )
    };

    install_compiled_binary(&discovered, &install_target)
}

fn cmake_install_prefix(build_config: &BuildConfig) -> Result<PathBuf, io::Error> {
    let bin_dir = build_config
        .target_binary_install_path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Install path has no parent dir"))?;

    let prefix = bin_dir.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::Other,
            "Install bin dir has no parent (cannot infer prefix)",
        )
    })?;

    Ok(prefix.to_path_buf())
}

fn run_cmake_install(build_config: &BuildConfig, prefix: &Path) -> Result<(), io::Error> {
    let build_dir = build_config.build_file_path.join("build");
    if !build_dir.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("CMake build directory not found at {}", build_dir.display()),
        ));
    }

    if !build_dir.join("cmake_install.cmake").is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "CMake install script not found (cmake_install.cmake missing)",
        ));
    }

    let status = Command::new("cmake")
        .arg("--install")
        .arg("build")
        .arg("--prefix")
        .arg(prefix)
        .current_dir(&build_config.build_file_path)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            format!("cmake --install failed with {status}"),
        ))
    }
}

fn discover_cmake_executable(build_dir: &Path) -> Result<PathBuf, io::Error> {
    let mut roots = Vec::new();
    let rundir = build_dir.join("rundir");
    if rundir.is_dir() {
        roots.push(rundir);
    }
    roots.push(build_dir.to_path_buf());

    let mut best: Option<(i64, SystemTime, PathBuf)> = None;

    for root in roots {
        let mut queue: std::collections::VecDeque<(PathBuf, usize)> =
            std::collections::VecDeque::from([(root, 0)]);

        while let Some((dir, depth)) = queue.pop_front() {
            if depth > 6 {
                continue;
            }

            let Ok(entries) = fs::read_dir(&dir) else {
                continue;
            };

            for entry in entries.flatten() {
                let path = entry.path();
                let name = entry.file_name();
                let name = name.to_string_lossy();

                if path.is_dir() {
                    if name == "CMakeFiles"
                        || name == ".git"
                        || name == "Testing"
                        || name == "_deps"
                        || name == "deps"
                    {
                        continue;
                    }
                    queue.push_back((path, depth + 1));
                    continue;
                }

                if !path.is_file() {
                    continue;
                }

                if name.ends_with(".o")
                    || name.ends_with(".a")
                    || name.ends_with(".so")
                    || name.contains(".so.")
                    || name.ends_with(".dylib")
                    || name.ends_with(".dll")
                    || name.ends_with(".cmake")
                    || name == "CMakeCache.txt"
                {
                    continue;
                }

                let Ok(meta) = entry.metadata() else {
                    continue;
                };

                if !is_executable(&meta) {
                    continue;
                }

                let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                let score = score_cmake_candidate(&path);

                match &best {
                    None => best = Some((score, mtime, path)),
                    Some((best_score, best_mtime, _)) => {
                        if score > *best_score || (score == *best_score && mtime > *best_mtime) {
                            best = Some((score, mtime, path));
                        }
                    }
                }
            }
        }
    }

    best.map(|(_, _, p)| p).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "Could not auto-detect a built executable in {}. Try --binary-name <name> or set --compiled-path and copy manually.",
                build_dir.display()
            ),
        )
    })
}

fn score_cmake_candidate(path: &Path) -> i64 {
    let s = path.to_string_lossy();
    let mut score = 0;
    if s.contains("/rundir/") {
        score += 1000;
    }
    if s.contains("/bin/") {
        score += 200;
    }
    score -= s.matches('/').count() as i64;
    score
}

fn is_executable(meta: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        (meta.permissions().mode() & 0o111) != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

#[cfg(test)]
mod run_command_tests {
    use super::*;
    use crate::cli::config::ApplicationConfig;
    use std::path::PathBuf;

    #[test]
    fn run_build_command_with_injected_noop_does_not_spawn() {
        let build_file_path = std::env::current_dir().expect("cwd");
        let app = ApplicationConfig {
            build_file_path: build_file_path.clone(),
            build_args: vec![],
            binary_install_path: build_file_path.clone(),
            binary_name: None,
        };
        let build_config = BuildConfig::build(app).expect("valid project dir with build files");
        let result = run_build_command_with(&build_config, |_, _, _| Ok(()));
        assert!(result.is_ok());
    }

    #[test]
    fn run_build_command_with_propagates_exec_errors() {
        let build_file_path = std::env::current_dir().expect("cwd");
        let app = ApplicationConfig {
            build_file_path: build_file_path.clone(),
            build_args: vec![],
            binary_install_path: PathBuf::new(),
            binary_name: None,
        };
        let build_config = BuildConfig::build(app).expect("build config");
        let result = run_build_command_with(&build_config, |_, _, _| Err(BuildError::InvalidPath));
        assert!(matches!(result, Err(BuildError::InvalidPath)));
    }
}
