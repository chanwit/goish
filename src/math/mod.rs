// math: Go's math package (subset).
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   math.Pi                             math::Pi
//   math.Abs(-1.5)                      math::Abs(-1.5)
//   math.Pow(2, 10)                     math::Pow(2.0, 10.0)
//   math.Sqrt(2)                        math::Sqrt(2.0)
//   math.Floor(3.7)                     math::Floor(3.7)
//   math.Ceil(3.2)                      math::Ceil(3.2)
//   math.Max(a, b) / math.Min(a, b)     math::Max(a, b) / math::Min(a, b)
//   math.IsNaN(x)                       math::IsNaN(x)
//   math.IsInf(x, sign)                 math::IsInf(x, sign)

use crate::types::{float64, int64};

// Sub-packages.
pub mod rand;

// ── constants ──────────────────────────────────────────────────────────

#[allow(non_upper_case_globals)] pub const Pi: float64 = std::f64::consts::PI;
#[allow(non_upper_case_globals)] pub const E: float64  = std::f64::consts::E;
#[allow(non_upper_case_globals)] pub const Sqrt2: float64 = std::f64::consts::SQRT_2;
#[allow(non_upper_case_globals)] pub const Ln2: float64 = std::f64::consts::LN_2;
#[allow(non_upper_case_globals)] pub const Ln10: float64 = std::f64::consts::LN_10;

#[allow(non_upper_case_globals)] pub const MaxFloat64: float64 = f64::MAX;
#[allow(non_upper_case_globals)] pub const SmallestNonzeroFloat64: float64 = f64::MIN_POSITIVE;
#[allow(non_upper_case_globals)] pub const Inf: float64 = f64::INFINITY;
#[allow(non_upper_case_globals)] pub const NaN: float64 = f64::NAN;

#[allow(non_upper_case_globals)] pub const MaxInt64: int64 = i64::MAX;
#[allow(non_upper_case_globals)] pub const MinInt64: int64 = i64::MIN;
#[allow(non_upper_case_globals)] pub const MaxInt32: i32 = i32::MAX;
#[allow(non_upper_case_globals)] pub const MinInt32: i32 = i32::MIN;

// ── funcs ──────────────────────────────────────────────────────────────

#[allow(non_snake_case)] pub fn Abs(x: float64) -> float64 { x.abs() }
#[allow(non_snake_case)] pub fn Pow(x: float64, y: float64) -> float64 { x.powf(y) }
#[allow(non_snake_case)] pub fn Sqrt(x: float64) -> float64 { x.sqrt() }
#[allow(non_snake_case)] pub fn Cbrt(x: float64) -> float64 { x.cbrt() }
#[allow(non_snake_case)] pub fn Floor(x: float64) -> float64 { x.floor() }
#[allow(non_snake_case)] pub fn Ceil(x: float64) -> float64 { x.ceil() }
#[allow(non_snake_case)] pub fn Round(x: float64) -> float64 { x.round() }
#[allow(non_snake_case)] pub fn Trunc(x: float64) -> float64 { x.trunc() }
#[allow(non_snake_case)] pub fn Mod(x: float64, y: float64) -> float64 { x % y }

#[allow(non_snake_case)] pub fn Sin(x: float64) -> float64 { x.sin() }
#[allow(non_snake_case)] pub fn Cos(x: float64) -> float64 { x.cos() }
#[allow(non_snake_case)] pub fn Tan(x: float64) -> float64 { x.tan() }
#[allow(non_snake_case)] pub fn Asin(x: float64) -> float64 { x.asin() }
#[allow(non_snake_case)] pub fn Acos(x: float64) -> float64 { x.acos() }
#[allow(non_snake_case)] pub fn Atan(x: float64) -> float64 { x.atan() }
#[allow(non_snake_case)] pub fn Atan2(y: float64, x: float64) -> float64 { y.atan2(x) }

#[allow(non_snake_case)] pub fn Exp(x: float64) -> float64 { x.exp() }
#[allow(non_snake_case)] pub fn Log(x: float64) -> float64 { x.ln() }
#[allow(non_snake_case)] pub fn Log2(x: float64) -> float64 { x.log2() }
#[allow(non_snake_case)] pub fn Log10(x: float64) -> float64 { x.log10() }

#[allow(non_snake_case)] pub fn Max(a: float64, b: float64) -> float64 { a.max(b) }
#[allow(non_snake_case)] pub fn Min(a: float64, b: float64) -> float64 { a.min(b) }

#[allow(non_snake_case)] pub fn IsNaN(x: float64) -> bool { x.is_nan() }

/// math.IsInf(x, sign) — sign > 0 checks +Inf, sign < 0 checks -Inf, sign == 0 either.
#[allow(non_snake_case)]
pub fn IsInf(x: float64, sign: int64) -> bool {
    if sign > 0 { x == f64::INFINITY }
    else if sign < 0 { x == f64::NEG_INFINITY }
    else { x.is_infinite() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_are_sane() {
        assert!((Pi - 3.141_592_653_589_793).abs() < 1e-12);
        assert_eq!(MaxInt64, i64::MAX);
    }

    #[test]
    fn basic_funcs() {
        assert_eq!(Abs(-3.5), 3.5);
        assert_eq!(Pow(2.0, 10.0), 1024.0);
        assert!((Sqrt(2.0) - Sqrt2).abs() < 1e-12);
        assert_eq!(Floor(3.7), 3.0);
        assert_eq!(Ceil(3.2), 4.0);
        assert_eq!(Round(2.5), 3.0);
        assert_eq!(Trunc(3.7), 3.0);
    }

    #[test]
    fn max_min() {
        assert_eq!(Max(1.5, 2.5), 2.5);
        assert_eq!(Min(1.5, 2.5), 1.5);
    }

    #[test]
    fn nan_and_inf() {
        assert!(IsNaN(NaN));
        assert!(!IsNaN(1.0));
        assert!(IsInf(Inf, 1));
        assert!(IsInf(-Inf, -1));
        assert!(!IsInf(1.0, 0));
    }
}
