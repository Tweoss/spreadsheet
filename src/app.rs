use std::fs;

use egui::{Label, RichText, Widget};
use egui_tiles::Tree;

use crate::{
    panes::table::Table,
    tree::{Pane, PaneKind, TreeBehavior, create_tree},
};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct App {
    key: String,
    file_path: String,
    table: Table,
    error: Option<String>,
    tree: Tree<Pane>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            key: String::new(),
            file_path: "./data.txt".to_owned(),
            table: Table::default(),
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
        out.init();

        out
    }
    fn init(&mut self) {
        self.error = self.table.init().err();
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
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        ui.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    if ui.button("Save").clicked() {
                        if let Err(e) = ron::ser::to_string(&self)
                            .map_err(|e| e.to_string())
                            .and_then(|s| {
                                fs::write(self.file_path.clone(), s.as_bytes())
                                    .map_err(|e| e.to_string())
                            })
                        {
                            self.error = Some(e);
                        } else {
                            self.error = None;
                        }
                    }
                    if ui.button("Open").clicked() {
                        match fs::read(self.file_path.clone())
                            .map_err(|e| e.to_string())
                            .and_then(|s| {
                                ron::de::from_bytes::<Self>(&s).map_err(|e| e.to_string())
                            }) {
                            Err(e) => {
                                self.error = Some(e);
                            }
                            Ok(v) => {
                                self.error = None;
                                *self = v;
                                self.init();
                            }
                        }
                    }
                });
                ui.add_space(16.0);

                egui::widgets::global_theme_preference_buttons(ui);

                ui.menu_button("New Tab", |ui| {
                    for kind in PaneKind::enumerate_default().into_iter() {
                        if ui.button(kind.to_string()).clicked() {
                            let id = self.tree.tiles.insert_pane(Pane { kind });
                            if let Some(root) = self.tree.root {
                                self.tree.move_tile_to_container(id, root, usize::MAX, true);
                            } else {
                                self.tree.root = Some(id);
                            }
                        }
                    }
                });

                ui.text_edit_singleline(&mut self.file_path);
            });
        });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            if let Some(e) = &self.error {
                Label::new(RichText::new(e).strong()).ui(&mut *ui);
            }

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
