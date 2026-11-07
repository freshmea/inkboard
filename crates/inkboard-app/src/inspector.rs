//! Selection geometry and explicit frame hierarchy; independent of the floating style palette.

use crate::app::App;
use crate::i18n::{t, tr};
use eframe::egui::{self, Align2, Ui, vec2};
use inkboard_core::{Id, Kind, Object, Page, Rect};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum TreeKey {
    Root,
    Object(Id),
    Group(Option<Id>, Id),
}

struct Row {
    key: TreeKey,
    ids: Vec<Id>,
    label: String,
    depth: usize,
    branch: bool,
}

pub struct Inspector {
    pub open: bool,
    selection: Vec<Id>,
    revision: u64,
    geometry: [f64; 5],
    before: Option<Rect>,
    dirty: bool,
    scale: u8,
    collapsed: HashSet<TreeKey>,
    rows: Vec<Row>,
    tree_version: Option<(Id, u64)>,
}

impl Default for Inspector {
    fn default() -> Self {
        Self {
            open: false,
            selection: Vec::new(),
            revision: u64::MAX,
            geometry: [0.0; 5],
            before: None,
            dirty: false,
            scale: 1,
            collapsed: HashSet::new(),
            rows: Vec::new(),
            tree_version: None,
        }
    }
}

fn object_label(o: &Object) -> String {
    let title = if let Kind::Box(b) = &o.kind { b.label.text.to_plain() } else { String::new() };
    if o.is_frame && !title.is_empty() {
        return title.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    }
    let kind = if o.is_frame {
        t("Frame")
    } else {
        match o.kind {
            Kind::Box(_) => t("Shape"),
            Kind::Image(_) => t("Image"),
            Kind::Pen(_) => t("Pen"),
            Kind::Connector(_) => t("Connector"),
            Kind::Table(_) => t("Table"),
        }
    };
    let title: String = title.chars().take(36).map(|c| if c.is_control() { ' ' } else { c }).collect();
    if title.is_empty() { format!("{kind} · {}", o.id.0.chars().take(6).collect::<String>()) } else { format!("{kind} · {title}") }
}

fn tree_rows(page: &Page, collapsed: &HashSet<TreeKey>) -> Vec<Row> {
    let mut children: HashMap<TreeKey, Vec<TreeKey>> = HashMap::new();
    let objects: HashMap<Id, &Object> = page.objects.iter().map(|o| (o.id.clone(), o)).collect();
    let mut groups: HashMap<TreeKey, Vec<Id>> = HashMap::new();
    for o in page.objects.iter().rev() {
        let parent = o.parent.as_ref().filter(|p| objects.contains_key(*p)).map_or(TreeKey::Root, |p| TreeKey::Object(p.clone()));
        let key = TreeKey::Object(o.id.clone());
        if let Some(g) = &o.group {
            let group = TreeKey::Group(o.parent.clone(), g.clone());
            if !groups.contains_key(&group) {
                children.entry(parent).or_default().push(group.clone());
            }
            groups.entry(group.clone()).or_default().push(o.id.clone());
            children.entry(group).or_default().push(key);
        } else {
            children.entry(parent).or_default().push(key);
        }
    }
    let mut stack: Vec<(TreeKey, usize)> = children.get(&TreeKey::Root).into_iter().flatten().rev().map(|k| (k.clone(), 0)).collect();
    let mut rows = Vec::new();
    let mut seen = HashSet::new();
    while let Some((key, depth)) = stack.pop() {
        if !seen.insert(key.clone()) {
            continue;
        }
        let (ids, label) = match &key {
            TreeKey::Object(id) => {
                let Some(o) = objects.get(id) else {
                    continue;
                };
                (vec![id.clone()], object_label(o))
            }
            TreeKey::Group(_, _) => (groups[&key].clone(), tr!("Group · {} objects", groups[&key].len())),
            TreeKey::Root => continue,
        };
        let branch = children.get(&key).is_some_and(|c| !c.is_empty());
        if !collapsed.contains(&key) {
            stack.extend(children.get(&key).into_iter().flatten().rev().map(|k| (k.clone(), depth + 1)));
        }
        rows.push(Row { key, ids, label, depth, branch });
    }
    rows
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    let mut state = std::mem::take(&mut app.inspector);
    if app.ui_demo.contains("inspector") {
        state.open = true;
    }
    let top = if ctx.content_rect().width() < 1180.0 { 116.0 } else { 76.0 };
    crate::theme::floating(ctx, "inspector", Align2::RIGHT_TOP, vec2(-12.0, top), 11, |ui| {
        ui.set_width(if state.open { 282.0 } else { 94.0 });
        if ui.button(if state.open { t("Hide details") } else { t("Details") }).clicked() {
            state.open = !state.open;
        }
        if state.open {
            let height = (ctx.content_rect().height() - top - 125.0).max(160.0);
            egui::ScrollArea::vertical().id_salt("inspector-body").max_height(height).min_scrolled_height(height).auto_shrink([false, false]).show(ui, |ui| {
                ui.set_width(282.0);
                selection(app, ui, &mut state);
                ui.separator();
                export_info(app, ui, &mut state);
                ui.separator();
                ui.strong(t("Objects and frames"));
                object_tree(app, ui, &mut state);
            });
        }
    });
    app.inspector = state;
}

fn selection(app: &mut App, ui: &mut Ui, state: &mut Inspector) {
    let ids = app.ed.page().expand_frames(&app.sel);
    let roots = app.ed.page().hierarchy_roots(&ids);
    let single = if let [root] = roots.as_slice() { app.ed.page().get(root).filter(|o| ids.len() == 1 || o.is_frame).cloned() } else { None };
    let bounds = single.as_ref().and_then(Object::frame).map(|f| f.rect()).or_else(|| app.ed.page().bounds_of(&ids));
    if state.selection != ids || state.revision != app.ed.revision() {
        state.selection.clone_from(&ids);
        state.revision = app.ed.revision();
        state.before = bounds;
        state.dirty = false;
        if let Some(b) = bounds {
            state.geometry = [b.x, b.y, b.w, b.h, single.as_ref().and_then(Object::frame).map_or(0.0, |f| f.angle.to_degrees())];
        }
    }
    ui.strong(tr!("Selected: {}", ids.len()));
    let writable = !app.ed.is_read_only() && app.editing.is_none() && app.drag.is_none();
    let unlocked = !ids.iter().any(|id| app.ed.page().get(id).is_some_and(|o| app.ed.page().is_locked(o)));
    if bounds.is_some() {
        ui.small(t("Position and size in canvas units"));
        ui.add_enabled_ui(writable && unlocked, |ui| {
            egui::Grid::new("geometry").num_columns(4).show(ui, |ui| {
                for (i, name) in ["X", "Y", "W", "H"].into_iter().enumerate() {
                    ui.label(name);
                    let range = if i < 2 { -1_000_000.0..=1_000_000.0 } else { 0.1..=1_000_000.0 };
                    state.dirty |= ui.add(egui::DragValue::new(&mut state.geometry[i]).speed(1.0).range(range).max_decimals(2)).changed();
                    if i % 2 == 1 {
                        ui.end_row();
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label(t("Rotation"));
                let can_rotate = !ids.iter().any(|id| app.ed.page().get(id).is_some_and(|o| matches!(o.kind, Kind::Table(_))));
                state.dirty |= ui.add_enabled(can_rotate, egui::DragValue::new(&mut state.geometry[4]).suffix("°").range(-360.0..=360.0)).changed();
            });
            if single.is_none() {
                ui.small(t("Rotation is relative for multiple objects."));
            }
            if ui.add_enabled(state.dirty, egui::Button::new(t("Apply geometry"))).clicked() {
                apply_geometry(app, &ids, state.before.unwrap(), state.geometry);
                state.revision = u64::MAX;
            }
        });
        image_info(app, ui, single.as_ref());
        ui.horizontal(|ui| {
            if ui.button(t("Fit selection")).clicked() {
                app.fit_view_with_inspector(true, true);
            }
            if ui.add_enabled(writable, egui::Button::new(if unlocked { t("Lock") } else { t("Unlock") })).clicked() {
                app.ed.set_locked(&roots, unlocked);
            }
        });
    } else {
        ui.label(t("Select an object to inspect its dimensions."));
    }
    ui.add_enabled_ui(writable && unlocked, |ui| {
        if ui.button(t("Frame selection")).clicked() {
            app.commit_editing();
            let c = app.view_center();
            let id = app.ed.frame_selection(&ids, Rect::new(c.x - 320.0, c.y - 240.0, 640.0, 480.0));
            app.sel = app.ed.page().expand_frames(&[id]);
        }
        if !roots.is_empty() {
            parent_control(app, ui, &roots, &ids);
        }
        if roots.len() == 1 && app.ed.page().get(&roots[0]).is_some_and(|o| o.is_frame) && ui.button(t("Release frame (keep contents)")).clicked() {
            app.ed.release_frame(&roots[0]);
            app.sel.retain(|id| app.ed.page().get(id).is_some());
        }
    });
}

fn image_info(app: &App, ui: &mut Ui, single: Option<&Object>) {
    if let Some(Object { kind: Kind::Image(image), .. }) = single
        && let Some(asset) = app.ed.doc.assets.get(&image.asset)
    {
        if asset.mime == "image/svg+xml" {
            ui.label(t("Original: vector SVG (no pixel resolution)"));
        } else if let Some((w, h)) = inkboard_formats::raster::declared_dimensions(&asset.data) {
            ui.label(tr!("Original image: {} × {} px", w, h));
        } else {
            ui.label(t("Original image dimensions unavailable"));
        }
    }
}

fn apply_geometry(app: &mut App, ids: &[Id], before: Rect, values: [f64; 5]) {
    if !values.iter().all(|v| v.is_finite()) || values[2] <= 0.0 || values[3] <= 0.0 {
        return;
    }
    if let [id] = ids
        && app.ed.page().get(id).is_some_and(|o| matches!(o.kind, Kind::Box(_) | Kind::Image(_)))
    {
        app.ed.update(id, |o| {
            let frame = match &mut o.kind {
                Kind::Box(b) => &mut b.frame,
                Kind::Image(i) => &mut i.frame,
                _ => unreachable!(),
            };
            frame.x = values[0];
            frame.y = values[1];
            frame.w = values[2];
            frame.h = values[3];
            frame.angle = values[4].to_radians();
        });
        return;
    }
    let roots = app.ed.page().hierarchy_roots(ids);
    let previous_angle =
        if let [root] = roots.as_slice() { app.ed.page().get(root).filter(|o| o.is_frame).and_then(Object::frame).map_or(0.0, |f| f.angle) } else { 0.0 };
    app.ed.transform_selection(ids, before, Rect::new(values[0], values[1], values[2], values[3]), values[4].to_radians() - previous_angle);
}

fn parent_control(app: &mut App, ui: &mut Ui, roots: &[Id], ids: &[Id]) {
    let current = app.ed.page().get(&roots[0]).and_then(|o| o.parent.clone());
    let mixed = roots.iter().any(|id| app.ed.page().get(id).is_some_and(|o| o.parent != current));
    let caption = if mixed { t("Mixed parents") } else { current.as_ref().and_then(|id| app.ed.page().get(id)).map_or_else(|| t("Page root"), object_label) };
    let choices: Vec<(Id, String, bool)> = app
        .ed
        .page()
        .objects
        .iter()
        .filter(|o| o.is_frame)
        .map(|o| (o.id.clone(), object_label(o), !ids.contains(&o.id) && !app.ed.page().is_locked(o)))
        .collect();
    ui.label(t("Parent frame"));
    egui::ComboBox::from_id_salt("parent-frame").selected_text(caption).width(260.0).show_ui(ui, |ui| {
        if ui.selectable_label(current.is_none() && !mixed, t("Page root")).clicked() {
            app.ed.reparent(roots, None);
        }
        for (id, label, enabled) in choices {
            if ui
                .push_id(&id, |ui| ui.add_enabled(enabled, egui::Button::selectable(current.as_ref() == Some(&id) && !mixed, &label)).on_hover_text(&label))
                .inner
                .clicked()
            {
                app.ed.reparent(roots, Some(&id));
            }
        }
    });
}

fn export_info(app: &App, ui: &mut Ui, state: &mut Inspector) {
    let ids = app.ed.page().expand_frames(&app.sel);
    let selection = (!ids.is_empty()).then_some(ids.as_slice());
    ui.strong(if selection.is_some() { t("PNG preview · selection") } else { t("PNG preview · page content") });
    ui.horizontal(|ui| {
        for scale in 1..=3 {
            ui.selectable_value(&mut state.scale, scale, format!("×{scale}"));
        }
    });
    if let Some(b) = inkboard_core::svg::export_bounds(app.ed.page(), selection, &inkboard_core::svg::SvgOptions::default()) {
        ui.label(format!("{:.0} × {:.0} px", b.w.ceil() * f64::from(state.scale), b.h.ceil() * f64::from(state.scale)));
        ui.small(t("Includes export margins. Set the same scale in Export."));
    }
    ui.small(t("Infinite canvas has no fixed resolution."));
}

fn object_tree(app: &mut App, ui: &mut Ui, state: &mut Inspector) {
    let version = (app.ed.current.clone(), app.ed.revision());
    if state.tree_version.as_ref() != Some(&version) {
        state.rows = tree_rows(app.ed.page(), &state.collapsed);
        state.tree_version = Some(version);
    }
    egui::ScrollArea::vertical().id_salt("object-tree").max_height(260.0).show_rows(ui, 30.0, state.rows.len(), |ui, range| {
        for index in range {
            let row = &state.rows[index];
            ui.push_id(&row.key, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(f32::from(u8::try_from(row.depth.min(8)).unwrap_or(8)) * 12.0);
                    if row.branch {
                        if ui.small_button(if state.collapsed.contains(&row.key) { "+" } else { "-" }).clicked() {
                            if !state.collapsed.remove(&row.key) {
                                state.collapsed.insert(row.key.clone());
                            }
                            state.tree_version = None;
                        }
                    } else {
                        ui.add_space(18.0);
                    }
                    let selected = row.ids.iter().all(|id| app.sel.contains(id));
                    if ui.add_sized([ui.available_width(), 30.0], egui::Button::selectable(selected, &row.label).truncate()).on_hover_text(&row.label).clicked()
                    {
                        app.commit_editing();
                        let ids = app.ed.page().expand_frames(&row.ids);
                        if ui.input(|i| i.modifiers.shift) {
                            if selected {
                                app.sel.retain(|id| !ids.contains(id));
                            } else {
                                for id in ids {
                                    if !app.sel.contains(&id) {
                                        app.sel.push(id);
                                    }
                                }
                            }
                        } else {
                            app.sel = ids;
                        }
                    }
                });
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::T;
    use egui_kittest::kittest::{NodeT, Queryable};

    #[test]
    fn selecting_a_child_frame_can_drag_it_without_moving_its_parent() {
        let mut t = T::new();
        let inner = t.app().ed.frame_selection(&[], Rect::new(200.0, 180.0, 200.0, 150.0));
        let outer = t.app().ed.frame_selection(std::slice::from_ref(&inner), Rect::default());
        let before = t.frame(&outer);
        t.app().sel = t.app().ed.page().expand_frames(std::slice::from_ref(&outer));
        t.h.step();
        t.drag((200.0, 260.0), (230.0, 270.0));
        assert_eq!(t.frame(&outer), before);
        assert!((t.frame(&inner).x - 230.0).abs() < 1.0);
    }

    #[test]
    fn both_frame_rows_and_all_parent_options_are_accessible() {
        let mut t = T::new();
        let first = t.app().ed.frame_selection(&[], Rect::new(100.0, 100.0, 200.0, 150.0));
        let second = t.app().ed.frame_selection(&[], Rect::new(350.0, 100.0, 200.0, 150.0));
        t.app().inspector.open = true;
        for _ in 0..4 {
            t.h.step();
        }
        t.h.get_by_label("Frame 2").click();
        for _ in 0..3 {
            t.h.step();
        }
        assert_eq!(t.app().sel, vec![second.clone()]);
        t.h.get_by_label("Frame 1").click();
        for _ in 0..3 {
            t.h.step();
        }
        assert_eq!(t.app().sel, vec![first]);
        t.h.get_by_value("Page root").click();
        t.h.step();
        // Both names remain listed; the selected frame itself is disabled to prevent cycles.
        assert!(t.h.query_all_by_label("Frame 1").count() >= 2);
        assert!(t.h.query_all_by_label("Frame 2").count() >= 2);
    }

    #[test]
    fn drawing_shapes_text_and_pen_inside_a_frame_adopts_them() {
        use crate::tools::Tool;
        let mut t = T::new();
        let frame = t.app().ed.frame_selection(&[], Rect::new(100.0, 100.0, 500.0, 400.0));
        for tool in [Tool::Shape(inkboard_core::BoxGeom::Rect), Tool::Text, Tool::Pen] {
            t.tool(tool);
            t.drag((180.0, 180.0), (240.0, 230.0));
            let object = t.app().ed.page().objects.last().unwrap().clone();
            assert_ne!(object.id, frame);
            assert_eq!(object.parent.as_ref(), Some(&frame), "{tool:?}");
            t.key(egui::Key::Escape);
        }
    }

    #[test]
    fn dropping_into_and_out_of_frame_updates_ownership_with_single_undo() {
        let mut t = T::new();
        let frame = t.app().ed.frame_selection(&[], Rect::new(200.0, 100.0, 300.0, 350.0));
        let shape = t.add_rect(20.0, 130.0, 60.0, 40.0);
        let before = t.app().ed.doc.clone();
        t.drag((50.0, 150.0), (280.0, 240.0));
        assert_eq!(t.app().ed.page().get(&shape).unwrap().parent.as_ref(), Some(&frame));
        t.app().ed.undo();
        assert_eq!(t.app().ed.doc, before);
        t.app().ed.redo();
        t.drag((280.0, 240.0), (70.0, 240.0));
        assert!(t.app().ed.page().get(&shape).unwrap().parent.is_none());
        t.app().ed.undo();
        assert_eq!(t.app().ed.page().get(&shape).unwrap().parent.as_ref(), Some(&frame));
    }

    #[test]
    fn pasting_objects_text_and_images_into_selected_frame_adopts_them() {
        let mut t = T::new();
        let frame = t.app().ed.frame_selection(&[], Rect::new(200.0, 100.0, 300.0, 350.0));
        let shape = t.add_rect(20.0, 130.0, 60.0, 40.0);
        t.app().sel = vec![shape];
        t.app().copy_selection();
        t.app().sel = vec![frame.clone()];
        let before = t.app().ed.doc.clone();
        t.app().paste();
        let pasted = t.app().sel[0].clone();
        assert_eq!(t.app().ed.page().get(&pasted).unwrap().parent.as_ref(), Some(&frame));
        t.app().ed.undo();
        assert_eq!(t.app().ed.doc, before);
        t.app().sel = vec![frame.clone()];
        t.app().paste_text("한글 필기");
        let pasted = t.app().sel[0].clone();
        assert_eq!(t.app().ed.page().get(&pasted).unwrap().parent.as_ref(), Some(&frame));
        t.app().sel = vec![frame.clone()];
        let at = t.app().paste_position();
        t.app().insert_rgba(1, 1, &[255, 255, 255, 255], at);
        let pasted = t.app().sel[0].clone();
        assert_eq!(t.app().ed.page().get(&pasted).unwrap().parent.as_ref(), Some(&frame));
    }

    #[test]
    fn inspector_button_opens_and_creates_an_undoable_frame() {
        let mut t = T::new();
        let a = t.add_rect(100.0, 100.0, 100.0, 60.0);
        t.app().sel = vec![a.clone()];
        t.h.step();
        t.h.step();
        t.h.get_by_label("Details").click();
        for _ in 0..4 {
            t.h.step();
        }
        assert!(t.app().inspector.open);
        assert_eq!(t.app().sel, vec![a.clone()], "selection before click");
        assert!(t.app().drag.is_none(), "canvas drag before click");
        let r = t.h.get_by_label("Frame selection").accesskit_node().bounding_box().unwrap();
        let button = egui::Rect::from_min_max(egui::pos2(r.x0 as f32, r.y0 as f32), egui::pos2(r.x1 as f32, r.y1 as f32));
        let area = t.h.ctx.memory(|m| m.area_rect(egui::Id::new("inspector"))).unwrap();
        assert!(area.contains_rect(button), "frame button {button:?} leaves inspector {area:?}");
        t.h.get_by_label("Frame selection").click();
        t.h.step();
        t.h.step();
        let frame = t.app().ed.page().get(&a).unwrap().parent.clone().unwrap_or_else(|| {
            let app = t.app();
            panic!("parent frame; selection {:?}, objects {}, drag {}", app.sel, app.ed.page().objects.len(), app.drag.is_some())
        });
        assert!(t.app().ed.page().get(&frame).unwrap().is_frame);
        assert_eq!(t.app().sel.len(), 2);
        let frame_width = t.frame(&frame).w;
        assert_eq!(t.app().inspector.geometry[2], frame_width);
        t.app().ed.undo();
        assert!(t.app().ed.page().get(&a).unwrap().parent.is_none());
        assert_eq!(t.app().ed.page().objects.len(), 1);
    }

    #[test]
    fn fit_selection_keeps_content_between_the_panels() {
        let mut t = T::new();
        let a = t.add_rect(0.0, 0.0, 760.0, 540.0);
        t.app().sel = vec![a];
        t.app().canvas_size = (1280.0, 800.0);
        t.app().fit_view_with_inspector(true, true);
        let view = t.app().ed.page().view;
        assert!(view.x >= 300.0);
        assert!(view.x + 760.0 * view.zoom <= 950.0);
    }

    #[test]
    fn geometry_and_clipboard_keep_frame_children_together() {
        let mut t = T::new();
        let a = t.add_rect(100.0, 100.0, 100.0, 60.0);
        let frame = t.app().ed.frame_selection(std::slice::from_ref(&a), Rect::default());
        let ids = t.app().ed.page().expand_frames(std::slice::from_ref(&frame));
        let before = t.app().ed.doc.clone();
        let b = t.app().ed.page().bounds_of(&ids).unwrap();
        apply_geometry(t.app(), &ids, b, [b.x + 80.0, b.y + 20.0, b.w * 2.0, b.h * 2.0, 0.0]);
        assert_eq!(t.frame(&a).w, 200.0);
        t.app().ed.undo();
        assert_eq!(t.app().ed.doc, before);
        t.app().sel = vec![frame];
        t.app().copy_selection();
        t.app().paste();
        let ids = t.app().sel.clone();
        assert_eq!(ids.len(), 2);
        let copied = t.app().ed.page().get(&ids[1]).unwrap();
        assert!(ids.contains(copied.parent.as_ref().unwrap()));
    }

    #[test]
    fn tree_folds_subtrees_and_direct_child_selection_remains_possible() {
        let mut t = T::new();
        let a = t.add_rect(100.0, 100.0, 100.0, 60.0);
        let frame = t.app().ed.frame_selection(std::slice::from_ref(&a), Rect::default());
        let page = t.app().ed.page();
        let rows = tree_rows(page, &HashSet::new());
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].ids, vec![frame.clone()]);
        assert_eq!(rows[1].ids, vec![a]);
        assert_eq!(rows[1].depth, 1);
        let collapsed = HashSet::from([TreeKey::Object(frame)]);
        assert_eq!(tree_rows(page, &collapsed).len(), 1);
    }

    #[test]
    fn selecting_frame_on_canvas_moves_children_and_undo_restores_them() {
        let mut t = T::new();
        let a = t.add_rect(100.0, 100.0, 100.0, 60.0);
        let frame = t.app().ed.frame_selection(std::slice::from_ref(&a), Rect::default());
        let before = t.app().ed.doc.clone();
        let f = t.frame(&frame);
        t.drag((f.x, f.y + 30.0), (f.x + 60.0, f.y + 70.0));
        assert!((t.frame(&a).x - 160.0).abs() < 1.0);
        t.app().ed.undo();
        assert_eq!(t.app().ed.doc, before);
    }

    #[test]
    fn enter_edits_the_frame_title_with_descendants_selected() {
        let mut t = T::new();
        let a = t.add_rect(100.0, 100.0, 100.0, 60.0);
        let frame = t.app().ed.frame_selection(&[a], Rect::default());
        t.app().sel = t.app().ed.page().expand_frames(std::slice::from_ref(&frame));
        t.h.step();
        t.key(egui::Key::Enter);
        assert_eq!(t.app().editing.as_ref().unwrap().id, frame);
    }
}
