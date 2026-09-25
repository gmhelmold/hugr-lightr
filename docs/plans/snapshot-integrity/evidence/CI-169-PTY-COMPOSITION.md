# PR #169: bounded complete PTY/CRI qualification


Continuation on 2026-09-19: source 35df606 passed the macOS 15 Intel/ARM
PTY controls, while complete CI 35419706288 remained inside the macOS 14
ARM workspace test step. The unfinished run is not a successful result;
its partial job log was unavailable through the API at review time. No
particular hanging syscall or cause is inferred from that status alone.

The native workflow now retains its existing profiles and controls and
adds macOS 14 and macos-latest. A separate full-CRI gate builds the native
library with lightr-run/vz, inventories its actual test binary, and executes
ALL listed methods together with four test threads and no filter or skip.
Compilation has a separate 900-second limit; test listing has 30 seconds;
the compiled suite has 90 seconds. Each timeout is recorded and rejects
the candidate. These are CI watchdog budgets, not product latency promises.

The gate retains command, exit, timeout, log hash, binary hash, source and
platform identities. Missing/duplicate methods, filtered/ignored counts,
nonzero exit and timeout cannot yield success. Its Python tests verify the
gate only, not PTY behavior. A first local watchdog test incorrectly assumed
its child printed before a short startup deadline; that assertion failed.
Timeout and successful-output observations are now tested separately, with
no minimum process-start speed assumed. The failed log is retained.

The runtime correction, seven native methods, original echo assertion,
compiled missing-reference control and complete CI are unchanged here.
The original five acceptance groups remain mandatory. The expanded native
workflow and every other current-head gate must pass before integration;
this diagnostic increment does not claim the pending macOS 14 issue fixed.
