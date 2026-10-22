use localboard_core::{git_info, scan, Board};
use serde_json::json;
use std::fs;

#[test]
fn discovery_skips_dependencies_and_recognizes_stacks() {
    let dir = tempfile::tempdir().unwrap();
    for path in ["web", "api", "node_modules/hidden", ".cache/hidden"] {
        fs::create_dir_all(dir.path().join(path)).unwrap();
    }
    fs::write(
        dir.path().join("web/package.json"),
        r#"{"dependencies":{"react":"19"},"scripts":{"dev":"vite"}}"#,
    )
    .unwrap();
    fs::write(dir.path().join("api/pyproject.toml"), "").unwrap();
    fs::write(dir.path().join("node_modules/hidden/Cargo.toml"), "").unwrap();
    fs::write(dir.path().join(".cache/hidden/Cargo.toml"), "").unwrap();
    let (projects, warnings) = scan(dir.path(), "root").unwrap();
    assert!(warnings.is_empty());
    assert_eq!(projects.len(), 2);
    let web = projects.iter().find(|p| p.name == "web").unwrap();
    assert!(web.stack.contains(&"React".into()));
    assert_eq!(web.scripts, vec!["dev"]);
}

#[test]
fn favorites_survive_rescan_and_restart_and_removal_keeps_files() {
    let source = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    fs::write(source.path().join("Cargo.toml"), "").unwrap();
    let mut board = Board::open(data.path().into()).unwrap();
    board.import_root(source.path().to_str().unwrap()).unwrap();
    let id = board.library.projects[0].id.clone();
    let root = board.library.roots[0].id.clone();
    board
        .dispatch("favorite", json!({"id":id,"value":true}))
        .unwrap();
    board.refresh_root(&root).unwrap();
    let mut reopened = Board::open(data.path().into()).unwrap();
    assert!(reopened.library.projects[0].favorite);
    reopened
        .dispatch("remove_root", json!({"id":root}))
        .unwrap();
    assert!(reopened.library.projects.is_empty());
    assert!(source.path().join("Cargo.toml").exists());
}

#[test]
fn corrupted_library_is_preserved() {
    let data = tempfile::tempdir().unwrap();
    let file = data.path().join("library.json");
    fs::write(&file, "broken").unwrap();
    assert!(Board::open(data.path().into()).is_err());
    assert_eq!(fs::read_to_string(file).unwrap(), "broken");
}

#[test]
fn missing_root_and_unregistered_project_are_rejected() {
    let data = tempfile::tempdir().unwrap();
    let mut board = Board::open(data.path().into()).unwrap();
    assert!(board
        .import_root("not-a-real-localboard-folder-29381")
        .is_err());
    assert!(board.dispatch("git_info", json!({"id":"unknown"})).is_err());
    assert!(!git_info(data.path()).available);
}

#[test]
fn duplicate_import_is_idempotent() {
    let source = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    fs::write(source.path().join("go.mod"), "").unwrap();
    let mut board = Board::open(data.path().into()).unwrap();
    board.import_root(source.path().to_str().unwrap()).unwrap();
    board.import_root(source.path().to_str().unwrap()).unwrap();
    assert_eq!(board.library.roots.len(), 1);
    assert_eq!(board.library.projects.len(), 1);
}
