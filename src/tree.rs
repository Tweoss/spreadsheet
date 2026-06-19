use std::fmt::Display;

use egui_tiles::SimplificationOptions;

use crate::panes::{
    dnd::{DnDView, DragAndDropDemo},
    scripts::edit_scripts_ui,
    table::Table,
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
    pub table: &'a mut Table,
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
                edit_scripts_ui(self, ui);
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
