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

/// A logic of API revision 3 that publishes, unpublishes and breaks the name rule.
#[sortie::mission(api_rev = 3)]
async fn publisher(ctx: Ctx<Demo>, _start: Start) -> sortie::Result<()> {
    match ctx.publish("front", &[3, 1]).await {
        Ok(p) => seen(format!(
            "published rev {} at {} changed {}",
            p.rev, p.updated_unix_ms, p.changed
        )),
        Err(e) => seen(format!("publish: {}", e.code())),
    }
    match ctx.unpublish("front").await {
        Ok(removed) => seen(format!("unpublished {removed}")),
        Err(e) => seen(format!("unpublish: {}", e.code())),
    }
    match ctx.publish("Front", &"x").await {
        Ok(_) => seen("bad name: published"),
        Err(e) => seen(format!(
            "bad name: {} on document {} at revisions.rs {}",
            e.code(),
            e.to_string().contains("on document \"Front\""),
            e.to_string().contains("revisions.rs:")
        )),
    }
    Ok(())
}

/// The scripted engine's side of [`publisher`]; `api_rev` is what its Start says.
async fn run_publisher(api_rev: u32, answer: impl Fn(&v1::StateOp) -> v1::state_result::Result) {
    let (up_tx, mut up_rx) = mpsc::unbounded_channel();
    let (down_tx, down_rx) = mpsc::channel(256);
    let handle = spawn_logic::<Demo, _>(
        LogicConn {
            up: up_tx,
            down: down_rx,
        },
        publisher,
        AttachOptions::default(),
    );
    let logic_up::Msg::Attach(attach) = next_up(&mut up_rx).await else {
        panic!("no Attach")
    };
    assert_eq!(attach.api_rev, 3);
    down_tx
        .try_send(down(logic_down::Msg::Start(v1::Start {
            session: "s1".into(),
            api_rev,
            ..Default::default()
        })))
        .unwrap();
    // Answer state ops until the mission has said what happened to each of its three calls.
    let told = || {
        SEEN.with(|v| {
            v.borrow()
                .iter()
                .filter(|s| !s.starts_with("sent "))
                .count()
        })
    };
    while told() < 3 && !handle.is_finished() {
        tokio::select! {
            msg = up_rx.recv() => match msg.and_then(|m| m.msg) {
                Some(logic_up::Msg::State(op)) => {
                    seen(format!("sent {:?}", op.op));
                    down_tx
                        .try_send(down(logic_down::Msg::State(v1::StateResult {
                            id: op.id,
                            result: Some(answer(&op)),
                        })))
                        .unwrap();
                }
                Some(_) => {}
                None => break,
            },
            () = tokio::task::yield_now() => {}
        }
    }
    down_tx
        .try_send(down(logic_down::Msg::Shutdown(v1::Shutdown::default())))
        .unwrap();
    while !handle.is_finished() {
        tokio::task::yield_now().await;
    }
    handle.assert_clean();
}

/// `ctx.publish` / `ctx.unpublish` are API revision 3: against an engine of revision 2 they
/// fail with `EngineTooOld` and nothing is sent.
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn publishing_against_an_engine_of_revision_2_sends_nothing() {
    SEEN.with(|v| v.borrow_mut().clear());
    let local = tokio::task::LocalSet::new();
    local
        .run_until(run_publisher(2, |op| panic!("nothing is sent: {op:?}")))
        .await;
    let got = SEEN.with(|v| v.borrow().clone());
    assert_eq!(
        got,
        [
            "publish: sortie::engine_too_old",
            "unpublish: sortie::engine_too_old",
            "bad name: sortie::engine_too_old on document true at revisions.rs true",
        ]
    );
}

/// Against an engine of revision 3 the JSON goes out as a `StateOp.publish`, the engine's
/// `published` answer resolves the future, and a refusal names the document and the call site.
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn publishing_sends_the_json_and_resolves_with_the_engine_s_answer() {
    SEEN.with(|v| v.borrow_mut().clear());
    let local = tokio::task::LocalSet::new();
    local
        .run_until(run_publisher(3, |op| match &op.op {
            Some(v1::state_op::Op::Publish(p)) if p.name == "front" => {
                assert_eq!(p.json, b"[3,1]");
                v1::state_result::Result::Published(v1::PublishedDoc {
                    logic: "logic".into(),
                    name: "front".into(),
                    rev: 7,
                    updated_unix_ms: 1000,
                    changed: true,
                })
            }
            Some(v1::state_op::Op::Unpublish(u)) => {
                v1::state_result::Result::Published(v1::PublishedDoc {
                    logic: "logic".into(),
                    name: u.name.clone(),
                    rev: 7,
                    updated_unix_ms: 1000,
                    changed: true,
                })
            }
            _ => v1::state_result::Result::Err(v1::CommandError {
                code: v1::ErrorCode::Invalid as i32,
                message: "a document name is 1 to 64 bytes of a-z, 0-9, `_` and `-`".into(),
                ..Default::default()
            }),
        }))
        .await;
    let got = SEEN.with(|v| v.borrow().clone());
    assert_eq!(
        got,
        [
            "sent Some(Publish(PublishDoc { name: \"front\", json: [91, 51, 44, 49, 93] }))",
            "published rev 7 at 1000 changed true",
            "sent Some(Unpublish(UnpublishDoc { name: \"front\" }))",
            "unpublished true",
            "sent Some(Publish(PublishDoc { name: \"Front\", json: [34, 120, 34] }))",
            "bad name: sortie::invalid on document true at revisions.rs true",
        ]
    );
}

/// A logic of API revision 3 that gives two airbases to red (wave 2, `ctx.airbase_control`).
#[sortie::mission(api_rev = 3)]
async fn airbase_owner(ctx: Ctx<Demo>, _start: Start) -> sortie::Result<()> {
    use sortie::sel::AirbaseSel;
    let r = ctx
        .airbase_control([
            AirbaseSel::name("Kutaisi"),
            AirbaseSel::name("Senaki-Kolkhi"),
        ])
        .owner(Coalition::Red)
        .auto_capture(false)
        .key("front-1")
        .await;
    match r {
        Ok(out) => {
            let world = ctx
                .world()
                .airbase_by_name("Kutaisi")
                .and_then(|a| a.coalition);
            seen(format!(
                "ok {} after {:?} second {:?} world {world:?}",
                out.ok_count(),
                out.bases[0].after.and_then(|s| s.owner),
                out.bases[1].error().map(|e| e.code()),
            ));
        }
        Err(e) => seen(format!("airbase_control: {}", e.code())),
    }
    Ok(())
}

/// The scripted engine's side of [`airbase_owner`]; `api_rev` is what its Start says. Every
/// command is answered with `reply`.
async fn run_airbase_owner(api_rev: u32, reply: Option<v1::Reply>) {
    let (up_tx, mut up_rx) = mpsc::unbounded_channel();
    let (down_tx, down_rx) = mpsc::channel(256);
    let handle = spawn_logic::<Demo, _>(
        LogicConn {
            up: up_tx,
            down: down_rx,
        },
        airbase_owner,
        AttachOptions::default(),
    );
    let logic_up::Msg::Attach(attach) = next_up(&mut up_rx).await else {
        panic!("no Attach")
    };
    assert_eq!(attach.api_rev, 3);
    down_tx
        .try_send(down(logic_down::Msg::Start(v1::Start {
            session: "s1".into(),
            api_rev,
            ..Default::default()
        })))
        .unwrap();
    while SEEN.with(|v| v.borrow().iter().all(|s| s.starts_with("sent "))) && !handle.is_finished()
    {
        tokio::select! {
            msg = up_rx.recv() => match msg.and_then(|m| m.msg) {
                Some(logic_up::Msg::Command(cmd)) => {
                    let Some(v1::command::Body::AirbaseControl(c)) = &cmd.body else {
                        panic!("{cmd:?}")
                    };
                    seen(format!(
                        "sent min_api_rev {} key {} bases {} owner {:?} auto {:?} silent {:?}",
                        cmd.min_api_rev,
                        cmd.key,
                        c.airbases.len(),
                        c.coalition.map(v1::Coalition::try_from),
                        c.auto_capture,
                        c.radio_silent
                    ));
                    down_tx
                        .try_send(down(logic_down::Msg::Result(v1::CommandResult {
                            id: cmd.id,
                            outcome: Some(v1::command_result::Outcome::Ok(
                                sortie::proto::LuaValue::number(1.0),
                            )),
                            reply: reply.clone(),
                            world_rev: 5,
                            ..Default::default()
                        })))
                        .unwrap();
                }
                Some(_) => {}
                None => break,
            },
            () = tokio::task::yield_now() => {}
        }
    }
    down_tx
        .try_send(down(logic_down::Msg::Shutdown(v1::Shutdown::default())))
        .unwrap();
    while !handle.is_finished() {
        tokio::task::yield_now().await;
    }
    handle.assert_clean();
}

/// `ctx.airbase_control` is API revision 3: against an engine of revision 2 it fails with
/// `EngineTooOld` and nothing is sent.
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn airbase_control_against_an_engine_of_revision_2_sends_nothing() {
    SEEN.with(|v| v.borrow_mut().clear());
    let local = tokio::task::LocalSet::new();
    local.run_until(run_airbase_owner(2, None)).await;
    let got = SEEN.with(|v| v.borrow().clone());
    assert_eq!(got, ["airbase_control: sortie::engine_too_old"]);
}

/// Against an engine of revision 3 the command carries `min_api_rev = 3`, the key and only
/// the settings that were given; the typed reply resolves the future, a base's own error is
/// that base's, and the replica holds the new owner when the future resolves.
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn airbase_control_sends_revision_3_and_resolves_with_the_typed_reply() {
    SEEN.with(|v| v.borrow_mut().clear());
    let red = v1::Coalition::Red as i32;
    let state = |coalition: i32| v1::AirbaseControlState {
        coalition,
        auto_capture: Some(false),
        radio_silent: Some(false),
    };
    let kutaisi = Eid::from_parts(3, 1).unwrap().to_u64();
    let reply = v1::Reply {
        airbases: vec![v1::AirbaseSnap {
            eid: kutaisi,
            name: "Kutaisi".into(),
            coalition: red,
            ..Default::default()
        }],
        airbase_control: Some(v1::AirbasesControlled {
            bases: vec![
                v1::AirbaseControlResult {
                    eid: kutaisi,
                    name: "Kutaisi".into(),
                    before: Some(state(v1::Coalition::Blue as i32)),
                    after: Some(state(red)),
                    err: None,
                },
                v1::AirbaseControlResult {
                    eid: 0,
                    name: "Senaki-Kolkhi".into(),
                    err: Some(v1::CommandError {
                        code: v1::ErrorCode::Gone as i32,
                        message: "airbase `Senaki-Kolkhi` does not exist".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            ],
        }),
        ..Default::default()
    };
    let local = tokio::task::LocalSet::new();
    local.run_until(run_airbase_owner(3, Some(reply))).await;
    let got = SEEN.with(|v| v.borrow().clone());
    assert_eq!(
        got,
        [
            "sent min_api_rev 3 key front-1 bases 2 owner Some(Ok(Red)) auto Some(false) \
             silent None",
            "ok 1 after Some(Red) second Some(\"sortie::gone\") world Some(Red)",
        ]
    );
}

/// A logic of API revision 3 that reads the stock of two warehouses (wave 2, `ctx.warehouse`
/// and `ctx.warehouses_stock`).
#[sortie::mission(api_rev = 3)]
async fn stock_reader(ctx: Ctx<Demo>, _start: Start) -> sortie::Result<()> {
    use sortie::cmd::{Amount, Liquid, StockClass};
    use sortie::sel::{AirbaseSel, WarehouseSel};
    let red = WarehouseSel::airbase(AirbaseSel::name("Senaki-Kolkhi"));
    match ctx.warehouse(red.clone()).stock().await {
        Ok(s) => {
            let weapons = match &s.weapons {
                Some(StockClass::Limited(m)) => format!("{m:?}"),
                other => format!("{other:?}"),
            };
            seen(format!(
                "whole aircraft {:?} weapons {weapons} jet fuel {:?}",
                s.aircraft,
                s.liquids.as_ref().and_then(|l| l.get(&Liquid::JetFuel))
            ));
        }
        Err(e) => {
            seen(format!("stock: {}", e.code()));
            return Ok(());
        }
    }
    let read = ctx
        .warehouses_stock([red, WarehouseSel::named("Kutaisi")])
        .items(["weapons.missiles.AIM_120C"])
        .liquids()
        .await?;
    let counts: Vec<Option<Amount>> = read
        .warehouses
        .iter()
        .map(|w| {
            w.value
                .as_ref()
                .and_then(|s| s.count("weapons.missiles.AIM_120C"))
        })
        .collect();
    seen(format!(
        "batch ok {} counts {counts:?} second {:?} eid {:?}",
        read.ok_count(),
        read.warehouses[1].error().map(|e| e.code()),
        read.warehouses[0].eid.map(Eid::index),
    ));
    Ok(())
}

/// The scripted engine's side of [`stock_reader`]; `api_rev` is what its Start says. Each query
/// is answered with the next of `replies`; the run ends once the logic has said `lines` things.
async fn run_stock_reader(api_rev: u32, replies: Vec<v1::Reply>, lines: usize) {
    let (up_tx, mut up_rx) = mpsc::unbounded_channel();
    let (down_tx, down_rx) = mpsc::channel(256);
    let handle = spawn_logic::<Demo, _>(
        LogicConn {
            up: up_tx,
            down: down_rx,
        },
        stock_reader,
        AttachOptions::default(),
    );
    let logic_up::Msg::Attach(attach) = next_up(&mut up_rx).await else {
        panic!("no Attach")
    };
    assert_eq!(attach.api_rev, 3);
    down_tx
        .try_send(down(logic_down::Msg::Start(v1::Start {
            session: "s1".into(),
            api_rev,
            ..Default::default()
        })))
        .unwrap();
    let mut replies = replies.into_iter();
    let said = || {
        SEEN.with(|v| {
            v.borrow()
                .iter()
                .filter(|s| !s.starts_with("sent "))
                .count()
        })
    };
    while said() < lines && !handle.is_finished() {
        tokio::select! {
            msg = up_rx.recv() => match msg.and_then(|m| m.msg) {
                Some(logic_up::Msg::Command(cmd)) => {
                    let Some(v1::command::Body::Query(v1::Query {
                        kind: Some(v1::query::Kind::WarehouseStock(q)),
                    })) = &cmd.body
                    else {
                        panic!("{cmd:?}")
                    };
                    seen(format!(
                        "sent min_api_rev {} key {:?} warehouses {} items {} liquids {}",
                        cmd.min_api_rev,
                        cmd.key,
                        q.warehouses.len(),
                        q.items.len(),
                        q.liquids
                    ));
                    down_tx
                        .try_send(down(logic_down::Msg::Result(v1::CommandResult {
                            id: cmd.id,
                            outcome: Some(v1::command_result::Outcome::Ok(
                                sortie::proto::LuaValue::number(1.0),
                            )),
                            reply: replies.next(),
                            ..Default::default()
                        })))
                        .unwrap();
                }
                Some(_) => {}
                None => break,
            },
            () = tokio::task::yield_now() => {}
        }
    }
    down_tx
        .try_send(down(logic_down::Msg::Shutdown(v1::Shutdown::default())))
        .unwrap();
    while !handle.is_finished() {
        tokio::task::yield_now().await;
    }
    handle.assert_clean();
}

/// The stock query is API revision 3: against an engine of revision 2 it fails with
/// `EngineTooOld` and nothing is sent.
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn the_stock_query_against_an_engine_of_revision_2_sends_nothing() {
    SEEN.with(|v| v.borrow_mut().clear());
    let local = tokio::task::LocalSet::new();
    local.run_until(run_stock_reader(2, Vec::new(), 1)).await;
    let got = SEEN.with(|v| v.borrow().clone());
    assert_eq!(got, ["stock: sortie::engine_too_old"]);
}

/// Against an engine of revision 3 the query carries `min_api_rev = 3` and no key; the typed
/// reply resolves it: an unlimited class, a limited one by name, liquids by name, the counts in
/// the order asked (a count the engine marks unlimited is `Amount::Unlimited`), a warehouse's own
/// error.
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn the_stock_query_sends_revision_3_and_resolves_with_the_typed_reply() {
    SEEN.with(|v| v.borrow_mut().clear());
    let senaki = Eid::from_parts(3, 1).unwrap().to_u64();
    let whole = v1::Reply {
        warehouse_stock: Some(v1::WarehousesStock {
            warehouses: vec![v1::WarehouseStock {
                name: "Senaki-Kolkhi".into(),
                eid: senaki,
                aircraft: Some(v1::StockClass {
                    unlimited: true,
                    entries: vec![],
                }),
                weapons: Some(v1::StockClass {
                    unlimited: false,
                    entries: vec![v1::StockEntry {
                        name: "weapons.missiles.AIM_120C".into(),
                        count: 20.0,
                    }],
                }),
                liquids: Some(v1::LiquidStock {
                    unlimited: false,
                    amounts: vec![v1::LiquidAmount {
                        liquid: v1::Liquid::JetFuel as i32,
                        amount: 1500.0,
                        ..Default::default()
                    }],
                }),
                ..Default::default()
            }],
        }),
        ..Default::default()
    };
    let item = |n: &str| v1::WarehouseItem {
        item: Some(v1::warehouse_item::Item::Name(n.into())),
    };
    let counted = v1::Reply {
        warehouse_stock: Some(v1::WarehousesStock {
            warehouses: vec![
                v1::WarehouseStock {
                    name: "Senaki-Kolkhi".into(),
                    eid: senaki,
                    items: vec![v1::ItemCount {
                        item: Some(item("weapons.missiles.AIM_120C")),
                        count: Some(14.0),
                        unlimited: false,
                    }],
                    ..Default::default()
                },
                v1::WarehouseStock {
                    name: "Kutaisi".into(),
                    err: Some(v1::CommandError {
                        code: v1::ErrorCode::Gone as i32,
                        message: "the warehouse of `Kutaisi` does not exist".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            ],
        }),
        ..Default::default()
    };
    let local = tokio::task::LocalSet::new();
    local
        .run_until(run_stock_reader(3, vec![whole, counted], 2))
        .await;
    let got = SEEN.with(|v| v.borrow().clone());
    assert_eq!(
        got,
        [
            "sent min_api_rev 3 key \"\" warehouses 1 items 0 liquids false",
            "whole aircraft Some(Unlimited) weapons {\"weapons.missiles.AIM_120C\": 20.0} jet \
             fuel Some(Counted(1500.0))",
            "sent min_api_rev 3 key \"\" warehouses 2 items 1 liquids true",
            "batch ok 1 counts [Some(Counted(14.0)), None] second Some(\"sortie::gone\") eid \
             Some(3)",
        ]
    );
}

/// A logic of API revision 3 that changes warehouse stock (wave 2, `ctx.warehouse_op`): a
/// transfer, a set of a liquid and an add, keyed.
#[sortie::mission(api_rev = 3)]
async fn stock_mover(ctx: Ctx<Demo>, _start: Start) -> sortie::Result<()> {
    use sortie::cmd::{Liquid, StockItem, StockOutcome};
    use sortie::sel::{AirbaseSel, WarehouseSel};
    let red = WarehouseSel::airbase(AirbaseSel::name("Senaki-Kolkhi"));
    let depot = WarehouseSel::named("Kobuleti");
    let out = ctx
        .warehouse_op()
        .transfer(red.clone(), depot.clone(), "weapons.missiles.AIM_120C", 6.0)
        .set(depot, Liquid::JetFuel, 5_000.0)
        .add(red, [4, 4, 7, 106], 1.0)
        .key("move-1")
        .await;
    match out {
        Ok(o) => {
            let c = &o.changes[0];
            seen(format!(
                "applied {} outcomes {:?} what {:?} from {} {:?}->{:?} eid {:?} to {:?} \
                 failed {}",
                o.applied,
                o.changes.iter().map(|c| c.outcome).collect::<Vec<_>>(),
                o.changes
                    .iter()
                    .map(|c| c.what.clone())
                    .collect::<Vec<StockItem>>()[1],
                c.warehouse.name,
                c.warehouse.before,
                c.warehouse.after,
                c.warehouse.eid.map(Eid::index),
                c.to.as_ref().map(|t| (t.name.clone(), t.before, t.after)),
                o.failed()
                    .filter(|c| c.outcome == StockOutcome::NotEnough)
                    .count()
            ));
        }
        Err(e) => seen(format!("op: {}", e.code())),
    }
    Ok(())
}

/// The scripted engine's side of [`stock_mover`]: one WarehouseOp answered with `reply`.
async fn run_stock_mover(api_rev: u32, reply: Option<v1::Reply>) {
    let (up_tx, mut up_rx) = mpsc::unbounded_channel();
    let (down_tx, down_rx) = mpsc::channel(256);
    let handle = spawn_logic::<Demo, _>(
        LogicConn {
            up: up_tx,
            down: down_rx,
        },
        stock_mover,
        AttachOptions::default(),
    );
    let logic_up::Msg::Attach(attach) = next_up(&mut up_rx).await else {
        panic!("no Attach")
    };
    assert_eq!(attach.api_rev, 3);
    down_tx
        .try_send(down(logic_down::Msg::Start(v1::Start {
            session: "s1".into(),
            api_rev,
            ..Default::default()
        })))
        .unwrap();
    let said = || {
        SEEN.with(|v| {
            v.borrow()
                .iter()
                .filter(|s| !s.starts_with("sent "))
                .count()
        })
    };
    let mut reply = reply;
    while said() < 1 && !handle.is_finished() {
        tokio::select! {
            msg = up_rx.recv() => match msg.and_then(|m| m.msg) {
                Some(logic_up::Msg::Command(cmd)) => {
                    let Some(v1::command::Body::WarehouseOp(w)) = &cmd.body else {
                        panic!("{cmd:?}")
                    };
                    let kinds: Vec<String> = w
                        .changes
                        .iter()
                        .map(|c| match &c.change {
                            Some(v1::stock_change::Change::Set(n)) => format!("set {n}"),
                            Some(v1::stock_change::Change::Add(n)) => format!("add {n}"),
                            Some(v1::stock_change::Change::Remove(n)) => format!("remove {n}"),
                            Some(v1::stock_change::Change::Transfer(t)) => {
                                format!("transfer {} to {:?}", t.amount, t.to)
                            }
                            None => "none".into(),
                        })
                        .collect();
                    seen(format!(
                        "sent min_api_rev {} key {:?} {kinds:?} liquid {:?}",
                        cmd.min_api_rev,
                        cmd.key,
                        w.changes[1].stock
                    ));
                    down_tx
                        .try_send(down(logic_down::Msg::Result(v1::CommandResult {
                            id: cmd.id,
                            outcome: Some(v1::command_result::Outcome::Ok(
                                sortie::proto::LuaValue::number(3.0),
                            )),
                            reply: reply.take(),
                            ..Default::default()
                        })))
                        .unwrap();
                }
                Some(_) => {}
                None => break,
            },
            () = tokio::task::yield_now() => {}
        }
    }
    down_tx
        .try_send(down(logic_down::Msg::Shutdown(v1::Shutdown::default())))
        .unwrap();
    while !handle.is_finished() {
        tokio::task::yield_now().await;
    }
    handle.assert_clean();
}

/// WarehouseOp is API revision 3: against an engine of revision 2 it fails with
/// `EngineTooOld` and nothing is sent.
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn the_warehouse_op_against_an_engine_of_revision_2_sends_nothing() {
    SEEN.with(|v| v.borrow_mut().clear());
    let local = tokio::task::LocalSet::new();
    local.run_until(run_stock_mover(2, None)).await;
    let got = SEEN.with(|v| v.borrow().clone());
    assert_eq!(got, ["op: sortie::engine_too_old"]);
}

/// Against an engine of revision 3 the op carries `min_api_rev = 3`, the key and the changes in
/// order (a liquid by its enum); the typed reply resolves it: applied, each change's outcome,
/// what it changed as given, the sides with their names, Eids and counts.
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn the_warehouse_op_sends_revision_3_and_resolves_with_the_typed_reply() {
    SEEN.with(|v| v.borrow_mut().clear());
    let senaki = Eid::from_parts(3, 1).unwrap().to_u64();
    let side = |name: &str, eid: u64, before: f64, after: f64| v1::StockSide {
        name: name.into(),
        eid,
        before: Some(before),
        after: Some(after),
        ..Default::default()
    };
    let done = v1::StockOutcome::Done as i32;
    let reply = v1::Reply {
        warehouse_op: Some(v1::StockChanges {
            applied: true,
            undo_failed: false,
            changes: vec![
                v1::StockChanged {
                    outcome: done,
                    warehouse: Some(side("Senaki-Kolkhi", senaki, 20.0, 14.0)),
                    to: Some(side("Kobuleti", 0, 0.0, 6.0)),
                    ..Default::default()
                },
                v1::StockChanged {
                    outcome: done,
                    warehouse: Some(side("Kobuleti", 0, 0.0, 5000.0)),
                    ..Default::default()
                },
                v1::StockChanged {
                    outcome: done,
                    warehouse: Some(side("Senaki-Kolkhi", senaki, 7.0, 8.0)),
                    ..Default::default()
                },
            ],
        }),
        ..Default::default()
    };
    let local = tokio::task::LocalSet::new();
    local.run_until(run_stock_mover(3, Some(reply))).await;
    let got = SEEN.with(|v| v.borrow().clone());
    assert_eq!(got.len(), 2, "{got:?}");
    assert_eq!(
        got[0],
        "sent min_api_rev 3 key \"move-1\" [\"transfer 6 to Some(WarehouseSel { sel: \
         Some(DcsName(\\\"Kobuleti\\\")) })\", \"set 5000\", \"add 1\"] liquid \
         Some(Liquid(JetFuel))"
    );
    assert_eq!(
        got[1],
        "applied true outcomes [Done, Done, Done] what Liquid(JetFuel) from Senaki-Kolkhi \
         Some(20.0)->Some(14.0) eid Some(3) to Some((\"Kobuleti\", Some(0.0), Some(6.0))) \
         failed 0"
    );
}

/// A logic of API revision 3 that looks up the names of items (wave 2, `ctx.item_names`): a
/// name and a wsType.
#[sortie::mission(api_rev = 3)]
async fn namer(ctx: Ctx<Demo>, _start: Start) -> sortie::Result<()> {
    use sortie::cmd::WarehouseItem;
    let out = ctx
        .item_names()
        .items([
            WarehouseItem::from("weapons.missiles.TEST_ONE"),
            WarehouseItem::from([4, 4, 7, 22]),
        ])
        .await;
    match out {
        Ok(n) => seen(format!(
            "names {:?} ws {:?} of {:?} unknown {} total {} cached {} dcs {}",
            n.names.iter().map(|x| x.name.as_str()).collect::<Vec<_>>(),
            n.ws_type_of("weapons.missiles.TEST_ONE"),
            n.names_of([4, 4, 7, 22]).collect::<Vec<_>>(),
            n.unknown.len(),
            n.total,
            n.cached,
            n.dcs_version
        )),
        Err(e) => seen(format!("names: {}", e.code())),
    }
    Ok(())
}

/// The scripted engine's side of [`namer`]: one names query answered with `reply`.
async fn run_namer(api_rev: u32, reply: Option<v1::Reply>) {
    let (up_tx, mut up_rx) = mpsc::unbounded_channel();
    let (down_tx, down_rx) = mpsc::channel(256);
    let handle = spawn_logic::<Demo, _>(
        LogicConn {
            up: up_tx,
            down: down_rx,
        },
        namer,
        AttachOptions::default(),
    );
    let logic_up::Msg::Attach(attach) = next_up(&mut up_rx).await else {
        panic!("no Attach")
    };
    assert_eq!(attach.api_rev, 3);
    down_tx
        .try_send(down(logic_down::Msg::Start(v1::Start {
            session: "s1".into(),
            api_rev,
            ..Default::default()
        })))
        .unwrap();
    let said = || {
        SEEN.with(|v| {
            v.borrow()
                .iter()
                .filter(|s| !s.starts_with("sent "))
                .count()
        })
    };
    let mut reply = reply;
    while said() < 1 && !handle.is_finished() {
        tokio::select! {
            msg = up_rx.recv() => match msg.and_then(|m| m.msg) {
                Some(logic_up::Msg::Command(cmd)) => {
                    let Some(v1::command::Body::Query(v1::Query {
                        kind: Some(v1::query::Kind::ItemNames(q)),
                    })) = &cmd.body
                    else {
                        panic!("{cmd:?}")
                    };
                    seen(format!(
                        "sent min_api_rev {} key {:?} items {}",
                        cmd.min_api_rev,
                        cmd.key,
                        q.items.len()
                    ));
                    down_tx
                        .try_send(down(logic_down::Msg::Result(v1::CommandResult {
                            id: cmd.id,
                            outcome: Some(v1::command_result::Outcome::Ok(
                                sortie::proto::LuaValue::number(2.0),
                            )),
                            reply: reply.take(),
                            ..Default::default()
                        })))
                        .unwrap();
                }
                Some(_) => {}
                None => break,
            },
            () = tokio::task::yield_now() => {}
        }
    }
    down_tx
        .try_send(down(logic_down::Msg::Shutdown(v1::Shutdown::default())))
        .unwrap();
    while !handle.is_finished() {
        tokio::task::yield_now().await;
    }
    handle.assert_clean();
}

/// The names query is API revision 3: against an engine of revision 2 it fails with
/// `EngineTooOld` and nothing is sent.
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn the_names_query_against_an_engine_of_revision_2_sends_nothing() {
    SEEN.with(|v| v.borrow_mut().clear());
    let local = tokio::task::LocalSet::new();
    local.run_until(run_namer(2, None)).await;
    let got = SEEN.with(|v| v.borrow().clone());
    assert_eq!(got, ["names: sortie::engine_too_old"]);
}

/// Against an engine of revision 3 the query carries `min_api_rev = 3`, no key and its lookups;
/// the typed reply resolves it: the names with their wsTypes, the lookups between them, the
/// unknown items, the size of DCS's map, whether it came from the engine's memory.
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn the_names_query_sends_revision_3_and_resolves_with_the_typed_reply() {
    SEEN.with(|v| v.borrow_mut().clear());
    let ws = |w: [u32; 4]| v1::WsType {
        level1: w[0],
        level2: w[1],
        level3: w[2],
        level4: w[3],
    };
    let reply = v1::Reply {
        item_names: Some(v1::ItemNameList {
            names: vec![
                v1::ItemName {
                    name: "weapons.missiles.TEST_ONE".into(),
                    ws_type: Some(ws([4, 4, 7, 106])),
                },
                v1::ItemName {
                    name: "weapons.missiles.TEST_TWO".into(),
                    ws_type: Some(ws([4, 4, 7, 22])),
                },
            ],
            unknown: vec![],
            total: 900,
            skipped: 3,
            dcs_version: "2.9.31.1".into(),
            cached: true,
            source: "Warehouse.getResourceMap()".into(),
        }),
        ..Default::default()
    };
    let local = tokio::task::LocalSet::new();
    local.run_until(run_namer(3, Some(reply))).await;
    let got = SEEN.with(|v| v.borrow().clone());
    assert_eq!(
        got,
        [
            "sent min_api_rev 3 key \"\" items 2",
            "names [\"weapons.missiles.TEST_ONE\", \"weapons.missiles.TEST_TWO\"] ws \
             Some([4, 4, 7, 106]) of [\"weapons.missiles.TEST_TWO\"] unknown 0 total 900 \
             cached true dcs 2.9.31.1",
        ]
    );
}

/// A logic of API revision 3 that spawns a ground pair turned as one formation, then the same
/// template with `.at(..)` (0.4.0, `SpawnAt.heading_mode`).
#[sortie::mission(api_rev = 3)]
async fn formation_spawner(ctx: Ctx<Demo>, _start: Start) -> sortie::Result<()> {
    use sortie::types::{Ground, TemplateRef};
    const PAIR: TemplateRef<Demo, Ground> = TemplateRef::new("SAM_PAIR");
    let p = Vec3::new(-30_000.0, 0.0, 460_000.0);
    match ctx
        .spawn_group(PAIR)
        .at_formation(p, 3.0)
        .key("pair-1")
        .await
    {
        Ok(g) => seen(format!("formation {}", g.name())),
        Err(e) => seen(format!("formation: {}", e.code())),
    }
    match ctx.spawn_group(PAIR).at(p, 3.0).key("pair-2").await {
        Ok(g) => seen(format!("at {}", g.name())),
        Err(e) => seen(format!("at: {}", e.code())),
    }
    Ok(())
}

/// The scripted engine's side of [`formation_spawner`]: each spawn noted with its revision and
/// heading mode, and answered with a group.
async fn run_formation_spawner(api_rev: u32) {
    let (up_tx, mut up_rx) = mpsc::unbounded_channel();
    let (down_tx, down_rx) = mpsc::channel(256);
    let handle = spawn_logic::<Demo, _>(
        LogicConn {
            up: up_tx,
            down: down_rx,
        },
        formation_spawner,
        AttachOptions::default(),
    );
    let logic_up::Msg::Attach(attach) = next_up(&mut up_rx).await else {
        panic!("no Attach")
    };
    assert_eq!(attach.api_rev, 3);
    down_tx
        .try_send(down(logic_down::Msg::Start(v1::Start {
            session: "s1".into(),
            api_rev,
            ..Default::default()
        })))
        .unwrap();
    let said = || {
        SEEN.with(|v| {
            v.borrow()
                .iter()
                .filter(|s| !s.starts_with("sent "))
                .count()
        })
    };
    let mut n = 0;
    while said() < 2 && !handle.is_finished() {
        tokio::select! {
            msg = up_rx.recv() => match msg.and_then(|m| m.msg) {
                Some(logic_up::Msg::Command(cmd)) => {
                    let Some(v1::command::Body::SpawnGroup(s)) = &cmd.body else {
                        panic!("{cmd:?}")
                    };
                    let Some(v1::spawn_group::Placement::At(at)) = &s.placement else {
                        panic!("{s:?}")
                    };
                    seen(format!(
                        "sent min_api_rev {} key {:?} heading {} mode {}",
                        cmd.min_api_rev, cmd.key, at.heading_rad, at.heading_mode
                    ));
                    n += 1;
                    let name = format!("SAM Pair#s1-{n}");
                    let mut ok = sortie::proto::LuaTable::new();
                    ok.set("name", name.as_str());
                    down_tx
                        .try_send(down(logic_down::Msg::Result(v1::CommandResult {
                            id: cmd.id,
                            eid: eid(n).into(),
                            outcome: Some(v1::command_result::Outcome::Ok(
                                sortie::proto::LuaValue::table(ok),
                            )),
                            ..Default::default()
                        })))
                        .unwrap();
                }
                Some(_) => {}
                None => break,
            },
            () = tokio::task::yield_now() => {}
        }
    }
    down_tx
        .try_send(down(logic_down::Msg::Shutdown(v1::Shutdown::default())))
        .unwrap();
    while !handle.is_finished() {
        tokio::task::yield_now().await;
    }
    handle.assert_clean();
}

/// A formation spawn is API revision 3: against an engine of revision 2 it fails with
/// `EngineTooOld` and nothing is sent; `.at(..)` still goes out as 0.1.0's command (no mode,
/// no minimum revision).
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn a_formation_spawn_against_an_engine_of_revision_2_sends_nothing() {
    SEEN.with(|v| v.borrow_mut().clear());
    let local = tokio::task::LocalSet::new();
    local.run_until(run_formation_spawner(2)).await;
    let got = SEEN.with(|v| v.borrow().clone());
    assert_eq!(
        got,
        [
            "formation: sortie::engine_too_old",
            "sent min_api_rev 0 key \"pair-2\" heading 3 mode 0",
            "at SAM Pair#s1-1",
        ]
    );
}

/// Against an engine of revision 3 the formation spawn carries `min_api_rev = 3` and the mode
/// FORMATION; `.at(..)` sends neither.
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn a_formation_spawn_sends_revision_3_and_its_mode() {
    SEEN.with(|v| v.borrow_mut().clear());
    let local = tokio::task::LocalSet::new();
    local.run_until(run_formation_spawner(3)).await;
    let got = SEEN.with(|v| v.borrow().clone());
    assert_eq!(
        got,
        [
            "sent min_api_rev 3 key \"pair-1\" heading 3 mode 1",
            "formation SAM Pair#s1-1",
            "sent min_api_rev 0 key \"pair-2\" heading 3 mode 0",
            "at SAM Pair#s1-2",
        ]
    );
}
