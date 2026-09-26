use std::process::Command;

fn assert_hot_plug_error(verb: &str, expected: &str) {
    let output = Command::new(env!("CARGO_BIN_EXE_lightr"))
        .args(["network", verb, "web", "ctr1"])
        .output()
        .expect("run lightr");

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(String::from_utf8(output.stdout).expect("utf-8 stdout"), "");
    assert_eq!(
        String::from_utf8(output.stderr).expect("utf-8 stderr"),
        format!("lightr: {expected}\n")
    );
}

#[test]
fn network_connect_reports_run_scoped_spawn_time_boundary() {
    assert_hot_plug_error(
        "connect",
        "network connect is not supported: networks are run-scoped and fixed at spawn time; recreate the run with `lightr run --network web ...` or configure `compose.yml` `networks:`",
    );
}

#[test]
fn network_disconnect_reports_run_scoped_spawn_time_boundary() {
    assert_hot_plug_error(
        "disconnect",
        "network disconnect is not supported: networks are run-scoped and fixed at spawn time; recreate the run without `--network web` or configure `compose.yml` `networks:`",
    );
}
