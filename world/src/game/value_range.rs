use rand::{distributions::uniform::SampleUniform, Rng};

pub struct ValueRange<T> {
    min: T,
    max: T,
}

impl<T> ValueRange<T>
where
    T: PartialOrd + SampleUniform + Copy + std::fmt::Display,
{
    pub fn new(a: T, b: T) -> Self {
        if a <= b {
            Self { min: a, max: b }
        } else {
            Self { min: b, max: a }
        }
    }

    pub fn random_value(&self) -> T {
        let mut rng = rand::thread_rng();
        rng.gen_range(self.min..=self.max)
    }

    pub fn min(&self) -> T {
        self.min
    }

    pub fn max(&self) -> T {
        self.max
    }
}
