//! The API-only point boundary copies inputs and outputs and skips an unlistened path.
use horizon_mod_api_macros::point;
use std::sync::atomic::Ordering;

mod mod_points {
    pub struct DoubleArgs {
        pub value: i64,
        pub result: Option<i64>,
    }
    impl DoubleArgs {
        pub fn __new(value: i64) -> Self {
            Self {
                value,
                result: None,
            }
        }
    }
    pub enum PointId {
        Double,
        DoubleReturn,
    }
    pub mod __private {
        use std::sync::atomic::AtomicBool;
        pub static POINTS: [AtomicBool; 2] = [AtomicBool::new(false), AtomicBool::new(false)];
    }
    pub mod dispatch {
        pub fn double(args: &mut super::DoubleArgs) {
            args.value += 1;
            if args.value < 0 {
                args.result = Some(42);
            }
        }
        pub fn double_return(args: &mut super::DoubleArgs) {
            args.result = args.result.map(|n| n + 10);
        }
    }
}

#[point(name = "double", free)]
fn double(value: i64) -> i64 {
    value * 2
}

#[test]
fn api_head_body_return_and_early_result() {
    use mod_points::__private::POINTS;
    assert_eq!(double(2), 4);
    POINTS[0].store(true, Ordering::Release);
    assert_eq!(double(2), 6);
    POINTS[1].store(true, Ordering::Release);
    assert_eq!(double(2), 16);
    assert_eq!(
        double(-2),
        42,
        "head result skips body and return listeners"
    );
}
