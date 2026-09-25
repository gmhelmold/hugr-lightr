//! Bounded parallel composition of the existing owned, temporary network test.
//! No alternate send path, retries, ignored errno, or external network target.
#[test]
fn independent_attach_lifecycles_do_not_interfere() {
    std::thread::scope(|scope| {
        let mut workers = Vec::new();
        for _ in 0..4 {
            workers.push(scope.spawn(|| {
                for _ in 0..8 {
                    super::attach_forward_dhcp_dns_then_refcount_self_stop();
                }
            }));
        }
        for worker in workers {
            worker.join().expect("owned attach lifecycle failed");
        }
    });
}

#[test]
fn sender_endpoint_lives_until_receiver_acknowledges() {
    use super::super::{pass_and_ack_observed, ATTACH_ACK};
    use std::io::Write;
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    use std::os::unix::net::{UnixDatagram, UnixStream};
    use std::sync::mpsc;
    use std::time::Duration;

    let (sender, mut receiver) = UnixStream::pair().unwrap();
    let (guest, host) = UnixDatagram::pair().unwrap();
    let sender_endpoint = host.as_raw_fd();
    let (arrived_tx, arrived_rx) = mpsc::channel();
    let (resume_tx, resume_rx) = mpsc::channel();
    let producer = std::thread::spawn(move || {
        pass_and_ack_observed(sender, host, b"owned-handoff", guest, || {
            arrived_tx.send(()).unwrap();
            resume_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        })
    });
    arrived_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    // SAFETY: F_GETFD only observes this process's saved descriptor number;
    // the kernel rejects a closed descriptor. No ownership is constructed.
    // No receiver FD has been installed yet, so it cannot recycle this number.
    let retained = unsafe { libc::fcntl(sender_endpoint, libc::F_GETFD) } >= 0;
    let (raw, payload) = crate::vswitch::passfd::recv_fd(&receiver).unwrap();
    // SAFETY: recv_fd transfers this fresh descriptor exclusively to the test.
    let received = unsafe { OwnedFd::from_raw_fd(raw) };
    receiver.write_all(&[ATTACH_ACK]).unwrap();
    resume_tx.send(()).unwrap();
    let guest = UnixDatagram::from(producer.join().unwrap().unwrap());
    let received = UnixDatagram::from(received);
    assert!(
        retained,
        "sender endpoint closed before receiver acknowledgement"
    );
    assert_eq!(payload, b"owned-handoff");
    guest.send(b"after-ack").unwrap();
    received
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut bytes = [0u8; 32];
    let count = received.recv(&mut bytes).unwrap();
    assert_eq!(&bytes[..count], b"after-ack");
}
