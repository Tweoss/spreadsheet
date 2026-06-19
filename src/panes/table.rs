use std::collections::{BTreeMap, HashSet};

use egui::{Align2, Color32, Context, Id, Margin, NumExt as _, RichText, Sense, Ui, Vec2};
use rhai::EvalAltResult;

use crate::scripts::Scripts;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Table {
    num_sticky_cols: usize,
    default_column: egui_table::Column,
    auto_size_mode: egui_table::AutoSizeMode,
    top_row_height: f32,
    row_height: f32,
    is_row_expanded: BTreeMap<u64, bool>,
    prefetched: Vec<egui_table::PrefetchInfo>,
    pub scripts: Scripts,
    pub groups: Vec<(String, Vec<String>)>,
}

impl Default for Table {
    fn default() -> Self {
        Self {
            num_sticky_cols: 1,
            default_column: egui_table::Column::new(50.0)
                .range(10.0..=150.0)
                .resizable(true),
            auto_size_mode: egui_table::AutoSizeMode::default(),
            top_row_height: 24.0,
            row_height: 18.0,
            is_row_expanded: Default::default(),
            prefetched: vec![],
            scripts: Scripts::default(),
            groups: Vec::new(),
        }
    }
}

impl Table {
    pub fn init(&mut self) -> Result<(), String> {
        self.scripts.init()?;

        // Make sure we list all script keys in the columns.
        let binding = self.scripts.borrow();
        let mut script_keys: HashSet<_> = binding.scripts().keys().collect();
        let mut to_remove = Vec::new();
        for (i, (_, columns)) in self.groups.iter().enumerate() {
            for (j, key) in columns.iter().enumerate() {
                // If the list of script keys doesn't contain this key, then drop it.
                if !script_keys.remove(key) {
                    to_remove.push((i, j));
                }
            }
        }
        // Drop the extra keys. In reverse order, otherwise indices will be invalidated.
        to_remove.reverse();
        for (i, j) in to_remove {
            self.groups[i].1.remove(j);
        }
        let remaining: Vec<_> = script_keys.iter().map(|k| (*k).clone()).collect();
        if !remaining.is_empty() {
            self.groups.push(("Remaining".to_string(), remaining));
        }
        drop(binding);
        Ok(())
    }

    fn was_row_prefetched(&self, row_nr: u64) -> bool {
        self.prefetched
            .iter()
            .any(|info| info.visible_rows.contains(&row_nr))
    }

    fn cell_content_ui(&mut self, row_nr: u64, col_nr: usize, ui: &mut egui::Ui) {
        assert!(
            self.was_row_prefetched(row_nr),
            "Was asked to show row {row_nr} which was not prefetched! This is a bug in egui_table."
        );

        if col_nr >= self.num_sticky_cols {
            let key_index = col_nr - self.num_sticky_cols;
            if let Some(key) = (self.scripts.nth_key(key_index))
                && let Some(b) = self.scripts.borrow().values().get(&key)
                && let Some(v) = b.get(&(row_nr as usize))
            {
                let text = format!("{:.2}", v);
                if *v == 0.0 {
                    ui.label(RichText::new(text).color(Color32::GRAY));
                    return;
                }
                let (integral, fractional) = text.split_once(".").unwrap_or((text.as_str(), ""));
                let rev_chars: Vec<char> = integral.chars().rev().collect();
                // Commas every three decimals.
                let a: Vec<String> = rev_chars.chunks(3).map(|c| c.iter().collect()).collect();
                let text = a.join(",");
                let separated_integer: String = text.chars().rev().collect();
                let strong = if ui.ctx().theme().default_visuals().dark_mode {
                    Color32::WHITE
                } else {
                    Color32::BLACK
                };
                ui.label(RichText::new(separated_integer + "." + fractional).color(strong));
                return;
            }
        }

        let is_expanded = self
            .is_row_expanded
            .get(&row_nr)
            .copied()
            .unwrap_or_default();
        let expandedness = ui.animate_bool(Id::new(row_nr), is_expanded);

        ui.vertical(|ui| {
            if col_nr == 0 {
                ui.horizontal(|ui| {
                    // Button to expand/collapse row:
                    let (_, response) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::click());
                    egui::collapsing_header::paint_default_icon(ui, expandedness, &response);
                    if response.clicked() {
                        // Toggle.
                        // Note: we use a map instead of a set so that we can animate opening and closing of each column.
                        self.is_row_expanded.insert(row_nr, !is_expanded);
                    }

                    ui.label(row_nr.to_string());
                });
            } else {
                ui.horizontal(|ui| {
                    ui.label(format!("({row_nr}, {col_nr})"));

                    if (row_nr + col_nr as u64).is_multiple_of(27) {
                        if !ui.is_sizing_pass() {
                            // During a sizing pass we don't truncate!
                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                        }
                        ui.label("Extra long cell that will be truncated with an ellipsis character because it is so long");
                    }
                });

                if 0.0 < expandedness {
                    ui.label("Expanded content");
                    ui.label("Blah blah blah…");
                }
            }
        });
    }
}

impl egui_table::TableDelegate for Table {
    fn prepare(&mut self, info: &egui_table::PrefetchInfo) {
        assert!(
            info.visible_rows.end <= self.scripts.num_rows() as u64,
            "Was asked to prefetch rows {:?}, but we only have {} rows. This is a bug in egui_table.",
            info.visible_rows,
            self.scripts.num_rows()
        );
        self.prefetched.push(info.clone());
    }

    fn header_cell_ui(&mut self, ui: &mut egui::Ui, cell_inf: &egui_table::HeaderCellInfo) {
        let egui_table::HeaderCellInfo {
            group_index,
            col_range,
            row_nr,
            ..
        } = cell_inf;

        let margin = 4;

        egui::Frame::NONE
            .inner_margin(Margin::symmetric(margin, 0))
            .show(ui, |ui| {
                #[expect(clippy::collapsible_else_if)]
                if *row_nr == 0 {
                    if 0 < col_range.start {
                        // Our special grouped column.
                        let sticky = true;
                        let text = self.groups[*group_index].0.to_string();
                        if sticky {
                            let font_id = egui::TextStyle::Heading.resolve(ui.style());
                            let text_color = ui.visuals().text_color();
                            let galley =
                                ui.painter()
                                    .layout(text, font_id, text_color, f32::INFINITY);

                            // Put the text leftmost in the clip rect (so it is always visible)
                            let mut pos = Align2::LEFT_CENTER
                                .anchor_size(
                                    ui.clip_rect().shrink(margin as _).left_center(),
                                    galley.size(),
                                )
                                .min;

                            // … but not so far to the right that it doesn't fit.
                            pos.x = pos.x.at_most(ui.max_rect().right() - galley.size().x);

                            ui.put(
                                egui::Rect::from_min_size(pos, galley.size()),
                                egui::Label::new(galley),
                            );
                        } else {
                            ui.heading(text);
                        }
                    }
                } else {
                    if col_range.start < self.num_sticky_cols {
                        egui::Sides::new().height(ui.available_height()).show(
                            ui,
                            |ui| {
                                ui.heading("Row");
                            },
                            |ui| {
                                ui.label("⬇");
                            },
                        );
                    } else {
                        let flattened_headers: Vec<_> =
                            self.groups.iter().flat_map(|g| g.1.iter()).collect();
                        let key_index = col_range.start - self.num_sticky_cols;
                        if let Some(key) = flattened_headers.get(key_index) {
                            ui.heading(key.to_string());
                        } else {
                            ui.heading(format!("Column {group_index}"));
                        }
                    }
                }
            });
    }

    // You can use row_ui to add some style or interaction to the entire row.
    fn row_ui(&mut self, ui: &mut Ui, _row_nr: u64) {
        if ui.rect_contains_pointer(ui.max_rect()) {
            ui.painter()
                .rect_filled(ui.max_rect(), 0.0, ui.visuals().code_bg_color);
        }

        if ui.response().interact(Sense::click()).clicked() {
            // Handle row clicks
        }
    }

    fn cell_ui(&mut self, ui: &mut egui::Ui, cell_info: &egui_table::CellInfo) {
        let egui_table::CellInfo { row_nr, col_nr, .. } = *cell_info;

        if row_nr % 2 == 1 {
            ui.painter()
                .rect_filled(ui.max_rect(), 0.0, ui.visuals().faint_bg_color);
        }

        egui::Frame::NONE
            .inner_margin(Margin::symmetric(4, 0))
            .show(ui, |ui| {
                self.cell_content_ui(row_nr, col_nr, ui);
            });
    }

    fn row_top_offset(&self, ctx: &Context, _table_id: Id, row_nr: u64) -> f32 {
        let fully_expanded_row_height = 48.0;

        self.is_row_expanded
            .range(0..row_nr)
            .map(|(expanded_row_nr, expanded)| {
                let how_expanded = ctx.animate_bool(Id::new(expanded_row_nr), *expanded);
                how_expanded * fully_expanded_row_height
            })
            .sum::<f32>()
            + row_nr as f32 * self.row_height
    }
}

impl Table {
    pub fn ui(&mut self, ui: &mut egui::Ui) -> Result<(), Box<EvalAltResult>> {
        let response = egui::Grid::new("settings").show(ui, |ui| {
            ui.label("Rows");
            let speed = 1.0 + 0.05 * self.scripts.num_rows() as f32;
            let mut error = None;
            ui.add(
                egui::DragValue::from_get_set(|v| {
                    #[allow(clippy::collapsible_if)]
                    if let Some(v) = v
                        && let v = v.round()
                        && v > 0.0
                    {
                        if let Err(e) = self.scripts.set_num_rows(v as usize) {
                            error = Some(e);
                        }
                    }
                    self.scripts.num_rows() as f64
                })
                .speed(speed)
                .range(0..=10_000),
            );
            if let Some(e) = error {
                return Err(e);
            }
            ui.end_row();

            ui.label("Height of top row");
            ui.add(egui::DragValue::new(&mut self.top_row_height).range(0.0..=100.0));
            ui.end_row();

            ui.label("Height of other rows");
            ui.add(egui::DragValue::new(&mut self.row_height).range(0.0..=100.0));
            ui.end_row();

            ui.label("Auto-size mode");
            ui.horizontal(|ui| {
                ui.radio_value(
                    &mut self.auto_size_mode,
                    egui_table::AutoSizeMode::Never,
                    "Never",
                );
                ui.radio_value(
                    &mut self.auto_size_mode,
                    egui_table::AutoSizeMode::Always,
                    "Always",
                );
                ui.radio_value(
                    &mut self.auto_size_mode,
                    egui_table::AutoSizeMode::OnParentResize,
                    "OnParentResize",
                );
            });
            ui.end_row();
            Ok(())
        });
        response.inner?;

        let id_salt = Id::new("table_demo");
        egui_table::Table::new().id_salt(id_salt).get_id(ui); // Note: must be here (in the correct outer `ui` scope) to be correct.

        ui.separator();

        self.prefetched.clear();

        let mut sum = self.num_sticky_cols;
        let header_groups = self
            .groups
            .iter()
            .map(|(_, cols)| {
                let next = sum + cols.len();
                let range = sum..next;
                sum = next;
                range
            })
            .collect();

        let table = egui_table::Table::new()
            .id_salt(id_salt)
            .num_rows(self.scripts.num_rows() as u64)
            .columns(vec![self.default_column; sum])
            .num_sticky_cols(self.num_sticky_cols)
            .headers([
                egui_table::HeaderRow {
                    height: self.top_row_height,
                    groups: header_groups,
                },
                egui_table::HeaderRow::new(self.top_row_height),
            ])
            .auto_size_mode(self.auto_size_mode);

        table.show(ui, self);
        Ok(())
    }
}
