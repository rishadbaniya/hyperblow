use crate::engine::{Engine, TorrentSource};
use hyperblow::parser::magnet_uri_parser::MagnetURIMeta;
use std::{
    env, fs,
    path::PathBuf,
    sync::{mpsc::Sender, Arc},
    thread,
};
use thiserror::Error;
use tokio::runtime::Builder;
use tracing::{debug, error, info};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CommandAction {
    File(PathBuf),
    Magnet(String),
    Quit,
}

impl CommandAction {
    fn kind(&self) -> &'static str {
        match self {
            Self::File(_) => "file",
            Self::Magnet(_) => "magnet",
            Self::Quit => "quit",
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum CommandInputError {
    #[error("type :file <path>, :magnet <uri>, :q, or :quit")]
    Empty,

    #[error("unknown command :{0}")]
    UnknownCommand(String),

    #[error("missing path: use :file <path-to-torrent>")]
    MissingFilePath,

    #[error("file does not exist: {0}")]
    FileNotFound(String),

    #[error("path is not a file: {0}")]
    PathIsNotFile(String),

    #[error("missing magnet URI: use :magnet <uri>")]
    MissingMagnetUri,

    #[error("invalid magnet URI")]
    InvalidMagnetUri,
}

#[derive(Debug)]
pub(crate) enum CommandExecutionResult {
    Loaded { message: String },
    Failed { input: String, message: String },
}

pub(crate) struct CommandParser;

impl CommandParser {
    pub(crate) fn parse(input: &str) -> Result<CommandAction, CommandInputError> {
        let input = input.trim();
        if input.is_empty() {
            return Err(CommandInputError::Empty);
        }

        let (command, argument) = Self::split(input);
        match command.to_ascii_lowercase().as_str() {
            "file" => Self::parse_file(argument),
            "magnet" => Self::parse_magnet(argument),
            "q" | "quit" => Ok(CommandAction::Quit),
            unknown => Err(CommandInputError::UnknownCommand(unknown.to_string())),
        }
    }

    pub(crate) fn split(input: &str) -> (&str, &str) {
        if let Some(index) = input.find(char::is_whitespace) {
            (&input[..index], input[index..].trim_start())
        } else {
            (input, "")
        }
    }

    fn parse_file(argument: &str) -> Result<CommandAction, CommandInputError> {
        let argument = argument.trim();
        if argument.is_empty() {
            return Err(CommandInputError::MissingFilePath);
        }

        let path = PathExpander::expand(argument);
        if !path.exists() {
            return Err(CommandInputError::FileNotFound(path.display().to_string()));
        }
        if !path.is_file() {
            return Err(CommandInputError::PathIsNotFile(path.display().to_string()));
        }
        Ok(CommandAction::File(path))
    }

    fn parse_magnet(argument: &str) -> Result<CommandAction, CommandInputError> {
        let uri = argument.trim();
        if uri.is_empty() {
            return Err(CommandInputError::MissingMagnetUri);
        }
        if !MagnetURIMeta::checkIfMagnetURIIsValid(uri) {
            return Err(CommandInputError::InvalidMagnetUri);
        }
        Ok(CommandAction::Magnet(uri.to_string()))
    }
}

pub(crate) struct CommandSuggester;

impl CommandSuggester {
    pub(crate) fn suggestions(input: &str, limit: usize) -> Vec<String> {
        let input = input.trim_start();
        if input.is_empty() {
            return vec!["file ".to_string(), "magnet ".to_string(), "q".to_string(), "quit".to_string()];
        }

        if !input.contains(char::is_whitespace) {
            return ["file ", "magnet ", "q", "quit"]
                .into_iter()
                .filter(|command| command.trim_end().starts_with(input))
                .map(ToOwned::to_owned)
                .collect();
        }

        let (command, argument) = CommandParser::split(input);
        match command.to_ascii_lowercase().as_str() {
            "file" => FilePathSuggester::suggestions(argument, limit),
            "magnet" => vec!["magnet magnet:?xt=urn:btih:".to_string()],
            _ => Vec::new(),
        }
    }

    pub(crate) fn display(input: &str, suggestion: &str) -> String {
        let (command, _) = CommandParser::split(input.trim_start());
        match command.to_ascii_lowercase().as_str() {
            "file" => suggestion
                .strip_prefix("file ")
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| suggestion.to_string()),
            "magnet" => suggestion
                .strip_prefix("magnet ")
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| suggestion.to_string()),
            _ => format!(":{suggestion}"),
        }
    }
}

pub(crate) struct CommandExecutor;

impl CommandExecutor {
    pub(crate) fn pending_message(action: &CommandAction) -> String {
        match action {
            CommandAction::File(path) => format!("Opening {}...", path.display()),
            CommandAction::Magnet(_) => "Opening magnet URI...".to_string(),
            CommandAction::Quit => "Quitting...".to_string(),
        }
    }

    pub(crate) fn spawn(action: CommandAction, input: String, engine: Arc<Engine>, command_result_sender: Sender<CommandExecutionResult>) {
        thread::spawn(move || {
            let action_kind = action.kind();
            debug!(source = action_kind, "command executor started");
            let source = match action {
                CommandAction::File(path) => TorrentSource::FilePath(path.to_string_lossy().into_owned()),
                CommandAction::Magnet(uri) => TorrentSource::MagnetURI(uri),
                CommandAction::Quit => {
                    let _ = command_result_sender.send(CommandExecutionResult::Loaded {
                        message: "Quit command handled".to_string(),
                    });
                    return;
                }
            };

            let result = Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())
                .and_then(|runtime| runtime.block_on(engine.spawn(source)).map_err(|error| error.to_string()));

            let execution_result = match result {
                Ok(handle) => {
                    let torrent_name = handle.name();
                    info!(source = action_kind, torrent = %torrent_name, "command loaded torrent");
                    CommandExecutionResult::Loaded {
                        message: format!("Loaded {torrent_name}"),
                    }
                }
                Err(message) => {
                    error!(source = action_kind, error = %message, "command failed to load torrent");
                    CommandExecutionResult::Failed { input, message }
                }
            };
            let _ = command_result_sender.send(execution_result);
        });
    }
}

struct FilePathSuggester;

impl FilePathSuggester {
    fn suggestions(argument: &str, limit: usize) -> Vec<String> {
        let Some(query) = FileCompletionQuery::new(argument) else {
            return Vec::new();
        };

        let Ok(entries) = fs::read_dir(&query.directory) else {
            return Vec::new();
        };

        let mut completions = entries
            .flatten()
            .filter_map(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                if !name.starts_with(&query.prefix) {
                    return None;
                }

                let file_type = entry.file_type().ok()?;
                let is_dir = file_type.is_dir();
                let is_torrent = name.ends_with(".torrent");
                Some(FileCompletion {
                    command: format!("file {}{}{}", query.display_parent, name, if is_dir { "/" } else { "" }),
                    is_dir,
                    is_torrent,
                    sort_name: name.to_ascii_lowercase(),
                })
            })
            .collect::<Vec<_>>();

        completions.sort_by(|left, right| {
            right
                .is_dir
                .cmp(&left.is_dir)
                .then_with(|| right.is_torrent.cmp(&left.is_torrent))
                .then_with(|| left.sort_name.cmp(&right.sort_name))
        });

        completions.into_iter().take(limit).map(|completion| completion.command).collect()
    }
}

#[derive(Debug)]
struct FileCompletionQuery {
    directory: PathBuf,
    display_parent: String,
    prefix: String,
}

impl FileCompletionQuery {
    fn new(argument: &str) -> Option<Self> {
        let argument = argument.trim_start();
        if argument.is_empty() {
            return Some(Self {
                directory: env::current_dir().ok()?,
                display_parent: String::new(),
                prefix: String::new(),
            });
        }

        let expanded = PathExpander::expand(argument);
        if argument.ends_with('/') || expanded.is_dir() {
            let display_parent = if argument.ends_with('/') {
                argument.to_string()
            } else {
                format!("{argument}/")
            };
            return Some(Self {
                directory: expanded,
                display_parent,
                prefix: String::new(),
            });
        }

        let display_parent = argument.rfind('/').map(|index| argument[..=index].to_string()).unwrap_or_default();
        let prefix = argument
            .rfind('/')
            .map(|index| argument[index + 1..].to_string())
            .unwrap_or_else(|| argument.to_string());
        let directory_argument = if display_parent.is_empty() {
            ".".to_string()
        } else {
            display_parent.clone()
        };

        Some(Self {
            directory: PathExpander::expand(&directory_argument),
            display_parent,
            prefix,
        })
    }
}

#[derive(Debug)]
struct FileCompletion {
    command: String,
    is_dir: bool,
    is_torrent: bool,
    sort_name: String,
}

struct PathExpander;

impl PathExpander {
    fn expand(path: &str) -> PathBuf {
        let expanded = Self::expand_env_vars(path);
        if expanded == "~" {
            return env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(expanded));
        }

        if let Some(rest) = expanded.strip_prefix("~/") {
            return env::var_os("HOME")
                .map(PathBuf::from)
                .map(|home| home.join(rest))
                .unwrap_or_else(|| PathBuf::from(expanded));
        }

        PathBuf::from(expanded)
    }

    fn expand_env_vars(path: &str) -> String {
        let mut expanded = String::with_capacity(path.len());
        let mut chars = path.chars().peekable();

        while let Some(character) = chars.next() {
            if character != '$' {
                expanded.push(character);
                continue;
            }

            if chars.peek() == Some(&'{') {
                chars.next();
                let mut name = String::new();
                for next in chars.by_ref() {
                    if next == '}' {
                        break;
                    }
                    name.push(next);
                }

                if name.is_empty() {
                    expanded.push_str("${}");
                } else if let Some(value) = env::var_os(&name) {
                    expanded.push_str(&value.to_string_lossy());
                } else {
                    expanded.push_str("${");
                    expanded.push_str(&name);
                    expanded.push('}');
                }
                continue;
            }

            let mut name = String::new();
            while let Some(next) = chars.peek().copied() {
                if next == '_' || next.is_ascii_alphanumeric() {
                    chars.next();
                    name.push(next);
                } else {
                    break;
                }
            }

            if name.is_empty() {
                expanded.push('$');
            } else if let Some(value) = env::var_os(&name) {
                expanded.push_str(&value.to_string_lossy());
            } else {
                expanded.push('$');
                expanded.push_str(&name);
            }
        }

        expanded
    }
}

#[cfg(test)]
mod tests {
    use super::{CommandAction, CommandInputError, CommandParser, CommandSuggester, PathExpander};
    use std::{
        env, fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    const VALID_MAGNET: &str = "magnet:?xt=urn:btih:08ada5a7a6183aae1e09d831df6748d566095a10&dn=Example";

    #[test]
    fn parses_file_command_with_existing_path() {
        let temp_dir = test_temp_dir();
        let torrent_path = temp_dir.join("sample torrent.torrent");
        fs::write(&torrent_path, b"not a real torrent").expect("test torrent path should be writable");

        assert_eq!(
            CommandParser::parse(&format!("file {}", torrent_path.display())),
            Ok(CommandAction::File(torrent_path))
        );

        fs::remove_dir_all(temp_dir).expect("temp dir should be removed");
    }

    #[test]
    fn parses_magnet_command() {
        assert_eq!(
            CommandParser::parse(&format!("magnet {VALID_MAGNET}")),
            Ok(CommandAction::Magnet(VALID_MAGNET.to_string()))
        );
    }

    #[test]
    fn rejects_invalid_command_input() {
        assert_eq!(CommandParser::parse(""), Err(CommandInputError::Empty));
        assert_eq!(CommandParser::parse("magnet"), Err(CommandInputError::MissingMagnetUri));
        assert_eq!(
            CommandParser::parse("wat"),
            Err(CommandInputError::UnknownCommand("wat".to_string()))
        );
    }

    #[test]
    fn parses_quit_command() {
        assert_eq!(CommandParser::parse("q"), Ok(CommandAction::Quit));
        assert_eq!(CommandParser::parse("quit"), Ok(CommandAction::Quit));
    }

    #[test]
    fn suggests_quit_commands() {
        assert!(CommandSuggester::suggestions("", 8).contains(&"quit".to_string()));
        assert_eq!(CommandSuggester::suggestions("qu", 8), vec!["quit".to_string()]);
    }

    #[test]
    fn expands_home_environment_variable_in_file_paths() {
        let Some(home) = env::var_os("HOME") else {
            return;
        };

        assert_eq!(PathExpander::expand("$HOME"), PathBuf::from(home.clone()));
        assert_eq!(PathExpander::expand("${HOME}"), PathBuf::from(home));
    }

    #[test]
    fn file_hints_display_paths_without_repeating_command_prefix() {
        assert_eq!(
            CommandSuggester::display("file $HOME", "file $HOME/Downloads/"),
            "$HOME/Downloads/".to_string()
        );
        assert_eq!(CommandSuggester::display("", "file "), ":file ".to_string());
    }

    #[test]
    fn suggests_file_paths_and_prioritizes_directories() {
        let temp_dir = test_temp_dir();
        fs::create_dir(temp_dir.join("alpha_dir")).expect("test directory should be created");
        fs::write(temp_dir.join("alpha.torrent"), b"torrent").expect("test torrent should be created");
        fs::write(temp_dir.join("alpha.txt"), b"text").expect("test text file should be created");

        let suggestions = CommandSuggester::suggestions(&format!("file {}", temp_dir.join("a").display()), 8);
        let display_parent = format!("{}/", temp_dir.display());

        assert_eq!(suggestions.first(), Some(&format!("file {display_parent}alpha_dir/")));
        assert!(suggestions.contains(&format!("file {display_parent}alpha.torrent")));

        fs::remove_dir_all(temp_dir).expect("temp dir should be removed");
    }

    fn test_temp_dir() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        let path = PathBuf::from("target")
            .join("ui-command-tests")
            .join(format!("t{}{}", std::process::id(), nonce % 1_000_000_000));
        fs::create_dir_all(&path).expect("temp dir should be created");
        path
    }
}
