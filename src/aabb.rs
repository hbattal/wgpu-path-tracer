use crate::interval::*;

#[derive(Copy, Clone, Debug)]
pub struct AABB {
    pub x: Interval,
    pub y: Interval,
    pub z: Interval,
}

impl AABB {
    pub fn new(x: Interval, y: Interval, z: Interval) -> AABB {
        let mut bbox = AABB { x: x, y: y, z: z };

        bbox.pad_to_minumums();

        bbox
    }

    pub fn new_point(a: glam::Vec3, b: glam::Vec3) -> AABB {
        let mut bbox = AABB {
            x: if a.x <= b.x {
                Interval::new(a.x, b.x)
            } else {
                Interval::new(b.x, a.x)
            },
            y: if a.y <= b.y {
                Interval::new(a.y, b.y)
            } else {
                Interval::new(b.y, a.y)
            },
            z: if a.z <= b.z {
                Interval::new(a.z, b.z)
            } else {
                Interval::new(b.z, a.z)
            },
        };

        bbox.pad_to_minumums();

        bbox
    }

    pub fn new_boxes(bbox1: &AABB, bbox2: &AABB) -> AABB {
        AABB {
            x: Interval::merge(&bbox1.x, &bbox2.x),
            y: Interval::merge(&bbox1.y, &bbox2.y),
            z: Interval::merge(&bbox1.z, &bbox2.z),
        }
    }

    pub fn area(&self) -> f32 {
        let e = glam::vec3(self.x.size(), self.y.size(), self.z.size());
        return e.x * e.y + e.y * e.z + e.z * e.x;
    }

    fn pad_to_minumums(&mut self) -> () {
        let delta = 0.0001;

        if self.x.size() < delta {
            self.x = self.x.expand(delta);
        }
        if self.y.size() < delta {
            self.y = self.y.expand(delta);
        }
        if self.z.size() < delta {
            self.z = self.z.expand(delta);
        }
    }

    pub const EMPTY: AABB = AABB {
        x: Interval::EMPTY,
        y: Interval::EMPTY,
        z: Interval::EMPTY,
    };

    pub const UNIVERSE: AABB = AABB {
        x: Interval::UNIVERSE,
        y: Interval::UNIVERSE,
        z: Interval::UNIVERSE,
    };
}
