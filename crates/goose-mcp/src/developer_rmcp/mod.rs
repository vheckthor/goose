mod lang;
mod shell;

use anyhow::Result;
use base64::Engine;
use etcetera::{choose_app_strategy, AppStrategy};
use indoc::formatdoc;
use std::{
    collections::HashMap,
    io::Cursor,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::process::Command;

use include_dir::{include_dir, Dir};
use rmcp::{
    model::{Content, ProtocolVersion, ServerCapabilities, ServerInfo},
    schemars, tool, ServerHandler,
};

use self::shell::{
    expand_path, format_command_for_platform, get_shell_config, is_absolute_path,
    normalize_line_endings,
};
use std::process::Stdio;
use std::sync::Mutex;
use xcap::{Monitor, Window};

use ignore::gitignore::{Gitignore, GitignoreBuilder};

// Embeds the prompts directory to the build
static PROMPTS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/developer/prompts");

#[derive(Debug, Clone)]
pub struct Developer {
    file_history: Arc<Mutex<HashMap<PathBuf, Vec<String>>>>,
    ignore_patterns: Arc<Gitignore>,
    instructions: String,
}

impl Default for Developer {
    fn default() -> Self {
        Self::new()
    }
}

impl Developer {
    pub fn new() -> Self {
        // Get base instructions and working directory
        let cwd = std::env::current_dir().expect("should have a current working dir");
        let os = std::env::consts::OS;

        let base_instructions = match os {
            "windows" => formatdoc! {r#"
                The developer extension gives you the capabilities to edit code files and run shell commands,
                and can be used to solve a wide range of problems.

                You can use the shell tool to run Windows commands (PowerShell or CMD).
                When using paths, you can use either backslashes or forward slashes.

                Use the shell tool as needed to locate files or interact with the project.

                Your windows/screen tools can be used for visual debugging. You should not use these tools unless
                prompted to, but you can mention they are available if they are relevant.

                operating system: {os}
                current directory: {cwd}

                "#,
                os=os,
                cwd=cwd.to_string_lossy(),
            },
            _ => formatdoc! {r#"
                The developer extension gives you the capabilities to edit code files and run shell commands,
                and can be used to solve a wide range of problems.

                You can use the shell tool to run any command that would work on the relevant operating system.
                Use the shell tool as needed to locate files or interact with the project.

                Your windows/screen tools can be used for visual debugging. You should not use these tools unless
                prompted to, but you can mention they are available if they are relevant.

                operating system: {os}
                current directory: {cwd}

                "#,
                os=os,
                cwd=cwd.to_string_lossy(),
            },
        };

        // choose_app_strategy().config_dir()
        // - macOS/Linux: ~/.config/goose/
        // - Windows:     ~\AppData\Roaming\Block\goose\config\
        // keep previous behavior of expanding ~/.config in case this fails
        let global_hints_path = choose_app_strategy(crate::APP_STRATEGY.clone())
            .map(|strategy| strategy.in_config_dir(".goosehints"))
            .unwrap_or_else(|_| {
                PathBuf::from(shellexpand::tilde("~/.config/goose/.goosehints").to_string())
            });

        // Create the directory if it doesn't exist
        let _ = std::fs::create_dir_all(global_hints_path.parent().unwrap());

        // Check for local hints in current directory
        let local_hints_path = cwd.join(".goosehints");

        // Read global hints if they exist
        let mut hints = String::new();
        if global_hints_path.is_file() {
            if let Ok(global_hints) = std::fs::read_to_string(&global_hints_path) {
                hints.push_str("\n### Global Hints\nThe developer extension includes some global hints that apply to all projects & directories.\n");
                hints.push_str(&global_hints);
            }
        }

        // Read local hints if they exist
        if local_hints_path.is_file() {
            if let Ok(local_hints) = std::fs::read_to_string(&local_hints_path) {
                if !hints.is_empty() {
                    hints.push_str("\n\n");
                }
                hints.push_str("### Project Hints\nThe developer extension includes some hints for working on the project in this directory.\n");
                hints.push_str(&local_hints);
            }
        }

        // Return base instructions directly when no hints are found
        let instructions = if hints.is_empty() {
            base_instructions
        } else {
            format!("{base_instructions}\n{hints}")
        };

        let mut builder = GitignoreBuilder::new(cwd.clone());
        let mut has_ignore_file = false;
        // Initialize ignore patterns
        // - macOS/Linux: ~/.config/goose/
        // - Windows:     ~\AppData\Roaming\Block\goose\config\
        let global_ignore_path = choose_app_strategy(crate::APP_STRATEGY.clone())
            .map(|strategy| strategy.in_config_dir(".gooseignore"))
            .unwrap_or_else(|_| {
                PathBuf::from(shellexpand::tilde("~/.config/goose/.gooseignore").to_string())
            });

        // Create the directory if it doesn't exist
        let _ = std::fs::create_dir_all(global_ignore_path.parent().unwrap());

        // Read global ignores if they exist
        if global_ignore_path.is_file() {
            let _ = builder.add(global_ignore_path);
            has_ignore_file = true;
        }

        // Check for local ignores in current directory
        let local_ignore_path = cwd.join(".gooseignore");

        // Read local ignores if they exist
        if local_ignore_path.is_file() {
            let _ = builder.add(local_ignore_path);
            has_ignore_file = true;
        }

        // Only use default patterns if no .gooseignore files were found
        // If the file is empty, we will not ignore any file
        if !has_ignore_file {
            // Add some sensible defaults
            let _ = builder.add_line(None, "**/.env");
            let _ = builder.add_line(None, "**/.env.*");
            let _ = builder.add_line(None, "**/secrets.*");
        }

        let ignore_patterns = builder.build().expect("Failed to build ignore patterns");

        Self {
            instructions,
            file_history: Arc::new(Mutex::new(HashMap::new())),
            ignore_patterns: Arc::new(ignore_patterns),
        }
    }

    // Helper method to check if a path should be ignored
    fn is_ignored(&self, path: &Path) -> bool {
        self.ignore_patterns.matched(path, false).is_ignore()
    }

    // Helper method to resolve a path relative to cwd with platform-specific handling
    fn resolve_path(&self, path_str: &str) -> Result<PathBuf, String> {
        let cwd = std::env::current_dir().expect("should have a current working dir");
        let expanded = expand_path(path_str);
        let path = Path::new(&expanded);

        let suggestion = cwd.join(path);

        match is_absolute_path(&expanded) {
            true => Ok(path.to_path_buf()),
            false => Err(format!(
                "The path {} is not an absolute path, did you possibly mean {}?",
                path_str,
                suggestion.to_string_lossy(),
            )),
        }
    }

    fn save_file_history(&self, path: &PathBuf) -> Result<(), String> {
        let mut history = self.file_history.lock().unwrap();
        let content = if path.exists() {
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?
        } else {
            String::new()
        };
        history.entry(path.clone()).or_default().push(content);
        Ok(())
    }

    // Helper function to handle Mac screenshot filenames that contain U+202F (narrow no-break space)
    fn normalize_mac_screenshot_path(&self, path: &Path) -> PathBuf {
        // Only process if the path has a filename
        if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
            // Check if this matches Mac screenshot pattern:
            // "Screenshot YYYY-MM-DD at H.MM.SS AM/PM.png"
            if let Some(captures) = regex::Regex::new(r"^Screenshot \d{4}-\d{2}-\d{2} at \d{1,2}\.\d{2}\.\d{2} (AM|PM)(?: \(\d+\))?\.png$")
                .ok()
                .and_then(|re| re.captures(filename))
            {

                // Get the AM/PM part
                let meridian = captures.get(1).unwrap().as_str();

                // Find the last space before AM/PM and replace it with U+202F
                let space_pos = filename.rfind(meridian)
                    .map(|pos| filename[..pos].trim_end().len())
                    .unwrap_or(0);

                if space_pos > 0 {
                    let parent = path.parent().unwrap_or(Path::new(""));
                    let new_filename = format!(
                        "{}{}{}",
                        &filename[..space_pos],
                        '\u{202F}',
                        &filename[space_pos+1..]
                    );
                    let new_path = parent.join(new_filename);

                    return new_path;
                }
            }
        }
        path.to_path_buf()
    }
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PathParam {
    #[schemars(
        description = "Absolute path to file or directory, e.g. `/repo/file.py` or `/repo`."
    )]
    pub path: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TextEditorParam {
    #[schemars(
        description = "Absolute path to file or directory, e.g. `/repo/file.py` or `/repo`."
    )]
    pub path: String,

    #[schemars(description = "Allowed options are: `view`, `write`, `str_replace`, undo_edit`.")]
    pub command: String,

    #[serde(default)]
    pub file_text: Option<String>,

    #[serde(default)]
    pub old_str: Option<String>,

    #[serde(default)]
    pub new_str: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ShellParam {
    #[schemars(description = "The bash command string")]
    pub command: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ScreenCaptureParam {
    #[schemars(description = "The display number to capture (0 is main display)")]
    #[serde(default)]
    pub display: Option<usize>,

    #[schemars(
        description = "Optional: the exact title of the window to capture. use the list_windows tool to find the available windows."
    )]
    #[serde(default)]
    pub window_title: Option<String>,
}

#[tool(tool_box)]
impl Developer {
    #[tool(
        description = "Execute a command in the shell. This will return the output and error concatenated into a single string, as you would see from running on the command line."
    )]
    async fn shell(&self, #[tool(aggr)] param: ShellParam) -> Result<String, String> {
        let command = &param.command;

        // Check if command might access ignored files and return early if it does
        let cmd_parts: Vec<&str> = command.split_whitespace().collect();
        for arg in &cmd_parts[1..] {
            // Skip command flags
            if arg.starts_with('-') {
                continue;
            }
            // Skip invalid paths
            let path = Path::new(arg);
            if !path.exists() {
                continue;
            }

            if self.is_ignored(path) {
                return Err(format!(
                    "The command attempts to access '{}' which is restricted by .gooseignore",
                    arg
                ));
            }
        }

        // Get platform-specific shell configuration
        let shell_config = get_shell_config();
        let cmd_with_redirect = format_command_for_platform(command);

        // Execute the command using platform-specific shell
        let child = Command::new(&shell_config.executable)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .kill_on_drop(true)
            .arg(&shell_config.arg)
            .arg(cmd_with_redirect)
            .spawn()
            .map_err(|e| e.to_string())?;

        // Wait for the command to complete and get output
        let output = child.wait_with_output().await.map_err(|e| e.to_string())?;

        let output_str = String::from_utf8_lossy(&output.stdout);

        // Check the character count of the output
        const MAX_CHAR_COUNT: usize = 400_000; // 409600 chars = 400KB
        let char_count = output_str.chars().count();
        if char_count > MAX_CHAR_COUNT {
            return Err(format!(
                    "Shell output from command '{}' has too many characters ({}). Maximum character count is {}.",
                    command,
                    char_count,
                    MAX_CHAR_COUNT
                ));
        }

        Ok(output_str.to_string())
    }

    #[tool(
        description = "Perform text editing operations on files.\n\nThe `command` parameter specifies the operation to perform. Allowed options are:\n- `view`: View the content of a file.\n- `write`: Create or overwrite a file with the given content\n- `str_replace`: Replace a string in a file with a new string.\n- `undo_edit`: Undo the last edit made to a file.\n\nTo use the write command, you must specify `file_text` which will become the new content of the file. Be careful with\nexisting files! This is a full overwrite, so you must include everything - not just sections you are modifying.\n\nTo use the str_replace command, you must specify both `old_str` and `new_str` - the `old_str` needs to exactly match one\nunique section of the original file, including any whitespace. Make sure to include enough context that the match is not\nambiguous. The entire original string will be replaced with `new_str`."
    )]
    async fn text_editor(&self, #[tool(aggr)] param: TextEditorParam) -> Result<String, String> {
        let path = self.resolve_path(&param.path)?;

        // Check if file is ignored before proceeding with any text editor operation
        // except for view which is allowed on any file
        if param.command != "view" && self.is_ignored(&path) {
            return Err(format!(
                "Access to '{}' is restricted by .gooseignore",
                path.display()
            ));
        }

        match param.command.as_str() {
            "view" => {
                if path.is_file() {
                    // Check file size first (400KB limit)
                    const MAX_FILE_SIZE: u64 = 400 * 1024; // 400KB in bytes
                    const MAX_CHAR_COUNT: usize = 400_000; // 409600 chars = 400KB

                    let file_size = std::fs::metadata(&path)
                        .map_err(|e| format!("Failed to get file metadata: {}", e))?
                        .len();

                    if file_size > MAX_FILE_SIZE {
                        return Err(format!(
                            "File '{}' is too large ({:.2}KB). Maximum size is 400KB to prevent memory issues.",
                            path.display(),
                            file_size as f64 / 1024.0
                        ));
                    }

                    let content = std::fs::read_to_string(&path)
                        .map_err(|e| format!("Failed to read file: {}", e))?;

                    let char_count = content.chars().count();
                    if char_count > MAX_CHAR_COUNT {
                        return Err(format!(
                            "File '{}' has too many characters ({}). Maximum character count is {}.",
                            path.display(),
                            char_count,
                            MAX_CHAR_COUNT
                        ));
                    }

                    let language = lang::get_language_identifier(&path);
                    let formatted = formatdoc! {"
                        ### {path}
                        ```{language}
                        {content}
                        ```
                        ",
                        path=path.display(),
                        language=language,
                        content=content,
                    };

                    Ok(formatted)
                } else {
                    Err(format!(
                        "The path '{}' does not exist or is not a file.",
                        path.display()
                    ))
                }
            }
            "write" => {
                let file_text = param.file_text.ok_or_else(|| {
                    "Missing 'file_text' parameter for 'write' command".to_string()
                })?;

                // Normalize line endings based on platform
                let normalized_text = normalize_line_endings(&file_text);

                // Write to the file
                std::fs::write(&path, normalized_text)
                    .map_err(|e| format!("Failed to write file: {}", e))?;

                // Try to detect the language from the file extension
                let language = lang::get_language_identifier(&path);

                Ok(formatdoc! {r#"
                    Successfully wrote to {path}

                    ### {path}
                    ```{language}
                    {content}
                    ```
                    "#,
                    path=path.display(),
                    language=language,
                    content=file_text,
                })
            }
            "str_replace" => {
                let old_str = param.old_str.ok_or_else(|| {
                    "Missing 'old_str' parameter for 'str_replace' command".to_string()
                })?;

                let new_str = param.new_str.ok_or_else(|| {
                    "Missing 'new_str' parameter for 'str_replace' command".to_string()
                })?;

                // Check if file exists and is active
                if !path.exists() {
                    return Err(format!(
                        "File '{}' does not exist, you can write a new file with the `write` command",
                        path.display()
                    ));
                }

                // Read content
                let content = std::fs::read_to_string(&path)
                    .map_err(|e| format!("Failed to read file: {}", e))?;

                // Ensure 'old_str' appears exactly once
                if content.matches(&old_str).count() > 1 {
                    return Err(
                        "'old_str' must appear exactly once in the file, but it appears multiple times"
                            .into(),
                    );
                }
                if content.matches(&old_str).count() == 0 {
                    return Err(
                        "'old_str' must appear exactly once in the file, but it does not appear in the file. Make sure the string exactly matches existing file content, including whitespace!".into(),
                    );
                }

                // Save history for undo
                self.save_file_history(&path)?;

                // Replace and write back with platform-specific line endings
                let new_content = content.replace(&old_str, &new_str);
                let normalized_content = normalize_line_endings(&new_content);
                std::fs::write(&path, &normalized_content)
                    .map_err(|e| format!("Failed to write file: {}", e))?;

                // Try to detect the language from the file extension
                let language = lang::get_language_identifier(&path);

                // Show a snippet of the changed content with context
                const SNIPPET_LINES: usize = 4;

                // Count newlines before the replacement to find the line number
                let replacement_line = content
                    .split(&old_str)
                    .next()
                    .expect("should split on already matched content")
                    .matches('\n')
                    .count();

                // Calculate start and end lines for the snippet
                let start_line = replacement_line.saturating_sub(SNIPPET_LINES);
                let end_line = replacement_line + SNIPPET_LINES + new_str.matches('\n').count();

                // Get the relevant lines for our snippet
                let lines: Vec<&str> = new_content.lines().collect();
                let snippet = lines
                    .iter()
                    .skip(start_line)
                    .take(end_line - start_line + 1)
                    .cloned()
                    .collect::<Vec<&str>>()
                    .join("\n");

                let output = formatdoc! {r#"
                    ```{language}
                    {snippet}
                    ```
                    "#,
                    language=language,
                    snippet=snippet
                };

                Ok(formatdoc! {r#"
                    The file {} has been edited, and the section now reads:
                    {}
                    Review the changes above for errors. Undo and edit the file again if necessary!
                    "#,
                    path.display(),
                    output
                })
            }
            "undo_edit" => {
                let mut history = self.file_history.lock().unwrap();
                if let Some(contents) = history.get_mut(&path) {
                    if let Some(previous_content) = contents.pop() {
                        // Write previous content back to file
                        std::fs::write(&path, previous_content)
                            .map_err(|e| format!("Failed to write file: {}", e))?;
                        Ok("Undid the last edit".to_string())
                    } else {
                        Err("No edit history available to undo".into())
                    }
                } else {
                    Err("No edit history available to undo".into())
                }
            }
            _ => Err(format!(
                "Unknown command '{}'. Valid commands are: view, write, str_replace, undo_edit",
                param.command
            )),
        }
    }

    #[tool(description = "List all available window titles that can be used with screen_capture.")]
    async fn list_windows(&self) -> Result<String, String> {
        let windows = Window::all().map_err(|_| "Failed to list windows".to_string())?;

        let window_titles: Vec<String> =
            windows.into_iter().map(|w| w.title().to_string()).collect();

        Ok(format!("Available windows:\n{}", window_titles.join("\n")))
    }

    #[tool(description = "Capture a screenshot of a specified display or window.")]
    async fn screen_capture(
        &self,
        #[tool(aggr)] param: ScreenCaptureParam,
    ) -> Result<Content, String> {
        let mut image = if let Some(window_title) = param.window_title {
            // Try to find and capture the specified window
            let windows = Window::all().map_err(|_| "Failed to list windows".to_string())?;

            let window = windows
                .into_iter()
                .find(|w| w.title() == window_title)
                .ok_or_else(|| format!("No window found with title '{}'", window_title))?;

            window
                .capture_image()
                .map_err(|e| format!("Failed to capture window '{}': {}", window_title, e))?
        } else {
            // Default to display capture if no window title is specified
            let display = param.display.unwrap_or(0);

            let monitors = Monitor::all().map_err(|_| "Failed to access monitors".to_string())?;
            let monitor = monitors.get(display).ok_or_else(|| {
                format!(
                    "{} was not an available monitor, {} found.",
                    display,
                    monitors.len()
                )
            })?;

            monitor
                .capture_image()
                .map_err(|e| format!("Failed to capture display {}: {}", display, e))?
        };

        // Resize the image to a reasonable width while maintaining aspect ratio
        let max_width = 768;
        if image.width() > max_width {
            let scale = max_width as f32 / image.width() as f32;
            let new_height = (image.height() as f32 * scale) as u32;
            image = xcap::image::imageops::resize(
                &image,
                max_width,
                new_height,
                xcap::image::imageops::FilterType::Lanczos3,
            )
        };

        let mut bytes: Vec<u8> = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut bytes), xcap::image::ImageFormat::Png)
            .map_err(|e| format!("Failed to write image buffer {}", e))?;

        // Convert to base64
        let data = base64::prelude::BASE64_STANDARD.encode(bytes);

        Ok(Content::image(data, "image/png"))
    }

    #[tool(description = "Process an image file from disk.")]
    async fn image_processor(&self, #[tool(aggr)] param: PathParam) -> Result<Content, String> {
        let path = {
            let p = self.resolve_path(&param.path)?;
            if cfg!(target_os = "macos") {
                self.normalize_mac_screenshot_path(&p)
            } else {
                p
            }
        };

        // Check if file is ignored before proceeding
        if self.is_ignored(&path) {
            return Err(format!(
                "Access to '{}' is restricted by .gooseignore",
                path.display()
            ));
        }

        // Check if file exists
        if !path.exists() {
            return Err(format!("File '{}' does not exist", path.display()));
        }

        // Check file size (10MB limit for image files)
        const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB in bytes
        let file_size = std::fs::metadata(&path)
            .map_err(|e| format!("Failed to get file metadata: {}", e))?
            .len();

        if file_size > MAX_FILE_SIZE {
            return Err(format!(
                "File '{}' is too large ({:.2}MB). Maximum size is 10MB.",
                path.display(),
                file_size as f64 / (1024.0 * 1024.0)
            ));
        }

        // Open and decode the image
        let image =
            xcap::image::open(&path).map_err(|e| format!("Failed to open image file: {}", e))?;

        // Resize if necessary (same logic as screen_capture)
        let mut processed_image = image;
        let max_width = 768;
        if processed_image.width() > max_width {
            let scale = max_width as f32 / processed_image.width() as f32;
            let new_height = (processed_image.height() as f32 * scale) as u32;
            processed_image = xcap::image::DynamicImage::ImageRgba8(xcap::image::imageops::resize(
                &processed_image,
                max_width,
                new_height,
                xcap::image::imageops::FilterType::Lanczos3,
            ));
        }

        // Convert to PNG and encode as base64
        let mut bytes: Vec<u8> = Vec::new();
        processed_image
            .write_to(&mut Cursor::new(&mut bytes), xcap::image::ImageFormat::Png)
            .map_err(|e| format!("Failed to write image buffer: {}", e))?;

        let data = base64::prelude::BASE64_STANDARD.encode(bytes);

        Ok(Content::image(data, "image/png"))
    }
}

#[tool(tool_box)]
impl ServerHandler for Developer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            instructions: Some(self.instructions.clone()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
