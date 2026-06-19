use egui::{Color32, Frame, Id, Ui, vec2};

pub struct DragAndDropDemo<'a> {
    pub columns: &'a mut Vec<(String, Vec<String>)>,
    pub view: &'a mut DnDView,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Default)]
pub struct DnDView {
    group_name: String,
}

/// What is being dragged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Location {
    col: usize,
    row: usize,
}

impl DragAndDropDemo<'_> {
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.label("This is a simple example of drag-and-drop in egui.");
        ui.label("Drag items between columns.");

        ui.text_edit_singleline(&mut self.view.group_name);
        if ui.button("Add Group").clicked() {
            self.columns.push((self.view.group_name.clone(), vec![]));
        }

        // If there is a drop, store the location of the item being dragged, and the destination for the drop.
        let mut from = None;
        let mut to = None;

        ui.columns(self.columns.len(), |uis| {
            let column_count = self.columns.len();
            for (col_idx, column) in self.columns.clone().into_iter().enumerate() {
                let ui = &mut uis[col_idx];

                let frame = Frame::default().inner_margin(4.0);

                let (_, dropped_payload) = ui.dnd_drop_zone::<Location, ()>(frame, |ui| {
                    ui.set_min_size(vec2(64.0, 100.0));
                    let (label, column) = &column;

                    // Only allow deletion if there's another column to move to.
                    if column_count > 1 {
                        ui.horizontal(|ui| {
                            // Upon deletion of this column, move contens to another column.
                            if ui.button("X").clicked() {
                                let other_index = if col_idx == 0 { 1 } else { col_idx - 1 };
                                let [other, current] = self
                                    .columns
                                    .get_disjoint_mut([other_index, col_idx])
                                    .unwrap();
                                other.1.append(&mut current.1);
                                self.columns.remove(col_idx);
                            }
                            ui.heading(label);
                        });
                    } else {
                        ui.heading(label);
                    }

                    for (row_idx, item) in column.iter().enumerate() {
                        let item_id = Id::new(("my_drag_and_drop_demo", col_idx, row_idx));
                        let item_location = Location {
                            col: col_idx,
                            row: row_idx,
                        };
                        let response = ui
                            .dnd_drag_source(item_id, item_location, |ui| {
                                ui.label(item);
                            })
                            .response;

                        // Detect drops onto this item:
                        if let (Some(pointer), Some(hovered_payload)) = (
                            ui.input(|i| i.pointer.interact_pos()),
                            response.dnd_hover_payload::<Location>(),
                        ) {
                            let rect = response.rect;

                            // Preview insertion:
                            let stroke = egui::Stroke::new(1.0, Color32::WHITE);
                            let insert_row_idx = if *hovered_payload == item_location {
                                // We are dragged onto ourselves
                                ui.painter().hline(rect.x_range(), rect.center().y, stroke);
                                row_idx
                            } else if pointer.y < rect.center().y {
                                // Above us
                                ui.painter().hline(rect.x_range(), rect.top(), stroke);
                                row_idx
                            } else {
                                // Below us
                                ui.painter().hline(rect.x_range(), rect.bottom(), stroke);
                                row_idx + 1
                            };

                            if let Some(dragged_payload) = response.dnd_release_payload() {
                                // The user dropped onto this item.
                                from = Some(dragged_payload);
                                to = Some(Location {
                                    col: col_idx,
                                    row: insert_row_idx,
                                });
                            }
                        }
                    }
                });

                if let Some(dragged_payload) = dropped_payload {
                    // The user dropped onto the column, but not on any one item.
                    from = Some(dragged_payload);
                    to = Some(Location {
                        col: col_idx,
                        row: usize::MAX, // Inset last
                    });
                }
            }
        });

        if let (Some(from), Some(mut to)) = (from, to) {
            if from.col == to.col {
                // Dragging within the same column.
                // Adjust row index if we are re-ordering:
                to.row -= (from.row < to.row) as usize;
            }

            let item = self.columns[from.col].1.remove(from.row);

            let column = &mut self.columns[to.col];
            to.row = to.row.min(column.1.len());
            column.1.insert(to.row, item);
        }
    }
}
