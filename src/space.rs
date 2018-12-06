use std::ops::{Add, Neg, Mul};
use std::marker::PhantomData;
use std::cell::{UnsafeCell, RefCell};

#[derive(Debug,Clone,Copy,PartialEq)]
pub struct Pair {
    pub x: f32,
    pub y: f32,
}

impl Pair {
    pub fn both(f: f32) -> Pair {
        Pair { x: f, y: f }
    }

    pub fn zero() -> Pair {
        Pair::both(0.0f32)
    }

    pub fn ang(&self) -> f32 {
        let ang = self.y.atan2(self.x);
        if ang < 0.0 {
            ang + ::std::f32::consts::PI
        } else {
            ang
        }
    }

    pub fn limag(&self) -> f32 {
        self.x + self.y
    }

    pub fn polar(head: f32) -> Pair {
        Pair { x: head.cos(), y: head.sin() }
    }

    pub fn mins(&self, other: &Pair) -> Pair {
        Pair { x: self.x.min(other.x), y: self.y.min(other.y) }
    }

    pub fn maxs(&self, other: &Pair) -> Pair {
        Pair { x: self.x.max(other.x), y: self.y.max(other.y) }
    }
}

impl Add for Pair {
    type Output = Pair;
    fn add(self, rhs: Pair) -> Pair {
        Pair { x: self.x + rhs.x, y: self.y + rhs.y }
    }
}

impl Neg for Pair {
    type Output = Pair;
    fn neg(self) -> Pair {
        Pair { x: -self.x, y: -self.y }
    }
}

impl Mul for Pair {
    type Output = Pair;
    fn mul(self, rhs: Pair) -> Pair {
        Pair { x: self.x * rhs.x, y: self.y * rhs.y }
    }
}

impl Mul<f32> for Pair {
    type Output = Pair;
    fn mul(self, rhs: f32) -> Pair {
        self * Pair::both(rhs)
    }
}

impl Default for Pair {
    fn default() -> Pair { Pair::zero() }
}

#[derive(Debug,Clone,PartialEq)]
pub struct AABB {
    pub org: Pair,
    pub dim: Pair,
}

impl AABB {
    pub fn new(mut org: Pair, mut dim: Pair) -> AABB {
        if dim.x < 0.0 {
            org.x += dim.x;
            dim.x = -dim.x;
        }
        if dim.y < 0.0 {
            org.y += dim.y;
            dim.y = -dim.y;
        }
        AABB { org: org, dim: dim }
    }

    pub fn from_corners(c1: Pair, c2: Pair) -> AABB {
        AABB::new(c1, c2 + (-c1))
    }

    pub fn empty() -> AABB {
        AABB { org: Pair::zero(), dim: Pair::zero() }
    }

    pub fn opp(&self) -> Pair { self.org + self.dim }

    pub fn contains(&self, p: Pair) -> bool {
        let opp = self.opp();
        p.x >= self.org.x && p.x < opp.x && p.y >= self.org.y && p.y < opp.y
    }

    pub fn unite(&self, other: &AABB) -> AABB {
        AABB::from_corners(
            self.org.mins(&other.org),
            self.opp().maxs(&other.opp()),
        )
    }

    pub fn enclose(&self, point: Pair) -> AABB {
        if !self.contains(point) {
            let ur = self.org + self.dim;
            AABB {
                org: self.org.mins(&point),
                dim: self.dim.maxs(&ur)
            }
        } else { self.clone() }
    }

    pub fn intersect(&self, other: &AABB) -> Option<AABB> {
        let org = self.org.maxs(&other.org);
        let opp = self.opp().mins(&other.opp());
        let dim = opp + (-org);
        if dim.x < 0.0 || dim.y < 0.0 {
            None
        } else {
            Some(AABB::new(org, dim))
        }
    }

    pub fn over_points<I: Iterator<Item=Pair>>(mut it: I) -> AABB {
        match it.next() {
            None => AABB::empty(),
            Some(pair) => {
                it.fold(AABB::new(pair, Pair::zero()), |a, e| a.enclose(e))
            },
        }
    }

    pub fn around(p: Pair, dim: Pair) -> AABB {
        let half = dim * 0.5;
        AABB::new(p + (-half), dim)
    }
}

impl Default for AABB {
    fn default() -> AABB { AABB::empty() }
}

pub trait SpaceQuery<'a, T: 'a> {
    type QueryIter: Iterator<Item=(Pair, &'a T)>;
    fn add_pt(&mut self, d: (Pair, T)) -> bool;
    fn query(&'a self, b: AABB) -> Self::QueryIter;
}

pub struct QuadTreeNode<T> {
    pub bound: AABB,
    pub children: Option<Box<QuadTreeChildren<T>>>,
    pub data: Vec<(Pair, T)>,
    pub max_data: usize,
}

pub struct QuadTreeChildren<T> {
    pub pp: UnsafeCell<QuadTreeNode<T>>,
    pub pn: UnsafeCell<QuadTreeNode<T>>,
    pub np: UnsafeCell<QuadTreeNode<T>>,
    pub nn: UnsafeCell<QuadTreeNode<T>>,
}

pub struct QuadTreeBuilder<T> {
    pub bound: AABB,
    pub max_data: usize,
    p: PhantomData<T>,
}

const DEFAULT_QUAD_SIZE: usize = 4;

impl<T> QuadTreeBuilder<T> {
    pub fn from_bound(bound: AABB) -> QuadTreeBuilder<T> {
        QuadTreeBuilder { bound: bound, max_data: DEFAULT_QUAD_SIZE, p: PhantomData }
    }

    pub fn with_max_data(self, max_data: usize) -> QuadTreeBuilder<T> {
        QuadTreeBuilder { max_data: max_data, ..self }
    }

    pub fn build(self) -> QuadTreeNode<T> {
        QuadTreeNode {
            bound: self.bound,
            children: None,
            data: Vec::new(),
            max_data: self.max_data,
        }
    }
}

impl<T> QuadTreeNode<T> {
    pub fn derive_child(&self, bound: AABB) -> QuadTreeNode<T> {
        QuadTreeNode {
            bound: bound,
            children: None,
            data: Vec::new(),
            max_data: self.max_data,
        }
    }

    pub fn subdivide(&mut self) {
        let halfdim = self.bound.dim * 0.5;
        let midp = self.bound.org + halfdim;

        let mut children = QuadTreeChildren {
            pp: UnsafeCell::new(self.derive_child(
                AABB::new(midp, halfdim)
            )),
            pn: UnsafeCell::new(self.derive_child(
                AABB::new(Pair { x: midp.x, y: self.bound.org.y }, halfdim)
            )),
            np: UnsafeCell::new(self.derive_child(
                AABB::new(Pair { x: self.bound.org.x, y: midp.y }, halfdim)
            )),
            nn: UnsafeCell::new(self.derive_child(
                AABB::new(self.bound.org, halfdim)
            )),
        };

        for datum in self.data.drain(..) {
            let d = RefCell::new(Some(datum));
            if !children.iter_mut().any(move |child| child.add_pt(d.borrow_mut().take().unwrap())) {
                panic!("Couldn't insert a point into any quadtree child!");
            }
        }

        self.children = Some(Box::new(children));
    }
}

pub struct QuadTreeChildrenIter<'a, T> {
    val: &'a QuadTreeChildren<T>,
    index: usize
}

impl<'a, T> Iterator for QuadTreeChildrenIter<'a, T> {
    type Item = &'a QuadTreeNode<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.index += 1;
        match self.index {
            1 => Some(unsafe { std::intrinsics::transmute(self.val.pp.get()) }),
            2 => Some(unsafe { std::intrinsics::transmute(self.val.pn.get()) }),
            3 => Some(unsafe { std::intrinsics::transmute(self.val.np.get()) }),
            4 => Some(unsafe { std::intrinsics::transmute(self.val.nn.get()) }),
            _ => None
        }
    }
}
impl<'a, T> std::iter::IntoIterator for &'a QuadTreeChildren<T> {
    type IntoIter = QuadTreeChildrenIter<'a, T>;
    type Item = &'a QuadTreeNode<T>;

    fn into_iter(self) -> Self::IntoIter { QuadTreeChildrenIter { val: self, index: 0 } }
}

pub struct QuadTreeChildrenIterMut<'a, T> {
    val: &'a QuadTreeChildren<T>,
    index: usize
}

impl<'a, T> Iterator for QuadTreeChildrenIterMut<'a, T> {
    type Item = &'a mut QuadTreeNode<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.index += 1;
        let v = match self.index {
            // safe because we'll only yield each once, so 
            1 => Some(unsafe { std::intrinsics::transmute(self.val.pp.get()) }),
            2 => Some(unsafe { std::intrinsics::transmute(self.val.pn.get()) }),
            3 => Some(unsafe { std::intrinsics::transmute(self.val.np.get()) }),
            4 => Some(unsafe { std::intrinsics::transmute(self.val.nn.get()) }),
            _ => None
        };
        self.index += 1;
        v
    }
}
impl<'a, T> std::iter::IntoIterator for &'a mut QuadTreeChildren<T> {
    type IntoIter = QuadTreeChildrenIterMut<'a, T>;
    type Item = &'a mut QuadTreeNode<T>;

    fn into_iter(self) -> Self::IntoIter { QuadTreeChildrenIterMut { val: self, index: 0 } }
}

impl<T> QuadTreeChildren<T> {
    fn iter(&self) -> QuadTreeChildrenIter<T> { self.into_iter() }
    fn iter_mut(&mut self) -> QuadTreeChildrenIterMut<T> { self.into_iter() }
}

pub struct QuadTreeQueryIterator<'a, T> {
    pub stack: Vec<&'a QuadTreeNode<T>>,
    pub index: usize,
    pub query: AABB,
}

impl<'a, T: 'a> Iterator for QuadTreeQueryIterator<'a, T> {
    type Item = (Pair, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.stack.is_empty() {
                return None;
            }

            {
                let top = self.stack.last().unwrap();

                while self.index < top.data.len() {
                    let datum = &top.data[self.index];
                    self.index += 1;
                    if self.query.contains(datum.0) {
                        return Some((datum.0, &datum.1));
                    }
                }
            }

            let top = self.stack.pop().unwrap();

            if let Some(children) = &top.children {
                for child in children.into_iter() {
                    if self.query.intersect(&child.bound).is_some() {
                        self.stack.push(child);
                    }
                }
            }
        }
    }
}

impl<'a, T: 'a> SpaceQuery<'a, T> for QuadTreeNode<T> {
    type QueryIter = QuadTreeQueryIterator<'a, T>;
    fn add_pt(&mut self, datum: (Pair, T)) -> bool {
        if !self.bound.contains(datum.0) {
            return false;
        }

        if self.data.len() >= self.max_data {
            self.subdivide();
        }

        if let Some(children) = &mut self.children {
            let d = RefCell::new(Some(datum));
            if !children.iter_mut().any(move |child| child.add_pt(d.borrow_mut().take().unwrap())) {
                panic!("Couldn't insert a point into any quadtree child");
            }
        } else {
            self.data.push(datum);
        }
        true
    }

    fn query(&'a self, b: AABB) -> QuadTreeQueryIterator<'a, T> {
        QuadTreeQueryIterator {
            stack: vec![&self],
            index: 0,
            query: b,
        }
    }
}
