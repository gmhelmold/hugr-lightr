//! Scoped synthetic stream failures, not native filesystem qualification.
use super::*;

struct Fake {
    entries: Vec<Vec<u8>>,
    calls: usize,
    error_at: Option<usize>,
}
impl Fake {
    fn new(entries: &[&[u8]]) -> Self {
        Self {
            entries: entries.iter().map(|n| n.to_vec()).collect(),
            calls: 0,
            error_at: None,
        }
    }
}
impl Names for Fake {
    fn next_name(&mut self) -> io::Result<Option<&[u8]>> {
        let index = self.calls;
        self.calls += 1;
        if self.error_at == Some(index) {
            return Err(io::Error::from_raw_os_error(libc::EIO));
        }
        Ok(self.entries.get(index).map(Vec::as_slice))
    }
}

#[test]
fn listing_read_failure_after_a_name_is_not_partial_success() {
    let mut stream = Fake::new(&[b"first"]);
    stream.error_at = Some(1);
    let result = collect(&mut stream, limits(), Wait::Try, || Ok(()));
    assert_eq!(result.unwrap_err().raw_os_error(), Some(libc::EIO));
    assert_eq!(stream.calls, 2);
}

#[test]
fn listing_cancellation_after_eof_rejects_the_result() {
    let cancellation = Cancellation::default();
    let wait = Wait::Until {
        deadline: Instant::now() + Duration::from_secs(60),
        cancellation: &cancellation,
    };
    let result = collect(&mut Fake::new(&[]), limits(), wait, || {
        cancellation.cancel();
        Ok(())
    });
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::Interrupted);
}

#[test]
fn listing_duplicates_and_malformed_names_are_not_silently_omitted() {
    let cases: &[&[&[u8]]] = &[
        &[b"a", b"a"],
        &[b".", b"."],
        &[b"..", b".."],
        &[b""],
        &[b"a/b"],
        &[b"a\0b"],
        &[b"\xff"],
    ];
    for names in cases {
        let result = collect(&mut Fake::new(names), limits(), Wait::Try, || Ok(()));
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);
    }
}

#[test]
fn listing_budget_failure_stops_without_scanning_the_remaining_names() {
    let mut stream = Fake::new(&[b"first", b"second", b"third"]);
    let result = collect(
        &mut stream,
        DirectoryLimits {
            max_entries: 1,
            max_name_bytes: 1024,
        },
        Wait::Try,
        || Ok(()),
    );
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidInput);
    assert_eq!(stream.calls, 2);
    let mut stream = Fake::new(&[b"ab", b"cd", b"e"]);
    assert!(collect(
        &mut stream,
        DirectoryLimits {
            max_entries: 32,
            max_name_bytes: 3
        },
        Wait::Try,
        || Ok(())
    )
    .is_err());
    assert_eq!(stream.calls, 2);
}

#[test]
fn listing_empty_stream_and_single_dot_variants_are_accepted() {
    let cases: &[&[&[u8]]] = &[&[], &[b"."], &[b".."], &[b"..", b"."]];
    for names in cases {
        assert!(collect(
            &mut Fake::new(names),
            DirectoryLimits {
                max_entries: 0,
                max_name_bytes: 0
            },
            Wait::Try,
            || Ok(())
        )
        .unwrap()
        .is_empty());
    }
}

#[test]
fn listing_checked_close_rejects_success_and_preserves_prior_error() {
    let closed = Err(io::Error::from_raw_os_error(libc::EIO));
    assert_eq!(
        finish_scan(Ok(vec!["name".into()]), closed)
            .unwrap_err()
            .raw_os_error(),
        Some(libc::EIO)
    );
    let primary = Err(io::Error::from_raw_os_error(libc::EINVAL));
    let closed = Err(io::Error::from_raw_os_error(libc::EBADF));
    assert_eq!(
        finish_scan(primary, closed).unwrap_err().raw_os_error(),
        Some(libc::EINVAL)
    );
    assert!(finish_scan(Ok(Vec::new()), Ok(())).unwrap().is_empty());
}
