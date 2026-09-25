"""Compile read-only topology controls in a disposable source tree; fixtures own their data."""
from __future__ import annotations
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[2]
BASE = "crates/lightr-store/src/store/foundation/"
PREFIX = "store::foundation::topology::"
CASES = [
    ("protected-overlap", "topology_native.rs",
     "if protected.contains(subject) || subject.contains(protected) {",
     "if false && (protected.contains(subject) || subject.contains(protected)) {",
     "unix::topology_rejects_equal_containing_and_contained_protected_roots", "protected overlap was accepted"),
    ("paired-overlap", "topology_native.rs",
     "if source.contains(destination) || destination.contains(source) {",
     "if false && (source.contains(destination) || destination.contains(source)) {",
     "unix::topology_rejects_source_destination_overlap_in_both_directions", "paired overlap was accepted"),
    ("identity-revalidation", "topology_native.rs", "if !self\n            .observed", "if false && !self\n            .observed",
     "unix::topology_retained_source_and_revalidation_detect_replacement", "changed topology was accepted"),
    ("unresolved-parent", "topology_native.rs",
     'if tail.iter().any(|p| *p == b"." || *p == b"..") {',
     'if false && tail.iter().any(|p| *p == b"." || *p == b"..") {',
     "unix::topology_missing_suffix_dot_and_dangling_link_are_explicitly_rejected", "unresolved dot components were accepted"),
    ("destination-protected-overlap", "topology_native.rs",
     "if protected.contains(subject) || subject.contains(protected) {",
     "if false && (protected.contains(subject) || subject.contains(protected)) {",
     "store::foundation::destination_tests::unix::destination_checks_each_protected_root_and_all_overlap_directions",
     "protected destination was accepted"),
    ("destination-missing-handle", "topology_native.rs",
     "target.missing.is_empty().then_some(&target.object)",
     "true.then_some(&target.object)",
     "store::foundation::destination_tests::unix::destination_missing_target_never_exposes_its_existing_ancestor",
     "ancestor exposed as destination"),
    ("destination-revalidation", "topology.rs",
     "observed.revalidate(wait)", "{ let _ = observed; wait.check() }",
     "store::foundation::destination_tests::unix::destination_revalidation_detects_replacement_without_retargeting_handle",
     "changed destination was accepted"),
    ("planned-root-resolution", "topology_native.rs",
     "ProtectedRoot::PlannedDirectory(path) => (path, Kind::ProposedDirectory),",
     "ProtectedRoot::PlannedDirectory(path) => (path, Kind::Directory),",
     "store::foundation::planned_roots_tests::unix::planned_roots_accept_absent_inventory_without_creating_names",
     "planned root inspection failed"),
    ("planned-root-native-ambiguity", "topology_native.rs",
     "if !self.missing.is_empty()", "if false && !self.missing.is_empty()",
     "store::foundation::planned_roots_tests::unix::planned_roots_reject_unproven_sibling_names",
     "unresolved sibling disjointness was guessed"),
    ("planned-root-public-ancestor", "topology_native.rs",
     "if protected.contains(subject) || subject.contains(protected) {",
     "if false && (protected.contains(subject) || subject.contains(protected)) {",
     "store::foundation::planned_roots_tests::unix::planned_roots_reject_public_ancestors",
     "planned root ancestor accepted"),
]


EMPTY_CASES = [('empty-independent-offset', 'topology_empty.rs', 'let scan = open_at(directory, b".", true)?;', 'let _ = open_at; let scan = directory.try_clone()?;', 'store::foundation::topology::topology_empty::tests::empty_destination_scan_has_an_independent_directory_offset', 'called `Result::unwrap_err()` on an `Ok` value'), ('empty-post-eof-cancellation', 'topology_empty.rs', 'wait.check()?; // Cancellation after EOF is still cancellation.', '// Deliberately omitted post-read checkpoint for this causal control.', 'store::foundation::topology::topology_empty::tests::empty_destination_scan_checks_cancellation_after_eof', 'called `Result::unwrap_err()` on an `Ok` value')]


DESCENDANT_CASES = [('descendant-no-follow', 'topology_descendant.rs', '| libc::O_NOFOLLOW', '| 0', 'store::foundation::topology::topology_descendant::tests::descendant_rejects_links_at_every_component', 'descendant symbolic link was followed'), ('descendant-component-identity', 'topology_descendant.rs', 'if key(&current)? != opened[index].1 {', 'if false && key(&current)? != opened[index].1 {', 'store::foundation::topology::topology_descendant::tests::descendant_revalidates_each_component_after_opening', 'substituted descendant directory accepted'), ('descendant-hardlink', 'topology_descendant.rs', 'if !directory && metadata.nlink() != 1 {', 'if false && !directory && metadata.nlink() != 1 {', 'store::foundation::topology::topology_descendant::tests::descendant_native_missing_type_and_hardlink_errors_are_preserved', 'called `Result::unwrap_err()` on an `Ok` value')]

SCRATCH_CASES = [('scratch-exclusive-file', 'anchored_scratch_native.rs', 'flags | libc::O_EXCL', 'flags', 'store::foundation::anchored_scratch::tests::unix::scratch_existing_names_are_never_adopted_or_truncated', 'existing file was adopted'), ('scratch-cleanup-identity', 'anchored_scratch_native.rs', 'if identity(&self.parent, &self.name)? != self.identity {', 'if false && identity(&self.parent, &self.name)? != self.identity {', 'store::foundation::anchored_scratch::tests::unix::scratch_cleanup_preserves_replacement_entries', 'replacement was removed'), ('scratch-collision-budget', 'anchored_scratch_native.rs', 'for _ in 0..32 {', 'for _ in 0..33 {', 'store::foundation::anchored_scratch::native::tests::scratch_atomic_reservation_has_a_finite_collision_budget', 'assertion `left == right` failed')]

LINK_CASES = [('link-leaf-identity', 'topology_link.rs', 'if key(&current)? != expected {', 'if false && key(&current)? != expected {', 'store::foundation::topology::topology_link::tests::source_link_detects_replacement_after_read', 'replaced source link accepted'), ('link-byte-bound', 'topology_link.rs', 'if used > limit {', 'if false && used > limit {', 'store::foundation::topology::topology_link::tests::source_link_exact_byte_budget_never_accepts_truncation', 'called `Result::unwrap_err()` on an `Ok` value'), ('link-final-cancellation', 'topology_link.rs', 'wait.check()?; // Source-link final checkpoint, including completed reads.', '// Deliberately omitted terminal checkpoint for causal control.', 'store::foundation::topology::topology_link::tests::source_link_cancellation_after_completed_validation_is_not_success', 'completed cancelled link read accepted'), ('link-pin-no-follow', 'topology_link.rs', 'libc::O_PATH | libc::O_NOFOLLOW | libc::O_CLOEXEC', 'libc::O_PATH | libc::O_CLOEXEC', 'store::foundation::topology::topology_link::tests::source_link_preserves_dangling_and_exact_target_text', 'dangling link text not preserved')]

LISTING_CASES = [('listing-independent-offset', 'topology_listing_native.rs', 'let scan = open_at(&directory, b".", true)?;', 'let _ = open_at; let scan = directory.try_clone()?;', 'store::foundation::topology::topology_listing::native::tests::listing_ignores_an_offset_previously_advanced_on_the_held_source', 'assertion `left == right` failed'), ('listing-strict-utf8', 'topology_listing_native.rs', 'let name =\n            std::str::from_utf8(name).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;', 'let lossy = String::from_utf8_lossy(name); let name = lossy.as_ref();', 'store::foundation::topology::topology_listing::native::tests::listing_non_utf8_native_name_is_an_error_not_an_omission', 'called `Result::unwrap_err()` on an `Ok` value'), ('listing-eof-cancellation', 'topology_listing_native.rs', 'wait.check()?; // EOF does not erase a cancellation/deadline.\n        let Some(name) = entry else { break };', 'let Some(name) = entry else { return Ok(names) };\n        wait.check()?;', 'store::foundation::topology::topology_listing::native::tests::collection::listing_cancellation_after_eof_rejects_the_result', 'called `Result::unwrap_err()` on an `Ok` value'), ('listing-nested-binding', 'topology_listing_native.rs', 'if key(&current)? != identity {', 'if false && key(&current)? != identity {', 'store::foundation::topology::topology_listing::native::tests::listing_revalidates_nested_binding_after_native_enumeration', 'called `Result::unwrap_err()` on an `Ok` value')]

DEST_ANCHOR_CASES = [('anchor-no-adopt', 'destination_anchor_native.rs', 'if result != 0 {', 'if false && result != 0 {', 'store::foundation::destination_anchor::tests::anchor_never_adopts_a_name_that_appears_after_preflight', 'called `Option::unwrap()` on a `None` value'), ('anchor-final-binding', 'topology_native.rs', 'if binding_changed {', 'if false && binding_changed {', 'store::foundation::destination_anchor::tests::anchor_final_validation_rejects_replacement_and_preserves_decoy', 'called `Option::unwrap()` on a `None` value'), ('anchor-cleanup-identity', 'destination_anchor_native.rs', 'impl OwnedDirectory {\n    fn remove(self) -> io::Result<()> {\n        if identity(&self.parent, &self.name)? != self.identity {', 'impl OwnedDirectory {\n    fn remove(self) -> io::Result<()> {\n        if false && identity(&self.parent, &self.name)? != self.identity {', 'store::foundation::destination_anchor::tests::anchor_final_validation_rejects_replacement_and_preserves_decoy', 'assertion failed')]


DEST_REPR_CASES = [
    (
        'dest-repr-no-adopt',
        'destination_name_probe_native.rs',
        'libc::O_RDWR | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC',
        'libc::O_RDWR | libc::O_CREAT | libc::O_NOFOLLOW | libc::O_CLOEXEC',
        'store::foundation::destination_name_probe::tests::races::destination_name_probe_never_adopts_foreign_name_after_empty_check',
        'called `Result::unwrap_err()` on an `Ok` value',
    ),
    (
        'dest-repr-cleanup-identity',
        'destination_name_probe_native.rs',
        'if identity(&self.parent, &self.name)? != self.identity {',
        'if false && identity(&self.parent, &self.name)? != self.identity {',
        'store::foundation::destination_name_probe::tests::races::destination_name_probe_replacement_is_preserved_and_reports_incomplete_cleanup',
        'failure.primary.is_none()',
    ),
    (
        'dest-repr-final-empty',
        'destination_name_probe.rs',
        '} else if let Err(error) =\n                topology::require_empty_handle(anchor.retained_handle(), wait)\n            {',
        '} else if let Err(error) = Ok::<(), io::Error>(()) {',
        'store::foundation::destination_name_probe::tests::races::destination_name_probe_final_empty_check_detects_foreign_root_entry',
        'called `Result::unwrap_err()` on an `Ok` value',
    ),
    (
        'dest-repr-final-binding',
        'destination_name_probe.rs',
        '} else if let Err(error) = anchor.revalidate(wait) {',
        '} else if let Err(error) = Ok::<(), io::Error>(()) {',
        'store::foundation::destination_name_probe::tests::races::destination_name_probe_final_binding_rejects_root_replacement',
        'called `Result::unwrap_err()` on an `Ok` value',
    ),
]


DEST_PREPARE_CASES = [
    (
        'dest-prepare-directory-no-follow',
        'destination_tree_prepare_native_entry.rs',
        '| libc::O_DIRECTORY\n            | libc::O_NOFOLLOW\n            | libc::O_CLOEXEC',
        '| libc::O_DIRECTORY\n            | libc::O_CLOEXEC',
        'store::foundation::destination_tree_prepare::tests::races::prepared_tree_directory_symlink_swap_before_open_is_refused',
        'directory symlink replacement was not refused by no-follow open',
    ),
    (
        'dest-prepare-no-adopt-file',
        'destination_tree_prepare_native_entry.rs',
        'libc::O_RDWR | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC;',
        'libc::O_RDWR | libc::O_CREAT | libc::O_NOFOLLOW | libc::O_CLOEXEC;',
        'store::foundation::destination_tree_prepare::tests::races::prepared_tree_never_adopts_foreign_file_after_representation',
        'called `Option::unwrap()` on a `None` value',
    ),
    (
        'dest-prepare-cleanup-identity',
        'destination_tree_prepare_native_entry.rs',
        'if identity(&self.parent, &self.name)? != self.identity {',
        'if false && identity(&self.parent, &self.name)? != self.identity {',
        'store::foundation::destination_tree_prepare::tests::races::prepared_tree_replacement_is_preserved_and_cleanup_is_incomplete',
        'assertion `left == right` failed',
    ),
    (
        'dest-prepare-exact-namespace',
        'destination_tree_prepare_native.rs',
        'require_child_count(&self.root, self.root_expected_children)?;',
        'let _ = self.root_expected_children;',
        'store::foundation::destination_tree_prepare::tests::races::prepared_tree_final_validation_rejects_unplanned_root_entry',
        'called `Option::unwrap()` on a `None` value',
    ),
    (
        'dest-prepare-leaf-binding',
        'destination_tree_prepare_native.rs',
        '            file.revalidate()?;',
        '            let _ = file;',
        'store::foundation::destination_tree_prepare::tests::races::prepared_tree_final_entry_revalidation_rejects_leaf_replacement',
        'called `Option::unwrap()` on a `None` value',
    ),
    (
        'dest-prepare-root-binding',
        'destination_tree_prepare.rs',
        '        if let Err(error) = anchor.revalidate(wait) {',
        '        if let Err(error) = Ok::<(), io::Error>(()) {',
        'store::foundation::destination_tree_prepare::tests::races::prepared_tree_final_binding_rejects_root_replacement',
        'called `Option::unwrap()` on a `None` value',
    ),
    (
        'dest-prepare-complete-root-binding',
        'destination_tree_prepare.rs',
        '                    if let Err(error) = this.anchor.revalidate(wait) {',
        '                    if let Err(error) = Ok::<(), io::Error>(()) {',
        'store::foundation::destination_tree_prepare::tests::races::prepared_tree_complete_rejects_root_replacement',
        'called `Result::unwrap_err()` on an `Ok` value',
    ),
    (
        'dest-prepare-complete-descendant-binding',
        'destination_tree_prepare.rs',
        '                    if let Err(error) = this.inner.revalidate_final_descendants(wait) {',
        '                    if let Err(error) = Ok::<(), io::Error>(()) {',
        'store::foundation::destination_tree_prepare::tests::races::prepared_tree_complete_rejects_leaf_replacement_after_inner_validation',
        'called `Result::unwrap_err()` on an `Ok` value',
    ),
    (
        'dest-prepare-complete-root-after-descendants',
        'destination_tree_prepare.rs',
        '                    if let Err(error) = this.inner.revalidate_final_descendants(wait) {\n                        return Err(fail_and_cleanup(this.inner, error));\n                    }\n                    if let Err(error) = after_final_descendants() {\n                        return Err(fail_and_cleanup(this.inner, error));\n                    }\n                    if let Err(error) = this.inner.revalidate_root_namespace(wait) {\n                        return Err(fail_and_cleanup(this.inner, error));\n                    }\n                    if let Err(error) = this.anchor.revalidate(wait) {\n                        // Final root binding after descendants, before disarm.\n                        return Err(fail_and_cleanup(this.inner, error));\n                    }',
        '                    if let Err(error) = this.inner.revalidate_final_descendants(wait) {\n                        return Err(fail_and_cleanup(this.inner, error));\n                    }\n                    if let Err(error) = this.anchor.revalidate(wait) {\n                        // Final root binding after descendants, before disarm.\n                        return Err(fail_and_cleanup(this.inner, error));\n                    }\n                    if let Err(error) = after_final_descendants() {\n                        return Err(fail_and_cleanup(this.inner, error));\n                    }\n                    if let Err(error) = this.inner.revalidate_root_namespace(wait) {\n                        return Err(fail_and_cleanup(this.inner, error));\n                    }',
        'store::foundation::destination_tree_prepare::tests::races::prepared_tree_complete_checks_root_after_descendants',
        'called `Result::unwrap_err()` on an `Ok` value',
    ),
]

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", type=Path, required=True)
    out = parser.parse_args().out.resolve()
    out.mkdir(parents=True, exist_ok=False)
    git = lambda *args: subprocess.check_output(["git", *args], cwd=ROOT, text=True).strip()
    receipt = dict(schema=1, checkout=git("rev-parse", "HEAD"), tree=git("rev-parse", "HEAD^{tree}"),
                   controls=[], production_protocol_enabled=False)

    def require(ok, message):
        if not ok:
            raise RuntimeError(message)

    def run(root, argv, name, limit=600):
        path = out / (name + ".log")
        env = os.environ.copy()
        env.update(CARGO_BUILD_JOBS="2", CARGO_INCREMENTAL="0", CARGO_PROFILE_TEST_DEBUG="0", CARGO_PROFILE_DEV_DEBUG="0")
        with path.open("wb") as log:
            child = subprocess.Popen(argv, cwd=root, env=env, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
            try:
                code = child.wait(timeout=limit)
            except subprocess.TimeoutExpired:
                os.killpg(child.pid, signal.SIGKILL)
                child.wait(timeout=30)
                raise RuntimeError("timeout is not a causal kill: " + name)
        return code, path.read_text(errors="replace")

    cargo = ["cargo", "+1.96.0", "test", "--locked", "-p", "lightr-store", "--lib"]
    suite = cargo + [PREFIX + "tests::", "--", "--nocapture"]
    expected = r"test result: ok\. 20 passed; 0 failed; 0 ignored;"
    destination_suite = cargo + ["store::foundation::destination_tests::", "--", "--nocapture"]
    destination_expected = r"test result: ok\. 11 passed; 0 failed; 0 ignored;"
    planned_suite = cargo + ["store::foundation::planned_roots_tests::", "--", "--nocapture"]
    planned_expected = r"test result: ok\. 17 passed; 0 failed; 0 ignored;"
    empty_suite = cargo + [PREFIX + "topology_empty::tests::", "--", "--nocapture"]
    empty_expected = r"test result: ok\. 11 passed; 0 failed; 0 ignored;"
    descendant_suite = cargo + [PREFIX + "topology_descendant::tests::", "--", "--nocapture"]
    descendant_expected = r"test result: ok\. 12 passed; 0 failed; 0 ignored;"
    scratch_suite = cargo + ["store::foundation::anchored_scratch::", "--", "--nocapture"]
    scratch_expected = r"test result: ok\. 15 passed; 0 failed; 0 ignored;"
    link_suite = cargo + [PREFIX + "topology_link::tests::", "--", "--nocapture"]
    link_expected = r"test result: ok\. 14 passed; 0 failed; 0 ignored;"
    listing_suite = cargo + [PREFIX + "topology_listing::native::tests::", "--", "--nocapture"]
    listing_expected = r"test result: ok\. 16 passed; 0 failed; 0 ignored;"
    anchor_suite = cargo + ["store::foundation::destination_anchor::tests::", "--", "--nocapture"]
    anchor_expected = r"test result: ok\. 11 passed; 0 failed; 0 ignored;"
    repr_suite = cargo + ["store::foundation::destination_name_probe::tests::", "--", "--nocapture"]
    repr_expected = r"test result: ok\. 14 passed; 0 failed; 0 ignored;"
    prepare_suite = cargo + ["store::foundation::destination_tree_prepare::tests::", "--", "--nocapture"]
    prepare_expected = r"test result: ok\. 31 passed; 0 failed; 0 ignored;"
    try:
        require(not git("status", "--porcelain", "--untracked-files=no"), "tracked source dirty")
        archive = subprocess.check_output(["git", "archive", "--format=zip", "HEAD"], cwd=ROOT)
        (out / "source.zip").write_bytes(archive)
        with tempfile.TemporaryDirectory(prefix="si01-topology-controls-") as temporary:
            root = Path(temporary).resolve()
            with zipfile.ZipFile(io.BytesIO(archive)) as z:
                for item in z.infolist():
                    require((root / item.filename).resolve().is_relative_to(root), "unsafe archive path")
                z.extractall(root)
            code, text = run(root, suite, "pristine")
            require(code == 0 and re.search(expected, text), "pristine suite absent or failed")
            code, text = run(root, destination_suite, "destination-pristine")
            require(code == 0 and re.search(destination_expected, text), "destination pristine suite absent or failed")
            code, text = run(root, planned_suite, "planned-pristine")
            require(code == 0 and re.search(planned_expected, text), "planned-root pristine suite absent or failed")
            code, text = run(root, empty_suite, "empty-pristine")
            require(code == 0 and re.search(empty_expected, text), "empty pristine suite absent or failed")
            code, text = run(root, descendant_suite, "descendant-pristine")
            require(code == 0 and re.search(descendant_expected, text), "descendant pristine suite absent or failed")
            code, text = run(root, scratch_suite, "scratch-pristine")
            require(code == 0 and re.search(scratch_expected, text), "scratch pristine suite absent or failed")
            code, text = run(root, link_suite, "link-pristine")
            require(code == 0 and re.search(link_expected, text), "link pristine suite failed")
            code, text = run(root, listing_suite, "listing-pristine")
            require(code == 0 and re.search(listing_expected, text), "listing pristine suite failed")
            code, text = run(root, anchor_suite, "anchor-pristine")
            require(code == 0 and re.search(anchor_expected, text), "anchor pristine suite failed")
            code, text = run(root, repr_suite, "repr-pristine")
            require(code == 0 and re.search(repr_expected, text), "destination representation pristine suite failed")
            code, text = run(root, prepare_suite, "prepare-pristine")
            require(code == 0 and re.search(prepare_expected, text), "destination prepare pristine suite failed")
            for label, name, old, new, test, message in CASES + EMPTY_CASES + DESCENDANT_CASES + SCRATCH_CASES + LINK_CASES + LISTING_CASES + DEST_ANCHOR_CASES + DEST_REPR_CASES + DEST_PREPARE_CASES:
                path = root / BASE / name
                original = path.read_text()
                require(original.count(old) == 1, "mutation seam mismatch: " + label)
                path.write_text(original.replace(old, new, 1))
                try:
                    code, _ = run(root, cargo + ["--no-run"], label + "-build")
                    require(code == 0, "mutant compilation failed: " + label)
                    full = test if test.startswith("store::") else PREFIX + "tests::" + test
                    code, text = run(root, cargo + [full, "--", "--exact"], label + "-test", 60)
                    require(code == 101 and f"test {full} ... FAILED" in text and message in text
                            and re.search(r"test result: FAILED\. 0 passed; 1 failed; 0 ignored;", text),
                            "expected causal assertion absent: " + label)
                    receipt["controls"].append(dict(name=label, build_exit=0, test_exit=101, test=full))
                finally:
                    path.write_text(original)
            code, text = run(root, suite, "restored")
            require(code == 0 and re.search(expected, text), "restored suite failed")
            code, text = run(root, destination_suite, "destination-restored")
            require(code == 0 and re.search(destination_expected, text), "destination restored suite failed")
            code, text = run(root, planned_suite, "planned-restored")
            require(code == 0 and re.search(planned_expected, text), "planned-root restored suite failed")
            code, text = run(root, empty_suite, "empty-restored")
            require(code == 0 and re.search(empty_expected, text), "empty restored suite failed")
            code, text = run(root, descendant_suite, "descendant-restored")
            require(code == 0 and re.search(descendant_expected, text), "descendant restored suite absent or failed")
            code, text = run(root, scratch_suite, "scratch-restored")
            require(code == 0 and re.search(scratch_expected, text), "scratch restored suite failed")
            code, text = run(root, link_suite, "link-restored")
            require(code == 0 and re.search(link_expected, text), "link restored suite failed")
            code, text = run(root, listing_suite, "listing-restored")
            require(code == 0 and re.search(listing_expected, text), "listing restored suite failed")
            code, text = run(root, anchor_suite, "anchor-restored")
            require(code == 0 and re.search(anchor_expected, text), "anchor restored suite failed")
            code, text = run(root, repr_suite, "repr-restored")
            require(code == 0 and re.search(repr_expected, text), "destination representation restored suite failed")
            code, text = run(root, prepare_suite, "prepare-restored")
            require(code == 0 and re.search(prepare_expected, text), "destination prepare restored suite failed")
        receipt["status"] = "TOPOLOGY_CONTROLS_PASSED"
    except Exception as error:
        receipt["status"] = "FAILED"
        receipt["error"] = str(error)
    receipt["artifacts"] = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(out.iterdir()) if p.is_file()}
    (out / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps(receipt, indent=2))
    return 0 if receipt["status"] == "TOPOLOGY_CONTROLS_PASSED" else 1


if __name__ == "__main__":
    raise SystemExit(main())
