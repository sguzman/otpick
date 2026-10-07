use crate::clipboard::copy_sensitive;
use crate::model::{Account, display_labels};
use crate::search::rank_accounts;
use crate::startup::StartupTrace;
use crate::totp::generate_current;
use eframe::egui;

pub struct PickerApp {
    accounts: Vec<Account>,
    display_labels: Vec<String>,
    query: String,
    ranked: Vec<usize>,
    selected: usize,
    focus_initialized: bool,
    notice: Option<String>,
    error: Option<String>,
    startup_trace: StartupTrace,
    first_frame_traced: bool,
}

impl PickerApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        accounts: Vec<Account>,
        notice: Option<String>,
        startup_trace: StartupTrace,
    ) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());

        let mut style = (*cc.egui_ctx.style_of(egui::Theme::Dark)).clone();
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(18.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(18.0));
        style
            .text_styles
            .insert(egui::TextStyle::Small, egui::FontId::proportional(15.0));
        cc.egui_ctx.set_style_of(egui::Theme::Dark, style);

        startup_trace.mark("egui-app-created");
        let ranked = rank_accounts(&accounts, "");
        let display_labels = display_labels(&accounts);

        Self {
            accounts,
            display_labels,
            query: String::new(),
            ranked,
            selected: 0,
            focus_initialized: false,
            notice,
            error: None,
            startup_trace,
            first_frame_traced: false,
        }
    }

    fn refresh_ranking(&mut self) {
        self.ranked = rank_accounts(&self.accounts, &self.query);
        self.selected = self.selected.min(self.ranked.len().saturating_sub(1));
    }

    fn move_selection(&mut self, delta: isize) {
        if self.ranked.is_empty() {
            self.selected = 0;
            return;
        }

        let last = self.ranked.len() - 1;
        self.selected = if delta.is_negative() {
            self.selected.saturating_sub(delta.unsigned_abs())
        } else {
            self.selected.saturating_add(delta as usize).min(last)
        };
    }

    fn copy_selected_and_close(&mut self, ctx: &egui::Context) {
        let Some(&account_index) = self.ranked.get(self.selected) else {
            return;
        };

        let code = generate_current(&self.accounts[account_index]);
        let result = copy_sensitive(&code).map_err(|error| error.to_string());

        match result {
            Ok(()) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            Err(error) => self.error = Some(error),
        }
    }
}

impl eframe::App for PickerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if !self.first_frame_traced {
            self.startup_trace.mark("first-frame");
            self.first_frame_traced = true;
        }

        let ctx = ui.ctx().clone();

        if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        if ctx.input(|input| input.key_pressed(egui::Key::ArrowUp)) {
            self.move_selection(-1);
        }
        if ctx.input(|input| input.key_pressed(egui::Key::ArrowDown)) {
            self.move_selection(1);
        }

        egui::CentralPanel::default().show(ui, |ui| {
            ui.add_space(10.0);

            let response = ui.add(
                egui::TextEdit::singleline(&mut self.query)
                    .desired_width(f32::INFINITY)
                    .hint_text("Search accounts..."),
            );

            if !self.focus_initialized {
                response.request_focus();
                self.focus_initialized = true;
            }

            if response.changed() {
                self.selected = 0;
                self.refresh_ranking();
                self.error = None;
            }

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);

            if let Some(notice) = &self.notice {
                ui.label(notice);
            } else if self.accounts.is_empty() {
                ui.label("No accounts in the vault.");
            } else if self.ranked.is_empty() {
                ui.label("No matches.");
            } else {
                for (row, account_index) in self.ranked.iter().copied().take(8).enumerate() {
                    let selected = row == self.selected;
                    if ui
                        .selectable_label(selected, &self.display_labels[account_index])
                        .clicked()
                    {
                        self.selected = row;
                    }
                }
            }

            if let Some(error) = &self.error {
                ui.add_space(8.0);
                ui.separator();
                ui.label(format!("Copy failed: {error}"));
            }
        });

        if ctx.input(|input| input.key_pressed(egui::Key::Enter)) {
            self.copy_selected_and_close(&ctx);
        }
    }
}
