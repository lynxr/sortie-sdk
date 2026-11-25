//! The SDK side of web API actions (`ctx.api().action(..)`) against a scripted engine: what
//! goes up the Logic stream (the declared set, each answer) for each way a logic answers.

use std::cell::RefCell;
use std::collections::BTreeMap;

use sortie::prelude::*;
use sortie::proto::v1::{self, action_reply::Answer, logic_down, logic_up};
use sortie::runtime::{AttachOptions, LogicConn, spawn_logic};
use sortie::types::{Scenario, Symbol};
use tokio::sync::mpsc;

#[derive(Clone, Copy, Debug, Default)]
struct Demo;

impl Scenario for Demo {
    const NAME: &'static str = "demo";
    const SCHEMA: u32 = 1;
    const SCHEMA_SHA256: &'static str =
        "066934cb939700310641d6371b1f6140cfc86c2c3e04757d5a10f8a4664f270b";
    const SYMBOLS: &'static [Symbol] = &[];
}

thread_local! {
    static SEEN: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn note(s: impl Into<String>) {
    SEEN.with(|v| v.borrow_mut().push(s.into()));
}

fn outcome(r: sortie::Result<()>) -> String {
    match r {
        Ok(()) => "ok".into(),
        Err(e) => e.code().into(),
    }
}

#[derive(serde::Deserialize)]
struct Go {
    to: String,
}

#[sortie::mission(api_rev = 3)]
async fn serve(ctx: Ctx<Demo>, _start: Start) -> sortie::Result<()> {
    let mut go = ctx.api().action("go")?;
    drop(ctx.api().action("tmp")?);
    while let Some(req) = go.next().await {
        let who = format!("{:?} {}", req.token(), req.role());
        match req.body() {
            b"\"bad status\"" => note(format!("503 {}", outcome(req.fail(503, "busy")))),
            b"\"weird\"" => {
                // A map whose keys are not strings is no JSON object.
                let mut weird = BTreeMap::new();
                weird.insert(vec![1], 2);
                note(format!("weird {}", outcome(req.reply(&weird))));
            }
            b"\"drop\"" => drop(req),
            _ => match req.json::<Go>() {
                Ok(g) => req.reply(&serde_json::json!({"to": g.to, "who": who}))?,
                Err(e) => req.fail(422, e)?,
            },
        }
    }
    Ok(())
}

fn down(msg: logic_down::Msg) -> v1::LogicDown {
    v1::LogicDown { msg: Some(msg) }
}

fn call(id: u64, name: &str, json: &str) -> v1::LogicDown {
    down(logic_down::Msg::Action(v1::ActionCall {
        id,
        name: name.into(),
        json: json.as_bytes().to_vec(),
        token: if id == 1 {
            "site".into()
        } else {
            String::new()
        },
        role: "operator".into(),
    }))
}

/// The next declared set or answer the logic sends.
async fn next(up: &mut mpsc::UnboundedReceiver<v1::LogicUp>) -> logic_up::Msg {
    loop {
        match up.recv().await.and_then(|m| m.msg) {
            Some(m @ (logic_up::Msg::Actions(_) | logic_up::Msg::ActionReply(_))) => return m,
            Some(_) => {}
            None => panic!("the logic hung up"),
        }
    }
}

async fn declared(up: &mut mpsc::UnboundedReceiver<v1::LogicUp>) -> Vec<String> {
    match next(up).await {
        logic_up::Msg::Actions(set) => set.names,
        other => panic!("expected a declared set, got {other:?}"),
    }
}

async fn answer(up: &mut mpsc::UnboundedReceiver<v1::LogicUp>, id: u64) -> Answer {
    match next(up).await {
        logic_up::Msg::ActionReply(r) if r.id == id => r.answer.expect("an answer"),
        other => panic!("expected the answer of {id}, got {other:?}"),
    }
}

fn unanswered(a: Answer) -> String {
    match a {
        Answer::Unanswered(why) => why,
        other => panic!("expected `unanswered`, got {other:?}"),
    }
}

/// Declaring sends the whole set, dropping a stream withdraws it, the session's end withdraws
/// everything; each request is answered once: JSON, a refusal, or `unanswered` when the logic
/// dropped it, answered with what cannot be sent, or did not declare it.
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn requests_are_answered_once_and_declarations_follow_the_streams() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let (up_tx, mut up) = mpsc::unbounded_channel();
            let (down_tx, down_rx) = mpsc::channel(256);
            let handle = spawn_logic::<Demo, _>(
                LogicConn {
                    up: up_tx,
                    down: down_rx,
                },
                serve,
                AttachOptions {
                    logic: "demo".into(),
                    ..AttachOptions::default()
                },
            );
            let session = "01SORTIE000000000000000001";
            down_tx
                .try_send(down(logic_down::Msg::Start(v1::Start {
                    session: session.into(),
                    kind: v1::StartKind::Fresh as i32,
                    api_rev: 3,
                    ..Default::default()
                })))
                .unwrap();
            assert_eq!(declared(&mut up).await, ["go"]);
            assert_eq!(declared(&mut up).await, ["go", "tmp"]);
            assert_eq!(declared(&mut up).await, ["go"]);

            down_tx
                .try_send(call(1, "go", r#"{"to": "west"}"#))
                .unwrap();
            let Answer::Json(j) = answer(&mut up, 1).await else {
                panic!("expected JSON")
            };
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&j).unwrap(),
                serde_json::json!({"to": "west", "who": "Some(\"site\") operator"})
            );
            down_tx.try_send(call(2, "go", r#"{"to": 3}"#)).unwrap();
            let Answer::Refused(r) = answer(&mut up, 2).await else {
                panic!("expected a refusal")
            };
            assert_eq!(r.status, 422);
            assert!(r.message.contains("does not fit"), "{}", r.message);
            down_tx.try_send(call(3, "go", "\"bad status\"")).unwrap();
            assert!(unanswered(answer(&mut up, 3).await).contains("400 to 499"));
            down_tx.try_send(call(4, "go", "\"weird\"")).unwrap();
            assert!(unanswered(answer(&mut up, 4).await).contains("JSON"));
            down_tx.try_send(call(5, "go", "\"drop\"")).unwrap();
            assert!(unanswered(answer(&mut up, 5).await).contains("without an answer"));
            down_tx.try_send(call(6, "tmp", "null")).unwrap();
            unanswered(answer(&mut up, 6).await);

            down_tx
                .try_send(down(logic_down::Msg::Ended(v1::SessionEnded {
                    session: session.into(),
                    ..Default::default()
                })))
                .unwrap();
            assert!(declared(&mut up).await.is_empty());
            // No session: a call that crossed the withdrawal is answered at once.
            down_tx.try_send(call(7, "go", "null")).unwrap();
            unanswered(answer(&mut up, 7).await);

            handle.assert_clean();
            assert_eq!(
                SEEN.with(|v| v.borrow().clone()),
                ["503 sortie::invalid", "weird sortie::invalid"]
            );
        })
        .await;
}

/// A logic attached to a scripted engine, its session started: what it sends up and what
/// the engine sends down.
fn started(
    mission: impl sortie::runtime::Mission<Demo>,
) -> (
    sortie::runtime::LogicHandle,
    mpsc::UnboundedReceiver<v1::LogicUp>,
    mpsc::Sender<v1::LogicDown>,
) {
    let (up_tx, up) = mpsc::unbounded_channel();
    let (down_tx, down_rx) = mpsc::channel(256);
    let handle = spawn_logic::<Demo, _>(
        LogicConn {
            up: up_tx,
            down: down_rx,
        },
        mission,
        AttachOptions {
            logic: "demo".into(),
            ..AttachOptions::default()
        },
    );
    down_tx
        .try_send(down(logic_down::Msg::Start(v1::Start {
            session: "01SORTIE000000000000000001".into(),
            kind: v1::StartKind::Fresh as i32,
            api_rev: 3,
            ..Default::default()
        })))
        .unwrap();
    (handle, up, down_tx)
}

/// Busy for 15 s before it reads each request, then answers it with "done".
#[sortie::mission(api_rev = 3)]
async fn slow(ctx: Ctx<Demo>, _start: Start) -> sortie::Result<()> {
    let mut go = ctx.api().action("go")?;
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(15)).await;
        let Some(req) = go.next().await else {
            return Ok(());
        };
        note(String::from_utf8_lossy(req.body()).into_owned());
        req.reply(&"done")?;
    }
}

/// The engine answers a caller 504 after 10 s: a request that waited that long in the logic's
/// queue never reaches the logic (it would run what the caller was told timed out). It is
/// answered `unanswered` as soon as a newer request arrives, or when the logic reads.
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn requests_older_than_the_engine_waits_never_reach_the_logic() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let (handle, mut up, down_tx) = started(slow);
            assert_eq!(declared(&mut up).await, ["go"]);
            let t0 = tokio::time::Instant::now();
            down_tx.try_send(call(1, "go", "\"old\"")).unwrap();
            tokio::time::sleep(std::time::Duration::from_secs(11)).await;
            down_tx.try_send(call(2, "go", "\"new\"")).unwrap();
            let why = unanswered(answer(&mut up, 1).await);
            assert!(why.contains("504") && why.contains("10 s"), "{why}");
            assert!(
                t0.elapsed() < std::time::Duration::from_secs(15),
                "answered while the logic was busy"
            );
            let Answer::Json(j) = answer(&mut up, 2).await else {
                panic!("expected JSON")
            };
            assert_eq!(j, b"\"done\"");
            // Nothing newer arrives: the logic finds it stale when it reads, 15 s later.
            down_tx.try_send(call(3, "go", "\"late\"")).unwrap();
            let why = unanswered(answer(&mut up, 3).await);
            assert!(why.contains("504"), "{why}");
            assert!(t0.elapsed() >= std::time::Duration::from_secs(30));
            assert_eq!(handle.errors(), Vec::<String>::new());
            assert_eq!(SEEN.with(|v| v.borrow().clone()), ["\"new\""]);
        })
        .await;
}

/// Declares `go` twice while the first stream lives.
#[sortie::mission(api_rev = 3)]
async fn twice(ctx: Ctx<Demo>, _start: Start) -> sortie::Result<()> {
    let _go = ctx.api().action("go")?;
    let again = ctx.api().action("go").expect_err("declared already");
    note(
        again
            .failure()
            .map(|f| f.message.clone())
            .unwrap_or_default(),
    );
    Ok(())
}

/// A second declaration of an action names the first one, in one sentence.
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn a_second_declaration_names_the_first() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let (handle, mut up, _down) = started(twice);
            assert_eq!(declared(&mut up).await, ["go"]);
            assert!(declared(&mut up).await.is_empty());
            handle.assert_clean();
            let seen = SEEN.with(|v| v.borrow().clone());
            let m = &seen[0];
            assert!(
                m.contains("is declared already (at ")
                    && m.contains("actions.rs:")
                    && m.contains("read its requests there, or drop that stream first"),
                "{m}"
            );
            assert!(!m.contains("  "), "a run of spaces: {m:?}");
        })
        .await;
}
