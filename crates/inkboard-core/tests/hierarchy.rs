#![allow(clippy::float_cmp)]

use inkboard_core::*;

fn shape(e: &mut Editor, x: f64) -> Id {
    e.insert(Object::new_box(BoxGeom::Rect, Frame::new(x, 100.0, 80.0, 40.0), Style::default(), Label::default()))
}

#[test]
fn overlapping_frame_background_does_not_mask_another_frame_border() {
    let mut e = Editor::new(Document::default());
    let inner = e.frame_selection(&[], Rect::new(150.0, 150.0, 200.0, 150.0));
    let outer = e.frame_selection(&[], Rect::new(100.0, 100.0, 500.0, 400.0));
    let index = e.page().pick(Pt::new(150.0, 230.0), 4.0).unwrap();
    assert_eq!(e.page().objects[index].id, inner);
    let index = e.page().pick(Pt::new(500.0, 400.0), 4.0).unwrap();
    assert_eq!(e.page().objects[index].id, outer);
}

#[test]
fn created_and_copied_frames_receive_distinct_incrementing_names() {
    let mut e = Editor::new(Document::default());
    let a = e.frame_selection(&[], Rect::new(0.0, 0.0, 200.0, 150.0));
    e.frame_selection(&[], Rect::new(300.0, 0.0, 200.0, 150.0));
    e.duplicate(&[a], Pt::new(24.0, 24.0));
    let names: Vec<String> = e.page().objects.iter().filter_map(|o| if let Kind::Box(b) = &o.kind { Some(b.label.text.to_plain()) } else { None }).collect();
    assert_eq!(names, ["Frame 1", "Frame 2", "Frame 3"]);
    assert!(e.page().objects.iter().all(|o| o.parent.is_none()), "overlapping copies remain independently selectable siblings");
}

#[test]
fn automatic_creation_uses_inner_frame_and_undo_removes_it_once() {
    let mut e = Editor::new(Document::default());
    let outer = e.frame_selection(&[], Rect::new(0.0, 0.0, 600.0, 500.0));
    let inner = e.frame_selection(&[], Rect::new(100.0, 100.0, 250.0, 250.0));
    assert!(e.reparent(std::slice::from_ref(&inner), Some(&outer)));
    let before = e.doc.clone();
    let id = e.insert_on_canvas(Object::new_box(BoxGeom::Rect, Frame::new(150.0, 150.0, 40.0, 40.0), Style::default(), Label::default()));
    assert_eq!(e.page().get(&id).unwrap().parent.as_ref(), Some(&inner));
    e.undo();
    assert_eq!(e.doc, before);
    e.redo();
    assert_eq!(e.page().get(&id).unwrap().parent.as_ref(), Some(&inner));
    e.set_locked(std::slice::from_ref(&inner), true);
    assert_eq!(e.page().frame_at(Pt::new(170.0, 170.0), &[]), Some(outer));
}

#[test]
fn automatic_membership_uses_rotated_interior_and_preserves_moving_subtree() {
    let mut e = Editor::new(Document::default());
    let outer = e.frame_selection(&[], Rect::new(0.0, 0.0, 600.0, 500.0));
    let inner = e.frame_selection(&[], Rect::new(100.0, 100.0, 200.0, 100.0));
    e.update(&inner, |o| {
        if let Kind::Box(b) = &mut o.kind {
            b.frame.angle = std::f64::consts::FRAC_PI_2;
        }
    });
    assert_eq!(e.page().frame_at(Pt::new(200.0, 220.0), &[]), Some(inner.clone()));
    assert_eq!(e.page().frame_at(Pt::new(110.0, 150.0), &[]), Some(outer.clone()));
    let child = shape(&mut e, 180.0);
    e.reparent(std::slice::from_ref(&child), Some(&inner));
    e.adopt_frames_at_position(std::slice::from_ref(&inner));
    assert_eq!(e.page().get(&inner).unwrap().parent.as_ref(), Some(&outer));
    assert_eq!(e.page().get(&child).unwrap().parent.as_ref(), Some(&inner));
    e.transact(|e| {
        e.translate(std::slice::from_ref(&inner), Pt::new(700.0, 0.0));
        e.adopt_frames_at_position(std::slice::from_ref(&inner));
    });
    assert!(e.page().get(&inner).unwrap().parent.is_none());
    assert_eq!(e.page().get(&child).unwrap().parent.as_ref(), Some(&inner));
    e.undo();
    assert_eq!(e.page().get(&inner).unwrap().parent.as_ref(), Some(&outer));
}

#[test]
fn nested_frames_move_once_and_undo_as_one_action() {
    let mut e = Editor::new(Document::default());
    let a = shape(&mut e, 100.0);
    let b = shape(&mut e, 220.0);
    let inner = e.frame_selection(&[a.clone(), b.clone()], Rect::default());
    let outer = e.frame_selection(std::slice::from_ref(&inner), Rect::default());
    assert_eq!(e.page().get(&inner).unwrap().parent.as_ref(), Some(&outer));
    assert_eq!(e.page().get(&a).unwrap().parent.as_ref(), Some(&inner));
    let before = e.doc.clone();
    e.translate(&[outer.clone(), inner, a.clone()], Pt::new(25.0, 10.0));
    assert_eq!(e.page().get(&a).unwrap().frame().unwrap().x, 125.0);
    assert_eq!(e.page().get(&b).unwrap().frame().unwrap().x, 245.0);
    assert!(e.undo());
    assert_eq!(e.doc, before);
    assert!(e.redo());
    assert_eq!(e.page().expand_frames(&[outer]).len(), 4);
}

#[test]
fn duplicate_remaps_parents_and_child_alone_detaches() {
    let mut e = Editor::new(Document::default());
    let a = shape(&mut e, 10.0);
    let f = e.frame_selection(std::slice::from_ref(&a), Rect::default());
    let ids = e.duplicate(std::slice::from_ref(&f), Pt::new(300.0, 0.0));
    assert_eq!(ids.len(), 2);
    let frame = ids.iter().find(|id| e.page().get(id).unwrap().is_frame).unwrap();
    let child = ids.iter().find(|id| !e.page().get(id).unwrap().is_frame).unwrap();
    assert_eq!(e.page().get(child).unwrap().parent.as_ref(), Some(frame));
    assert_ne!(frame, &f);
    assert_eq!(e.page().get(child).unwrap().frame().unwrap().x, 310.0);
    let isolated = Editor::clone_objects(e.page(), &[a], Pt::ZERO);
    assert!(isolated[0].parent.is_none());
}

#[test]
fn delete_cascades_but_release_preserves_children_and_undo_restores_membership() {
    let mut e = Editor::new(Document::default());
    let a = shape(&mut e, 10.0);
    let f = e.frame_selection(std::slice::from_ref(&a), Rect::default());
    let before = e.doc.clone();
    e.remove(std::slice::from_ref(&f));
    assert!(e.page().objects.is_empty());
    e.undo();
    assert_eq!(e.doc, before);
    e.release_frame(&f);
    assert_eq!(e.page().objects.len(), 1);
    assert!(e.page().get(&a).unwrap().parent.is_none());
    e.undo();
    assert_eq!(e.doc, before);
}

#[test]
fn reparent_rejects_cycles_and_preserves_world_geometry() {
    let mut e = Editor::new(Document::default());
    let a = shape(&mut e, 10.0);
    let inner = e.frame_selection(std::slice::from_ref(&a), Rect::default());
    let outer = e.frame_selection(std::slice::from_ref(&inner), Rect::default());
    let before = e.doc.clone();
    assert!(!e.reparent(std::slice::from_ref(&outer), Some(&inner)));
    assert!(!e.reparent(std::slice::from_ref(&inner), Some(&inner)));
    assert!(!e.reparent(std::slice::from_ref(&inner), Some(&a)));
    assert_eq!(e.doc, before);
    assert!(e.reparent(std::slice::from_ref(&a), Some(&outer)));
    assert_eq!(e.page().get(&a).unwrap().frame().unwrap().x, 10.0);
    e.undo();
    assert_eq!(e.doc, before);
}

#[test]
fn hierarchy_roundtrips_and_old_documents_keep_their_version() {
    let mut e = Editor::new(Document::default());
    let a = shape(&mut e, 10.0);
    let old = serial::to_json(&e.doc, false);
    assert!(old.contains("\"version\":2"));
    assert!(!old.contains("is_frame"));
    assert!(!old.contains("parent"));
    assert_eq!(serial::from_json(&old).unwrap(), e.doc);
    e.frame_selection(&[a], Rect::default());
    let new = serial::to_json(&e.doc, false);
    assert!(new.contains("\"version\":3"));
    assert_eq!(serial::from_json(&new).unwrap(), e.doc);
}

#[test]
fn malformed_parent_graph_is_repaired_and_inherited_locks_work() {
    let mut e = Editor::new(Document::default());
    let a = shape(&mut e, 10.0);
    let f = e.frame_selection(std::slice::from_ref(&a), Rect::default());
    e.set_locked(std::slice::from_ref(&f), true);
    assert!(e.page().is_locked(e.page().get(&a).unwrap()));
    e.set_locked(std::slice::from_ref(&f), false);
    e.update(&a, |o| o.is_frame = true);
    e.update(&f, |o| o.parent = Some(a.clone()));
    let repaired = serial::from_json(&serial::to_json(&e.doc, false)).unwrap();
    assert!(!repaired.pages[0].hierarchy_roots(&[a, f]).is_empty());
    let mut e = Editor::new(repaired);
    let orphan = shape(&mut e, 0.0);
    e.update(&orphan, |o| o.parent = Some(Id::from("missing")));
    serial::sanitize(&mut e.doc);
    assert!(e.page().get(&orphan).unwrap().parent.is_none());
}

#[test]
fn subtree_resize_preserves_offsets_and_is_undoable() {
    let mut e = Editor::new(Document::default());
    let a = shape(&mut e, 100.0);
    let f = e.frame_selection(std::slice::from_ref(&a), Rect::default());
    let ids = e.page().expand_frames(std::slice::from_ref(&f));
    let b = e.page().bounds_of(&ids).unwrap();
    let before = e.doc.clone();
    e.transform_selection(&[f], b, Rect::new(b.x, b.y, b.w * 2.0, b.h * 2.0), 0.0);
    assert_eq!(e.page().get(&a).unwrap().frame().unwrap().w, 160.0);
    e.undo();
    assert_eq!(e.doc, before);
}

#[test]
fn sanitizing_a_tail_into_a_cycle_breaks_the_cycle_itself() {
    let mut page = Page::new("cycle");
    for (id, parent) in [("tail", "a"), ("a", "b"), ("b", "a")] {
        let mut o = Object::new_box(BoxGeom::Rect, Frame::new(0.0, 0.0, 10.0, 10.0), Style::default(), Label::default());
        o.id = Id::from(id);
        o.parent = Some(Id::from(parent));
        o.is_frame = true;
        page.objects.push(o);
    }
    page.sanitize_hierarchy();
    let all: Vec<Id> = page.objects.iter().map(|o| o.id.clone()).collect();
    let roots = page.hierarchy_roots(&all);
    assert_eq!(roots.len(), 1);
    assert_eq!(page.expand_frames(&roots).len(), 3);
}
