//! The hook `#[sortie::test]` expands to.
//!
//! ```ignore
//! #[sortie::test(manifest = "fixtures/op_north.manifest.pb")]
//! async fn t(dcs: FakeDcs<OpNorth>) { ... }
//! // expands to
//! #[test]
//! fn t() {
//!     async fn t(dcs: FakeDcs<OpNorth>) { ... }   // same name as the test
//!     <FakeDcs<OpNorth> as ::sortie::testing::TestHarness>::run_test(
//!         Some(concat!(env!("CARGO_MANIFEST_DIR"), "/", "fixtures/op_north.manifest.pb")),
//!         t,
//!     )
//! }
//! ```

use std::future::Future;

/// A test fixture that owns its runtime. FakeDcs implements it for `FakeDcs<M>`:
/// a `current_thread` runtime with tokio's clock paused (`start_paused`), a `LocalSet`, the
/// manifest loaded from `manifest` (an absolute path) or built-in defaults when `None`.
pub trait TestHarness: Sized + 'static {
    fn run_test<F, Fut>(manifest: Option<&'static str>, body: F)
    where
        F: FnOnce(Self) -> Fut,
        Fut: Future<Output = ()>;
}
