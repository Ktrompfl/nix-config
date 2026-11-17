#![forbid(unsafe_code)]

pub type Color = u32;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    pub fn right(&self) -> i32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> i32 {
        self.y + self.h
    }

    pub fn is_empty(&self) -> bool {
        self.w <= 0 || self.h <= 0
    }

    pub fn center(&self) -> (i64, i64) {
        ((self.x + self.w / 2).into(), (self.y + self.h / 2).into())
    }

    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x.into() && y >= self.y.into() && x < self.right().into() && y < self.bottom().into()
    }

    pub fn intersect(&self, o: Rect) -> Rect {
        let (x, y) = (self.x.max(o.x), self.y.max(o.y));
        Rect::new(x, y, (self.right().min(o.right()) - x).max(0), (self.bottom().min(o.bottom()) - y).max(0))
    }

    pub fn union(&self, o: Rect) -> Rect {
        match (self.is_empty(), o.is_empty()) {
            (true, _) => o,
            (_, true) => *self,
            _ => {
                let (x, y) = (self.x.min(o.x), self.y.min(o.y));
                Rect::new(x, y, self.right().max(o.right()) - x, self.bottom().max(o.bottom()) - y)
            }
        }
    }

    pub fn inset(&self, d: i32) -> Rect {
        Rect::new(self.x + d, self.y + d, (self.w - 2 * d).max(0), (self.h - 2 * d).max(0))
    }

    pub fn scaled(&self, scale: f64) -> Rect {
        let f = |v: i32| (f64::from(v) * scale).round() as i32;
        let (x, y) = (f(self.x), f(self.y));
        Rect::new(x, y, f(self.right()) - x, f(self.bottom()) - y)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Region(Vec<(bool, Rect)>);

impl Region {
    pub fn add(&mut self, r: Rect) {
        self.0.push((true, r));
    }

    pub fn subtract(&mut self, r: Rect) {
        self.0.push((false, r));
    }

    pub fn contains(&self, x: f64, y: f64) -> bool {
        self.0.iter().rev().find(|(_, r)| r.contains(x, y)).is_some_and(|&(added, _)| added)
    }

    pub fn bounds(&self) -> Rect {
        self.0.iter().filter(|(added, _)| *added).map(|&(_, r)| r).reduce(|a, b| a.union(b)).unwrap_or_default()
    }
}
