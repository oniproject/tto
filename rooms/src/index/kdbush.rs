use std::marker::PhantomData;
use smallvec::SmallVec;
use super::{Shim, Entry};

type Index = usize;
const MAGIC: Index = 600;

pub struct KDBush<S: Shim> {
    data: Vec<Entry<S>>,
    node_size: usize,
    _marker: PhantomData<S>,
}

impl<S: Shim> KDBush<S> {
    pub fn new(node_size: usize) -> Self {
        Self {
            node_size,
            data: Vec::new(),
            _marker: PhantomData
        }
    }

    fn sort_kd(&mut self, left: Index, right: Index, axis: u8) {
        if right - left <= self.node_size {
            return;
        }

        let middle: Index = (left + right) / 2;
        self.select(middle, left, right, axis);

        let next_axis = (axis + 1) % 2;
        self.sort_kd(left, middle - 1, next_axis);
        self.sort_kd(middle + 1, right, next_axis);
    }

    fn select(&mut self, k: Index, mut left: Index, mut right: Index, axis: u8) {
        while right > left {
            if right - left > MAGIC {
                let n = (right - left + 1) as f32;
                let m = (k - left + 1) as f32;
                let z = n.ln();
                let s = 0.5 * (2.0 * z / 3.0).exp();
                let sd = 0.5 * (z * s * (n - s) / n).sqrt() *
                    if m - n / 2.0 < 0.0 { -1.0 } else { 1.0 };
                let sn = s / n;
                let kk = k as f32;
                let new_left  = left .max((     kk - m  * sn + sd) as Index);
                let new_right = right.min((kk + (n - m) * sn + sd) as Index);
                self.select(k, new_left, new_right, axis);
            }

            let axis = axis as usize;
            let t = self.data[k].point[axis];
            let mut i = left;
            let mut j = right;

            self.swap_item(left, k);
            if self.data[right].point[axis] > t {
                self.swap_item(left, right);
            }

            while i < j {
                self.swap_item(i, j);
                i += 1;
                j -= 1;
                while self.data[i].point[axis] < t { i += 1 };
                while self.data[j].point[axis] > t { j -= 1 };
            }

            if self.data[left].point[axis] == t {
                self.swap_item(left, j);
            } else {
                j += 1;
                self.swap_item(j, right);
            }

            if j <= k { left = j + 1; }
            if k <= j { right = j - 1; }
        }
    }

    fn swap_item(&mut self, i: Index, j: Index) {
        self.data.swap(i, j);
    }

    fn traversal<V>(&self, min: [S; 2], max: [S; 2], mut visitor: V)
        where V: FnMut(&Entry<S>)
    {
        let [minx, miny] = min;
        let [maxx, maxy] = max;

        let mut stack: SmallVec<[_; 32]> = smallvec![(0, self.data.len() - 1, 0u8)];
        while let Some((left, right, axis)) = stack.pop() {
            if right - left <= self.node_size {
                for i in left..=right {
                    visitor(&self.data[i]);
                }
                continue;
            }

            let middle = (left + right) / 2;
            let e = &self.data[middle];
            visitor(e);

            let [x, y] = e.point;

            let next_axis = (axis + 1) % 2;
            if if axis == 0 { minx <= x } else { miny <= y } {
                stack.push((left, middle - 1, next_axis));
            }
            if if axis == 0 { maxx >= x } else { maxy >= y } {
                stack.push((middle + 1, right, next_axis));
            }
        }
    }
}

impl<S: Shim> super::SpatialIndex<S> for KDBush<S> {
    fn fill<I>(&mut self, pts: I)
        where I: Iterator<Item=(u32, [S; 2])>
    {
        self.data.clear();
        self.data.extend(pts.map(Entry::from));
        self.sort_kd(0, self.data.len() - 1, 0);
    }

    fn range<V>(&self, min: [S; 2], max: [S; 2], mut visitor: V)
        where V: FnMut(u32)
    {
        self.traversal(min, max, |e| {
            if S::in_rect(e.point, min, max) {
                visitor(e.index);
            }
        });
    }

    fn within<V>(&self, center: [S; 2], radius: S, mut visitor: V)
        where V: FnMut(u32)
    {
        let r2 = radius * radius;
        let [qx, qy] = center;
        let min = [qx - radius, qy - radius];
        let max = [qx + radius, qy + radius];

        self.traversal(min, max, |e| {
            if S::in_circle2(e.point, center, r2) {
                visitor(e.index);
            }
        });
    }
}
