use std::time::Duration;

use enumflags2::{BitFlag, BitFlags};
use shared::models::terrain_info::Vector3;

use splines::{Interpolation, Key, Spline};

use crate::shared::constants::SplineFlag;

pub struct MovementSpline {
    state: MovementSplineState,
    spline_flags: BitFlags<SplineFlag>,
    spline_id: u32,
    spline: Option<Spline<f32, Vector3>>,
    elapsed_time: Duration,
    total_time: Duration,
}

impl MovementSpline {
    pub fn new() -> Self {
        Self {
            state: MovementSplineState::Idle,
            spline_flags: SplineFlag::empty(),
            spline_id: 0, // TODO
            spline: None,
            elapsed_time: Duration::ZERO,
            total_time: Duration::ZERO,
        }
    }

    // The positional parameters are the starting position and a path. The path must contain the
    // positions that the entity is passing through, without the starting position. Due to the client
    // implementation, this method must add a new point before the starting position, which is
    // an extrapolation of what a "0th position" would be (in maths:
    // starting_position.lerp(path[0], -1)), and add a new point after the last point in `path` that
    // is the last point of the path, duplicated.
    // This manipulation is normally only required for Catmull-Rom paths (to generate the
    // correct curves), but due to the client implementation, it is also required for linear paths.
    pub fn init(
        &mut self,
        starting_position: &Vector3,
        path: &Vec<Vector3>,
        velocity: f32,
        linear: bool,
    ) -> Duration {
        self.state = MovementSplineState::Moving;

        // Calculate each segment length then the total length; each key is the time needed to
        // reach that knot with at the given velocity
        let inverted_velocity = 1000. / velocity;
        let mut total_time = 0.;
        let spline_keys: Vec<Key<_, _>> = path
            .iter()
            .zip(Self::segments_length(starting_position, path, linear))
            .map(|(point, seg_length)| {
                total_time += seg_length * inverted_velocity;
                Key::new(total_time, *point, Interpolation::Linear)
            })
            .collect();

        // First, add the extrapolated "0th" point with a key of zero because traveling
        // from there to the starting position is instantaneous (we don't actually travel from
        // there)
        // Then add the starting position, still with a key of 0 (because we start from there so
        // we reach it instantaneously too)
        let mut full_path = vec![
            Key::new(
                0.,
                starting_position.lerp(&path[0], -1.),
                Interpolation::Linear,
            ),
            Key::new(0., *starting_position, Interpolation::Linear),
        ];
        // Then add all the points up to (and including) the destination with their respective
        // keys
        full_path.extend(spline_keys.clone());
        // Then finally duplicate the destination (see the comment above this method for a detailed
        // explanation)
        full_path.push(*spline_keys.last().unwrap());

        self.spline = Some(Spline::from_vec(full_path));
        self.elapsed_time = Duration::ZERO;
        let total_time = Duration::from_millis(total_time as u64);
        self.total_time = total_time;
        total_time
    }

    pub fn state(&self) -> MovementSplineState {
        self.state
    }

    pub fn update(&mut self, dt: Duration) -> (Vector3, MovementSplineState) {
        self.elapsed_time += dt;
        let spline = self
            .spline
            .as_ref()
            .expect("updating a MovementSpline with no spline");

        match spline.sample(self.elapsed_time.as_millis() as f32) {
            Some(new_position) => (new_position, self.state),
            None => {
                self.state = MovementSplineState::Arrived;
                (spline.keys().last().unwrap().value, self.state)
            }
        }
    }

    pub fn reset(&mut self) {
        self.state = MovementSplineState::Idle;
        self.spline = None;
    }

    pub fn spline_flags(&self) -> BitFlags<SplineFlag> {
        self.spline_flags
    }

    pub fn elapsed_time(&self) -> Duration {
        self.elapsed_time
    }

    pub fn total_time(&self) -> Duration {
        self.total_time
    }

    pub fn id(&self) -> u32 {
        self.spline_id
    }

    pub fn path(&self) -> Vec<Vector3> {
        self.spline
            .as_ref()
            .map(|s| s.keys().into_iter().map(|k| k.value).collect())
            .unwrap_or_default()
    }

    // Calculates the length of each segment depending on the type of path (linear or catmull-rom)
    fn segments_length(starting_position: &Vector3, path: &Vec<Vector3>, linear: bool) -> Vec<f32> {
        let mut lengths: Vec<f32> = vec![0.; path.len()];

        if linear {
            // Linear is easy, get each segment vector and calculate its length
            for (index, point) in path.iter().enumerate() {
                let previous_point = if index == 0 {
                    *starting_position
                } else {
                    path[index - 1]
                };

                lengths[index] = (*point - previous_point).len();
            }
        } else {
            // Catmull-rom is a bit harder... We use 3 steps per segment to calculate the length
            todo!("see https://github.com/mangosone/server/blob/master/src/game/movement/spline.cpp#L165-L183")
        }

        lengths
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MovementSplineState {
    Idle,
    Moving,
    Arrived,
}
