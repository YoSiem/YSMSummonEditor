//! Database connection and dump folder settings.

use eframe::egui::{self, RichText, Ui};
use ysm_data::AppSettings;

use super::DialogContext;
use crate::action::Action;
use crate::icons;
use crate::theme::color;
use crate::widgets::{self, modal};
use crate::worker::{Command, Event};

enum TestState {
    Idle,
    Running,
    Passed(String),
    Failed(String),
}

pub struct SettingsDialog {
    working: AppSettings,
    port_text: String,
    show_password: bool,
    test: TestState,
    validation: Option<String>,
    /// The connection cannot be tested in demo mode; say so instead of failing oddly.
    demo: bool,
}

impl SettingsDialog {
    pub fn new(current: &AppSettings, demo: bool) -> Self {
        Self {
            working: current.clone(),
            port_text: current.database.port.to_string(),
            show_password: false,
            test: TestState::Idle,
            validation: None,
            demo,
        }
    }

    pub fn on_event(&mut self, event: &Event) {
        if let Event::ConnectionTested(result) = event {
            self.test = match result {
                Ok(server) => TestState::Passed(server.clone()),
                Err(error) => TestState::Failed(error.clone()),
            };
        }
    }

    /// Copies the port text into the settings; returns a message when the form is invalid.
    fn validate(&mut self) -> Option<String> {
        let database = &mut self.working.database;
        match self.port_text.trim().parse::<u16>() {
            Ok(port) if port > 0 => database.port = i32::from(port),
            _ => return Some("Port must be a number between 1 and 65535.".to_owned()),
        }
        database.server = database.server.trim().to_owned();
        database.database = database.database.trim().to_owned();
        database.user_id = database.user_id.trim().to_owned();
        self.working.dump_folder_path = self.working.dump_folder_path.trim().to_owned();
        if !database.is_configured() {
            return Some("Server, Database and User ID are required.".to_owned());
        }
        None
    }

    pub fn show(&mut self, ctx: &egui::Context, id: egui::Id, env: &mut DialogContext<'_>) -> bool {
        let mut save = false;
        let mut cancelled = false;

        let outcome = modal::show(ctx, id, icons::SETTINGS, color::ACCENT_BRIGHT, "Settings", 560.0, |ui| {
            modal::caption(ui, "Database (MSSQL)");
            self.database_form(ui, env);

            ui.add_space(10.0);
            modal::caption(ui, "Client dump folder");
            ui.label(
                widgets::muted("Scanned recursively for summon and skill images (png, jpg, bmp, webp, tga, dds).")
                    .small(),
            );
            let row = egui::Vec2::new(ui.available_width(), 30.0);
            ui.allocate_ui_with_layout(row, egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if widgets::button(ui, icons::FOLDER, "Browse").clicked()
                    && let Some(folder) = rfd::FileDialog::new().set_title("Select dump folder").pick_folder()
                {
                    self.working.dump_folder_path = folder.to_string_lossy().into_owned();
                }
                ui.add(egui::TextEdit::singleline(&mut self.working.dump_folder_path).desired_width(f32::INFINITY));
            });

            if let Some(message) = &self.validation {
                ui.add_space(6.0);
                modal::note(ui, icons::WARNING, message, color::WARNING);
            }

            modal::footer(ui, |ui| {
                if widgets::button(ui, icons::CLOSE, "Cancel").clicked() {
                    cancelled = true;
                }
                if widgets::primary_button(ui, icons::SAVE, "Save").clicked() {
                    self.validation = self.validate();
                    save = self.validation.is_none();
                }
            });
        });

        if save {
            env.actions.push(Action::ApplySettings(Box::new(self.working.clone())));
            return false;
        }
        let escaped = outcome.dismissed && ctx.input(|input| input.key_pressed(egui::Key::Escape));
        !(cancelled || escaped)
    }

    fn database_form(&mut self, ui: &mut Ui, env: &mut DialogContext<'_>) {
        let database = &mut self.working.database;
        let mut edited = false;

        egui::Grid::new("database-form").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
            ui.label("Server");
            ui.horizontal(|ui| {
                edited |= ui
                    .add(egui::TextEdit::singleline(&mut database.server).hint_text("host or IP").desired_width(280.0))
                    .changed();
                ui.label("Port");
                edited |= ui.add(egui::TextEdit::singleline(&mut self.port_text).desired_width(64.0)).changed();
            });
            ui.end_row();

            ui.label("Database");
            edited |= ui.add(egui::TextEdit::singleline(&mut database.database).desired_width(280.0)).changed();
            ui.end_row();

            ui.label("User ID");
            edited |= ui.add(egui::TextEdit::singleline(&mut database.user_id).desired_width(280.0)).changed();
            ui.end_row();

            ui.label("Password");
            ui.horizontal(|ui| {
                edited |= ui
                    .add(
                        egui::TextEdit::singleline(&mut database.password)
                            .password(!self.show_password)
                            .desired_width(280.0),
                    )
                    .changed();
                let (icon, hint) =
                    if self.show_password { (icons::HIDE, "Hide password") } else { (icons::SHOW, "Show password") };
                if widgets::icon_button(ui, icon, hint).clicked() {
                    self.show_password = !self.show_password;
                }
            });
            ui.end_row();

            ui.label("");
            edited |= ui
                .checkbox(&mut database.trust_server_certificate, "Trust server certificate")
                .on_hover_text("Accept the server's certificate without validating it (usual for local servers).")
                .changed();
            ui.end_row();
        });

        if edited {
            self.test = TestState::Idle;
            self.validation = None;
        }

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let running = matches!(self.test, TestState::Running);
            let test = ui
                .add_enabled(
                    !running && !self.demo,
                    egui::Button::new(widgets::icon_text(icons::CONNECTION, "Test connection")),
                )
                .on_disabled_hover_text(if self.demo { "Not available in demo mode" } else { "Test in progress" });
            if test.clicked() {
                self.validation = self.validate();
                if self.validation.is_none() {
                    self.test = TestState::Running;
                    env.actions.push(Action::Command(Command::TestConnection(self.working.database.clone())));
                }
            }
            match &self.test {
                TestState::Idle => {}
                TestState::Running => {
                    ui.spinner();
                    ui.label(widgets::muted("Connecting…"));
                }
                TestState::Passed(server) => {
                    ui.label(RichText::new(format!("{}  Connected", icons::SUCCESS)).color(color::SUCCESS))
                        .on_hover_text(server.as_str());
                }
                TestState::Failed(_) => {
                    ui.label(RichText::new(format!("{}  Failed", icons::ERROR)).color(color::DANGER));
                }
            }
        });
        if let TestState::Failed(error) = &self.test {
            ui.add(egui::Label::new(RichText::new(error).small().color(color::DANGER)).wrap());
        }
        ui.label(
            widgets::muted(
                "The password is stored in plain text in settings.json under %LOCALAPPDATA%\\YSMSummonEditor.",
            )
            .small(),
        );
    }
}
