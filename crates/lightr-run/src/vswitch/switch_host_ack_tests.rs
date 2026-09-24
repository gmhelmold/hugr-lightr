//! Control-reply ordering only; complete descriptor/data lifecycles are tested separately.
use super::super::pass_and_ack_observed;
use std::io::{self, Read, Write};
use std::os::fd::OwnedFd;
use std::os::unix::net::{UnixDatagram, UnixStream};
use std::sync::mpsc;
use std::time::Duration;

fn closed_peer_reply(reply: Option<u8>) -> io::Result<OwnedFd> {
    let (client, mut peer) = UnixStream::pair()?;
    let (guest, host) = UnixDatagram::pair()?;
    let (closed_tx, closed_rx) = mpsc::channel();
    let receiver = std::thread::spawn(move || -> io::Result<()> {
        peer.set_read_timeout(Some(Duration::from_secs(5)))?;
        // This unit peer reads metadata only. It does not certify FD installation.
        let mut payload = [0u8; 3];
        peer.read_exact(&mut payload)?;
        assert_eq!(&payload, b"ack");
        if let Some(byte) = reply {
            peer.write_all(&[byte])?;
        }
        drop(peer);
        closed_tx
            .send(())
            .expect("caller retains close notification");
        Ok(())
    });
    let result = pass_and_ack_observed(client, host, b"ack", guest, || {
        closed_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("bounded peer reply/close");
    });
    receiver.join().expect("peer thread did not panic")?;
    result
}

#[test]
fn fast_ack_survives_peer_close_before_caller_reads() {
    closed_peer_reply(Some(super::super::ATTACH_ACK))
        .expect("completed ACK must survive peer close; configure timeout before send");
}

#[test]
fn closed_peer_bad_ack_remains_invalid_data() {
    assert_eq!(
        closed_peer_reply(Some(0xff)).unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
}

#[test]
fn closed_peer_without_ack_remains_unexpected_eof() {
    assert_eq!(
        closed_peer_reply(None).unwrap_err().kind(),
        io::ErrorKind::UnexpectedEof
    );
}
