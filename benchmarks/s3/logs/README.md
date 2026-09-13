# S3 Logs Evidence

Frozen S3-4 contract for `lightr logs`.

| Case | Witness | Required result |
|---|---|---|
| stdout | `fixtures/stdout.log` | stdout selection emits only `out-*` bytes. |
| stderr | `fixtures/stderr.log` | stderr selection emits only `err-*` bytes. |
| both | both fixtures | stdout then stderr, each byte-preserved. |
| tail | fixture final two lines | `--tail 2` emits final two complete lines. |
| follow | handler/core poll-cap tests | exited-and-drained stops; live/vanished run stops at 3,000 polls. |
| missing run | `unknown_run_id_exits_1` | `Error: No such container: <id>` and exit 1. |
| timestamps | `timestamp_disclosure_is_explicitly_mtime_only` | stderr states no per-line timestamps and names log-file last-modified time. |

Raw logs contain no line timestamps. `-t` and `--since` may only expose or compare
file mtime. They must not fabricate line timestamps.

Mutation probes:

- Remove follow poll-cap branch: `bounded_follow_stops_after_drain_or_poll_cap` or
  `follow_poll_cap_is_bounded` fails.
- Remove mtime/no-per-line disclosure: `timestamp_disclosure_is_explicitly_mtime_only` fails.
