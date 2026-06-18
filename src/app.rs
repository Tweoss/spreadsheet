use std::{collections::HashSet, fs};

use egui::{Label, RichText, ScrollArea, TextEdit, Widget};
use egui_tiles::Tree;

use crate::{
    table::TableDemo,
    tree::{Pane, TreeBehavior, create_tree},
};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct App {
    key: String,
    file_path: String,
    table: TableDemo,
    error: Option<String>,
    tree: Tree<Pane>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            key: String::new(),
            file_path: "./data.txt".to_owned(),
            table: TableDemo::default(),
            error: None,
            tree: create_tree(),
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
        out.error = out.table.scripts.init().err();
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
                        if ui.button("Save").clicked() {
                            let to_string = ron::ser::to_string(&self);
                            if let Ok(v) = to_string {
                                let _ = fs::write(self.file_path.clone(), v.as_bytes());
                            }
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_theme_preference_buttons(ui);

                ui.text_edit_singleline(&mut self.file_path);
            });
        });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            if let Some(e) = &self.error {
                Label::new(RichText::new(e).strong()).ui(&mut *ui);
            }

            ui.separator();

            let mut behavior = TreeBehavior {
                table: &mut self.table,
                error: &mut self.error,
                key: &mut self.key,
            };
            self.tree.ui(&mut behavior, ui);

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                egui::warn_if_debug_build(ui);
            });
        });
    }
}
