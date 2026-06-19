use std::collections::HashSet;

use egui::{ScrollArea, TextEdit, Widget};

use crate::tree::TreeBehavior;

pub fn edit_scripts_ui(
    tree: &mut TreeBehavior<'_>,
    ui: &mut egui::Ui,
) -> egui::scroll_area::ScrollAreaOutput<()> {
    ScrollArea::vertical().show(ui, |ui| {
        let mut dirty = false;
        let mut to_remove = HashSet::new();
        let mut scripts = tree.table.scripts.borrow_mut();
        for (group, headers) in &tree.table.groups {
            ui.heading(group);

            for key in headers {
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
        }
        drop(scripts);

        for key in to_remove {
            tree.table.scripts.remove_script(&key);
            for group in tree.table.groups.iter_mut() {
                group.1.retain(|k| k != &key);
            }
        }
        if dirty {
            if let Err(e) = tree.table.scripts.eval() {
                *tree.error = Some(e.to_string())
            } else {
                *tree.error = None;
            }
        }

        ui.separator();
        ui.text_edit_singleline(tree.key);
        let clicked = ui.button("Add Column").clicked();
        if !tree.key.is_empty() && !tree.table.scripts.contains_key(tree.key) && clicked {
            tree.table.scripts.add_script(tree.key.clone());
            if let Some(last) = tree.table.groups.last_mut() {
                last.1.push(tree.key.clone());
            } else {
                tree.table
                    .groups
                    .push(("Remaining".to_string(), vec![tree.key.clone()]));
            }
            *tree.key = String::new();
        }
    })
}
