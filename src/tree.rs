use std::{collections::HashSet, fmt::Display};

use egui::{ScrollArea, TextEdit, Widget};
use egui_tiles::SimplificationOptions;

use crate::{
    dnd::{DnDView, DragAndDropDemo},
    table::TableDemo,
};

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Pane {
    pub kind: PaneKind,
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub enum PaneKind {
    Table,
    Scripts,
    Groups { view: DnDView },
}

impl PaneKind {
    pub fn enumerate_default() -> Vec<Self> {
        vec![
            Self::Table,
            Self::Scripts,
            Self::Groups {
                view: DnDView::default(),
            },
        ]
    }
    pub fn str(&self) -> &'static str {
        Self::enumerate_str()[match self {
            PaneKind::Table => 0,
            PaneKind::Scripts => 1,
            PaneKind::Groups { .. } => 2,
        }]
    }
    pub fn enumerate_str() -> &'static [&'static str] {
        &["Table", "Scripts", "Groups"]
    }
}

impl Display for PaneKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.str())
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

    fn simplification_options(&self) -> egui_tiles::SimplificationOptions {
        SimplificationOptions {
            all_panes_must_have_tabs: true,
            ..Default::default()
        }
    }

    fn is_tab_closable(&self, _: &egui_tiles::Tiles<Pane>, _: egui_tiles::TileId) -> bool {
        true
    }

    fn pane_ui(
        &mut self,
        ui: &mut egui::Ui,
        _tile_id: egui_tiles::TileId,
        pane: &mut Pane,
    ) -> egui_tiles::UiResponse {
        match &mut pane.kind {
            PaneKind::Table => {
                if let Err(e) = self.table.ui(ui) {
                    *self.error = Some(e.to_string());
                }
            }
            PaneKind::Scripts => {
                ScrollArea::vertical().show(ui, |ui| {
                    let flattened = self.table.groups.iter().flat_map(|g| g.1.iter());
                    let mut dirty = false;
                    let mut to_remove = HashSet::new();
                    let mut scripts = self.table.scripts.borrow_mut();
                    for key in flattened {
                        let script = scripts.scripts().get_mut(key).unwrap_or_else(|| {
                            panic!("should have script for {key} when groups contains key {key}")
                        });
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
                    drop(scripts);
                    for key in to_remove {
                        self.table.scripts.remove_script(&key);
                        for group in self.table.groups.iter_mut() {
                            group.1.retain(|k| k != &key);
                        }
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
                        if let Some(last) = self.table.groups.last_mut() {
                            last.1.push(self.key.clone());
                        } else {
                            self.table
                                .groups
                                .push(("Remaining".to_string(), vec![self.key.clone()]));
                        }
                        *self.key = String::new();
                    }
                });
            }
            PaneKind::Groups { view } => {
                (DragAndDropDemo {
                    columns: &mut self.table.groups,
                    view,
                })
                .ui(ui);
            }
        }
        egui_tiles::UiResponse::None
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
    let groups = tiles.insert_pane(Pane {
        kind: PaneKind::Groups {
            view: DnDView::default(),
        },
    });
    let left = tiles.insert_tab_tile(vec![table, groups]);
    let root = tiles.insert_horizontal_tile(vec![left, scripts]);

    egui_tiles::Tree::new("my_tree", root, tiles)
}
