use crate::icons;
use crate::tools::Tool;
use eframe::egui::{Color32, CursorIcon, Painter, Pos2, Rect, Stroke, pos2, vec2};
use inkboard_core::BoxGeom;

struct Glyph {
    name: &'static str,
    tip: Option<Pos2>,
}

fn glyph(tool: Tool) -> Option<Glyph> {
    let (name, tip) = match tool {
        Tool::Select | Tool::Hand | Tool::Text | Tool::Shape(BoxGeom::Text) => return None,
        Tool::Pen => ("pen", Some(pos2(4.0, 20.0))),
        Tool::Highlighter => ("highlighter", Some(pos2(5.0, 19.0))),
        Tool::Eraser => ("eraser", Some(pos2(4.0, 15.0))),
        Tool::Frame => ("frame", None),
        Tool::Shape(BoxGeom::Rect) => ("rect", None),
        Tool::Shape(BoxGeom::Ellipse) => ("ellipse", None),
        Tool::Shape(BoxGeom::Diamond) => ("diamond", None),
        Tool::Shape(BoxGeom::Triangle) => ("triangle", None),
        Tool::Sticky | Tool::Shape(BoxGeom::Sticky) => ("sticky", None),
        Tool::Line => ("line", None),
        Tool::Arrow => ("arrow", None),
        Tool::Table => ("table", None),
        Tool::Vote => ("vote", None),
        Tool::Laser => ("laser", None),
    };
    Some(Glyph { name, tip })
}

pub fn paint(painter: &Painter, tool: Tool, at: Pos2) {
    let Some(glyph) = glyph(tool) else { return };
    let Some(texture) = icons::texture(painter.ctx(), glyph.name) else { return };
    let rect = Rect::from_min_size(glyph.tip.map_or(at + vec2(6.0, 6.0), |tip| at - tip.to_vec2()), vec2(24.0, 24.0));
    let uv = Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0));
    for offset in [vec2(-1.0, -1.0), vec2(0.0, -1.0), vec2(1.0, -1.0), vec2(-1.0, 0.0), vec2(1.0, 0.0), vec2(-1.0, 1.0), vec2(0.0, 1.0), vec2(1.0, 1.0)] {
        painter.image(texture.id(), rect.translate(offset), uv, Color32::WHITE);
    }
    let ink = Color32::from_rgb(0x20, 0x23, 0x28);
    painter.image(texture.id(), rect, uv, ink);
    if glyph.tip.is_none() {
        painter.circle_filled(at, 2.8, Color32::WHITE);
        painter.circle_stroke(at, 1.5, Stroke::new(1.5, ink));
    }
    painter.ctx().set_cursor_icon(CursorIcon::None);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::T;
    use eframe::egui::{self, Event, Modifiers, PointerButton, Shape};
    use inkboard_core::{BgPreset, Kind, Pt};

    fn cursor_meshes<'a>(t: &'a T, name: &str) -> Vec<&'a egui::epaint::ClippedShape> {
        let texture = icons::texture(&t.h.ctx, name).unwrap().id();
        let canvas = t.h.state().canvas_rect;
        t.h.output()
            .shapes
            .iter()
            .filter(|s| {
                s.clip_rect == canvas
                    && s.shape.visual_bounding_rect().size() == vec2(24.0, 24.0)
                    && matches!(&s.shape, Shape::Mesh(m) if m.texture_id == texture)
            })
            .collect()
    }

    #[test]
    fn tool_cursors_show_toolbar_glyphs_with_a_contrasting_halo() {
        let mut t = T::new();
        for preset in [BgPreset::White, BgPreset::Black] {
            t.app().ed.doc.background.preset = preset;
            for tool in [
                Tool::Pen,
                Tool::Highlighter,
                Tool::Eraser,
                Tool::Frame,
                Tool::Shape(BoxGeom::Rect),
                Tool::Shape(BoxGeom::Ellipse),
                Tool::Shape(BoxGeom::Diamond),
                Tool::Shape(BoxGeom::Triangle),
                Tool::Line,
                Tool::Arrow,
                Tool::Sticky,
                Tool::Table,
                Tool::Vote,
                Tool::Laser,
            ] {
                t.tool(tool);
                let at = t.at(400.0, 300.0);
                t.mv(at);
                assert_eq!(t.h.output().platform_output.cursor_icon, CursorIcon::None, "{preset:?} {tool:?}");
                let glyph = glyph(tool).unwrap();
                let meshes = cursor_meshes(&t, glyph.name);
                assert_eq!(meshes.len(), 9, "{preset:?} {tool:?}");
                for mesh in &meshes[..8] {
                    let Shape::Mesh(mesh) = &mesh.shape else { unreachable!() };
                    assert!(mesh.vertices.iter().all(|v| v.color == Color32::WHITE));
                }
                let Shape::Mesh(mesh) = &meshes[8].shape else { unreachable!() };
                assert!(mesh.vertices.iter().all(|v| v.color == Color32::from_rgb(0x20, 0x23, 0x28)));
            }
        }
    }

    #[test]
    fn pen_cursor_tip_matches_the_stroke_and_stays_visible_during_drawing() {
        let mut t = T::new();
        t.tool(Tool::Pen);
        let at = t.at(400.0, 300.0);
        t.mv(at);
        let meshes = cursor_meshes(&t, "pen");
        assert_eq!(meshes.last().unwrap().shape.visual_bounding_rect().min + vec2(4.0, 20.0), at);
        t.button(at, true, Modifiers::NONE);
        let end = at + vec2(40.0, 20.0);
        t.mv(end);
        assert_eq!(t.h.output().platform_output.cursor_icon, CursorIcon::None);
        assert_eq!(cursor_meshes(&t, "pen").len(), 9);
        t.button(end, false, Modifiers::NONE);
        let objects = t.objects();
        let Kind::Pen(pen) = &objects[0].kind else { panic!("pen stroke") };
        assert!(pen.pts[0].dist(Pt::new(400.0, 300.0)) < 0.01);
    }

    #[test]
    fn tool_cursor_does_not_replace_panels_popups_or_contextual_cursors() {
        let mut t = T::new();
        t.tool(Tool::Pen);
        let canvas = t.at(400.0, 300.0);
        t.mv(canvas);
        assert_eq!(cursor_meshes(&t, "pen").len(), 9);
        t.mv(pos2(30.0, 30.0));
        assert!(cursor_meshes(&t, "pen").is_empty());
        assert_ne!(t.h.output().platform_output.cursor_icon, CursorIcon::None);
        t.mv(canvas);
        t.h.event(Event::PointerButton { pos: canvas, button: PointerButton::Secondary, pressed: true, modifiers: Modifiers::NONE });
        t.h.step();
        t.h.event(Event::PointerButton { pos: canvas, button: PointerButton::Secondary, pressed: false, modifiers: Modifiers::NONE });
        t.h.step();
        assert!(egui::Popup::is_any_open(&t.h.ctx));
        assert!(cursor_meshes(&t, "pen").is_empty());
        t.h.key_press(egui::Key::Escape);
        t.h.step();
        t.tool(Tool::Text);
        t.mv(canvas);
        assert_eq!(t.h.output().platform_output.cursor_icon, CursorIcon::Text);
        t.tool(Tool::Hand);
        t.mv(canvas);
        assert_eq!(t.h.output().platform_output.cursor_icon, CursorIcon::Grab);
        t.button(canvas, true, Modifiers::NONE);
        assert_eq!(t.h.output().platform_output.cursor_icon, CursorIcon::Grabbing);
        t.button(canvas, false, Modifiers::NONE);
        t.tool(Tool::Select);
        let id = t.add_rect(350.0, 250.0, 150.0, 120.0);
        t.app().sel = vec![id.clone()];
        t.mv(canvas);
        assert_eq!(t.h.output().platform_output.cursor_icon, CursorIcon::Move);
        let corner = t.at(350.0, 250.0);
        t.mv(corner);
        assert_eq!(t.h.output().platform_output.cursor_icon, CursorIcon::ResizeNwSe);
        t.tool(Tool::Pen);
        t.app().start_editing(&id, None);
        t.mv(canvas);
        assert!(t.app().editing.is_some());
        assert!(cursor_meshes(&t, "pen").is_empty());
        assert_ne!(t.h.output().platform_output.cursor_icon, CursorIcon::None);
    }

    #[test]
    fn tool_cursor_yields_to_space_pan_and_disappears_when_pointer_leaves() {
        let mut t = T::new();
        t.tool(Tool::Pen);
        let at = t.at(400.0, 300.0);
        t.mv(at);
        t.h.event(Event::Key { key: egui::Key::Space, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE });
        t.h.step();
        assert_eq!(t.h.output().platform_output.cursor_icon, CursorIcon::Grab);
        assert!(cursor_meshes(&t, "pen").is_empty());
        t.button(at, true, Modifiers::NONE);
        assert_eq!(t.h.output().platform_output.cursor_icon, CursorIcon::Grabbing);
        t.mv(at + vec2(20.0, 10.0));
        t.button(at + vec2(20.0, 10.0), false, Modifiers::NONE);
        t.h.event(Event::Key { key: egui::Key::Space, physical_key: None, pressed: false, repeat: false, modifiers: Modifiers::NONE });
        t.h.step();
        assert_eq!(cursor_meshes(&t, "pen").len(), 9);
        t.h.event(Event::PointerGone);
        t.h.step();
        assert!(cursor_meshes(&t, "pen").is_empty());
        assert_ne!(t.h.output().platform_output.cursor_icon, CursorIcon::None);
        assert!(t.objects().is_empty());
    }
}
