// math/rand: Go's math/rand, ported (xoshiro256** PRNG, no deps).
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   rand.Intn(100)                      rand::Intn(100)
//   rand.Int63()                        rand::Int63()
//   rand.Float64()                      rand::Float64()
//   rand.Seed(42)                       rand::Seed(42);
//   rand.Shuffle(n, func(i,j) {...})    rand::Shuffle(&mut v)       // shuffles in place
//
// Defaults to a thread-local generator seeded from SystemTime.
// For reproducibility use `Rand::new(seed)` and call methods on it directly.

use crate::types::{float64, int, int64};
use std::cell::RefCell;

// ── xoshiro256** core ──────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct Rand {
    s: [u64; 4],
}

impl Rand {
    pub fn new(seed: u64) -> Self {
        // SplitMix64 to expand a single seed word into 4 state words.
        let mut sm = seed.wrapping_add(0x9E3779B97F4A7C15);
        let mut s = [0u64; 4];
        for slot in &mut s {
            sm = sm.wrapping_add(0x9E3779B97F4A7C15);
            let mut z = sm;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
            z ^= z >> 31;
            *slot = z;
        }
        Rand { s }
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

    #[allow(non_snake_case)]
    pub fn Int63(&mut self) -> int64 {
        (self.next_u64() >> 1) as int64
    }

    #[allow(non_snake_case)]
    pub fn Int(&mut self) -> int {
        self.Int63() as int
    }

    #[allow(non_snake_case)]
    pub fn Intn(&mut self, n: int) -> int {
        if n <= 0 { panic!("rand.Intn: n <= 0"); }
        let r = self.Int63() as u64;
        (r % (n as u64)) as int
    }

    #[allow(non_snake_case)]
    pub fn Float64(&mut self) -> float64 {
        // Go's rand.Float64 returns [0.0, 1.0). 53-bit mantissa.
        let bits = self.next_u64() >> 11;
        bits as f64 / ((1u64 << 53) as f64)
    }

    #[allow(non_snake_case)]
    pub fn Shuffle<T>(&mut self, v: &mut [T]) {
        // Fisher-Yates.
        let n = v.len();
        for i in (1..n).rev() {
            let j = self.Intn((i + 1) as int) as usize;
            v.swap(i, j);
        }
    }
}

// ── global/default generator ───────────────────────────────────────────

thread_local! {
    static DEFAULT: RefCell<Rand> = RefCell::new(Rand::new(seed_from_time()));
}

fn seed_from_time() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x12345678)
}

/// rand.Seed(s) — re-seed the thread-local generator.
#[allow(non_snake_case)]
pub fn Seed(s: int64) {
    DEFAULT.with(|r| *r.borrow_mut() = Rand::new(s as u64));
}

#[allow(non_snake_case)]
pub fn Int63() -> int64 {
    DEFAULT.with(|r| r.borrow_mut().Int63())
}

#[allow(non_snake_case)]
pub fn Int() -> int {
    DEFAULT.with(|r| r.borrow_mut().Int())
}

#[allow(non_snake_case)]
pub fn Intn(n: int) -> int {
    DEFAULT.with(|r| r.borrow_mut().Intn(n))
}

#[allow(non_snake_case)]
pub fn Float64() -> float64 {
    DEFAULT.with(|r| r.borrow_mut().Float64())
}

#[allow(non_snake_case)]
pub fn Shuffle<T>(v: &mut [T]) {
    DEFAULT.with(|r| r.borrow_mut().Shuffle(v));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reproducible_with_seed() {
        let mut r1 = Rand::new(42);
        let mut r2 = Rand::new(42);
        assert_eq!(r1.Int63(), r2.Int63());
        assert_eq!(r1.Int63(), r2.Int63());
    }

    #[test]
    fn intn_in_range() {
        Seed(1234);
        for _ in 0..100 {
            let n = Intn(10);
            assert!((0..10).contains(&n));
        }
    }

    #[test]
    fn float64_in_range() {
        Seed(7);
        for _ in 0..100 {
            let f = Float64();
            assert!((0.0..1.0).contains(&f));
        }
    }

    #[test]
    fn shuffle_permutes() {
        let mut v: Vec<i64> = (0..100).collect();
        let original = v.clone();
        Seed(99);
        Shuffle(&mut v);
        assert_ne!(v, original);
        v.sort();
        assert_eq!(v, original);
    }
}
