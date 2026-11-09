//! The logic side of the Logic protocol against a scripted engine, on tokio's paused clock.

use sortie::prelude::*;
use sortie::proto::v1::{self, logic_down, logic_up};
use sortie::runtime::{AttachOptions, LogicConn, spawn_logic};
use sortie::types::{Scenario, Symbol, SymbolKind};
use tokio::sync::mpsc;

#[derive(Clone, Copy, Debug, Default)]
struct Demo;

impl Scenario for Demo {
    const NAME: &'static str = "demo";
    const SCHEMA: u32 = 1;
    const SCHEMA_SHA256: &'static str =
        "066934cb939700310641d6371b1f6140cfc86c2c3e04757d5a10f8a4664f270b";
    const SYMBOLS: &'static [Symbol] = &[Symbol::new(SymbolKind::Flag, "DONE")];
}

async fn mission(ctx: Ctx<Demo>, start: Start) -> sortie::Result<()> {
    assert!(start.is_fresh());
    ctx.message(Coalition::Blue)
        .text("hello")
        .secs(5)
        .key("hello")
        .await?;
    ctx.wait_until(|w| w.model_time() >= 1.0).await;
    ctx.end_session(Outcome::BlueWin).text("done").await
}

fn down(msg: logic_down::Msg) -> v1::LogicDown {
    v1::LogicDown { msg: Some(msg) }
}

async fn next_command(up: &mut mpsc::UnboundedReceiver<v1::LogicUp>) -> v1::Command {
    loop {
        match up.recv().await.and_then(|m| m.msg) {
            Some(logic_up::Msg::Command(c)) => return c,
            Some(_) => {}
            None => panic!("logic hung up"),
        }
    }
}

#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn mission_runs_against_a_scripted_engine() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let (up_tx, mut up_rx) = mpsc::unbounded_channel();
            let (down_tx, down_rx) = mpsc::channel(256);
            let opts = AttachOptions {
                logic: "demo".into(),
                ..AttachOptions::default()
            };
            let handle = spawn_logic::<Demo, _>(
                LogicConn {
                    up: up_tx,
                    down: down_rx,
                },
                mission,
                opts,
            );

            let Some(logic_up::Msg::Attach(attach)) = up_rx.recv().await.and_then(|m| m.msg) else {
                panic!("no Attach")
            };
            assert_eq!(attach.scenario, "demo");
            assert_eq!(attach.symbols_used, ["flag:DONE"]);
            assert_eq!(attach.scenario_schema_sha256.len(), 32);

            down_tx
                .try_send(down(logic_down::Msg::Start(v1::Start {
                    session: "01SORTIE000000000000000001".into(),
                    kind: v1::StartKind::Fresh as i32,
                    turn: "build-1".into(),
                    ..Default::default()
                })))
                .unwrap();

            let msg = next_command(&mut up_rx).await;
            assert_eq!(msg.key, "hello");
            assert_eq!(msg.session, "01SORTIE000000000000000001");
            assert!(
                msg.origin
                    .as_ref()
                    .is_some_and(|o| o.file.ends_with("runtime.rs"))
            );
            let Some(v1::command::Body::Message(m)) = &msg.body else {
                panic!("{msg:?}")
            };
            assert_eq!((m.text.as_str(), m.seconds), ("hello", 5));
            down_tx
                .try_send(down(logic_down::Msg::Result(v1::CommandResult {
                    id: msg.id,
                    ..Default::default()
                })))
                .unwrap();

            down_tx
                .try_send(down(logic_down::Msg::Tick(v1::Tick {
                    n: 1,
                    model_time: 1.0,
                    ..Default::default()
                })))
                .unwrap();
            let end = next_command(&mut up_rx).await;
            let Some(v1::command::Body::EndSession(e)) = &end.body else {
                panic!("{end:?}")
            };
            assert_eq!(e.outcome, v1::Outcome::BlueWin as i32);
            down_tx
                .try_send(down(logic_down::Msg::Result(v1::CommandResult {
                    id: end.id,
                    ..Default::default()
                })))
                .unwrap();
            down_tx
                .try_send(down(logic_down::Msg::Ended(v1::SessionEnded {
                    session: "01SORTIE000000000000000001".into(),
                    outcome: v1::Outcome::BlueWin as i32,
                    reason: v1::EndReason::Logic as i32,
                    ..Default::default()
                })))
                .unwrap();
            down_tx
                .try_send(down(logic_down::Msg::Shutdown(v1::Shutdown::default())))
                .unwrap();
            while !handle.is_finished() {
                tokio::task::yield_now().await;
            }
            assert_eq!(handle.outcome(), Some(Outcome::BlueWin));
            handle.assert_clean();
        })
        .await;
}

#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn command_errors_surface_as_typed_errors() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let (up_tx, mut up_rx) = mpsc::unbounded_channel();
            let (down_tx, down_rx) = mpsc::channel(256);
            let failing = async |ctx: Ctx<Demo>, _start: Start| -> sortie::Result<()> {
                match ctx
                    .flag(sortie::types::FlagRef::new("DONE"))
                    .set(true)
                    .await
                {
                    Err(sortie::Error::Gone(f)) => {
                        assert_eq!(f.op, "flag_set");
                        Ok(())
                    }
                    other => panic!("{other:?}"),
                }
            };
            let handle = spawn_logic::<Demo, _>(
                LogicConn {
                    up: up_tx,
                    down: down_rx,
                },
                failing,
                AttachOptions::default(),
            );
            down_tx
                .try_send(down(logic_down::Msg::Start(v1::Start {
                    session: "s".into(),
                    ..Default::default()
                })))
                .unwrap();
            let cmd = next_command(&mut up_rx).await;
            let err = v1::CommandError {
                code: v1::ErrorCode::Gone as i32,
                message: "x".into(),
                lua_traceback: String::new(),
            };
            down_tx
                .try_send(down(logic_down::Msg::Result(v1::CommandResult {
                    id: cmd.id,
                    outcome: Some(v1::command_result::Outcome::Err(err)),
                    ..Default::default()
                })))
                .unwrap();
            for _ in 0..10 {
                tokio::task::yield_now().await;
            }
            handle.assert_clean();
        })
        .await;
}

async fn next_state_op(up: &mut mpsc::UnboundedReceiver<v1::LogicUp>) -> v1::StateOp {
    loop {
        match up.recv().await.and_then(|m| m.msg) {
            Some(logic_up::Msg::State(op)) => return op,
            Some(_) => {}
            None => panic!("logic hung up"),
        }
    }
}

fn state_reply(id: u64, result: v1::state_result::Result) -> v1::LogicDown {
    down(logic_down::Msg::State(v1::StateResult {
        id,
        result: Some(result),
    }))
}

#[derive(Default, Serialize, Deserialize)]
struct Counter {
    n: u32,
}

async fn bump(ctx: Ctx<Demo>, _start: Start) -> sortie::Result<()> {
    let counter = ctx.state::<Counter>("counter").await?;
    counter.update(|c| c.n += 1).await?;
    assert_eq!((counter.get().n, counter.revision()), (6, 4));
    ctx.end_session(Outcome::Draw).await
}

/// `StateDoc::update` is compare-and-swap: on CONFLICT it re-reads and re-applies.
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn state_update_retries_after_a_conflict() {
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
                bump,
                AttachOptions::default(),
            );
            down_tx
                .try_send(down(logic_down::Msg::Start(v1::Start {
                    session: "s".into(),
                    ..Default::default()
                })))
                .unwrap();
            let doc = |revision: u64, n: u32| {
                v1::state_result::Result::Doc(v1::StateDoc {
                    revision,
                    json: serde_json::to_vec(&Counter { n }).unwrap(),
                    ..Default::default()
                })
            };
            // Initial read: not stored yet.
            let get = next_state_op(&mut up_rx).await;
            assert!(matches!(get.op, Some(v1::state_op::Op::Get(_))));
            down_tx
                .try_send(state_reply(
                    get.id,
                    v1::state_result::Result::Doc(v1::StateDoc::default()),
                ))
                .unwrap();
            // First write loses the race.
            let put = next_state_op(&mut up_rx).await;
            let Some(v1::state_op::Op::Put(p)) = &put.op else {
                panic!("{put:?}")
            };
            assert_eq!(p.expected_revision, 0);
            down_tx
                .try_send(state_reply(
                    put.id,
                    v1::state_result::Result::Err(v1::CommandError {
                        code: v1::ErrorCode::Conflict as i32,
                        message: "revision 3 is newer".into(),
                        lua_traceback: String::new(),
                    }),
                ))
                .unwrap();
            // Re-read, re-apply on the newer value, write again.
            let get = next_state_op(&mut up_rx).await;
            down_tx.try_send(state_reply(get.id, doc(3, 5))).unwrap();
            let put = next_state_op(&mut up_rx).await;
            let Some(v1::state_op::Op::Put(p)) = &put.op else {
                panic!("{put:?}")
            };
            assert_eq!(p.expected_revision, 3);
            let written: serde_json::Value = serde_json::from_slice(&p.json).unwrap();
            assert_eq!(written, serde_json::json!({"n": 6}));
            down_tx.try_send(state_reply(put.id, doc(4, 6))).unwrap();

            let end = next_command(&mut up_rx).await;
            assert!(matches!(end.body, Some(v1::command::Body::EndSession(_))));
            down_tx
                .try_send(down(logic_down::Msg::Result(v1::CommandResult {
                    id: end.id,
                    ..Default::default()
                })))
                .unwrap();
            for _ in 0..10 {
                tokio::task::yield_now().await;
            }
            handle.assert_clean();
        })
        .await;
}

async fn waits_forever(ctx: Ctx<Demo>, _start: Start) -> sortie::Result<()> {
    // Never answered: the session ends first.
    ctx.message(Coalition::Red)
        .text("never shown")
        .secs(5)
        .key("never")
        .await?;
    unreachable!("the message cannot succeed after the session ended");
}

/// A session that ends under a pending command fails it with `SessionEnded`, which is the
/// normal way out of a mission and not a logic error.
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn session_end_fails_pending_commands_quietly() {
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
                waits_forever,
                AttachOptions::default(),
            );
            down_tx
                .try_send(down(logic_down::Msg::Start(v1::Start {
                    session: "s".into(),
                    ..Default::default()
                })))
                .unwrap();
            let msg = next_command(&mut up_rx).await;
            assert_eq!(msg.key, "never");
            down_tx
                .try_send(down(logic_down::Msg::Ended(v1::SessionEnded {
                    session: "s".into(),
                    outcome: v1::Outcome::TimeLimit as i32,
                    reason: v1::EndReason::TimeLimit as i32,
                    ..Default::default()
                })))
                .unwrap();
            for _ in 0..10 {
                tokio::task::yield_now().await;
            }
            assert_eq!(handle.outcome(), Some(Outcome::TimeLimit));
            assert_eq!(handle.sessions(), 1);
            handle.assert_clean();
        })
        .await;
}

async fn waits_for_a_red_plane(ctx: Ctx<Demo>, _start: Start) -> sortie::Result<()> {
    ctx.wait_until(|w| w.units().coalition(Coalition::Red).air().any())
        .await;
    ctx.end_session(Outcome::RedWin).await
}

/// A tick whose delta does not continue the replica's revision makes the logic ask for a
/// resync instead of failing; the engine's snapshot replaces the replica and ticks go on.
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn a_revision_gap_is_healed_by_a_resync_snapshot() {
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
                waits_for_a_red_plane,
                AttachOptions::default(),
            );
            let snapshot = |rev: u64, entities: Vec<v1::EntitySnap>| v1::WorldSnapshot {
                session: "s".into(),
                rev,
                entities,
                ..Default::default()
            };
            down_tx
                .try_send(down(logic_down::Msg::Start(v1::Start {
                    session: "s".into(),
                    snapshot: Some(snapshot(5, vec![])),
                    ..Default::default()
                })))
                .unwrap();
            let su27 = v1::EntitySnap {
                eid: sortie::proto::eid::pack(1, 1),
                name: "Red CAP 1-1".into(),
                type_name: "Su-27".into(),
                coalition: v1::Coalition::Red as i32,
                category: v1::Category::Airplane as i32,
                ..Default::default()
            };
            let ground = v1::EntitySnap {
                eid: sortie::proto::eid::pack(2, 1),
                name: "Truck-1".into(),
                coalition: v1::Coalition::Red as i32,
                category: v1::Category::GroundUnit as i32,
                ..Default::default()
            };
            // The delta starts at 7: revisions 5..7 were never seen.
            down_tx
                .try_send(down(logic_down::Msg::Tick(v1::Tick {
                    n: 1,
                    model_time: 0.1,
                    delta: Some(v1::WorldDelta {
                        from_rev: 7,
                        rev: 8,
                        upserts: vec![ground.clone()],
                        ..Default::default()
                    }),
                    ..Default::default()
                })))
                .unwrap();
            let resync = loop {
                match up_rx.recv().await.and_then(|m| m.msg) {
                    Some(logic_up::Msg::Resync(r)) => break r,
                    Some(_) => {}
                    None => panic!("logic hung up"),
                }
            };
            assert_eq!(resync.have_rev, 5);
            assert!(resync.reason.contains('7'), "{}", resync.reason);
            // A second stale tick before the snapshot does not ask again.
            down_tx
                .try_send(down(logic_down::Msg::Tick(v1::Tick {
                    n: 2,
                    model_time: 0.2,
                    delta: Some(v1::WorldDelta {
                        from_rev: 8,
                        rev: 9,
                        upserts: vec![su27.clone()],
                        ..Default::default()
                    }),
                    ..Default::default()
                })))
                .unwrap();
            down_tx
                .try_send(down(logic_down::Msg::Resync(snapshot(
                    9,
                    vec![ground, su27],
                ))))
                .unwrap();
            down_tx
                .try_send(down(logic_down::Msg::Tick(v1::Tick {
                    n: 3,
                    model_time: 0.3,
                    delta: Some(v1::WorldDelta {
                        from_rev: 9,
                        rev: 9,
                        ..Default::default()
                    }),
                    ..Default::default()
                })))
                .unwrap();
            let end = next_command(&mut up_rx).await;
            assert!(
                matches!(end.body, Some(v1::command::Body::EndSession(_))),
                "{end:?}"
            );
            let mut resyncs = 0;
            while let Ok(m) = up_rx.try_recv() {
                if matches!(m.msg, Some(logic_up::Msg::Resync(_))) {
                    resyncs += 1;
                }
            }
            assert_eq!(resyncs, 0, "only one resync request");
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

async fn two_messages(ctx: Ctx<Demo>, _start: Start) -> sortie::Result<()> {
    let first = ctx.message(Coalition::Blue).text("first").secs(5);
    let second = ctx
        .message(Coalition::Blue)
        .text("second")
        .secs(5)
        .key("second")
        .after(&first);
    first.await?;
    second.await?;
    Ok(())
}

/// The (id, after) of the commands one logic sends in session `01SORTIE…0001`, in order.
async fn ids_of_one_logic(opts: AttachOptions) -> Vec<(String, String)> {
    let (up_tx, mut up_rx) = mpsc::unbounded_channel();
    let (down_tx, down_rx) = mpsc::channel(256);
    let handle = spawn_logic::<Demo, _>(
        LogicConn {
            up: up_tx,
            down: down_rx,
        },
        two_messages,
        opts,
    );
    down_tx
        .try_send(down(logic_down::Msg::Start(v1::Start {
            session: "01SORTIE000000000000000001".into(),
            ..Default::default()
        })))
        .unwrap();
    let mut ids = Vec::new();
    for _ in 0..2 {
        let cmd = next_command(&mut up_rx).await;
        ids.push((cmd.id.clone(), cmd.after.clone()));
        down_tx
            .try_send(down(logic_down::Msg::Result(v1::CommandResult {
                id: cmd.id,
                ..Default::default()
            })))
            .unwrap();
    }
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }
    handle.assert_clean();
    ids
}

/// Command ids are `<session4>-<proc>-<n>`: two logic processes in one session (a restarted
/// logic counts from 1 again) never share an id, so `.after(..)` names this process's command.
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn command_ids_differ_between_logic_processes() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let fixed = ids_of_one_logic(AttachOptions {
                proc_token: "k3x9q2".into(),
                ..AttachOptions::default()
            })
            .await;
            assert_eq!(
                fixed,
                [
                    ("0001-k3x9q2-1".to_owned(), String::new()),
                    ("0001-k3x9q2-2".to_owned(), "0001-k3x9q2-1".to_owned()),
                ]
            );

            let a = ids_of_one_logic(AttachOptions::default()).await;
            let b = ids_of_one_logic(AttachOptions::default()).await;
            for ids in [&a, &b] {
                let (id, _) = &ids[0];
                let parts: Vec<&str> = id.split('-').collect();
                assert_eq!(parts.len(), 3, "{id}");
                assert_eq!((parts[0], parts[2]), ("0001", "1"), "{id}");
                assert_eq!(parts[1].len(), sortie::runtime::PROC_TOKEN_LEN, "{id}");
                assert_eq!(ids[1].1, *id, "`after` names the first command");
            }
            assert_ne!(a[0].0, b[0].0, "a drawn token per process");
        })
        .await;
}
