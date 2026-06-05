use std::env;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use eframe::egui::{self, RichText};
use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

const USER_BASE_KEY: &str = r"Software\Classes\Applications\Explorer.exe\Drives";
const MACHINE_BASE_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\DriveIcons";

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    let result = if args.is_empty() {
        launch_gui().map_err(|error| AppError::Gui(error.to_string()))
    } else {
        run_cli(args)
    };

    match result {
        Ok(()) => {}
        Err(AppError::Help) => {
            print_usage();
        }
        Err(error) => {
            eprintln!("error: {error}");
            eprintln!();
            print_usage();
            std::process::exit(1);
        }
    }
}

fn run_cli(args: Vec<String>) -> Result<(), AppError> {
    let config = Config::parse(args)?;

    match config.action {
        Action::Set { icon_path } => {
            let prepared_icon = prepare_icon_path(&icon_path)?;
            set_drive_icon(config.drive, config.scope, &prepared_icon.path)?;

            println!(
                "Set {}: drive icon for {}.",
                config.drive,
                config.scope.description()
            );
            println!(
                "Registry key: {}",
                config.scope.default_icon_key(config.drive)
            );
            if let Some(source_png) = &prepared_icon.converted_from {
                println!(
                    "Converted PNG \"{}\" to \"{}\".",
                    source_png.display(),
                    prepared_icon.path.display()
                );
            }
            println!("Icon value: \"{}\"", prepared_icon.path.display());
            println!("Close and reopen File Explorer to apply the change.");

            if matches!(config.scope, Scope::Machine) {
                println!("Machine scope writes to HKLM and requires an elevated terminal.");
            }
        }
        Action::Remove => {
            remove_drive_icon(config.drive, config.scope)?;

            println!(
                "Removed {}: drive icon override for {}.",
                config.drive,
                config.scope.description()
            );
            println!("Close and reopen File Explorer to apply the change.");
        }
    }

    Ok(())
}

fn launch_gui() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([560.0, 420.0])
            .with_min_inner_size([500.0, 360.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Drive Icon Setter",
        options,
        Box::new(|creation_context| {
            configure_gui_style(&creation_context.egui_ctx);
            Ok(Box::new(DriveIconApp::default()))
        }),
    )
}

fn configure_gui_style(context: &egui::Context) {
    context.global_style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(10.0, 10.0);
        style.spacing.button_padding = egui::vec2(12.0, 7.0);
    });
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum Scope {
    User,
    Machine,
}

impl Scope {
    fn from_arg(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "user" | "current-user" | "hkcu" => Some(Self::User),
            "machine" | "all-users" | "hklm" => Some(Self::Machine),
            _ => None,
        }
    }

    fn root(self) -> RegKey {
        match self {
            Self::User => RegKey::predef(HKEY_CURRENT_USER),
            Self::Machine => RegKey::predef(HKEY_LOCAL_MACHINE),
        }
    }

    fn base_key(self) -> &'static str {
        match self {
            Self::User => USER_BASE_KEY,
            Self::Machine => MACHINE_BASE_KEY,
        }
    }

    fn root_name(self) -> &'static str {
        match self {
            Self::User => "HKCU",
            Self::Machine => "HKLM",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::User => "the current user",
            Self::Machine => "all users",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::User => "Current user",
            Self::Machine => "All users",
        }
    }

    fn drive_key(self, drive: char) -> String {
        format!(r"{}\{drive}", self.base_key())
    }

    fn default_icon_key(self, drive: char) -> String {
        format!(
            r"{}\{}\DefaultIcon",
            self.root_name(),
            self.drive_key(drive)
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum StatusKind {
    Success,
    Error,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct StatusMessage {
    kind: StatusKind,
    text: String,
}

#[derive(Debug)]
struct DriveIconApp {
    drive_input: String,
    icon_path: String,
    scope: Scope,
    status: Option<StatusMessage>,
}

impl Default for DriveIconApp {
    fn default() -> Self {
        Self {
            drive_input: String::new(),
            icon_path: String::new(),
            scope: Scope::User,
            status: None,
        }
    }
}

impl eframe::App for DriveIconApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.vertical_centered_justified(|ui| {
                ui.heading("Drive Icon Setter");
            });

            ui.add_space(6.0);
            ui.label("Pick a drive, choose a .ico or .png file, then apply the registry override.");
            ui.add_space(12.0);

            egui::Grid::new("drive_icon_form")
                .num_columns(2)
                .spacing([14.0, 12.0])
                .show(ui, |ui| {
                    ui.label("Drive letter");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.drive_input)
                            .hint_text("F or F:")
                            .desired_width(90.0),
                    );
                    ui.end_row();

                    ui.label("Image file");
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.icon_path)
                                .hint_text(r"C:\Icons\Drive.ico or Drive.png")
                                .desired_width(300.0),
                        );

                        if ui.button("Browse...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Icon or PNG files", &["ico", "png"])
                                .pick_file()
                            {
                                self.icon_path = path.display().to_string();
                                self.status = None;
                            }
                        }
                    });
                    ui.end_row();

                    ui.label("Scope");
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut self.scope, Scope::User, Scope::User.label());
                        ui.radio_value(&mut self.scope, Scope::Machine, Scope::Machine.label());
                    });
                    ui.end_row();
                });

            ui.add_space(6.0);
            self.scope_note(ui);
            ui.add_space(8.0);
            self.registry_target(ui);
            ui.add_space(14.0);

            ui.horizontal(|ui| {
                let has_drive = !self.drive_input.trim().is_empty();
                let can_apply = has_drive && !self.icon_path.trim().is_empty();

                if ui
                    .add_enabled(can_apply, egui::Button::new("Apply icon"))
                    .clicked()
                {
                    self.apply_icon();
                }

                if ui
                    .add_enabled(has_drive, egui::Button::new("Remove override"))
                    .clicked()
                {
                    self.remove_icon();
                }
            });

            if let Some(status) = &self.status {
                ui.add_space(14.0);
                let color = match status.kind {
                    StatusKind::Success => egui::Color32::from_rgb(24, 128, 72),
                    StatusKind::Error => egui::Color32::from_rgb(184, 48, 48),
                };
                ui.label(RichText::new(&status.text).color(color));
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.separator();
                ui.label(
                    RichText::new(
                        "Close and reopen File Explorer after applying or removing an icon.",
                    )
                    .small(),
                );
            });
        });
    }
}

impl DriveIconApp {
    fn scope_note(&self, ui: &mut egui::Ui) {
        match self.scope {
            Scope::User => {
                ui.label(
                    RichText::new("Current user writes HKCU and does not require elevation.")
                        .small(),
                );
            }
            Scope::Machine => {
                ui.label(
                    RichText::new("All users writes HKLM and requires running as administrator.")
                        .small()
                        .strong(),
                );
            }
        }
    }

    fn registry_target(&self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Registry target").small().strong());

        match parse_drive(self.drive_input.trim()) {
            Ok(drive) => {
                ui.monospace(self.scope.default_icon_key(drive));
            }
            Err(_) => {
                ui.monospace(format!(
                    r"{}\...\<drive>\DefaultIcon",
                    self.scope.root_name()
                ));
            }
        }
    }

    fn apply_icon(&mut self) {
        let result = (|| {
            let drive = parse_drive(self.drive_input.trim())?;
            let icon_path = PathBuf::from(strip_surrounding_quotes(self.icon_path.trim()));
            let prepared_icon = prepare_icon_path(&icon_path)?;

            set_drive_icon(drive, self.scope, &prepared_icon.path)?;

            let mut message = format!(
                "Set {drive}: to \"{}\" for {}.",
                prepared_icon.path.display(),
                self.scope.description()
            );

            if let Some(source_png) = &prepared_icon.converted_from {
                message.push_str(&format!(
                    " Converted \"{}\" to ICO first.",
                    source_png.display()
                ));
            }

            Ok(message)
        })();

        self.set_status(result);
    }

    fn remove_icon(&mut self) {
        let result = (|| {
            let drive = parse_drive(self.drive_input.trim())?;

            remove_drive_icon(drive, self.scope)?;

            Ok(format!(
                "Removed {drive}: icon override for {}.",
                self.scope.description()
            ))
        })();

        self.set_status(result);
    }

    fn set_status(&mut self, result: Result<String, AppError>) {
        self.status = Some(match result {
            Ok(text) => StatusMessage {
                kind: StatusKind::Success,
                text,
            },
            Err(error) => StatusMessage {
                kind: StatusKind::Error,
                text: error.to_string(),
            },
        });
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum Action {
    Set { icon_path: PathBuf },
    Remove,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct Config {
    drive: char,
    scope: Scope,
    action: Action,
}

impl Config {
    fn parse(args: Vec<String>) -> Result<Self, AppError> {
        if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
            return Err(AppError::Help);
        }

        let mut drive = None;
        let mut icon_path = None;
        let mut remove = false;
        let mut scope = Scope::User;
        let mut index = 0;

        while index < args.len() {
            match args[index].as_str() {
                "--scope" => {
                    index += 1;
                    let value = args.get(index).ok_or_else(|| {
                        AppError::Usage("--scope requires user or machine".into())
                    })?;
                    scope = Scope::from_arg(value).ok_or_else(|| {
                        AppError::Usage(format!(
                            "invalid scope '{value}', expected user or machine"
                        ))
                    })?;
                }
                "--remove" => {
                    remove = true;
                }
                value if value.starts_with("--") => {
                    return Err(AppError::Usage(format!("unknown option '{value}'")));
                }
                value if drive.is_none() => {
                    drive = Some(parse_drive(value)?);
                }
                value if icon_path.is_none() => {
                    icon_path = Some(PathBuf::from(strip_surrounding_quotes(value)));
                }
                value => {
                    return Err(AppError::Usage(format!("unexpected argument '{value}'")));
                }
            }

            index += 1;
        }

        let drive = drive.ok_or_else(|| AppError::Usage("missing drive letter".into()))?;

        let action = if remove {
            if icon_path.is_some() {
                return Err(AppError::Usage(
                    "--remove cannot be combined with an icon path".into(),
                ));
            }

            Action::Remove
        } else {
            let icon_path = icon_path
                .ok_or_else(|| AppError::Usage("missing .ico or .png file path".into()))?;
            Action::Set { icon_path }
        };

        Ok(Self {
            drive,
            scope,
            action,
        })
    }
}

fn parse_drive(value: &str) -> Result<char, AppError> {
    let trimmed = strip_surrounding_quotes(value).trim_end_matches(['\\', '/']);
    let drive = match trimmed.as_bytes() {
        [letter] if letter.is_ascii_alphabetic() => *letter as char,
        [letter, b':'] if letter.is_ascii_alphabetic() => *letter as char,
        _ => {
            return Err(AppError::Usage(format!(
                "invalid drive '{value}', expected a drive letter like F or F:"
            )));
        }
    };

    Ok(drive.to_ascii_uppercase())
}

fn strip_surrounding_quotes(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(value)
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct PreparedIcon {
    path: PathBuf,
    converted_from: Option<PathBuf>,
}

fn prepare_icon_path(path: &Path) -> Result<PreparedIcon, AppError> {
    let image_path = path.canonicalize().map_err(|error| {
        AppError::Usage(format!(
            "could not find image file '{}': {error}",
            path.display()
        ))
    })?;

    if has_extension(&image_path, "ico") {
        return Ok(PreparedIcon {
            path: image_path,
            converted_from: None,
        });
    }

    if has_extension(&image_path, "png") {
        let icon_path = convert_png_to_ico(&image_path)?;

        return Ok(PreparedIcon {
            path: icon_path,
            converted_from: Some(image_path),
        });
    }

    Err(AppError::Usage(
        "Choose a .ico file, or a .png file that can be converted to .ico.".into(),
    ))
}

fn has_extension(path: &Path, extension: &str) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case(extension))
}

fn convert_png_to_ico(png_path: &Path) -> Result<PathBuf, AppError> {
    let source_image = image::open(png_path)?;
    let icon_image = normalize_icon_image(source_image);

    let mut icon_path = png_path.to_path_buf();
    icon_path.set_extension("ico");

    let mut icon_file = File::create(&icon_path).map_err(|error| {
        AppError::Usage(format!(
            "could not create converted icon '{}': {error}",
            icon_path.display()
        ))
    })?;

    icon_image.write_to(&mut icon_file, ImageFormat::Ico)?;

    Ok(icon_path.canonicalize().unwrap_or(icon_path))
}

fn normalize_icon_image(source_image: DynamicImage) -> DynamicImage {
    let scaled_image = if source_image.width() > 256 || source_image.height() > 256 {
        source_image.thumbnail(256, 256)
    } else {
        source_image
    };
    let scaled_image = scaled_image.to_rgba8();

    if scaled_image.width() == scaled_image.height() {
        return DynamicImage::ImageRgba8(scaled_image);
    }

    let side = scaled_image.width().max(scaled_image.height());
    let mut canvas = RgbaImage::from_pixel(side, side, Rgba([0, 0, 0, 0]));
    let x = i64::from((side - scaled_image.width()) / 2);
    let y = i64::from((side - scaled_image.height()) / 2);

    image::imageops::overlay(&mut canvas, &scaled_image, x, y);

    DynamicImage::ImageRgba8(canvas)
}

fn set_drive_icon(drive: char, scope: Scope, icon_path: &Path) -> Result<(), AppError> {
    let default_icon_key = format!(r"{}\{drive}\DefaultIcon", scope.base_key());
    let quoted_icon_path = format!("\"{}\"", icon_path.display());
    let root = scope.root();
    let (key, _) = root.create_subkey(default_icon_key)?;

    key.set_value("", &quoted_icon_path)?;

    Ok(())
}

fn remove_drive_icon(drive: char, scope: Scope) -> Result<(), AppError> {
    let root = scope.root();
    let drive_key = scope.drive_key(drive);

    match root.delete_subkey_all(drive_key) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn print_usage() {
    println!(
        "Usage:
  drive-icon-setter <drive-letter> <icon.ico|image.png> [--scope user|machine]
  drive-icon-setter <drive-letter> --remove [--scope user|machine]

Examples:
  drive-icon-setter F C:\\Icons\\Backup.ico
  drive-icon-setter F C:\\Icons\\Backup.png
  drive-icon-setter F C:\\Windows\\Backup.ico --scope machine
  drive-icon-setter F --remove

Notes:
  png files are converted to sibling .ico files before the registry is updated.
  user scope writes HKCU and affects only the current account.
  machine scope writes HKLM, affects all users, and requires elevation."
    );
}

#[derive(Debug)]
enum AppError {
    Help,
    Usage(String),
    Io(io::Error),
    Image(image::ImageError),
    Gui(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Help => Ok(()),
            Self::Usage(message) => write!(formatter, "{message}"),
            Self::Io(error) => write!(formatter, "{error}"),
            Self::Image(error) => write!(formatter, "could not convert image: {error}"),
            Self::Gui(message) => write!(formatter, "{message}"),
        }
    }
}

impl Error for AppError {}

impl From<io::Error> for AppError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<image::ImageError> for AppError {
    fn from(error: image::ImageError) -> Self {
        Self::Image(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, process};

    #[test]
    fn parses_user_scope_set_command() {
        let config = Config::parse(vec!["f:".into(), r"C:\Icons\Drive.ico".into()]).unwrap();

        assert_eq!(config.drive, 'F');
        assert_eq!(config.scope, Scope::User);
        assert_eq!(
            config.action,
            Action::Set {
                icon_path: PathBuf::from(r"C:\Icons\Drive.ico")
            }
        );
    }

    #[test]
    fn parses_machine_scope_remove_command() {
        let config = Config::parse(vec![
            "g".into(),
            "--remove".into(),
            "--scope".into(),
            "machine".into(),
        ])
        .unwrap();

        assert_eq!(config.drive, 'G');
        assert_eq!(config.scope, Scope::Machine);
        assert_eq!(config.action, Action::Remove);
    }

    #[test]
    fn rejects_invalid_drive() {
        let error = Config::parse(vec!["drive".into(), r"C:\Icons\Drive.ico".into()])
            .expect_err("drive names should not be accepted");

        assert!(error.to_string().contains("invalid drive"));
    }

    #[test]
    fn converts_png_to_sibling_ico() {
        let test_dir = env::temp_dir().join(format!("drive-icon-setter-test-{}", process::id()));
        let _ = fs::remove_dir_all(&test_dir);
        fs::create_dir_all(&test_dir).unwrap();

        let png_path = test_dir.join("drive.png");
        let png_image = RgbaImage::from_pixel(48, 32, Rgba([24, 128, 72, 255]));
        png_image
            .save_with_format(&png_path, ImageFormat::Png)
            .unwrap();

        let prepared_icon = prepare_icon_path(&png_path).unwrap();

        assert_eq!(
            prepared_icon.converted_from,
            Some(png_path.canonicalize().unwrap())
        );
        assert!(has_extension(&prepared_icon.path, "ico"));
        assert!(prepared_icon.path.exists());

        let _ = fs::remove_dir_all(&test_dir);
    }
}
