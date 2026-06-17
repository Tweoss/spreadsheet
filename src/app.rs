use std::collections::HashSet;

use egui::{Label, RichText, Widget};

use crate::{scripts::wrap_script, table::TableDemo};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct App {
    key: String,
    script: String,

    #[serde(skip)] // This how you opt-out of serialization of a field
    value: f32,

    table: TableDemo,

    error: Option<String>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            key: String::new(),
            script: "Hello World!".to_owned(),
            value: 2.7,
            table: TableDemo::default(),
            error: None,
        }
    }
}

impl App {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        let mut out: Self = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        };
        out.table.scripts.init();
        out
    }
}

impl eframe::App for App {
    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::Panel::top("top_panel").show_inside(ui, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::MenuBar::new().ui(ui, |ui| {
                // NOTE: no File->Quit on web pages!
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ui.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_theme_preference_buttons(ui);
            });
        });

        egui::Panel::right("editor_panel").show_inside(ui, |ui| {
            let mut dirty = false;
            let mut to_remove = HashSet::new();
            for (key, script) in self.table.scripts.scripts_mut().iter_mut() {
                ui.horizontal(|ui| {
                    if ui.button("X").clicked() {
                        to_remove.insert(key.clone());
                    }
                    ui.heading(key);
                });
                if ui.text_edit_multiline(&mut script.text).changed() {
                    dirty = true;
                    script.ast = None;
                }
            }
            for key in to_remove {
                self.table.scripts.remove_script(&key);
            }
            if dirty {
                if let Err(e) = self.table.scripts.eval() {
                    eprintln!("found  error");
                    self.error = Some(e.to_string())
                } else {
                    eprintln!("found no error");
                    self.error = None;
                }
            }

            ui.text_edit_singleline(&mut self.key);
            if !self.key.is_empty()
                && !self.table.scripts.contains_key(&self.key)
                && ui.button("Add Column").clicked()
            {
                self.table.scripts.add_script(self.key.clone());
                self.key = String::new();
            }
        });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading("eframe template");

            if let Some(e) = &self.error {
                Label::new(RichText::new(e).strong()).ui(&mut *ui);
            }

            ui.separator();

            ui.add(egui::github_link_file!(
                "https://github.com/emilk/eframe_template/blob/main/",
                "Source code."
            ));

            if let Err(e) = self.table.ui(ui) {
                self.error = Some(e.to_string());
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                egui::warn_if_debug_build(ui);
            });
        });
    }
}
