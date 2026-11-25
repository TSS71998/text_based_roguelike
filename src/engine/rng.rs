//! xoshiro256** random numbers, seeded through SplitMix64.

#[derive(Clone, Debug)]
pub struct RandomNumberGenerator {
    s: [u64; 4],
}

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

pub trait RangeNum: Copy + PartialOrd {
    fn sample(rng: &mut RandomNumberGenerator, min: Self, max: Self) -> Self;
}

macro_rules! range_int {
    ($($t:ty),*) => {$(
        impl RangeNum for $t {
            fn sample(rng: &mut RandomNumberGenerator, min: Self, max: Self) -> Self {
                if max <= min { return min; }
                let span = (max as i128 - min as i128) as u64;
                (min as i128 + rng.below(span) as i128) as $t
            }
        }
    )*};
}

range_int!(i32, i64, u32, u64, usize, isize, u8, i8, u16, i16);

impl RangeNum for f32 {
    fn sample(rng: &mut RandomNumberGenerator, min: Self, max: Self) -> Self {
        min + (max - min) * rng.next_f32()
    }
}

impl RangeNum for f64 {
    fn sample(rng: &mut RandomNumberGenerator, min: Self, max: Self) -> Self {
        min + (max - min) * rng.next_f64()
    }
}

impl RandomNumberGenerator {
    #[allow(clippy::new_without_default)]
    pub fn new() -> RandomNumberGenerator {
        RandomNumberGenerator::seeded(super::platform::entropy_seed())
    }

    pub fn seeded(seed: u64) -> RandomNumberGenerator {
        let mut sm = seed;
        let s = [
            splitmix64(&mut sm),
            splitmix64(&mut sm),
            splitmix64(&mut sm),
            splitmix64(&mut sm),
        ];
        RandomNumberGenerator { s }
    }

    pub fn next_u64(&mut self) -> u64 {
        let result = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;

        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        result
    }

    fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            return 0;
        }
        loop {
            let m = (self.next_u64() as u128) * (n as u128);
            let low = m as u64;
            if low >= n.wrapping_neg() % n {
                return (m >> 64) as u64;
            }
        }
    }

    pub fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 * (1.0 / (1u64 << 24) as f32)
    }

    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    pub fn range<T: RangeNum>(&mut self, min: T, max: T) -> T {
        T::sample(self, min, max)
    }

    pub fn roll_dice(&mut self, n: i32, die_type: i32) -> i32 {
        (0..n.max(0))
            .map(|_| self.range(1, die_type.max(1) + 1))
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_in_range() {
        let (mut a, mut b) = (
            RandomNumberGenerator::seeded(7),
            RandomNumberGenerator::seeded(7),
        );
        for _ in 0..1000 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
        for _ in 0..1000 {
            let v = a.range(-3, 4);
            assert!((-3..4).contains(&v));
            let d = a.roll_dice(3, 6);
            assert!((3..=18).contains(&d));
            let f = a.range(0.5f32, 1.5);
            assert!((0.5..1.5).contains(&f));
        }
        assert_eq!(a.range(5, 5), 5);
    }

    #[test]
    fn roughly_uniform() {
        let mut r = RandomNumberGenerator::seeded(1);
        let mut counts = [0u32; 6];
        for _ in 0..60_000 {
            counts[r.range(0usize, 6)] += 1;
        }
        assert!(
            counts.iter().all(|&c| (9_000..11_000).contains(&c)),
            "{counts:?}"
        );
    }
}
