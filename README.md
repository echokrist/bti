# bti

`bti` (build and install) is a command-line tool written in Rust that automates building a project and installing the resulting binary to a standard location on your system. It detects the build system used by a project, runs the appropriate build command with sensible release defaults, and copies the compiled output to your local binary directory.

## How it works

### 1. Argument parsing (`src/cli/config.rs`)

When you run `bti`, it reads command-line arguments and constructs an `ApplicationConfig` containing:

- `build_file_path` - the directory of the project to build (defaults to the current directory)
- `build_args` - optional custom arguments to pass to the build tool
- `binary_install_path` - where to install the compiled binary after a successful build

The install path defaults to platform-specific locations:

| Platform        | Default install path                        |
|-----------------|---------------------------------------------|
| Linux / macOS   | `~/.local/bin/<project-name>`               |
| Windows         | `%LOCALAPPDATA%\Microsoft\WindowsApps\<project-name>` |

### 2. Build system detection (`src/cli/build.rs`)

`bti` scans the project directory for well-known build files to determine which build system to use:

| File detected              | Build system  |
|----------------------------|---------------|
| `Cargo.toml`               | Cargo (Rust)  |
| `CMakeLists.txt`           | CMake         |
| `pom.xml`                  | Maven         |
| `mvnw` / `mvnw.cmd`        | Maven Wrapper |
| `build.gradle` / `.kts`    | Gradle        |
| `gradlew` / `gradlew.bat`  | Gradle Wrapper|
| `package.json`             | npm           |
| `bun.lockb`                | Bun           |
| `build.zig`                | Zig           |
| `go.mod`                   | Go            |

If no supported build file is found, `bti` exits with an error.

### 3. Building the project

Once the build system is identified, `bti` constructs a release-optimized build command using the number of available CPU cores for parallelism, then executes it with output streamed directly to your terminal. Default commands per tool:

- **Cargo**: `cargo build --release -j <cores>`
- **CMake**: configures with `-DCMAKE_BUILD_TYPE=Release`, then `cmake --build build --config Release -j <cores>`
- **Zig**: `zig build -Doptimize=ReleaseSafe -j <cores>`
- **Maven**: `mvn clean verify -T <cores> -DskipTests -P release`
- **Gradle**: `gradle clean build --configuration-cache --parallel --max-workers=<cores> -PbuildType=release`
- **npm / Bun**: `npm run build` / `bun run build`
- **Go**: `go build -p <cores> -ldflags="-s -w" -o bin/<name> .`

You can override all of these by passing `--build-args`.

### 4. Installing the binary

After a successful build, `bti` copies the compiled binary from the build output directory to the resolved install path. On Unix systems it also sets executable permissions (`chmod 755`).

## Installation

Requires [Rust](https://rustup.rs/) to be installed.

```bash
git clone <repo-url>
cd bti
cargo run --release
```

## Usage

```
bti [OPTIONS]
```

### Options

| Flag                          | Short | Description                                                        |
|-------------------------------|-------|--------------------------------------------------------------------|
| `--build-path <path>`         | `--bp`| Path to the project directory to build (default: current directory)|
| `--build-args "<args>"`       | `--ba`| Custom arguments to pass to the detected build tool                |
| `--compiled-path <path>`      | `--cp`| Directory where the binary will be installed                       |
| `--list-binaries`             | `--lb`| List all binaries currently in the install directory               |
| `--version`                   | `--v` | Print the version                                                  |
| `--help`                      | `-h`  | Print the command list                                             |

### Examples

Build and install the project in the current directory:

```bash
bti
```

Build a project located elsewhere:

```bash
bti --build-path ~/projects/myapp
```

Pass custom build arguments:

```bash
bti --build-args "build --release --features some-feature"
```

Install the binary to a custom directory:

```bash
bti --compiled-path ~/bin
```

## Project structure

```
src/
  main.rs           Entry point; calls cli::lib::run()
  cli/
    mod.rs          Module declarations
    config.rs       Argument parsing and ApplicationConfig construction
    build.rs        Build system detection, BuildConfig, build execution, binary install
    commands.rs     CLI flag definitions and display
    lib.rs          Orchestrates config -> build -> install pipeline; contains tests
```

## License

MIT - see [LICENSE](LICENSE) for details.
