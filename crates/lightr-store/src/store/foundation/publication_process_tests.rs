//! Child helpers are not independent passing witnesses; the two parent tests
//! drive them in distinct processes against private disposable Store roots.
use super::publication_tests::{identity, path};
use super::{LeasedStagedFile, Phase, PreparationWork, StoreLocks, Wait};
use lightr_core::Digest;
use std::fs::File;
use std::io::{self, BufRead, Cursor, Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;
const BYTES: &[u8] = b"process payload";
const CHILD: &str = "store::foundation::publication_process_tests::publication_process_entry";
struct ChildOwner(Child);
impl Drop for ChildOwner {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn start(root: &std::path::Path, mode: &str) -> ChildOwner {
    ChildOwner(
        Command::new(std::env::current_exe().unwrap())
            .args(["--exact", CHILD, "--nocapture"])
            .env("LIGHTR_SI01_PUBLICATION_TEST_ROOT", root)
            .env("LIGHTR_SI01_PUBLICATION_TEST_MODE", mode)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    )
}

#[test]
fn publication_process_entry() {
    let Some(root) = std::env::var_os("LIGHTR_SI01_PUBLICATION_TEST_ROOT") else {
        return;
    };
    let mode = std::env::var("LIGHTR_SI01_PUBLICATION_TEST_MODE").unwrap();
    let domain = StoreLocks::open_existing(std::path::Path::new(&root)).unwrap();
    let lease = domain.shared(Wait::Try).unwrap();
    let digest = Digest::of_bytes(BYTES);
    if mode == "stop-after-install" {
        let stage =
            LeasedStagedFile::copy(&lease, &mut Cursor::new(BYTES), None, [45; 16]).unwrap();
        let _ = lease.prepare_with(digest, Some(stage), Wait::Try, [45; 16], &|phase, _| {
            if phase == Phase::PayloadDirectory {
                println!("SI01_INSTALLED");
                io::stdout().flush()?;
                io::stdin().read_exact(&mut [0u8; 1])?;
                panic!("parent must terminate the child before this boundary");
            }
            Ok(())
        });
        panic!("stopped child unexpectedly returned");
    }
    assert_eq!(mode, "reuse");
    let proof = lease.prepare_existing(digest, Wait::Try, [46; 16]).unwrap();
    assert_eq!(proof.work(), PreparationWork::Reused);
    assert_eq!(proof.len(), BYTES.len() as u64);
    assert_eq!(
        std::fs::read(path(std::path::Path::new(&root), "objects", digest)).unwrap(),
        BYTES
    );
    println!("SI01_REUSED");
}

#[test]
fn publication_hard_killed_installer_is_requalified_by_a_new_process() {
    let root = TempDir::new().unwrap();
    let digest = Digest::of_bytes(BYTES);
    let mut child = start(root.path(), "stop-after-install");
    let stdout = child.0.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel();
    let reader = thread::spawn(move || {
        for line in io::BufReader::new(stdout).lines() {
            let line = line.unwrap();
            if line.contains("SI01_INSTALLED") {
                let _ = tx.send(());
            }
        }
    });
    rx.recv_timeout(Duration::from_secs(15))
        .expect("child did not reach post-install boundary");
    let payload = path(root.path(), "objects", digest);
    let old = File::open(&payload).unwrap();
    assert!(!path(root.path(), "objects-ready", digest).exists());
    child.0.kill().unwrap();
    child.0.wait().unwrap();
    reader.join().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let proof = lease.prepare_existing(digest, Wait::Try, [47; 16]).unwrap();
    assert_eq!(proof.work(), PreparationWork::Requalified);
    assert_ne!(super::lease_io::identity(&old).unwrap(), identity(&payload));
    assert_eq!(std::fs::read(&payload).unwrap(), BYTES);
    assert!(path(root.path(), "objects-ready", digest).exists());
}

#[test]
fn publication_another_process_reuses_the_immutable_confirmed_payload() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let digest = Digest::of_bytes(BYTES);
    {
        let lease = locks.shared(Wait::Try).unwrap();
        lease
            .prepare_reader(&mut Cursor::new(BYTES), None, Wait::Try, [48; 16])
            .unwrap();
    }
    let payload = path(root.path(), "objects", digest);
    let old = File::open(&payload).unwrap();
    let mut child = start(root.path(), "reuse");
    let stdout = child.0.stdout.take().unwrap();
    let reader = thread::spawn(move || {
        io::BufReader::new(stdout)
            .lines()
            .collect::<io::Result<Vec<_>>>()
            .unwrap()
    });
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            assert!(status.success());
            break;
        }
        assert!(Instant::now() < deadline, "child reuse watchdog expired");
        thread::sleep(Duration::from_millis(5));
    }
    let lines = reader.join().unwrap();
    assert!(lines.iter().any(|l| l.contains("SI01_REUSED")));
    assert_eq!(super::lease_io::identity(&old).unwrap(), identity(&payload));
}
