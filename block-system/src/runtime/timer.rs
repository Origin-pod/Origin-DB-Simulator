//! Cross-platform timer
//!
//! Uses `std::time::Instant` on native targets and `js_sys::Date::now()`
//! on `wasm32` targets (where `Instant::now()` panics with
//! "time not implemented on this platform").

// ── Native implementation ───────────────────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use std::time::Instant;

    #[derive(Clone)]
    pub struct Timer {
        start: Instant,
    }

    impl Timer {
        pub fn now() -> Self {
            Self {
                start: Instant::now(),
            }
        }

        pub fn elapsed_ms(&self) -> f64 {
            self.start.elapsed().as_secs_f64() * 1000.0
        }
    }
}

// ── WASM implementation ─────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
mod imp {
    #[derive(Clone)]
    pub struct Timer {
        start_ms: f64,
    }

    impl Timer {
        pub fn now() -> Self {
            Self {
                start_ms: js_sys::Date::now(),
            }
        }

        pub fn elapsed_ms(&self) -> f64 {
            js_sys::Date::now() - self.start_ms
        }
    }
}

pub use imp::Timer;
