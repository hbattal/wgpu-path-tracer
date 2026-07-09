use std::f32::INFINITY;
use std::ops::Add;

#[derive(Debug, Clone, Copy)]
pub struct Interval {
    pub min: f32,
    pub max: f32,
}

impl Interval {
    pub fn new(min: f32, max: f32) -> Interval {
        Interval { min: min, max: max }
    }

    pub fn merge(a: &Interval, b: &Interval) -> Interval {
        Interval {
            min: if a.min <= b.min { a.min } else { b.min },
            max: if a.max >= b.max { a.max } else { b.max },
        }
    }

    pub fn size(&self) -> f32 {
        self.max - self.min
    }

    pub fn expand(&self, delta: f32) -> Interval {
        let padding = delta / 2.0;
        Interval {
            min: self.min - padding,
            max: self.max + padding,
        }
    }

    pub const EMPTY: Interval = Interval {
        min: INFINITY,
        max: -INFINITY,
    };

    pub const UNIVERSE: Interval = Interval {
        min: -INFINITY,
        max: INFINITY,
    };
}

impl Add<f32> for Interval {
    type Output = Interval;

    fn add(self, displacement: f32) -> Interval {
        Interval {
            min: self.min + displacement,
            max: self.max + displacement,
        }
    }
}
