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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ColLocation {
    col_idx: usize,
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
        let mut from: Option<std::sync::Arc<Location>> = None;
        let mut to = None;
        let mut col_from = None;
        let mut col_to = None;

        ui.columns(self.columns.len(), |uis| {
            let column_count = self.columns.len();
            for (col_idx, (label, column)) in self.columns.clone().into_iter().enumerate() {
                let ui = &mut uis[col_idx];
                let col_id = Id::new(("my_drag_and_drop_demo", col_idx));
                let label = &label;

                ui.vertical_centered(|ui| {
                    let frame = Frame::default().inner_margin(4.0);

                    // Only allow deletion and column DnD if there's another column to move to.
                    if column_count > 1 {
                        ui.horizontal(|ui| {
                            ui.set_min_size(vec2(64.0, 40.0));
                            let dropped = ui
                                .dnd_drag_source(col_id, ColLocation { col_idx }, |ui| {
                                    // Upon deletion of this column, move contents to another column.
                                    if ui.button("X").clicked() {
                                        let other_index =
                                            if col_idx == 0 { 1 } else { col_idx - 1 };
                                        let [other, current] = self
                                            .columns
                                            .get_disjoint_mut([other_index, col_idx])
                                            .unwrap();
                                        other.1.append(&mut current.1);
                                        self.columns.remove(col_idx);
                                    }
                                    ui.heading(label);
                                    // Fill out the column with the frame background color
                                    // (and make it draggable)
                                    ui.allocate_space(vec2(ui.available_width(), 0.0));
                                })
                                .response;
                            let response = dropped;
                            if let Some(pointer) = ui.input(|i| i.pointer.interact_pos())
                                && let Some(payload) = response.dnd_hover_payload::<ColLocation>()
                            {
                                let rect = response.rect;
                                // Preview insertion:
                                let stroke = egui::Stroke::new(2.0, Color32::BLUE);
                                let (insert_idx, stroke_pos) = if payload.col_idx == col_idx {
                                    // We are dragged onto ourselves
                                    (col_idx, rect.center().x)
                                } else if pointer.x < rect.center().x {
                                    // Above us
                                    (col_idx, rect.left())
                                } else {
                                    // Below us
                                    (col_idx + 1, rect.right())
                                };
                                ui.painter().vline(stroke_pos, rect.y_range(), stroke);
                                if let Some(dropped) = response.dnd_release_payload::<ColLocation>()
                                {
                                    col_from = Some(dropped);
                                    col_to = Some(ColLocation {
                                        col_idx: insert_idx,
                                    });
                                }
                            }
                        });
                    } else {
                        ui.heading(label);
                    }
                    ui.dnd_drop_zone::<Location, ()>(frame, |ui| {
                        ui.set_min_size(vec2(64.0, 40.0));
                        ui.allocate_space(vec2(ui.available_width(), 0.0));

                        for (row_idx, item) in column.iter().enumerate() {
                            let item_id = Id::new(("my_drag_and_drop_demo", col_idx, row_idx));
                            let item_location = Location {
                                col: col_idx,
                                row: row_idx,
                            };
                            let response = ui
                                .dnd_drag_source(item_id, item_location, |ui| {
                                    ui.label(item).hovered();
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
                });
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

        if let (Some(from), Some(mut to)) = (col_from, col_to)
            && from.col_idx != to.col_idx
        {
            let item = self.columns.remove(from.col_idx);
            if from.col_idx < to.col_idx {
                to.col_idx -= 1;
            }
            self.columns
                .insert(to.col_idx.min(self.columns.len()), item);
        }
    }
}
