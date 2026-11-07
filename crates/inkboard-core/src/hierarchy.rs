//! Explicit frame ownership. Geometry is kept in world space, including for nested frames.

use crate::{BoxGeom, Editor, Frame, Id, Kind, Label, Object, Page, Pt, Rect, RichText, Style};
use std::collections::{HashMap, HashSet, VecDeque};

impl Page {
    /// Allocate readable frame names without changing existing user-written titles.
    #[must_use]
    pub fn next_frame_name(&self, title: &str) -> String {
        let base = title.rsplit_once(' ').filter(|(_, n)| n.parse::<u64>().is_ok()).map_or(title, |(base, _)| base);
        let base = if base.trim().is_empty() { "Frame" } else { base };
        let prefix = format!("{base} ");
        let last = self
            .objects
            .iter()
            .filter(|o| o.is_frame)
            .filter_map(|o| {
                let Kind::Box(b) = &o.kind else { return None };
                b.label.text.to_plain().strip_prefix(&prefix)?.parse::<u64>().ok()
            })
            .max()
            .unwrap_or(0);
        format!("{base} {}", last.saturating_add(1))
    }

    /// Innermost visible, unlocked frame containing a world point. Moving subtrees are excluded.
    #[must_use]
    pub fn frame_at(&self, point: Pt, excluded: &[Id]) -> Option<Id> {
        let excluded: HashSet<&Id> = excluded.iter().collect();
        let mut best: Option<(usize, f64, Id)> = None;
        for o in &self.objects {
            if !o.is_frame || excluded.contains(&o.id) || !self.is_visible(o) || self.is_locked(o) {
                continue;
            }
            let Some(frame) = o.frame() else { continue };
            let local = frame.to_local(point);
            if local.x.abs() > frame.w / 2.0 || local.y.abs() > frame.h / 2.0 {
                continue;
            }
            let mut seen = HashSet::new();
            let mut parent = o.parent.as_ref();
            while let Some(id) = parent {
                if !seen.insert(id) {
                    break;
                }
                parent = self.get(id).and_then(|p| p.parent.as_ref());
            }
            let depth = seen.len();
            let area = frame.w * frame.h;
            if best.as_ref().is_none_or(|(d, a, _)| depth > *d || (depth == *d && area <= *a)) {
                best = Some((depth, area, o.id.clone()));
            }
        }
        best.map(|(_, _, id)| id)
    }

    /// Changes only selected roots; descendants keep their explicit ownership.
    #[must_use]
    pub fn automatic_parent_changes(&self, ids: &[Id]) -> Vec<Object> {
        let excluded = self.expand_frames(ids);
        self.hierarchy_roots(ids)
            .iter()
            .filter_map(|id| {
                let o = self.get(id)?;
                if self.is_locked(o) {
                    return None;
                }
                let mut candidates_excluded = excluded.clone();
                if o.is_frame
                    && let Some(frame) = o.frame()
                {
                    // An overlapping copy must not become a child of its same-sized source.
                    candidates_excluded.extend(
                        self.objects
                            .iter()
                            .filter(|candidate| {
                                candidate.is_frame
                                    && candidate.frame().is_some_and(|f| {
                                        frame.corners().iter().any(|p| {
                                            let local = f.to_local(*p);
                                            local.x.abs() > f.w / 2.0 || local.y.abs() > f.h / 2.0
                                        }) || f.w * f.h <= frame.w * frame.h
                                    })
                            })
                            .map(|candidate| candidate.id.clone()),
                    );
                }
                let parent = self.frame_at(self.bounds(o).center(), &candidates_excluded);
                if o.parent == parent {
                    return None;
                }
                let mut changed = o.clone();
                changed.parent = parent;
                Some(changed)
            })
            .collect()
    }

    /// Includes descendants once, preserving the page's drawing order. Cycles cannot loop forever.
    #[must_use]
    pub fn expand_frames(&self, ids: &[Id]) -> Vec<Id> {
        let mut children: HashMap<&Id, Vec<&Id>> = HashMap::new();
        for o in &self.objects {
            if let Some(p) = &o.parent {
                children.entry(p).or_default().push(&o.id);
            }
        }
        let mut seen: HashSet<&Id> = ids.iter().collect();
        let mut queue: VecDeque<&Id> = ids.iter().collect();
        while let Some(id) = queue.pop_front() {
            for child in children.get(id).into_iter().flatten() {
                if seen.insert(child) {
                    queue.push_back(child);
                }
            }
        }
        self.objects.iter().filter(|o| seen.contains(&o.id)).map(|o| o.id.clone()).collect()
    }

    /// Selected roots; selecting both a container and its child changes the child only once.
    #[must_use]
    pub fn hierarchy_roots(&self, ids: &[Id]) -> Vec<Id> {
        let selected: HashSet<&Id> = ids.iter().collect();
        let map: HashMap<&Id, &Object> = self.objects.iter().map(|o| (&o.id, o)).collect();
        ids.iter()
            .filter(|id| {
                let mut parent = map.get(id).and_then(|o| o.parent.as_ref());
                let mut seen = HashSet::new();
                while let Some(p) = parent {
                    if selected.contains(p) || !seen.insert(p) {
                        return false;
                    }
                    parent = map.get(p).and_then(|o| o.parent.as_ref());
                }
                map.contains_key(id)
            })
            .cloned()
            .collect()
    }

    /// Tests inherited lock/visibility without recursion, even on malformed input.
    pub(crate) fn ancestors_match(&self, o: &Object, test: impl Fn(&Object) -> bool) -> bool {
        if test(o) {
            return true;
        }
        if o.parent.is_none() {
            return false;
        }
        let mut current = o.parent.as_ref();
        let mut seen = HashSet::new();
        while let Some(id) = current {
            if !seen.insert(id) {
                break;
            }
            let Some(parent) = self.get(id) else {
                break;
            };
            if test(parent) {
                return true;
            }
            current = parent.parent.as_ref();
        }
        false
    }

    /// Repairs missing parents, invalid containers and cycles in imported native files.
    pub fn sanitize_hierarchy(&mut self) {
        for o in &mut self.objects {
            if !matches!(&o.kind, Kind::Box(b) if b.geom == BoxGeom::Rect) {
                o.is_frame = false;
            }
        }
        let frames: HashSet<Id> = self.objects.iter().filter(|o| o.is_frame).map(|o| o.id.clone()).collect();
        for o in &mut self.objects {
            if o.parent.as_ref().is_some_and(|p| p == &o.id || !frames.contains(p)) {
                o.parent = None;
            }
        }
        let mut parents: HashMap<Id, Id> = self.objects.iter().filter_map(|o| o.parent.as_ref().map(|p| (o.id.clone(), p.clone()))).collect();
        let mut done = HashSet::new();
        for o in &self.objects {
            let mut path = HashSet::new();
            let mut current = o.id.clone();
            while !done.contains(&current) {
                if !path.insert(current.clone()) {
                    parents.remove(&current);
                    break;
                }
                let Some(p) = parents.get(&current) else {
                    break;
                };
                current = p.clone();
            }
            done.extend(path);
        }
        for o in &mut self.objects {
            o.parent = parents.get(&o.id).cloned();
        }
    }
}

impl Editor {
    pub fn adopt_frames_at_position(&mut self, ids: &[Id]) {
        let changes = self.page().automatic_parent_changes(ids);
        self.transact(|e| {
            for object in changes {
                e.replace(object);
            }
        });
    }

    /// Interactive creation/paste includes automatic ownership in the same undo entry.
    pub fn insert_on_canvas(&mut self, object: Object) -> Id {
        self.insert_all_on_canvas(vec![object]).remove(0)
    }

    pub fn insert_all_on_canvas(&mut self, objects: Vec<Object>) -> Vec<Id> {
        self.transact(|e| {
            let ids = e.insert_all(objects);
            e.adopt_frames_at_position(&ids);
            ids
        })
    }

    /// Creates a frame around selected roots, or an empty frame at `empty`.
    pub fn frame_selection(&mut self, ids: &[Id], empty: Rect) -> Id {
        let ids = self.page().expand_groups(ids);
        let roots = self.page().hierarchy_roots(&ids);
        let bounds = self.page().bounds_of(&ids).map_or(empty, |b| Rect::new(b.x - 24.0, b.y - 52.0, b.w + 48.0, b.h + 76.0));
        let parents: Vec<Option<Id>> = roots.iter().filter_map(|id| self.page().get(id).map(|o| o.parent.clone())).collect();
        let parent = parents.first().filter(|p| parents.iter().all(|x| x == *p)).cloned().flatten();
        let label =
            Label { text: RichText::plain(self.page().next_frame_name("Frame")), align: crate::Align::Left, valign: crate::VAlign::Top, ..Label::default() };
        let mut obj = Object::new_box(BoxGeom::Rect, Frame::from_rect(bounds), Style { fill: None, ..Style::default() }, label);
        obj.is_frame = true;
        obj.parent = parent;
        if let Some(o) = roots.first().and_then(|id| self.page().get(id)) {
            obj.layer.clone_from(&o.layer);
        }
        self.transact(|e| {
            let id = e.insert(obj);
            e.update_many(&roots, |o| {
                o.parent = Some(id.clone());
            });
            // Insert directly behind the enclosed subtree, without placing a nested frame below its parent.
            let mut order: Vec<Id> = e.page().objects.iter().filter(|o| o.id != id).map(|o| o.id.clone()).collect();
            let at = order.iter().position(|x| ids.contains(x)).unwrap_or(order.len());
            order.insert(at, id.clone());
            e.set_object_order(order);
            id
        })
    }

    /// Reparents selected roots. Rejects cycles, locked contents and non-frame parents.
    pub fn reparent(&mut self, ids: &[Id], parent: Option<&Id>) -> bool {
        let ids = self.page().expand_groups(ids);
        if ids.is_empty() || ids.iter().any(|id| self.page().get(id).is_some_and(|o| self.page().is_locked(o))) {
            return false;
        }
        if let Some(p) = parent
            && (ids.contains(p) || !self.page().get(p).is_some_and(|o| o.is_frame && !self.page().is_locked(o)))
        {
            return false;
        }
        let roots = self.page().hierarchy_roots(&ids);
        self.update_many(&roots, |o| o.parent = parent.cloned());
        true
    }

    /// Removes only the container; children retain their geometry and join its former parent.
    pub fn release_frame(&mut self, id: &Id) {
        let Some(frame) = self.page().get(id).filter(|o| o.is_frame && !self.page().is_locked(o)).cloned() else {
            return;
        };
        let children: Vec<Id> = self.page().objects.iter().filter(|o| o.parent.as_ref() == Some(id)).map(|o| o.id.clone()).collect();
        self.transact(|e| {
            e.update_many(&children, |o| o.parent.clone_from(&frame.parent));
            e.remove(std::slice::from_ref(id));
        });
    }

    /// Applies a bounding-box transform to a selection/subtree as one history entry.
    pub fn transform_selection(&mut self, ids: &[Id], before: Rect, after: Rect, radians: f64) {
        if ![after.x, after.y, after.w, after.h, radians].iter().all(|x| x.is_finite()) || after.w <= 0.0 || after.h <= 0.0 {
            return;
        }
        let ids = self.page().expand_frames(ids);
        if ids.iter().any(|id| self.page().get(id).is_some_and(|o| self.page().is_locked(o))) {
            return;
        }
        self.update_many(&ids, |o| {
            o.scale_about(Pt::new(before.x, before.y), after.w / before.w.max(0.001), after.h / before.h.max(0.001));
            o.translate(Pt::new(after.x - before.x, after.y - before.y));
            if radians != 0.0 {
                o.rotate_about(after.center(), radians);
            }
        });
    }
}
