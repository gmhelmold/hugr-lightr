#!/usr/bin/env bash
# KPI 3 — Real-container cold-start / footprint A/B vs containerd.
#
# SOURCE OF TRUTH: lightr-cri handoff §3
#   crates/lightr-cri/docs/handoff/bench-cas-kpis-request.md
#
# CLAIM TO SIGN: starting a REAL container (nginx/agnhost) reaches serving in
# time/footprint at parity-or-better vs containerd, same image + host.
#
# PROBE: extend the vendored lightr-cri bench harness (crates/lightr-cri/ci/bench.sh, schema
# lightr-cri.bench/v1) in_scope block to drive a real `crictl run` of a pullable
# image and curl it. The harness cold-start / RSS / recovery probes already
# exist — point them at a real workload backed by the real CAS backend.
#
# PASS BAR:
#   - time-to-serving (spawn → first 200 from curl)  <= containerd, OR within
#     the harness budget signed in lightr-cri docs/bench/
#   - RSS / idle footprint                            <= containerd
#   - same image + same host for both sides (A/B fairness)
#
# MEASURE WITH (real backend, on a Linux box):
#   BACKEND=lightr bash crates/lightr-cri/ci/bench.sh   # in_scope real workload
#   # the harness drives crictl run <nginx|agnhost>, curls it, records ms + RSS,
#   # and signs a lightr-cri.bench/v1 JSON. Run the containerd side identically.
#
# NOTE: this unblocks the runtime-tier critest networking specs (port-mapping
# ×2, portforward ×2) in crates/lightr-cri/ci/critest-skips.txt — they need a real
# image serving HTTP in the pod netns.
#
# DORMANT GUARD: FAILS CLOSED until a Linux runner is attached and the real CAS
# backend can execute a pulled image's binary (the fake cannot). No unmeasured
# time/RSS is ever emitted.
set -euo pipefail

echo "KPI 3 — real-container cold-start/footprint A/B vs containerd"
echo "spec: crates/lightr-cri/docs/handoff/bench-cas-kpis-request.md §3"

HARNESS="crates/lightr-cri/ci/bench.sh"
if [ ! -f "$HARNESS" ]; then
  echo "::error::lightr-cri bench harness not found at $HARNESS (vendored workspace)."
  exit 1
fi

if [ "${KPI_BACKEND_READY:-0}" != "1" ]; then
  echo "::error::KPI 3 not yet wired — real image execution over the CAS backend required."
  echo "Set KPI_BACKEND_READY=1 once the real backend can run a pulled image, then drive"
  echo "the vendored harness: BACKEND=lightr bash $HARNESS"
  echo "Fail-closed: refusing to emit an unmeasured cold-start/RSS number."
  exit 1
fi

# --- real probe goes here (only reached once the backend capability lands) ---
echo "::error::KPI 3 probe body not implemented — invoke vendored harness with real backend."
exit 1
