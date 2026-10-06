use std::process::Command;

fn vitals(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_vitals"))
        .args(args)
        .output()
        .expect("run vitals")
}

#[test]
fn prints_version() {
    let out = vitals(&["--version"]);
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        format!("vitals {}\n", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn prints_help() {
    let out = vitals(&["--help"]);
    assert!(out.status.success());
    assert!(
        String::from_utf8(out.stdout)
            .unwrap()
            .starts_with("vitals:")
    );
}

#[test]
fn rejects_unknown_option() {
    let out = vitals(&["--bogus"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8(out.stderr).unwrap().contains("--bogus"));
}

#[test]
fn prints_one_plain_line_with_metrics() {
    let dir = std::env::temp_dir().join(format!("vitals-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let state = dir.join("cpu");
    let state = state.to_str().unwrap();

    let out = vitals(&["--plain", "--no-clock", "--state", state]);
    assert!(out.status.success());
    let line = String::from_utf8(out.stdout).unwrap();
    assert_eq!(line.lines().count(), 1);
    assert!(line.starts_with("cpu "), "{line:?}");
    assert!(line.contains(" mem "), "{line:?}");
    assert!(line.contains(" swap "), "{line:?}");
    assert!(std::fs::read_to_string(dir.join("cpu")).is_ok());

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn tmux_line_has_style_markup_and_clock() {
    let dir = std::env::temp_dir().join(format!("vitals-test-tmux-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let state = dir.join("cpu");

    let out = Command::new(env!("CARGO_BIN_EXE_vitals"))
        .args(["--state", state.to_str().unwrap()])
        .env("TZ", "UTC")
        .output()
        .unwrap();
    assert!(out.status.success());
    let line = String::from_utf8(out.stdout).unwrap();
    assert!(line.starts_with("#[fg=colour114]cpu "), "{line:?}");
    assert!(
        line.contains("#[fg=colour244]· #[fg=colour39]20"),
        "{line:?}"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}
