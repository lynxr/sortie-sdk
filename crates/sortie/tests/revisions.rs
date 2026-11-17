//! Revisions declared by the logic and typed replies, against a scripted engine on tokio's
//! paused clock: what the runtime sends in Attach and Command, what it learns from Start, and
//! how a reply reaches the replica before the next tick.

use std::cell::RefCell;
use std::rc::Rc;

use sortie::prelude::*;
use sortie::proto::v1::{self, logic_down, logic_up};
use sortie::runtime::{AttachOptions, LogicConn, Revisions, spawn_logic};
use sortie::types::{Eid, Scenario, Symbol};
use tokio::sync::mpsc;

#[derive(Clone, Copy, Debug, Default)]
struct Demo;

impl Scenario for Demo {
    const NAME: &'static str = "demo";
    const SCHEMA: u32 = 1;
    const SCHEMA_SHA256: &'static str =
        "0000000000000000000000000000000000000000000000000000000000000000";
    const SYMBOLS: &'static [Symbol] = &[];
}

fn down(msg: logic_down::Msg) -> v1::LogicDown {
    v1::LogicDown { msg: Some(msg) }
}

async fn next_up(up: &mut mpsc::UnboundedReceiver<v1::LogicUp>) -> logic_up::Msg {
    loop {
        match up.recv().await.and_then(|m| m.msg) {
            Some(logic_up::Msg::Done(_)) => {}
            Some(m) => return m,
            None => panic!("logic hung up"),
        }
    }
}

thread_local! {
    static SEEN: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn seen(s: impl Into<String>) {
    SEEN.with(|v| v.borrow_mut().push(s.into()));
}

/// A wave-1 logic: declares API revision 1, events stay at 0.
#[sortie::mission(api_rev = 1)]
async fn declared(ctx: Ctx<Demo>, _start: Start) -> sortie::Result<()> {
    seen(format!(
        "declared {:?}, engine {}, events {}, writer {:?}",
        ctx.declared_revisions(),
        ctx.engine_api_rev(),
        ctx.event_rev(),
        ctx.writer()
    ));
    ctx.message(Coalition::Blue).text("hi").await?;
    Ok(())
}

#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn the_attribute_declares_the_revisions_and_start_tells_the_engine_s() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let (up_tx, mut up_rx) = mpsc::unbounded_channel();
            let (down_tx, down_rx) = mpsc::channel(256);
            let handle = spawn_logic::<Demo, _>(
                LogicConn {
                    up: up_tx,
                    down: down_rx,
                },
                declared,
                AttachOptions::default(),
            );
            let logic_up::Msg::Attach(attach) = next_up(&mut up_rx).await else {
                panic!("no Attach")
            };
            assert_eq!((attach.api_rev, attach.event_rev), (1, 0));
            assert!(attach.required_capabilities.is_empty());
            // A 0.1.0 engine sends none of the new Start fields.
            down_tx
                .try_send(down(logic_down::Msg::Start(v1::Start {
                    session: "s1".into(),
                    ..Default::default()
                })))
                .unwrap();
            let logic_up::Msg::Command(cmd) = next_up(&mut up_rx).await else {
                panic!("no command")
            };
            // A 0.1.0 operation: no minimum, and the Attach revision applies (api_rev 0).
            assert_eq!((cmd.min_api_rev, cmd.api_rev), (0, 0));
            down_tx
                .try_send(down(logic_down::Msg::Result(v1::CommandResult {
                    id: cmd.id,
                    ..Default::default()
                })))
                .unwrap();
            down_tx
                .try_send(down(logic_down::Msg::Shutdown(v1::Shutdown::default())))
                .unwrap();
            while !handle.is_finished() {
                tokio::task::yield_now().await;
            }
            handle.assert_clean();
            let got = SEEN.with(|v| v.borrow().clone());
            assert_eq!(
                got,
                vec![format!(
                    "declared {:?}, engine 0, events 0, writer \"\"",
                    Revisions::new(1, 0)
                )]
            );
        })
        .await;
}

fn eid(n: u32) -> Eid {
    Eid::from_u64(sortie::proto::eid::pack(n, 1)).unwrap()
}

#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn reply_snaps_reach_the_replica_before_the_caller_resumes() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let (up_tx, mut up_rx) = mpsc::unbounded_channel();
            let (down_tx, down_rx) = mpsc::channel(256);
            let found: Rc<RefCell<Option<String>>> = Rc::default();
            let out = found.clone();
            let mission = move |ctx: Ctx<Demo>, _start: Start| {
                let out = out.clone();
                async move {
                    ctx.message(Coalition::Blue).text("spawn something").await?;
                    // No tick came between the result and this line.
                    *out.borrow_mut() = ctx.world().entity(eid(7)).map(|e| e.name.clone());
                    Ok(())
                }
            };
            let handle = spawn_logic::<Demo, _>(
                LogicConn {
                    up: up_tx,
                    down: down_rx,
                },
                mission,
                AttachOptions::default(),
            );
            let logic_up::Msg::Attach(attach) = next_up(&mut up_rx).await else {
                panic!("no Attach")
            };
            assert_eq!(
                (attach.api_rev, attach.event_rev),
                (0, 0),
                "no attribute: 0.1.0"
            );
            down_tx
                .try_send(down(logic_down::Msg::Start(v1::Start {
                    session: "s1".into(),
                    snapshot: Some(v1::WorldSnapshot {
                        session: "s1".into(),
                        rev: 4,
                        ..Default::default()
                    }),
                    api_rev: 1,
                    ..Default::default()
                })))
                .unwrap();
            let logic_up::Msg::Command(cmd) = next_up(&mut up_rx).await else {
                panic!("no command")
            };
            let unit = v1::EntitySnap {
                eid: eid(7).to_u64(),
                name: "Red CAP#0001-1-1".into(),
                category: v1::Category::Airplane as i32,
                ..Default::default()
            };
            down_tx
                .try_send(down(logic_down::Msg::Result(v1::CommandResult {
                    id: cmd.id,
                    world_rev: 5,
                    reply: Some(v1::Reply {
                        entities: vec![unit.clone()],
                        ..Default::default()
                    }),
                    ..Default::default()
                })))
                .unwrap();
            while found.borrow().is_none() && !handle.is_finished() {
                tokio::task::yield_now().await;
            }
            assert_eq!(found.borrow().as_deref(), Some("Red CAP#0001-1-1"));
            // The tick that carries the same facts applies on top (the replica did not move).
            down_tx
                .try_send(down(logic_down::Msg::Tick(v1::Tick {
                    n: 1,
                    model_time: 1.0,
                    delta: Some(v1::WorldDelta {
                        from_rev: 4,
                        rev: 5,
                        upserts: vec![unit],
                        ..Default::default()
                    }),
                    ..Default::default()
                })))
                .unwrap();
            down_tx
                .try_send(down(logic_down::Msg::Shutdown(v1::Shutdown::default())))
                .unwrap();
            while !handle.is_finished() {
                tokio::task::yield_now().await;
            }
            handle.assert_clean();
        })
        .await;
}

/// What a wave-1 builder did against the engine.
fn too_old<T>(name: &str, r: sortie::Result<T>) {
    match r {
        Err(e) if e.code() == "sortie::engine_too_old" => seen(format!("{name}: engine_too_old")),
        Err(e) => seen(format!("{name}: {}", e.code())),
        Ok(_) => seen(format!("{name}: sent")),
    }
}

/// CMP-3 (spec 9.1): every wave-1 builder against a 0.1.0 engine.
#[sortie::mission(api_rev = 1)]
async fn wave_1_builders(ctx: Ctx<Demo>, _start: Start) -> sortie::Result<()> {
    let p = MapPoint::new(1.0, 2.0);
    too_old(
        "spawn_static",
        ctx.spawn_static("Cafe", "USA").at(p, 0.0).await,
    );
    too_old(
        "spawn_batch",
        ctx.spawn_batch()
            .static_object(ctx.spawn_static("Cafe", "USA").at(p, 0.0))
            .key("b")
            .await,
    );
    too_old(
        "destroy(object)",
        ctx.destroy(sortie::sel::Obj::static_object("x")).await,
    );
    too_old(
        "remove_junk",
        ctx.remove_junk([sortie::sel::Volume::sphere_on_ground(p, 10.0)])
            .await,
    );
    too_old("map.circle", ctx.map().circle("c", p, 10.0).await);
    too_old("map.remove", ctx.map().remove("c").await);
    too_old("menus.tree", ctx.menus().tree("t").command("a", "A").await);
    too_old("menus.remove", ctx.menus().remove("t").await);
    too_old("dcs.land.height", ctx.dcs().land().height(p).await);
    // A 0.1.0 operation still goes out.
    ctx.message(Coalition::Blue).text("0.1.0").await?;
    seen("message: sent");
    Ok(())
}

/// CMP-3: a newer SDK cannot widen a command against an older engine. A 0.1.0 engine's Start
/// has no API revision (0): every wave-1 builder fails with `EngineTooOld` before anything is
/// sent, and the 0.1.0 operations still work.
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn every_wave_1_builder_is_engine_too_old_against_a_0_1_0_engine() {
    SEEN.with(|v| v.borrow_mut().clear());
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let (up_tx, mut up_rx) = mpsc::unbounded_channel();
            let (down_tx, down_rx) = mpsc::channel(256);
            let handle = spawn_logic::<Demo, _>(
                LogicConn {
                    up: up_tx,
                    down: down_rx,
                },
                wave_1_builders,
                AttachOptions::default(),
            );
            let logic_up::Msg::Attach(_) = next_up(&mut up_rx).await else {
                panic!("no Attach")
            };
            down_tx
                .try_send(down(logic_down::Msg::Start(v1::Start {
                    session: "s1".into(),
                    ..Default::default()
                })))
                .unwrap();
            // The first command on the wire is the 0.1.0 message: nothing of wave 1 was sent.
            let logic_up::Msg::Command(cmd) = next_up(&mut up_rx).await else {
                panic!("no command")
            };
            assert!(
                matches!(cmd.body, Some(v1::command::Body::Message(_))),
                "{cmd:?}"
            );
            down_tx
                .try_send(down(logic_down::Msg::Result(v1::CommandResult {
                    id: cmd.id,
                    ..Default::default()
                })))
                .unwrap();
            down_tx
                .try_send(down(logic_down::Msg::Shutdown(v1::Shutdown::default())))
                .unwrap();
            while !handle.is_finished() {
                tokio::task::yield_now().await;
            }
            let got = SEEN.with(|v| v.borrow().clone());
            let expected: Vec<String> = [
                "spawn_static",
                "spawn_batch",
                "destroy(object)",
                "remove_junk",
                "map.circle",
                "map.remove",
                "menus.tree",
                "menus.remove",
                "dcs.land.height",
            ]
            .iter()
            .map(|n| format!("{n}: engine_too_old"))
            .chain(["message: sent".to_owned()])
            .collect();
            assert_eq!(got, expected);
        })
        .await;
}

fn cap(name: &str, state: v1::CapabilityState) -> v1::Capability {
    v1::Capability {
        name: name.into(),
        state: state as i32,
        live: v1::LiveStatus::Unverified as i32,
        ..Default::default()
    }
}

/// A logic that names its requirements and reads the table before and after a change.
#[sortie::mission(api_rev = 1, event_rev = 1, requires = [sortie::caps::DRAW, sortie::caps::call::mission::LAND_GET_HEIGHT])]
async fn reads_capabilities(ctx: Ctx<Demo>, _start: Start) -> sortie::Result<()> {
    use sortie::caps;
    let t = ctx.capabilities();
    seen(format!(
        "start: {} entries, draw {}, dcs_call {:?}",
        t.len(),
        t.is_available(caps::DRAW),
        t.state(caps::DCS_CALL)
    ));
    ctx.message(Coalition::Blue).text("sync").await?;
    let c = ctx.capability(caps::DCS_CALL).expect("listed");
    seen(format!(
        "after: dcs_call {:?} ({}), {} quarantined",
        v1::CapabilityState::try_from(c.state).unwrap(),
        c.note,
        ctx.capabilities()
            .in_state(v1::CapabilityState::Quarantined)
            .count()
    ));
    Ok(())
}

/// `requires = [..]` goes out in Attach; `ctx.capabilities()` is Start's table and follows
/// `CapabilityChanged` (spec 4.1).
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn requirements_go_in_attach_and_capability_changed_updates_the_table() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            SEEN.with(|v| v.borrow_mut().clear());
            let (up_tx, mut up_rx) = mpsc::unbounded_channel();
            let (down_tx, down_rx) = mpsc::channel(256);
            let handle = spawn_logic::<Demo, _>(
                LogicConn {
                    up: up_tx,
                    down: down_rx,
                },
                reads_capabilities,
                AttachOptions::default(),
            );
            let logic_up::Msg::Attach(attach) = next_up(&mut up_rx).await else {
                panic!("no Attach")
            };
            assert_eq!(
                attach.required_capabilities,
                ["draw", "call:mission:land.getHeight"]
            );
            down_tx
                .try_send(down(logic_down::Msg::Start(v1::Start {
                    session: "s1".into(),
                    api_rev: 1,
                    event_rev: 1,
                    capabilities: vec![
                        cap("draw", v1::CapabilityState::Available),
                        cap("dcs_call", v1::CapabilityState::Available),
                        cap(
                            "call:mission:land.getHeight",
                            v1::CapabilityState::Available,
                        ),
                    ],
                    ..Default::default()
                })))
                .unwrap();
            let logic_up::Msg::Command(cmd) = next_up(&mut up_rx).await else {
                panic!("no command")
            };
            // The change arrives before the result the logic waits for.
            let mut q = cap("dcs_call", v1::CapabilityState::Quarantined);
            q.note = "DCS crashed".into();
            down_tx
                .try_send(down(logic_down::Msg::Event(v1::Event {
                    kind: Some(v1::event::Kind::CapabilityChanged(v1::CapabilityChanged {
                        capability: Some(q),
                        previous: v1::CapabilityState::Available as i32,
                    })),
                    ..Default::default()
                })))
                .unwrap();
            down_tx
                .try_send(down(logic_down::Msg::Result(v1::CommandResult {
                    id: cmd.id,
                    ..Default::default()
                })))
                .unwrap();
            down_tx
                .try_send(down(logic_down::Msg::Shutdown(v1::Shutdown::default())))
                .unwrap();
            while !handle.is_finished() {
                tokio::task::yield_now().await;
            }
            handle.assert_clean();
            let got = SEEN.with(|v| v.borrow().clone());
            assert_eq!(
                got,
                [
                    "start: 3 entries, draw true, dcs_call Available",
                    "after: dcs_call Quarantined (DCS crashed), 1 quarantined"
                ]
            );
        })
        .await;
}
