use std::{collections::HashSet, fmt::Display};

use egui::{ScrollArea, TextEdit, Widget};

use crate::table::TableDemo;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Pane {
    pub kind: PaneKind,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub enum PaneKind {
    Table,
    Scripts,
}

impl Display for PaneKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                PaneKind::Table => "Table",
                PaneKind::Scripts => "Scripts",
            }
        )
    }
}

pub struct TreeBehavior<'a> {
    pub table: &'a mut TableDemo,
    pub error: &'a mut Option<String>,
    pub key: &'a mut String,
}

impl egui_tiles::Behavior<Pane> for TreeBehavior<'_> {
    fn tab_title_for_pane(&mut self, pane: &Pane) -> egui::WidgetText {
        pane.kind.to_string().into()
    }

    fn pane_ui(
        &mut self,
        ui: &mut egui::Ui,
        _tile_id: egui_tiles::TileId,
        pane: &mut Pane,
    ) -> egui_tiles::UiResponse {
        // You can make your pane draggable like so:
        let response = if ui
            .add(egui::Button::new("Drag me!").sense(egui::Sense::drag()))
            .drag_started()
        {
            egui_tiles::UiResponse::DragStarted
        } else {
            egui_tiles::UiResponse::None
        };

        match pane.kind {
            PaneKind::Table => {
                if let Err(e) = self.table.ui(ui) {
                    *self.error = Some(e.to_string());
                }
            }
            PaneKind::Scripts => {
                ScrollArea::vertical().show(ui, |ui| {
                    let mut dirty = false;
                    let mut to_remove = HashSet::new();
                    for (key, script) in self.table.scripts.borrow_mut().scripts().iter_mut() {
                        ui.horizontal(|ui| {
                            if ui.button("X").clicked() {
                                to_remove.insert(key.clone());
                            }
                            ui.heading(key);
                        });

                        if TextEdit::multiline(&mut script.text)
                            .desired_rows(1)
                            .ui(ui)
                            .changed()
                        {
                            dirty = true;
                            script.ast = None;
                        }
                    }
                    for key in to_remove {
                        self.table.scripts.remove_script(&key);
                    }
                    if dirty {
                        if let Err(e) = self.table.scripts.eval() {
                            *self.error = Some(e.to_string())
                        } else {
                            *self.error = None;
                        }
                    }

                    ui.separator();
                    ui.text_edit_singleline(self.key);
                    let clicked = ui.button("Add Column").clicked();
                    if !self.key.is_empty() && !self.table.scripts.contains_key(self.key) && clicked
                    {
                        self.table.scripts.add_script(self.key.clone());
                        *self.key = String::new();
                    }
                });
            }
        }
        response
    }
}

pub fn create_tree() -> egui_tiles::Tree<Pane> {
    let mut tiles = egui_tiles::Tiles::default();

    let table = tiles.insert_pane(Pane {
        kind: PaneKind::Table,
    });
    let scripts = tiles.insert_pane(Pane {
        kind: PaneKind::Scripts,
    });
    let root = tiles.insert_vertical_tile(vec![table, scripts]);

    egui_tiles::Tree::new("my_tree", root, tiles)
}
