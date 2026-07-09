use crate::aabb::*;
use crate::object::*;
use std::cmp::Ordering;
use std::rc::Rc;

//https://github.com/jbikker/tinybvh/blob/main/external/madmann91/bvh/v2/sweep_sah_builder.h
//I also don't have multiple primitives per leaf

pub struct BvhNode {
    left: Rc<dyn Hittable>,
    right: Rc<dyn Hittable>,
    bbox: AABB,
    bvh_index: i32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BvhGPU {
    pub right: i32,
    pub typ: u32, //node or primitve
    pub x: [f32; 2],
    pub y: [f32; 2],
    pub z: [f32; 2],
}

impl BvhNode {
    pub fn new(mut list: Vec<Rc<dyn Hittable>>) -> BvhNode {
        let len = list.len();
        let mut index = -1;
        BvhNode::actual(&mut list, 0, len, &mut index)
    }

    fn actual(
        objects: &mut Vec<Rc<dyn Hittable>>,
        start: usize,
        end: usize,
        index: &mut i32,
    ) -> BvhNode {
        let mut bbox = AABB::EMPTY;
        *index += 1;
        let ind_copy = *index;

        for object_index in start..end {
            bbox = AABB::new_boxes(&bbox, &objects[object_index].bounding_box());
        }

        let object_span = end - start;

        let left: Rc<dyn Hittable>;
        let right: Rc<dyn Hittable>;

        if object_span == 1 {
            left = objects[start].clone();
            right = objects[start].clone();
            *index += 2;
        } else if object_span == 2 {
            left = objects[start].clone();
            right = objects[start + 1].clone();
            *index += 2;
        } else {
            let mut best_cost = f32::INFINITY;
            let mut best_axis = 0;
            let mut best_index = 1;

            for i in 0..3 {
                objects[start..end].sort_by(|a, b| BvhNode::comp(a, b, i));

                let mut bbox_right = AABB::EMPTY;
                let mut boxes_right: Vec<AABB> = vec![];

                for rev in (start..end).rev() {
                    bbox_right = AABB::new_boxes(&bbox_right, &objects[rev].bounding_box());
                    boxes_right.push(bbox_right);
                }

                boxes_right.reverse();
                let mut bbox_left = AABB::EMPTY;

                //skips the cases where its none on either side
                for j in 1..object_span {
                    bbox_left = AABB::new_boxes(&bbox_left, &objects[start + j - 1].bounding_box());

                    let cost = j as f32 * bbox_left.area()
                        + (object_span - j) as f32 * boxes_right[j].area();

                    if cost < best_cost {
                        best_cost = cost;
                        best_axis = i;
                        best_index = j;
                    }
                }
            }

            objects[start..end].sort_by(|a, b| BvhNode::comp(a, b, best_axis));

            let mid = start + best_index;

            left = Rc::new(BvhNode::actual(objects, start, mid, index));
            right = Rc::new(BvhNode::actual(objects, mid, end, index));
        }

        let bbox1 = left.bounding_box();
        let bbox2 = right.bounding_box();

        BvhNode {
            left: left,
            right: right,
            bbox: AABB::new_boxes(&bbox1, &bbox2),
            bvh_index: ind_copy,
        }
    }

    fn comp(a: &Rc<dyn Hittable>, b: &Rc<dyn Hittable>, axis_index: usize) -> Ordering {
        let obj1 = a.centroid();
        let obj2 = b.centroid();

        //is this bad? test
        if obj1[axis_index] < obj2[axis_index] {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    }
}

impl Hittable for BvhNode {
    fn bounding_box(&self) -> AABB {
        self.bbox
    }

    fn bvh_index(&self) -> i32 {
        self.bvh_index
    }

    fn shader_format(&self, linear: &mut Vec<BvhGPU>) {
        //first add the yourself (the parent of your children) to the vec first

        let mut index = self.right.bvh_index();
        if index == -1 {
            //parent of primitive
            index = self.bvh_index + 2;
        }

        linear.push(BvhGPU {
            right: index,
            typ: 1, //for BvhNode
            x: [self.bbox.x.min, self.bbox.x.max],
            y: [self.bbox.y.min, self.bbox.y.max],
            z: [self.bbox.z.min, self.bbox.z.max],
        });

        self.left.shader_format(linear);
        self.right.shader_format(linear);
    }
}

/*use crate::aabb::*;
use crate::object::*;
use std::cmp::Ordering;
use std::rc::Rc;

pub struct BvhNode {
    left: Rc<dyn Hittable>,
    right: Rc<dyn Hittable>,
    bbox: AABB,
    bvh_index: i32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BvhGPU {
    pub right: i32,
    pub typ: u32, //node or primitve
    pub x: [f32; 2],
    pub y: [f32; 2],
    pub z: [f32; 2],
}

impl BvhNode {
    pub fn new(mut list: Vec<Rc<dyn Hittable>>) -> BvhNode {
        let len = list.len();
        let mut index = -1;
        BvhNode::actual(&mut list, 0, len, &mut index)
    }

    fn actual(
        objects: &mut Vec<Rc<dyn Hittable>>,
        start: usize,
        end: usize,
        index: &mut i32,
    ) -> BvhNode {
        let mut bbox = AABB::EMPTY;
        *index += 1;
        let ind_copy = *index;

        for object_index in start..end {
            bbox = AABB::new_boxes(&bbox, &objects[object_index].bounding_box());
        }

        let axis = bbox.longest_axis();

        let comparator = match axis {
            0 => BvhNode::box_x_compare,
            1 => BvhNode::box_y_compare,
            _ => BvhNode::box_z_compare,
        };

        let object_span = end - start;

        let left: Rc<dyn Hittable>;
        let right: Rc<dyn Hittable>;

        if object_span == 1 {
            left = objects[start].clone();
            right = objects[start].clone();
            *index += 2;
        } else if object_span == 2 {
            left = objects[start].clone();
            right = objects[start + 1].clone();
            *index += 2;
        } else {
            objects[start..end].sort_by(|a, b| comparator(a, b));

            let mid = start + object_span / 2;

            left = Rc::new(BvhNode::actual(objects, start, mid, index));
            right = Rc::new(BvhNode::actual(objects, mid, end, index));
        }

        let bbox1 = left.bounding_box();
        let bbox2 = right.bounding_box();

        BvhNode {
            left: left,
            right: right,
            bbox: AABB::new_boxes(&bbox1, &bbox2),
            bvh_index: ind_copy,
        }
    }

    fn box_compare(a: &Rc<dyn Hittable>, b: &Rc<dyn Hittable>, axis_index: usize) -> Ordering {
        let bbox1 = a.bounding_box();
        let bbox2 = b.bounding_box();

        let a_axis_interval = bbox1.axis_interval(axis_index);
        let b_axis_interval = bbox2.axis_interval(axis_index);

        if a_axis_interval.min < b_axis_interval.min {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    }

    fn box_x_compare(a: &Rc<dyn Hittable>, b: &Rc<dyn Hittable>) -> Ordering {
        BvhNode::box_compare(a, b, 0)
    }

    fn box_y_compare(a: &Rc<dyn Hittable>, b: &Rc<dyn Hittable>) -> Ordering {
        BvhNode::box_compare(a, b, 1)
    }

    fn box_z_compare(a: &Rc<dyn Hittable>, b: &Rc<dyn Hittable>) -> Ordering {
        BvhNode::box_compare(a, b, 2)
    }
}

impl Hittable for BvhNode {
    fn bounding_box(&self) -> AABB {
        self.bbox
    }

    fn bvh_index(&self) -> i32 {
        self.bvh_index
    }

    fn shader_format(&self, linear: &mut Vec<BvhGPU>) {
        //first add the yourself (the parent of your children) to the vec first

        let mut index = self.right.bvh_index();
        if index == -1 {
            //parent of primitive
            index = self.bvh_index + 2;
        }

        linear.push(BvhGPU {
            right: index,
            typ: 1, //for BvhNode
            x: [self.bbox.x.min, self.bbox.x.max],
            y: [self.bbox.y.min, self.bbox.y.max],
            z: [self.bbox.z.min, self.bbox.z.max],
        });

        self.left.shader_format(linear);
        self.right.shader_format(linear);
    }
}

*/
