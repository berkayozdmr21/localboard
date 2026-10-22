use localboard_core::Board;
use serde_json::{json, Value};
use std::{
    fs,
    net::TcpListener,
    time::{Duration, Instant},
};

fn fixture() -> (tempfile::TempDir, tempfile::TempDir, Board, String) {
    let source = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    fs::write(source.path().join("package.json"), "{}").unwrap();
    let mut board = Board::open(data.path().into()).unwrap();
    board.import_root(source.path().to_str().unwrap()).unwrap();
    let id = board.library.projects[0].id.clone();
    (source, data, board, id)
}
fn add(
    board: &mut Board,
    project: &str,
    name: &str,
    exe: &str,
    args: Value,
    port: Option<u16>,
) -> String {
    board
        .dispatch(
            "service_add",
            json!({"projectId":project,"name":name,"executable":exe,"args":args,"port":port}),
        )
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .into()
}
fn wait(board: &mut Board, id: &str, status: &str) {
    let until = Instant::now() + Duration::from_secs(6);
    loop {
        let state = board.dispatch("services", json!({})).unwrap();
        if state["services"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["id"] == id && s["status"] == status)
        {
            return;
        }
        assert!(Instant::now() < until, "Process did not reach {status}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn real_process_streams_both_outputs_and_exit_code() {
    let (_source, _data, mut b, p) = fixture();
    let id = add(
        &mut b,
        &p,
        "output",
        "node",
        json!([
            "-e",
            "console.log('stdout marker'); console.error('stderr marker');process.exitCode=7"
        ]),
        None,
    );
    b.dispatch("service_start", json!({"id":id})).unwrap();
    wait(&mut b, &id, "exited");
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let logs = b.dispatch("service_logs", json!({"id":id})).unwrap();
        let text = logs.to_string();
        if text.contains("stdout marker") && text.contains("stderr marker") {
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        b.dispatch("services", json!({})).unwrap()["services"][0]["exitCode"],
        7
    );
}

#[test]
fn occupied_port_blocks_launch_without_touching_listener() {
    let (_s, _d, mut b, p) = fixture();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let id = add(
        &mut b,
        &p,
        "conflict",
        "node",
        json!(["-e", "setInterval(()=>{},1000)"]),
        Some(port),
    );
    assert!(b
        .dispatch("service_start", json!({"id":id}))
        .unwrap_err()
        .contains("portu kullanımda"));
    assert!(localboard_core::processes::port_busy(port));
}

#[test]
fn own_live_process_can_be_stopped_but_not_deleted() {
    let (_s, _d, mut b, p) = fixture();
    let id = add(
        &mut b,
        &p,
        "live",
        "node",
        json!(["-e", "setInterval(()=>{},1000)"]),
        None,
    );
    b.dispatch("service_start", json!({"id":id})).unwrap();
    assert!(b.dispatch("service_start", json!({"id":id})).is_err());
    assert!(b.dispatch("service_delete", json!({"id":id})).is_err());
    b.dispatch("service_stop", json!({"id":id})).unwrap();
    wait(&mut b, &id, "exited");
    b.dispatch("service_delete", json!({"id":id})).unwrap();
    assert!(b.dispatch("services", json!({})).unwrap()["services"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn workspace_failure_rolls_back_only_new_services() {
    let (_s, _d, mut b, p) = fixture();
    let old = add(
        &mut b,
        &p,
        "preexisting",
        "node",
        json!(["-e", "setInterval(()=>{},1000)"]),
        None,
    );
    let good = add(
        &mut b,
        &p,
        "new",
        "node",
        json!(["-e", "setInterval(()=>{},1000)"]),
        None,
    );
    let bad = add(
        &mut b,
        &p,
        "missing",
        "localboard-no-such-executable-991",
        json!([]),
        None,
    );
    b.dispatch("service_start", json!({"id":old})).unwrap();
    let group = b
        .dispatch(
            "workspace_add",
            json!({"name":"dev","serviceIds":[old,good,bad]}),
        )
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(b.dispatch("workspace_start", json!({"id":group})).is_err());
    let state = b.dispatch("services", json!({})).unwrap();
    let running: Vec<_> = state["services"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["status"] == "running")
        .collect();
    assert_eq!(running.len(), 1);
    assert_eq!(running[0]["id"], old);
}

#[test]
fn configuration_persists_without_autostart() {
    let (_s, data, mut b, p) = fixture();
    let id = add(
        &mut b,
        &p,
        "persistent",
        "node",
        json!(["-e", "setInterval(()=>{},1000)"]),
        None,
    );
    b.dispatch("workspace_add", json!({"name":"dev","serviceIds":[id]}))
        .unwrap();
    drop(b);
    let mut reopened = Board::open(data.path().into()).unwrap();
    let state = reopened.dispatch("services", json!({})).unwrap();
    assert_eq!(state["services"][0]["status"], "stopped");
    assert_eq!(state["workspaces"][0]["name"], "dev");
}

#[test]
fn malformed_service_and_unknown_project_rejected() {
    let (_s, _d, mut b, p) = fixture();
    for input in [
        json!({"projectId":"unknown","name":"x","executable":"node","args":[]}),
        json!({"projectId":p,"name":"x","executable":"node","args":"not an array"}),
        json!({"projectId":p,"name":"x","executable":"node","args":[],"port":99999}),
    ] {
        assert!(b.dispatch("service_add", input).is_err());
    }
    assert!(b.dispatch("port_check", json!({"port":0})).is_err());
}

#[test]
fn log_memory_is_bounded() {
    let (_s, _d, mut b, p) = fixture();
    let id = add(
        &mut b,
        &p,
        "flood",
        "node",
        json!(["-e", "for(let i=0;i<1000;i++)console.log('line '+i)"]),
        None,
    );
    b.dispatch("service_start", json!({"id":id})).unwrap();
    wait(&mut b, &id, "exited");
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let logs = b.dispatch("service_logs", json!({"id":id})).unwrap();
        let lines = logs.as_array().unwrap();
        if lines.last().is_some_and(|l| l["text"] == "line 999") {
            assert_eq!(lines.len(), 600);
            assert_eq!(lines[0]["text"], "line 400");
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn removed_project_cannot_launch_stored_service() {
    let (_s, _d, mut b, p) = fixture();
    let id = add(&mut b, &p, "orphan", "node", json!([]), None);
    let root = b.library.roots[0].id.clone();
    b.dispatch("remove_root", json!({"id":root})).unwrap();
    assert!(b.dispatch("service_start", json!({"id":id})).is_err());
}
