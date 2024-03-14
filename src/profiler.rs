use std::iter::zip;

pub struct Profiler<const COUNT: usize, const SAMPLES: usize> {
    sample_i: usize,
    names: [&'static str; COUNT],
    profiles: [Profile<SAMPLES>; COUNT],
}

#[derive(Clone, Copy)]
struct Profile<const SAMPLES: usize> {
    sum: i32,
    micros: [i32; SAMPLES],
}

impl<const COUNT: usize, const SAMPLES: usize> Profiler<COUNT, SAMPLES> {
    pub const fn new(names: [&'static str; COUNT]) -> Self {
        Self {
            sample_i: 0,
            names,
            profiles: [Profile {
                sum: 0,
                micros: [0; SAMPLES],
            }; COUNT],
        }
    }

    pub fn add_sample(&mut self, micro: i32, profile_i: usize) {
        self.profiles[profile_i].add_sample(micro, self.sample_i);
    }
    pub fn end_frame(&mut self) {
        self.sample_i = (self.sample_i + 1) % SAMPLES;
    }

    pub fn summary(&self) -> String {
        let mut out = String::new();
        let mut total: i32 = 0;

        for (name, profile) in zip(&self.names, &self.profiles) {
            let ave = profile.average();
            out.push_str(&format!("{:<16}{:>4} μs\n", name, ave));
            total += ave;
        }

        if total == 0 {
            out + "Total              0 μs (NA fps)"
        } else {
            out + &format!(
                "Total           {:>4} μs ({} fps)",
                total,
                1_000_000 / total
            )
        }
    }
}

impl<const SAMPLES: usize> Profile<SAMPLES> {
    fn average(&self) -> i32 {
        self.sum / (SAMPLES as i32)
    }

    pub fn add_sample(&mut self, micro: i32, next_i: usize) {
        let last = self.micros[next_i];
        self.micros[next_i] = micro;

        self.sum += micro - last;
    }
}

impl<const SAMPLES: usize> Default for Profile<SAMPLES> {
    fn default() -> Self {
        Profile {
            sum: 0,
            micros: [0; SAMPLES],
        }
    }
}
