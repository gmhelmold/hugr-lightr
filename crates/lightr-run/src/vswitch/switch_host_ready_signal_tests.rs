//! The fixture's producer signals a listening endpoint, never pathname presence.
use super::super::{ctl_sock_path, detach, run_switch_host_observed};
use crate::network::NetworkRegistry;
use std::os::unix::net::UnixStream;
use std::sync::mpsc;
use std::time::Duration;

#[test]
fn ready_signal_follows_listen_and_allows_clean_teardown() {
    let root = tempfile::Builder::new()
        .prefix("c9-signal-")
        .tempdir_in("/tmp")
        .unwrap();
    let id = "signal".to_string();
    let reg = NetworkRegistry::create(root.path(), &id).unwrap();
    reg.join("member", &[], &[]).unwrap();
    let home = root.path().to_owned();
    let host_id = id.clone();
    let (ready_tx, ready_rx) = mpsc::channel();
    let (resume_tx, resume_rx) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        run_switch_host_observed(&home, &host_id, || {
            ready_tx.send(()).unwrap();
            resume_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        })
    });
    ready_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let ctl = ctl_sock_path(root.path(), &id);
    let probe = UnixStream::connect(&ctl);
    let accepting = probe.is_ok();
    drop(probe); // An EOF probe never announces a member.
    resume_tx.send(()).unwrap();
    detach(root.path(), &id, "member").unwrap();
    worker
        .join()
        .expect("owned host thread must finish")
        .unwrap();
    assert!(!ctl.exists(), "owned listener path must be reclaimed");
    assert!(
        accepting,
        "readiness signal preceded the accepting listener"
    );
}
