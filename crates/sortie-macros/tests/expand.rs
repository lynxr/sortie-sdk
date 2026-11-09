//! The macros used the way users write them, through `sortie::`.

use std::future::Future;
use std::path::Path;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

use sortie::prelude::*;
use sortie::testing::TestHarness;
use sortie::types::{Scenario, Symbol};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Demo;

impl Scenario for Demo {
    const NAME: &'static str = "demo";
    const SCHEMA: u32 = 1;
    const SCHEMA_SHA256: &'static str =
        "0000000000000000000000000000000000000000000000000000000000000000";
    const SYMBOLS: &'static [Symbol] = &[];
}

#[sortie::mission]
pub async fn run(_ctx: Ctx<Demo>, start: Start) -> sortie::Result<()> {
    let _fresh = start.is_fresh();
    Ok(())
}

mod spelled_out {
    use super::Demo;

    /// Fully qualified types and `Result<(), Error>` are the same types.
    #[sortie::mission]
    pub(crate) async fn run(
        mut ctx: sortie::Ctx<Demo>,
        _: sortie::Start,
    ) -> Result<(), sortie::Error> {
        ctx = ctx.clone();
        drop(ctx);
        Ok(())
    }
}

fn assert_mission<M: Scenario, F: sortie::runtime::Mission<M>>(_: F) {}

#[test]
fn a_mission_is_what_main_and_the_fake_take() {
    assert_mission::<Demo, _>(run);
    assert_mission::<Demo, _>(spelled_out::run);
}

/// A harness that records what `#[sortie::test]` hands it and polls the body to completion.
#[derive(Debug)]
pub struct Probe {
    manifest: Option<&'static str>,
}

impl TestHarness for Probe {
    fn run_test<F, Fut>(manifest: Option<&'static str>, body: F)
    where
        F: FnOnce(Self) -> Fut,
        Fut: Future<Output = ()>,
    {
        let mut fut = pin!(body(Probe { manifest }));
        let mut cx = Context::from_waker(Waker::noop());
        loop {
            if let Poll::Ready(()) = fut.as_mut().poll(&mut cx) {
                return;
            }
            std::thread::yield_now();
        }
    }
}

const FIXTURE: &str = "../../examples/op_north/logic/fixtures/op_north.manifest.pb";

#[sortie::test(manifest = "../../examples/op_north/logic/fixtures/op_north.manifest.pb")]
async fn the_manifest_path_is_joined_to_the_crate_dir(probe: Probe) {
    let path = probe.manifest.expect("a manifest path");
    assert_eq!(
        path,
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/",
            "../../examples/op_north/logic/fixtures/op_north.manifest.pb"
        )
    );
    assert!(path.ends_with(FIXTURE));
    assert!(Path::new(path).is_file(), "{path}");
}

#[sortie::test(manifest = "../../examples/op_north/logic/fixtures/op_north.manifest.json")]
async fn json_manifests_are_accepted(probe: Probe) {
    assert!(
        probe
            .manifest
            .is_some_and(|p| p.ends_with(".manifest.json"))
    );
}

#[sortie::test]
async fn without_a_manifest_the_harness_gets_none(probe: Probe) {
    assert_eq!(probe.manifest, None);
    std::future::ready(()).await;
}

#[sortie::test]
#[should_panic(expected = "boom")]
async fn other_attributes_are_kept(_probe: Probe) {
    panic!("boom");
}

#[sortie::test]
#[ignore = "only checks that #[ignore] passes through"]
async fn ignored_tests_stay_ignored(_probe: Probe) {
    panic!("must not run");
}

#[sortie::test]
async fn an_explicit_unit_return_is_fine(mut probe: Probe) -> () {
    probe.manifest = Some("x");
    assert!(probe.manifest.is_some());
}
