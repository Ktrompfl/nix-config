#![forbid(unsafe_code)]

use geom::{Color, Rect};
use std::{collections::HashMap, rc::Rc};
use wm::{
    Arrangement, Cause, Cx, Deco, Dir, Hit, Output, OutputId, Placed, WindowId, WindowInfo, WindowManager,
    WorkspaceInfo,
};

pub struct Theme {
    pub background: Color,
    pub border: Color,
    pub border_focused: Color,
    pub title: Color,
    pub title_focused: Color,
    pub title_text: Color,
    pub title_text_focused: Color,
    pub font: Rc<str>,
    pub border_width: i32,
    pub title_height: i32,
    pub show_titles: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    Horizontal,
    Vertical,
    Tabbed,
}

enum Node {
    Window(WindowId),
    Split(Split),
}

struct Split {
    layout: Layout,
    kids: Vec<Node>,
    weights: Vec<f64>,
    focus: usize,
}

impl Split {
    fn new(layout: Layout, kids: Vec<Node>) -> Self {
        let weights = vec![1.0; kids.len()];
        Self { layout, kids, weights, focus: 0 }
    }

    fn insert(&mut self, at: usize, node: Node) {
        let weight =
            if self.weights.is_empty() { 1.0 } else { self.weights.iter().sum::<f64>() / self.weights.len() as f64 };
        self.kids.insert(at, node);
        self.weights.insert(at, weight);
    }

    fn remove(&mut self, i: usize) {
        self.kids.remove(i);
        self.weights.remove(i);
    }

    fn focus_path(&self) -> Vec<usize> {
        let mut path = vec![];
        let mut split = self;
        while let Some(node) = split.kids.get(split.focus) {
            path.push(split.focus);
            match node {
                Node::Split(s) => split = s,
                Node::Window(_) => break,
            }
        }
        path
    }

    fn find(&self, w: WindowId) -> Option<Vec<usize>> {
        self.kids.iter().enumerate().find_map(|(i, node)| match node {
            Node::Window(x) if *x == w => Some(vec![i]),
            Node::Split(s) => s.find(w).map(|p| [vec![i], p].concat()),
            Node::Window(_) => None,
        })
    }

    fn get(&self, path: &[usize]) -> Option<&Node> {
        let (first, rest) = path.split_first()?;
        match (self.kids.get(*first)?, rest.is_empty()) {
            (node, true) => Some(node),
            (Node::Split(s), false) => s.get(rest),
            (Node::Window(_), false) => None,
        }
    }

    fn split(&mut self, path: &[usize]) -> &mut Split {
        path.iter().fold(self, |s, &i| match &mut s.kids[i] {
            Node::Split(child) => child,
            Node::Window(_) => unreachable!("path leads through a window"),
        })
    }

    fn node(&mut self, path: &[usize]) -> &mut Node {
        let (last, parent) = path.split_last().unwrap();
        &mut self.split(parent).kids[*last]
    }

    fn parent_of_focus(&self) -> Vec<usize> {
        let mut path = self.focus_path();
        if let Some(Node::Window(_)) = self.get(&path) {
            path.pop();
        }
        path
    }

    fn child_rect(&self, r: Rect, i: usize, tab_height: i32) -> Rect {
        match self.layout {
            Layout::Tabbed => Rect { y: r.y + tab_height, h: (r.h - tab_height).max(1), ..r },
            layout => wm::split(r, &self.weights, i, layout == Layout::Horizontal),
        }
    }

    fn focused_window(&self) -> Option<WindowId> {
        match self.kids.get(self.focus)? {
            Node::Window(w) => Some(*w),
            Node::Split(s) => s.focused_window(),
        }
    }
}

fn toggle(layout: Layout, a: Layout, b: Layout) -> Layout {
    if layout == a { b } else { a }
}

pub struct TreeWm {
    theme: Theme,
    outputs: Vec<Output>,
    shown: HashMap<OutputId, usize>,
    homes: Vec<Option<OutputId>>,
    workspaces: Vec<Split>,
    fullscreen: Vec<Option<WindowId>>,
    titles: HashMap<WindowId, Rc<str>>,
    history: Vec<WindowId>,
    urgent: Vec<WindowId>,
    lift: usize,
    active: usize,
    warps: u64,
}

impl TreeWm {
    pub fn new(theme: Theme) -> Self {
        let workspaces = (0..10).map(|_| Split::new(Layout::Horizontal, vec![])).collect();
        let (titles, history, shown) = (HashMap::new(), vec![], HashMap::new());
        let (outputs, homes, fullscreen) = (vec![], vec![None; 10], vec![None; 10]);
        let (lift, active, urgent, warps) = (0, 1, vec![], 0);
        Self { theme, outputs, shown, homes, workspaces, fullscreen, titles, history, urgent, lift, active, warps }
    }

    fn root(&mut self) -> &mut Split {
        &mut self.workspaces[self.active]
    }

    pub fn focused(&self) -> Option<WindowId> {
        self.fullscreen[self.active].or_else(|| self.workspaces[self.active].focused_window())
    }

    fn selected(&self) -> Vec<usize> {
        let mut path = self.workspaces[self.active].focus_path();
        path.truncate(path.len().saturating_sub(self.lift).max(1));
        path
    }

    pub fn warp_pointer(&mut self) {
        self.warps += 1;
    }

    pub fn focus_dir(&mut self, dir: Dir) {
        self.warp_pointer();
        match self.neighbor(dir) {
            _ if self.cycle_tab(dir) => {}
            Some(n) => self.focus_window(n),
            None => self.focus_output(dir),
        }
    }

    pub fn focus_parent(&mut self) {
        let depth = self.workspaces[self.active].focus_path().len();
        self.lift = (self.lift + 1).min(depth.saturating_sub(1));
        self.warp_pointer();
    }

    pub fn focus_previous(&mut self) {
        self.warp_pointer();
        if let Some(&w) = self.history.iter().rev().nth(1) {
            self.focus_window(w);
        }
    }

    pub fn move_dir(&mut self, dir: Dir) {
        self.warp_pointer();
        let Some(cur) = self.focused() else { return };
        let other = self.neighbor(dir);
        let root = self.root();
        let Some((a, b, other)) = other.and_then(|other| Some((root.find(cur)?, root.find(other)?, other))) else {
            return self.move_to_output(dir);
        };
        *root.node(&a) = Node::Window(other);
        *root.node(&b) = Node::Window(cur);
        self.focus_window(cur);
    }

    fn selected_parent(&mut self) -> &mut Split {
        let mut path = self.selected();
        path.pop();
        self.root().split(&path)
    }

    fn toggle_layout(&mut self, a: Layout, b: Layout) {
        let split = self.selected_parent();
        split.layout = toggle(split.layout, a, b);
    }

    pub fn toggle_split(&mut self) {
        self.toggle_layout(Layout::Horizontal, Layout::Vertical);
    }

    pub fn toggle_tabbed(&mut self) {
        self.toggle_layout(Layout::Tabbed, Layout::Horizontal);
    }

    pub fn split(&mut self) {
        let path = self.selected();
        let layout = toggle(self.selected_parent().layout, Layout::Horizontal, Layout::Vertical);
        let root = self.root();
        if root.get(&path).is_some() {
            let node = std::mem::replace(root.node(&path), Node::Split(Split::new(layout, vec![])));
            *root.node(&path) = Node::Split(Split::new(layout, vec![node]));
        }
        self.lift = 0;
    }

    pub fn resize(&mut self, dir: Dir, amount: i32) {
        let (horizontal, forward) = (matches!(dir, Dir::Left | Dir::Right), matches!(dir, Dir::Right | Dir::Down));
        let wanted = if horizontal { Layout::Horizontal } else { Layout::Vertical };
        let path = self.selected();
        for depth in (0..path.len()).rev() {
            let (parent, i) = (&path[..depth], path[depth]);
            let rect = self.rect_of(parent);
            let split = self.workspaces[self.active].split(parent);
            let j = if forward { i + 1 } else { i.wrapping_sub(1) };
            if split.layout != wanted || j >= split.kids.len() {
                continue;
            }
            let total: f64 = split.weights.iter().sum();
            let delta = f64::from(amount) * total / f64::from(if horizontal { rect.w } else { rect.h }.max(1));
            let min = total * 0.05;
            if split.weights[i] + split.weights[j] <= 2.0 * min {
                return;
            }
            let delta = delta.max(min - split.weights[i]).min(split.weights[j] - min);
            split.weights[i] += delta;
            split.weights[j] -= delta;
            return;
        }
    }

    pub fn toggle_fullscreen(&mut self) {
        let current = self.fullscreen[self.active];
        self.fullscreen[self.active] = match current {
            Some(_) => None,
            None => self.focused(),
        };
    }

    pub fn show(&mut self, workspace: usize) {
        self.warp_pointer();
        self.show_workspace(workspace);
    }

    fn show_workspace(&mut self, workspace: usize) {
        self.lift = 0;
        if self.output_showing(workspace).is_none() {
            let home = self.homes[workspace].filter(|&o| self.output(o).is_some());
            if let Some(output) = home.or_else(|| self.focused_output()) {
                self.shown.insert(output, workspace);
                self.homes[workspace] = Some(output);
            }
        }
        self.active = workspace;
        self.note_focus();
    }

    pub fn focus_output(&mut self, dir: Dir) {
        self.warp_pointer();
        if let Some(o) = self.output_towards(dir) {
            self.activate_output(o);
        }
    }

    pub fn activate_output(&mut self, o: OutputId) {
        if let Some(&workspace) = self.shown.get(&o) {
            self.active = workspace;
            self.lift = 0;
            self.note_focus();
        }
    }

    pub fn move_to_output(&mut self, dir: Dir) {
        self.warp_pointer();
        let target = self.output_towards(dir).and_then(|o| self.shown.get(&o).copied());
        if let (Some(w), Some(workspace)) = (self.focused(), target) {
            self.remove(w);
            self.insert(workspace, w);
            self.active = workspace;
        }
    }

    fn output(&self, id: OutputId) -> Option<&Output> {
        self.outputs.iter().find(|o| o.id == id)
    }

    fn output_showing(&self, workspace: usize) -> Option<OutputId> {
        self.shown.iter().find(|(_, w)| **w == workspace).map(|(o, _)| *o)
    }

    fn focused_output(&self) -> Option<OutputId> {
        self.output_showing(self.active).or_else(|| self.outputs.first().map(|o| o.id))
    }

    fn output_towards(&self, dir: Dir) -> Option<OutputId> {
        let from = self.output(self.focused_output()?)?.full;
        wm::neighbor(from, dir, self.outputs.iter().map(|o| (o.id, o.full)))
    }

    fn usable(&self, workspace: usize) -> Rect {
        self.output_showing(workspace).and_then(|o| self.output(o)).map_or_else(Rect::default, |o| o.usable)
    }

    fn assign_outputs(&mut self) {
        let present = |o: &OutputId, outputs: &[Output]| outputs.iter().any(|x| x.id == *o);
        self.shown.retain(|o, _| present(o, &self.outputs));
        for home in &mut self.homes {
            *home = home.filter(|o| present(o, &self.outputs));
        }
        for output in self.outputs.clone() {
            for &workspace in &output.workspaces {
                if let Some(home) = self.homes.get_mut(workspace) {
                    home.get_or_insert(output.id);
                }
            }
            if self.shown.contains_key(&output.id) {
                continue;
            }
            let free = |w: &usize| !self.shown.values().any(|s| s == w);
            let configured = output.workspaces.iter().copied().filter(|w| *w < self.homes.len()).find(free);
            let homed = (0..self.homes.len()).filter(free).find(|&w| self.homes[w] == Some(output.id));
            let empty = (1..10).chain([0]).filter(free).find(|&w| self.workspaces[w].kids.is_empty());
            if let Some(workspace) = configured.or(homed).or(empty) {
                self.shown.insert(output.id, workspace);
                self.homes[workspace] = Some(output.id);
            }
        }
        if self.output_showing(self.active).is_none()
            && let Some(&workspace) = self.outputs.first().and_then(|o| self.shown.get(&o.id))
        {
            self.active = workspace;
        }
    }

    pub fn move_to(&mut self, workspace: usize) {
        self.warp_pointer();
        if let Some(w) = self.focused().filter(|_| workspace != self.active) {
            self.remove(w);
            self.insert(workspace, w);
        }
    }

    fn rect_of(&self, path: &[usize]) -> Rect {
        let mut split = &self.workspaces[self.active];
        let mut rect = self.usable(self.active);
        for &i in path {
            rect = split.child_rect(rect, i, self.theme.title_height);
            match &split.kids[i] {
                Node::Split(s) => split = s,
                Node::Window(_) => break,
            }
        }
        rect
    }

    fn cycle_tab(&mut self, dir: Dir) -> bool {
        let root = self.root();
        let parent = root.parent_of_focus();
        let split = root.split(&parent);
        let target = match dir {
            Dir::Left => split.focus.checked_sub(1),
            Dir::Right => Some(split.focus + 1).filter(|&i| i < split.kids.len()),
            _ => None,
        };
        match target.filter(|_| split.layout == Layout::Tabbed) {
            Some(i) => split.focus = i,
            None => return false,
        }
        self.note_focus();
        true
    }

    fn neighbor(&self, dir: Dir) -> Option<WindowId> {
        let current = self.focused()?;
        let mut arrangement = Arrangement::default();
        self.arrange(&mut arrangement);
        let placed = |w| arrangement.windows.iter().find(|p| p.window == w).map(|p| p.rect);
        wm::neighbor(placed(current)?, dir, arrangement.windows.iter().map(|p| (p.window, p.rect)))
    }

    fn insert(&mut self, workspace: usize, w: WindowId) {
        let root = &mut self.workspaces[workspace];
        let path = root.focus_path();
        let at = match root.get(&path) {
            Some(Node::Window(_)) => path.last().unwrap() + 1,
            _ => 0,
        };
        let split = root.split(&root.parent_of_focus());
        let at = at.min(split.kids.len());
        split.insert(at, Node::Window(w));
        split.focus = at;
        self.lift = 0;
        self.note_focus();
    }

    fn remove(&mut self, w: WindowId) {
        self.history.retain(|&h| h != w);
        for fullscreen in &mut self.fullscreen {
            if *fullscreen == Some(w) {
                *fullscreen = None;
            }
        }
        for root in &mut self.workspaces {
            let Some(mut path) = root.find(w) else { continue };
            while let Some(i) = path.pop() {
                let split = root.split(&path);
                split.remove(i);
                if split.focus > i || split.focus >= split.kids.len() {
                    split.focus = split.focus.saturating_sub(1);
                }
                if !split.kids.is_empty() {
                    break;
                }
            }
            self.lift = 0;
            return;
        }
    }

    fn select(&mut self, w: WindowId) -> Option<usize> {
        self.workspaces.iter_mut().enumerate().find_map(|(i, root)| {
            let path = root.find(w)?;
            for depth in 0..path.len() {
                root.split(&path[..depth]).focus = path[depth];
            }
            Some(i)
        })
    }

    fn focus_window(&mut self, w: WindowId) {
        if let Some(i) = self.select(w) {
            self.show_workspace(i);
            self.note_focus();
        }
    }

    fn note_focus(&mut self) {
        if let Some(w) = self.focused() {
            self.history.retain(|&h| h != w);
            self.history.push(w);
            self.urgent.retain(|&u| u != w);
        }
    }

    fn workspace_of(&self, w: WindowId) -> Option<usize> {
        self.workspaces.iter().position(|root| root.find(w).is_some())
    }

    fn empty(&self, workspace: usize) -> bool {
        self.workspaces[workspace].kids.is_empty() && self.fullscreen[workspace].is_none()
    }

    fn pointer_rect(&self, out: &Arrangement) -> Option<Rect> {
        let output = self.output(self.output_showing(self.active)?)?;
        if self.fullscreen[self.active].is_some() {
            return Some(output.full);
        }
        if self.lift > 0 {
            return Some(self.rect_of(&self.selected()));
        }
        let focus = out.focus.and_then(|w| out.windows.iter().find(|p| p.window == w));
        Some(focus.map_or(output.usable, |p| p.rect))
    }

    fn title(&self, w: WindowId) -> Rc<str> {
        self.titles.get(&w).cloned().unwrap_or_else(|| "".into())
    }

    fn text(&self, rect: Rect, text: Rc<str>, focused: bool) -> Deco {
        let t = &self.theme;
        let (fg, bg) = if focused { (t.title_text_focused, t.title_focused) } else { (t.title_text, t.title) };
        Deco::Text { rect, text, font: t.font.clone(), fg, bg }
    }

    fn layout_split(&self, s: &Split, r: Rect, out: &mut Arrangement) {
        if s.layout != Layout::Tabbed {
            for (i, kid) in s.kids.iter().enumerate() {
                self.layout_node(kid, s.child_rect(r, i, 0), true, out);
            }
            return;
        }
        let Some(kid) = s.kids.get(s.focus) else { return };
        let bar = Rect { h: self.theme.title_height, ..r };
        let tabs = vec![1.0; s.kids.len()];
        for (i, node) in s.kids.iter().enumerate() {
            let window = match node {
                Node::Window(w) => Some(*w),
                Node::Split(s) => s.focused_window(),
            };
            let title = window.map(|w| self.title(w)).unwrap_or_default();
            let focused = i == s.focus && window.is_some_and(|w| out.focus == Some(w));
            out.decorations.push(self.text(wm::split(bar, &tabs, i, true), title, focused));
        }
        self.layout_node(kid, s.child_rect(r, s.focus, bar.h), false, out);
    }

    fn layout_node(&self, node: &Node, r: Rect, titled: bool, out: &mut Arrangement) {
        let Node::Window(w) = node else {
            let Node::Split(s) = node else { unreachable!() };
            return self.layout_split(s, r, out);
        };
        let (t, activated) = (&self.theme, out.focus == Some(*w));
        let color = if activated { t.border_focused } else { t.border };
        out.decorations.extend(edges(r, t.border_width).map(|edge| Deco::Fill(edge, color)));
        let mut inner = r.inset(t.border_width);
        if titled && t.show_titles {
            out.decorations.push(self.text(Rect { h: t.title_height, ..inner }, self.title(*w), activated));
            inner = Rect { y: inner.y + t.title_height, h: inner.h - t.title_height, ..inner };
        }
        let rect = Rect { w: inner.w.max(1), h: inner.h.max(1), ..inner };
        out.windows.push(Placed { window: *w, rect, activated, fullscreen: false });
    }

    fn outline_selection(&self, out: &mut Arrangement) {
        if self.lift == 0 {
            return;
        }
        let (r, width, color) = (self.rect_of(&self.selected()), self.theme.border_width, self.theme.border_focused);
        out.decorations.extend(edges(r, width).map(|edge| Deco::Fill(edge, color)));
    }
}

impl WindowManager for TreeWm {
    fn outputs_changed(&mut self, outputs: &[Output]) {
        self.outputs = outputs.to_vec();
        self.assign_outputs();
    }

    fn window_added(&mut self, w: WindowId, info: &WindowInfo) {
        self.titles.insert(w, info.title.as_str().into());
        self.insert(self.active, w);
    }

    fn window_changed(&mut self, w: WindowId, info: &WindowInfo) -> bool {
        let changed = self.titles.get(&w).is_none_or(|t| **t != *info.title);
        if changed {
            self.titles.insert(w, info.title.as_str().into());
        }
        changed
    }

    fn window_removed(&mut self, w: WindowId) {
        self.titles.remove(&w);
        self.urgent.retain(|&u| u != w);
        self.remove(w);
    }

    fn focus(&mut self, w: WindowId, cause: Cause) {
        let visible = self.workspace_of(w).is_some_and(|i| self.output_showing(i).is_some());
        match cause {
            Cause::Pointer | Cause::Keyboard | Cause::User => self.focus_window(w),
            Cause::Activation if visible => self.focus_window(w),
            _ if self.focused() != Some(w) && !self.urgent.contains(&w) => self.urgent.push(w),
            _ => {}
        }
    }

    fn fullscreen(&mut self, w: WindowId, on: bool, output: Option<OutputId>) {
        let Some(mut i) = self.workspace_of(w) else { return };
        let target = output.and_then(|o| self.shown.get(&o).copied()).filter(|&t| on && t != i);
        if let Some(target) = target {
            let focused = self.focused() == Some(w);
            self.remove(w);
            self.insert(target, w);
            if focused {
                self.active = target;
            }
            i = target;
        }
        match on {
            true => {
                self.fullscreen[i] = Some(w);
                if i == self.active {
                    self.select(w);
                    self.note_focus();
                }
            }
            false if self.fullscreen[i] == Some(w) => self.fullscreen[i] = None,
            false => {}
        }
    }

    fn pointer(&mut self, at: Hit, _: &mut Cx) -> bool {
        match at {
            Hit::Window(w) if self.focused() != Some(w) => self.focus_window(w),
            Hit::Output(o)
                if self.focused_output() != Some(o) && self.shown.get(&o).is_some_and(|&w| self.empty(w)) =>
            {
                self.activate_output(o)
            }
            _ => return false,
        }
        true
    }

    fn button(&mut self, at: Hit, _: u32, pressed: bool, cx: &mut Cx) -> bool {
        if pressed && let Hit::Window(_) = at {
            self.pointer(at, cx);
        }
        false
    }

    fn workspaces(&self) -> Vec<WorkspaceInfo> {
        let shown = |i: usize| self.output_showing(i).is_some();
        let used = |i: usize| shown(i) || !self.workspaces[i].kids.is_empty();
        let output = |i: usize| self.output_showing(i).or(self.homes[i]).or_else(|| self.focused_output());
        let urgent = |i: usize| self.urgent.iter().any(|&w| self.workspaces[i].find(w).is_some());
        let workspace =
            |i: usize| WorkspaceInfo { name: i.to_string(), output: output(i), active: shown(i), urgent: urgent(i) };
        (1..10).chain([0]).filter(|&i| used(i)).map(workspace).collect()
    }

    fn activate_workspace(&mut self, name: &str) {
        if let Some(i) = name.parse().ok().filter(|&i: &usize| i < self.workspaces.len()) {
            self.show_workspace(i);
        }
    }

    fn arrange(&self, out: &mut Arrangement) {
        out.background = self.theme.background;
        out.focus = self.focused();
        for output in &self.outputs {
            let Some(&workspace) = self.shown.get(&output.id) else { continue };
            match self.fullscreen[workspace] {
                Some(window) => {
                    let activated = out.focus == Some(window);
                    out.windows.push(Placed { window, rect: output.full, activated, fullscreen: true })
                }
                None => self.layout_split(&self.workspaces[workspace], output.usable, out),
            }
        }
        self.outline_selection(out);
        out.pointer = self.pointer_rect(out);
        out.warps = self.warps;
    }
}

fn edges(r: Rect, width: i32) -> [Rect; 4] {
    [
        Rect { h: width, ..r },
        Rect { y: r.bottom() - width, h: width, ..r },
        Rect { w: width, ..r },
        Rect { x: r.right() - width, w: width, ..r },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wm() -> TreeWm {
        let theme = Theme {
            background: 0,
            border: 1,
            border_focused: 2,
            title: 3,
            title_focused: 4,
            title_text: 5,
            title_text_focused: 6,
            font: "sans 9".into(),
            border_width: 1,
            title_height: 4,
            show_titles: false,
        };
        let mut wm = TreeWm::new(theme);
        let area = Rect::new(0, 0, 100, 100);
        wm.outputs_changed(&[Output { id: 1, full: area, usable: area, workspaces: vec![1] }]);
        wm
    }

    fn placed(wm: &TreeWm) -> Vec<(u64, Rect)> {
        let mut a = Arrangement::default();
        wm.arrange(&mut a);
        a.windows.iter().map(|p| (p.window.0, p.rect)).collect()
    }

    fn add(wm: &mut TreeWm, ids: &[u64]) {
        for &id in ids {
            wm.window_added(WindowId(id), &WindowInfo::default());
        }
    }

    #[test]
    fn tiles_and_focuses_new_windows() {
        let mut wm = wm();
        add(&mut wm, &[1, 2]);
        assert_eq!(placed(&wm), [(1, Rect::new(1, 1, 48, 98)), (2, Rect::new(51, 1, 48, 98))]);
        assert_eq!(wm.focused(), Some(WindowId(2)));
        wm.focus_dir(Dir::Left);
        assert_eq!(wm.focused(), Some(WindowId(1)));
    }

    #[test]
    fn removing_last_window_of_nested_split_prunes_it() {
        let mut wm = wm();
        add(&mut wm, &[1]);
        wm.split();
        add(&mut wm, &[2]);
        wm.window_removed(WindowId(2));
        wm.window_removed(WindowId(1));
        assert!(placed(&wm).is_empty());
        add(&mut wm, &[3]);
        assert_eq!(placed(&wm), [(3, Rect::new(1, 1, 98, 98))]);
    }

    #[test]
    fn tabbed_shows_only_focused_and_cycles() {
        let mut wm = wm();
        add(&mut wm, &[1, 2]);
        wm.toggle_tabbed();
        assert_eq!(placed(&wm), [(2, Rect::new(1, 5, 98, 94))]);
        wm.focus_dir(Dir::Left);
        assert_eq!(placed(&wm), [(1, Rect::new(1, 5, 98, 94))]);
    }

    fn tab_colors(wm: &TreeWm) -> Vec<Color> {
        let mut a = Arrangement::default();
        wm.arrange(&mut a);
        a.decorations.iter().filter_map(|d| if let Deco::Text { bg, .. } = d { Some(*bg) } else { None }).collect()
    }

    #[test]
    fn only_tab_holding_focus_uses_focused_colors() {
        let mut wm = wm();
        add(&mut wm, &[1, 2]);
        wm.split();
        add(&mut wm, &[3]);
        wm.toggle_tabbed();
        assert_eq!(tab_colors(&wm), [3, 4]);
        wm.focus(WindowId(1), Cause::Keyboard);
        assert_eq!(tab_colors(&wm), [3, 3]);
    }

    #[test]
    fn fullscreen_covers_output_and_clears_on_close() {
        let mut wm = wm();
        add(&mut wm, &[1, 2]);
        wm.fullscreen(WindowId(1), true, None);
        assert_eq!(placed(&wm), [(1, Rect::new(0, 0, 100, 100))]);
        wm.window_removed(WindowId(1));
        assert_eq!(placed(&wm), [(2, Rect::new(1, 1, 98, 98))]);
    }

    #[test]
    fn move_to_workspace_hides_window() {
        let mut wm = wm();
        add(&mut wm, &[1]);
        wm.move_to(2);
        assert!(placed(&wm).is_empty());
        wm.show(2);
        assert_eq!(placed(&wm).len(), 1);
    }

    #[test]
    fn outputs_show_their_workspaces_and_windows_move_between_them() {
        let mut wm = wm();
        let rect = Rect::new(0, 0, 100, 100);
        let a = Output { id: 1, full: rect, usable: rect, workspaces: vec![1] };
        let rect = Rect::new(100, 0, 50, 100);
        let b = Output { id: 2, full: rect, usable: rect, workspaces: vec![6] };
        wm.outputs_changed(&[a.clone(), b]);
        add(&mut wm, &[1]);
        wm.show(6);
        add(&mut wm, &[2]);
        assert_eq!(placed(&wm), [(1, Rect::new(1, 1, 98, 98)), (2, Rect::new(101, 1, 48, 98))]);
        wm.move_to_output(Dir::Left);
        assert_eq!(placed(&wm), [(1, Rect::new(1, 1, 48, 98)), (2, Rect::new(51, 1, 48, 98))]);
        wm.move_dir(Dir::Left);
        assert_eq!(placed(&wm), [(2, Rect::new(1, 1, 48, 98)), (1, Rect::new(51, 1, 48, 98))]);
        wm.move_dir(Dir::Right);
        assert_eq!(placed(&wm), [(1, Rect::new(1, 1, 48, 98)), (2, Rect::new(51, 1, 48, 98))]);
        wm.move_dir(Dir::Right);
        assert_eq!(placed(&wm), [(1, Rect::new(1, 1, 98, 98)), (2, Rect::new(101, 1, 48, 98))]);
        wm.outputs_changed(&[a]);
        wm.show(6);
        assert_eq!(placed(&wm), [(2, Rect::new(1, 1, 98, 98))]);
    }

    #[test]
    fn focus_reaches_empty_outputs() {
        let mut wm = wm();
        let rect = Rect::new(0, 0, 100, 100);
        let a = Output { id: 1, full: rect, usable: rect, workspaces: vec![1] };
        let rect = Rect::new(100, 0, 50, 100);
        let b = Output { id: 2, full: rect, usable: rect, workspaces: vec![6] };
        wm.outputs_changed(&[a, b]);
        add(&mut wm, &[1]);
        wm.focus_dir(Dir::Right);
        assert_eq!(wm.focused(), None);
        add(&mut wm, &[2]);
        assert_eq!(placed(&wm)[1], (2, Rect::new(101, 1, 48, 98)));
        wm.activate_output(1);
        assert_eq!(wm.focused(), Some(WindowId(1)));
    }

    #[test]
    fn resize_moves_the_shared_edge() {
        let mut wm = wm();
        add(&mut wm, &[1, 2]);
        wm.focus_dir(Dir::Left);
        wm.resize(Dir::Right, 20);
        assert_eq!(placed(&wm), [(1, Rect::new(1, 1, 68, 98)), (2, Rect::new(71, 1, 28, 98))]);
        wm.resize(Dir::Left, 20);
        assert_eq!(placed(&wm)[0].1, Rect::new(1, 1, 68, 98));
    }

    #[test]
    fn focus_parent_applies_layout_to_container() {
        let mut wm = wm();
        add(&mut wm, &[1]);
        wm.split();
        add(&mut wm, &[2]);
        wm.focus_parent();
        wm.split();
        add(&mut wm, &[3]);
        assert_eq!(placed(&wm).len(), 3);
        wm.focus_previous();
        assert_eq!(wm.focused(), Some(WindowId(2)));
    }

    fn pointer(wm: &TreeWm) -> Option<Rect> {
        let mut a = Arrangement::default();
        wm.arrange(&mut a);
        a.pointer
    }

    fn two_outputs(wm: &mut TreeWm) {
        let a =
            Output { id: 1, full: Rect::new(0, 0, 100, 100), usable: Rect::new(0, 10, 100, 90), workspaces: vec![1] };
        let b =
            Output { id: 2, full: Rect::new(100, 0, 50, 100), usable: Rect::new(100, 0, 50, 100), workspaces: vec![6] };
        wm.outputs_changed(&[a, b]);
    }

    #[test]
    fn resize_never_panics_on_tight_weights() {
        let mut wm = wm();
        add(&mut wm, &[1, 2, 3]);
        for _ in 0..20 {
            wm.resize(Dir::Left, 50);
        }
        add(&mut wm, &[4]);
        for dir in [Dir::Left, Dir::Right, Dir::Left] {
            wm.focus_dir(Dir::Left);
            wm.resize(dir, 50);
        }
        let mut wm = super::tests::wm();
        add(&mut wm, &(1..=21).collect::<Vec<_>>());
        wm.resize(Dir::Left, 10);
        wm.resize(Dir::Right, -10);
        assert_eq!(placed(&wm).len(), 21);
        assert!(placed(&wm).iter().all(|p| p.1.w >= 1 && p.1.h >= 1));
    }

    #[test]
    fn pointer_rect_follows_focused_node() {
        let mut wm = wm();
        two_outputs(&mut wm);
        assert_eq!(pointer(&wm), Some(Rect::new(0, 10, 100, 90)));
        add(&mut wm, &[1]);
        wm.split();
        add(&mut wm, &[2]);
        assert_eq!(pointer(&wm), Some(placed(&wm)[1].1));
        wm.focus_parent();
        assert_eq!(pointer(&wm), Some(Rect::new(0, 10, 100, 90)));
        wm.focus_output(Dir::Right);
        assert_eq!(pointer(&wm), Some(Rect::new(100, 0, 50, 100)));
        wm.focus_output(Dir::Left);
        wm.fullscreen(WindowId(1), true, None);
        assert_eq!(pointer(&wm), Some(Rect::new(0, 0, 100, 100)));
    }

    fn warps(wm: &TreeWm) -> u64 {
        let mut a = Arrangement::default();
        wm.arrange(&mut a);
        a.warps
    }

    #[test]
    fn only_commands_request_explicit_warps() {
        let mut wm = wm();
        two_outputs(&mut wm);
        let quiet = |wm: &mut TreeWm, f: &dyn Fn(&mut TreeWm)| {
            let before = warps(wm);
            f(wm);
            assert_eq!(warps(wm), before);
        };
        quiet(&mut wm, &|wm| add(wm, &[1, 2, 3]));
        quiet(&mut wm, &|wm| wm.window_removed(WindowId(3)));
        quiet(&mut wm, &|wm| wm.focus(WindowId(1), Cause::Activation));
        quiet(&mut wm, &|wm| wm.toggle_split());
        quiet(&mut wm, &|wm| wm.toggle_fullscreen());
        quiet(&mut wm, &|wm| wm.toggle_fullscreen());
        quiet(&mut wm, &|wm| wm.resize(Dir::Right, 10));
        quiet(&mut wm, &|wm| wm.activate_workspace("3"));
        quiet(&mut wm, &|wm| {
            wm.pointer(Hit::Window(WindowId(2)), &mut Cx::default());
            wm.pointer(Hit::Output(2), &mut Cx::default());
        });
        let commands: [&dyn Fn(&mut TreeWm); 8] = [
            &|wm| wm.focus_dir(Dir::Left),
            &|wm| wm.move_dir(Dir::Right),
            &|wm| wm.focus_parent(),
            &|wm| wm.focus_previous(),
            &|wm| wm.show(1),
            &|wm| wm.move_to(4),
            &|wm| wm.focus_output(Dir::Right),
            &|wm| wm.warp_pointer(),
        ];
        for command in commands {
            let before = warps(&wm);
            command(&mut wm);
            assert!(warps(&wm) > before);
        }
    }

    #[test]
    fn hover_focuses_windows_and_empty_outputs_only() {
        let mut wm = wm();
        two_outputs(&mut wm);
        add(&mut wm, &[1, 2]);
        let mut cx = Cx::default();
        assert!(!wm.pointer(Hit::Window(WindowId(2)), &mut cx));
        assert!(wm.pointer(Hit::Window(WindowId(1)), &mut cx));
        assert_eq!(wm.focused(), Some(WindowId(1)));
        assert!(!wm.pointer(Hit::Output(1), &mut cx));
        assert!(wm.pointer(Hit::Output(2), &mut cx));
        assert_eq!(wm.focused(), None);
        assert!(!wm.button(Hit::Window(WindowId(2)), 0x110, true, &mut cx));
        assert_eq!(wm.focused(), Some(WindowId(2)));
    }

    #[test]
    fn client_fullscreen_and_activation_do_not_steal_focus() {
        let mut wm = wm();
        add(&mut wm, &[1]);
        wm.move_to(2);
        add(&mut wm, &[2]);
        wm.fullscreen(WindowId(1), true, None);
        assert_eq!(wm.focused(), Some(WindowId(2)));
        assert_eq!(placed(&wm), [(2, Rect::new(1, 1, 98, 98))]);
        wm.fullscreen(WindowId(2), false, None);
        wm.focus(WindowId(1), Cause::Client);
        assert_eq!(wm.focused(), Some(WindowId(2)));
        assert!(wm.workspaces().iter().any(|w| w.name == "2" && w.urgent));
        wm.focus(WindowId(1), Cause::Activation);
        assert_eq!(wm.focused(), Some(WindowId(2)));
        assert!(wm.workspaces().iter().any(|w| w.name == "2" && w.urgent));
        wm.show(2);
        assert!(wm.workspaces().iter().all(|w| !w.urgent));
        wm.show(1);
        wm.focus(WindowId(1), Cause::Client);
        wm.show(2);
        wm.focus(WindowId(1), Cause::Activation);
        assert_eq!(wm.focused(), Some(WindowId(1)));
        assert!(wm.workspaces().iter().all(|w| !w.urgent));
        assert_eq!(placed(&wm), [(1, Rect::new(0, 0, 100, 100))]);
        wm.fullscreen(WindowId(1), false, None);
        assert_eq!(placed(&wm), [(1, Rect::new(1, 1, 98, 98))]);
    }
}
