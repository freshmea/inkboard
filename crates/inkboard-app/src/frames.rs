//! Frame creation presets and screen-space container titles.

use crate::app::App;
use crate::i18n::t;
use crate::render::{Cam, col};
use crate::tools::Tool;
use eframe::egui::{self, Color32, Galley, Painter, pos2, vec2};
use inkboard_core::{Align, BoxGeom, FontFamily, Frame, Id, Kind, Label, Object, Page, Pt, Rect, RichText, Style, VAlign, View};
use std::sync::Arc;

pub const TITLE_SIZE: f32 = 14.0;
const TITLE_GAP: f32 = 8.0;

#[derive(Clone, Copy, Debug)]
pub struct FramePreset {
    pub name: &'static str,
    pub width: u32,
    pub height: u32,
}

pub const PRESETS: [FramePreset; 5] = [
    FramePreset { name: "1080p", width: 1920, height: 1080 },
    FramePreset { name: "4K", width: 3840, height: 2160 },
    FramePreset { name: "Phone", width: 390, height: 844 },
    FramePreset { name: "Tablet", width: 834, height: 1194 },
    FramePreset { name: "Foldable phone", width: 884, height: 1104 },
];

pub fn object(page: &Page, rect: Rect) -> Object {
    let label = Label { text: RichText::plain(page.next_frame_name("Frame")), align: Align::Left, valign: VAlign::Top, ..Label::default() };
    let mut object = Object::new_box(BoxGeom::Rect, Frame::from_rect(rect), Style { fill: None, ..Style::default() }, label);
    object.is_frame = true;
    object
}

impl App {
    pub(crate) fn insert_frame_preset(&mut self, preset: FramePreset) {
        if self.view_only() {
            return;
        }
        self.commit_editing();
        let view = self.ed.page().view;
        let (width, height) = (f64::from(preset.width), f64::from(preset.height));
        let (canvas_w, canvas_h) = (f64::from(self.canvas_size.0), f64::from(self.canvas_size.1));
        // Inserting selects the frame, which opens the style panel even if it was hidden before this click.
        let left = if self.zen_mode { 0.0 } else { 300.0 };
        let right = if self.inspector.open && !self.zen_mode { 330.0 } else { 0.0 };
        let room = (canvas_w - left - right).max(120.0);
        let screen_center = Pt::new(left + room / 2.0, canvas_h / 2.0);
        let center = Pt::new((screen_center.x - view.x) / view.zoom, (screen_center.y - view.y) / view.zoom);
        let rect = Rect::new(center.x - width / 2.0, center.y - height / 2.0, width, height);
        let id = self.ed.insert_on_canvas(object(self.ed.page(), rect));
        self.sel = vec![id];
        self.set_tool(Tool::Select);
        let zoom = ((room - 64.0).max(40.0) / width).min((canvas_h - 180.0).max(40.0) / height).clamp(0.05, 1.0);
        self.set_view(View { x: screen_center.x - center.x * zoom, y: screen_center.y - center.y * zoom, zoom });
    }

    pub(crate) fn pick_canvas(&mut self, cam: &Cam, screen: egui::Pos2) -> Option<usize> {
        title_hit(&self.measurer.0, self.ed.page(), cam, screen).or_else(|| self.pick_at(cam.w(screen), 6.0 / f64::from(cam.zoom)))
    }
}

#[derive(Debug)]
pub struct FrameTitle {
    pub rect: egui::Rect,
    pub galley: Arc<Galley>,
}

pub fn title_layout(ctx: &egui::Context, cam: &Cam, object: &Object) -> Option<FrameTitle> {
    if !object.is_frame {
        return None;
    }
    let Kind::Box(boxed) = &object.kind else { return None };
    let text: String = boxed.label.text.to_plain().chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    let text = if text.is_empty() { t("Frame") } else { text };
    let galley = ctx.fonts_mut(|fonts| fonts.layout_no_wrap(text, crate::fonts::font_id(FontFamily::Sans, TITLE_SIZE, false), Color32::PLACEHOLDER));
    let bounds = boxed.frame.aabb();
    let corner = cam.s(Pt::new(bounds.x, bounds.y));
    let rect = egui::Rect::from_min_size(pos2(corner.x, corner.y - TITLE_GAP - galley.size().y), galley.size());
    Some(FrameTitle { rect, galley })
}

pub fn title_hit(ctx: &egui::Context, page: &Page, cam: &Cam, screen: egui::Pos2) -> Option<usize> {
    page.objects.iter().enumerate().rev().find_map(|(index, object)| {
        (page.is_visible(object)
            && page.layer(&object.layer).is_none_or(|layer| !layer.locked)
            && title_layout(ctx, cam, object).is_some_and(|title| title.rect.expand2(vec2(3.0, 2.0)).contains(screen)))
        .then_some(index)
    })
}

pub fn paint_titles(painter: &Painter, page: &Page, cam: &Cam, dark: bool, editing: Option<&Id>, ghost: &[Id]) {
    for object in &page.objects {
        if !page.is_visible(object) || editing == Some(&object.id) {
            continue;
        }
        if let Some(title) = title_layout(painter.ctx(), cam, object)
            && title.rect.intersects(painter.clip_rect())
        {
            let opacity = object.style.opacity * if ghost.contains(&object.id) { 0.3 } else { 1.0 };
            painter.galley(title.rect.min, title.galley, col(object.style.stroke, dark, opacity));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{Lang, set_for_thread};
    use crate::keymap::{Action, Keymap, Shortcut};
    use crate::tests::T;
    use eframe::egui::{Event, Key, Modifiers, Shape};
    use egui_kittest::Harness;
    use egui_kittest::kittest::{NodeT, Queryable};

    fn setup() -> T {
        set_for_thread(Some(Lang::En));
        T::new()
    }

    fn cam(t: &mut T) -> Cam {
        let app = t.app();
        let view = app.ed.page().view;
        Cam { origin: app.canvas_rect.min + vec2(view.x as f32, view.y as f32), zoom: view.zoom as f32 }
    }

    fn title(t: &mut T, id: &Id) -> FrameTitle {
        let camera = cam(t);
        let app = t.app();
        title_layout(&app.measurer.0, &camera, app.ed.page().get(id).unwrap()).unwrap()
    }

    fn text_shapes(shapes: &[egui::epaint::ClippedShape], text: &str) -> Vec<egui::epaint::TextShape> {
        fn collect(shape: &Shape, text: &str, out: &mut Vec<egui::epaint::TextShape>) {
            match shape {
                Shape::Text(shape) if shape.galley.job.text == text => out.push(shape.clone()),
                Shape::Vec(shapes) => shapes.iter().for_each(|shape| collect(shape, text, out)),
                _ => {}
            }
        }
        let mut out = Vec::new();
        shapes.iter().for_each(|shape| collect(&shape.shape, text, &mut out));
        out
    }

    #[test]
    fn toolbar_frame_is_between_eraser_and_rectangle_and_drag_is_undoable() {
        let mut t = setup();
        let eraser = t.h.get_by_label("Eraser").accesskit_node().bounding_box().unwrap();
        let frame = t.h.get_by_label("Frame").accesskit_node().bounding_box().unwrap();
        let rectangle = t.h.get_by_label("Rectangle").accesskit_node().bounding_box().unwrap();
        assert!(eraser.x1 < frame.x0 && frame.x1 < rectangle.x0);
        t.h.get_by_label("Frame").click();
        t.h.step();
        assert_eq!(t.app().tool, Tool::Frame);
        t.drag((100.0, 100.0), (400.0, 300.0));
        let object = t.objects().remove(0);
        assert!(object.is_frame);
        assert_eq!(object.frame().unwrap(), Frame::new(100.0, 100.0, 300.0, 200.0));
        let Kind::Box(boxed) = &object.kind else { unreachable!() };
        assert_eq!(boxed.label.text.to_plain(), "Frame 1");
        assert_eq!(t.app().tool, Tool::Select);
        assert_eq!(t.app().sel, vec![object.id]);
        assert!(t.app().editing.is_none());
        t.app().undo();
        assert!(t.objects().is_empty());
        t.app().redo();
        assert!(t.objects()[0].is_frame);
    }

    #[test]
    fn shortcut_s_is_registered_customizable_and_preserves_save_bindings() {
        let mut t = setup();
        assert_eq!(Keymap::default().action_for(Key::S, Modifiers::NONE), Some(Action::ToolFrame));
        assert_eq!(Keymap::default().action_for(Key::S, Modifiers::COMMAND), Some(Action::Save));
        assert_eq!(Keymap::default().action_for(Key::S, Modifiers::COMMAND | Modifiers::SHIFT), Some(Action::SaveAs));
        t.key(Key::S);
        assert_eq!(t.app().tool, Tool::Frame);
        t.app().keymap.remove(Action::ToolFrame, Shortcut::key(Key::S));
        t.app().keymap.add(Action::ToolFrame, Shortcut::key(Key::K)).unwrap();
        t.tool(Tool::Select);
        t.key(Key::S);
        assert_eq!(t.app().tool, Tool::Select);
        t.key(Key::K);
        assert_eq!(t.app().tool, Tool::Frame);
    }

    #[test]
    fn every_preset_inserts_exact_logical_dimensions_from_the_menu() {
        for (name, width, height) in
            [("1080p", 1920.0, 1080.0), ("4K", 3840.0, 2160.0), ("Phone", 390.0, 844.0), ("Tablet", 834.0, 1194.0), ("Foldable phone", 884.0, 1104.0)]
        {
            let mut t = setup();
            t.app().set_view(View { x: -250.0, y: 130.0, zoom: 0.1 });
            t.h.step();
            let view = t.app().ed.page().view;
            let center = Pt::new((800.0 - view.x) / view.zoom, (425.0 - view.y) / view.zoom);
            t.h.get_by_label("Frame presets").click();
            t.h.step();
            let label = format!("{name} · {width} × {height}");
            t.h.get_by_label(&label).click();
            t.h.step();
            let objects = t.objects();
            assert_eq!(objects.len(), 1);
            let object = &objects[0];
            assert!(object.is_frame);
            let frame = object.frame().unwrap();
            assert_eq!((frame.w, frame.h), (width, height));
            assert!(frame.center().dist(center) < 0.001);
            assert_eq!(t.app().sel, vec![object.id.clone()]);
            assert_eq!(t.app().tool, Tool::Select);
            let camera = cam(&mut t);
            let bounds = camera.rect_s(frame.aabb());
            assert!(bounds.width() > 100.0 && bounds.height() > 100.0);
            let canvas = t.app().canvas_rect;
            assert!(canvas.contains_rect(bounds));
            assert!(canvas.contains_rect(title(&mut t, &object.id).rect));
            t.app().undo();
            assert!(t.objects().is_empty());
        }
    }

    #[test]
    fn toolbar_frame_nests_and_locked_tool_stays_active() {
        let mut t = setup();
        let outer = t.app().ed.frame_selection(&[], Rect::new(0.0, 0.0, 600.0, 500.0));
        t.tool(Tool::Frame);
        t.app().tool_lock = true;
        t.drag((100.0, 100.0), (400.0, 300.0));
        let inner = t.objects().into_iter().find(|object| object.id != outer).unwrap();
        assert_eq!(inner.parent, Some(outer));
        assert_eq!(t.app().tool, Tool::Frame);
        assert_eq!(t.app().sel, vec![inner.id]);
        t.app().undo();
        assert_eq!(t.objects().len(), 1);
    }

    #[test]
    fn titles_stay_outside_rotated_frames_at_constant_screen_size_and_are_not_duplicated() {
        let mut t = setup();
        let id = t.app().ed.frame_selection(&[], Rect::new(100.0, 100.0, 300.0, 200.0));
        t.app().ed.update(&id, |object| {
            if let Kind::Box(boxed) = &mut object.kind {
                boxed.frame.angle = 2.1;
            }
        });
        let mut size = None;
        let world_bounds = t.frame(&id).aabb();
        for zoom in [0.15, 0.5, 1.0, 2.5] {
            t.app().set_view(View { x: 450.0 - world_bounds.x * zoom, y: 250.0 - world_bounds.y * zoom, zoom });
            t.h.step();
            let layout = title(&mut t, &id);
            let camera = cam(&mut t);
            let bounds = camera.rect_s(t.frame(&id).aabb());
            assert!((bounds.left() - layout.rect.left()).abs() < 0.001);
            assert!((bounds.top() - layout.rect.bottom() - TITLE_GAP).abs() < 0.001);
            assert_eq!(*size.get_or_insert(layout.rect.size()), layout.rect.size());
            let drawn = text_shapes(&t.h.output().shapes, "Frame 1");
            assert_eq!(drawn.len(), 1, "one exterior title at zoom {zoom}");
            assert_eq!(drawn[0].pos, layout.rect.min);
            assert!(drawn[0].galley.job.sections.iter().all(|section| section.format.font_id.size == 14.0));
        }
    }

    #[test]
    fn title_can_select_drag_and_double_click_edit_without_resizing_frame() {
        let mut t = setup();
        let child = t.add_rect(180.0, 180.0, 80.0, 60.0);
        let id = t.app().ed.frame_selection(std::slice::from_ref(&child), Rect::default());
        t.app().set_view(View { x: 350.0, y: 180.0, zoom: 0.5 });
        t.h.step();
        let before = t.frame(&id);
        let child_before = t.frame(&child);
        let at = title(&mut t, &id).rect.center();
        let camera = cam(&mut t);
        let world = camera.w(at);
        t.drag((world.x, world.y), (world.x + 40.0, world.y + 20.0));
        assert!(t.app().sel.contains(&id) && t.app().sel.contains(&child));
        assert_eq!(t.frame(&id).x, before.x + 40.0);
        assert_eq!(t.frame(&child).x, child_before.x + 40.0);
        t.app().undo();
        assert_eq!(t.frame(&id), before);
        assert_eq!(t.frame(&child), child_before);
        t.app().sel.clear();
        t.h.step();
        let at = title(&mut t, &id).rect.center();
        t.mv(at);
        let start_time = t.h.ctx.input(|input| input.time);
        for (index, pressed) in [true, false, true, false].into_iter().enumerate() {
            t.h.input_mut().time = Some(start_time + 0.05 * (index + 1) as f64);
            t.button(at, pressed, Modifiers::NONE);
        }
        t.h.step();
        assert_eq!(t.app().editing.as_ref().unwrap().id, id);
        let layout = title(&mut t, &id);
        let probe = t.app().editor_probe.as_ref().unwrap();
        assert!((probe.rows[0].0 - layout.rect.left()).abs() < 1.0);
        assert!((probe.rows[0].1 - layout.rect.top()).abs() < 2.0);
        t.h.key_press(Key::S);
        t.h.event(Event::Text("s".to_owned()));
        t.h.step();
        assert_eq!(t.app().tool, Tool::Select);
        assert!(t.app().editing.as_ref().unwrap().buf.contains('s'));
        t.app().editing.as_mut().unwrap().buf = "A long frame title that must never resize the container".to_owned();
        t.h.step();
        t.key(Key::Enter);
        assert!(t.app().editing.is_none());
        assert_eq!(t.frame(&id), before);
        let Kind::Box(boxed) = &t.app().ed.page().get(&id).unwrap().kind else { unreachable!() };
        assert_eq!(boxed.label.text.to_plain(), "A long frame title that must never resize the container");
    }

    #[test]
    fn title_paints_and_selects_when_frame_body_is_culled() {
        let mut t = setup();
        let id = t.app().ed.frame_selection(&[], Rect::new(100.0, 0.0, 300.0, 200.0));
        t.app().set_view(View { x: 350.0, y: 865.0, zoom: 1.0 });
        t.h.step();
        let at = title(&mut t, &id).rect.center();
        let canvas = t.app().canvas_rect;
        let camera = cam(&mut t);
        assert!(canvas.contains(at));
        assert!(!canvas.intersects(camera.rect_s(t.frame(&id).aabb())));
        assert_eq!(text_shapes(&t.h.output().shapes, "Frame 1").len(), 1);
        t.mv(at);
        t.button(at, true, Modifiers::NONE);
        t.button(at, false, Modifiers::NONE);
        assert_eq!(t.app().sel, vec![id]);
    }

    #[test]
    fn zen_presets_use_the_full_canvas_even_when_details_were_open() {
        let mut t = setup();
        t.app().inspector.open = true;
        t.key(Key::F4);
        for _ in 0..3 {
            t.h.step();
        }
        t.h.get_by_label("Frame presets").click();
        t.h.step();
        t.h.get_by_label("4K · 3840 × 2160").click();
        t.h.step();
        let object = t.objects().remove(0);
        let camera = cam(&mut t);
        let bounds = camera.rect_s(object.frame().unwrap().aabb());
        let canvas = t.app().canvas_rect;
        assert!(bounds.center().distance(canvas.center()) < 0.01);
        assert!(canvas.contains_rect(bounds));
        assert!(canvas.contains_rect(title(&mut t, &object.id).rect));
        assert!(t.app().zen_mode && t.app().inspector.open);
        t.key(Key::F4);
        assert!(!t.app().zen_mode && t.app().inspector.open);
    }

    #[test]
    fn toolbar_remains_scrollable_on_small_windows() {
        set_for_thread(Some(Lang::En));
        for width in [390.0, 820.0, 900.0, 1024.0, 1320.0, 1360.0, 1600.0] {
            let mut harness = Harness::builder().with_size(vec2(width, 850.0)).build_eframe(|cc| App::build(cc, None, None, false));
            for _ in 0..4 {
                harness.step();
            }
            let ctx = harness.state_mut().measurer.0.clone();
            let tools = ctx.memory(|memory| memory.area_rect(egui::Id::new("tools"))).unwrap();
            let title = ctx.memory(|memory| memory.area_rect(egui::Id::new("title"))).unwrap();
            eprintln!("window {width}, toolbar width {}, title right {}", tools.width(), title.right());
            assert!(tools.width() <= width - 16.0, "toolbar {tools:?} in {width}");
            assert!(!tools.intersects(title), "toolbar {tools:?} overlaps title {title:?} in {width}");
            harness.state_mut().toggle_zen();
            for _ in 0..4 {
                harness.step();
            }
            let tools = ctx.memory(|memory| memory.area_rect(egui::Id::new("tools"))).unwrap();
            assert!(tools.left() >= 8.0 && tools.right() <= width - 8.0, "Zen toolbar {tools:?} in {width}");
            assert!(tools.top() < 20.0 && tools.bottom() < 80.0);
        }
    }
}
