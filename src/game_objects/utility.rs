use std::time::Instant;

use cgmath::{InnerSpace, Vector3, Zero};

// const MAX_COUNTER_DV: f32 = -1.;
const MAX_SQR_VEL: f32 = 200.;

pub struct VectorDamp {
    last_time: Instant,
    velocity: Vector3<f32>,
    strength: f32,
}
impl VectorDamp {
    pub fn new(strength: f32) -> Self {
        Self {
            last_time: Instant::now(),
            velocity: Vector3::zero(),
            strength,
        }
    }

    pub fn smooth_follow(&mut self, current: Vector3<f32>, target: Vector3<f32>) -> Vector3<f32> {
        let difference = current - target;

        let elapsed_time = self.last_time.elapsed().as_secs_f32();
        self.last_time = Instant::now();
        // lag too large, snap to target
        if elapsed_time > 2.0 / self.strength {
            println!("[Warning] Lerp lag (elapsed time:{elapsed_time}), snapping to target");
            self.velocity = Vector3::zero();
            return target;
        }

        // dy = h/2( dy/dt(y) + dy/dt(y + h*dy/dt(y)) )

        let delta_vel =
            -self.strength * (2. * self.velocity + self.strength * difference) * elapsed_time;

        // let counter_dv = delta_vel.dot(self.velocity) / self.velocity.magnitude2();
        // if counter_dv <= MAX_COUNTER_DV {
        //     delta_vel += (MAX_COUNTER_DV - counter_dv) * self.velocity;
        // }
        self.velocity += delta_vel;
        let sqr_vel = self.velocity.magnitude2();
        if sqr_vel > MAX_SQR_VEL {
            println!("[Warning] Vel maxed out, square vel: {sqr_vel}");
            self.velocity *= (MAX_SQR_VEL / sqr_vel).sqrt();
        }

        let delta_pos = (self.velocity) * elapsed_time;

        current + delta_pos

        // let exp = (-self.strength * elapsed_time).exp();
        // let k = (self.velocity + self.strength * difference) * elapsed_time;

        // self.velocity = (self.velocity - k * self.strength) * exp;

        // (k + difference) * exp
    }

    pub fn reset_last_time(&mut self) {
        self.last_time = Instant::now();
        self.velocity = Vector3::zero();
    }
}

// pub struct IDCollection<T> {
//     collection: HashSet<usize, T>,
//     // missing: VecDeque<usize>,
//     next_id: usize,
// }
// impl<T> IDCollection<T> {
//     pub fn new() -> Self {
//         Self {
//             collection: vec![],
//             missing: VecDeque::new(),
//             next_id: 0,
//         }
//     }

//     pub fn push(&mut self, item: impl Into<T>) -> usize {
//         match self.missing.pop_front() {
//             Some(id) => {
//                 *self.collection.get_mut(id).unwrap() = item.into();
//                 id
//             }
//             None => {
//                 self.collection.push(item.into());
//                 self.next_id += 1;
//                 self.next_id - 1
//             }
//         }
//     }
//     pub fn remove(&mut self, id: usize) {
//         self.missing.push_back(id);
//     }

//     pub fn get(&self, id: usize) -> &T {
//         &self.collection[id]
//     }
//     pub fn get_mut(&mut self, id: usize) -> &mut T {
//         &mut self.collection[id]
//     }

//     pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
//         self.collection.iter_mut()
//     }
// }
// impl<T: Default> IDCollection<T> {
//     pub fn push_default(&mut self) -> usize {
//         self.push(T::default())
//     }
// }
