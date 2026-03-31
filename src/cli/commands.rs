use std::fmt;

pub enum CommandList {
    BuildPath,
    BuildArgs,
    CompiledPath,
    ListBinaries,
    Version,
    Help,
}

impl fmt::Display for CommandList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            CommandList::BuildPath => "--build-path (or --bp)",
            CommandList::BuildArgs => "--build-args (or --ba)",
            CommandList::CompiledPath => "--compiled-path (or --cp)",
            CommandList::ListBinaries => "--list-binaries (or --lb)",
            CommandList::Version => "--version (or --v)",
            CommandList::Help => "--help (or -h)",
        };
        write!(f, "{}", name)
    }
}

impl CommandList {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "--build-path" | "--bp" => Some(CommandList::BuildPath),
            "--build-args" | "--ba" => Some(CommandList::BuildArgs),
            "--compiled-path" | "--cp" => Some(CommandList::CompiledPath),
            "--list-binaries" | "--lb" => Some(CommandList::ListBinaries),
            "--version" | "--v" => Some(CommandList::Version),
            "--help" | "-h" => Some(CommandList::Help),
            _ => None,
        }
    }

    pub fn to_string() -> String {
        [
            Self::BuildPath,
            Self::BuildArgs,
            Self::CompiledPath,
            Self::ListBinaries,
            Self::Version,
            Self::Help,
        ]
        .iter()
        .map(|c| format!("{}", c))
        .collect::<Vec<_>>()
        .join("\n")
    }
}
