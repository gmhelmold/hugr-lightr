"""Compiled cooperative-copy regressions in an isolated archive, never live stores."""
from __future__ import annotations
import argparse
import hashlib
import io
import json
from pathlib import Path
import re
import subprocess
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[2]
STORE = "crates/lightr-store/src/store/"
CAPTURE = "store::foundation::capture_cancellation_tests::"
COPY = "store::cas::preparation::cancellation_tests::"
CORE = "core::digest::checked_tests::"

# Select the original module family; retain counts and portable log filenames.
SUITES = [
    ("lightr-core", CORE + "checked_hash_", "checked_hash_", 7),
    ("lightr-store", CAPTURE + "capture_cancellation_", "capture_cancellation_", 8),
    ("lightr-store", COPY + "checkpoint_", "checkpoint_", 7),
]
CASES = [
    ("copy-routing", "lightr-store", STORE+"foundation/publication.rs",
     "LeasedStagedFile::copy_with_wait(self, reader, expected, wait, id)",
     "LeasedStagedFile::copy(self, reader, expected, id)",
     CAPTURE+"capture_cancellation_after_read_stops_before_a_second_read",
     "cancelled capture must not call the source again"),
    ("read-retry-checkpoint", "lightr-store", STORE+"cas/preparation.rs",
     "    loop {\n        checkpoint()?;\n        let size = match reader.read",
     "    loop {\n        let size = match reader.read",
     CAPTURE+"capture_cancellation_is_not_retried_as_os_interrupted",
     "cancelled capture must not call the source again"),
    ("write-retry-checkpoint", "lightr-store", STORE+"cas/preparation.rs",
     "    while !bytes.is_empty() {\n        checkpoint()?;\n        let size = match writer.write",
     "    while !bytes.is_empty() {\n        let size = match writer.write",
     COPY+"checkpoint_write_interruption_is_not_blindly_retried",
     "assertion `left == right` failed"),
    ("hash-initial-checkpoint", "lightr-core", "crates/lightr-core/src/core/digest.rs",
     "        loop {\n            checkpoint()?;\n            let count = match reader.read",
     "        loop {\n            let count = match reader.read",
     CORE+"checked_hash_checkpoint_interruption_is_terminal",
     "read after cancellation"),
    ("sync-completion-checkpoint", "lightr-store", STORE+"cas/preparation.rs",
     "        sync(&file).map_err(|error| failure(Phase::PayloadSync, error))?;\n        checkpoint().map_err(|error| failure(Phase::PayloadSync, error))?;",
     "        sync(&file).map_err(|error| failure(Phase::PayloadSync, error))?;",
     COPY+"checkpoint_sync_completion_cannot_hide_cancellation",
     "called `Option::unwrap()` on a `None` value"),
]


def need(ok, message):
    if not ok:
        raise ValueError(message)


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument("--out", required=True, type=Path)
    args=parser.parse_args()
    out=args.out.resolve(); out.mkdir(parents=True, exist_ok=False)
    git=lambda *a: subprocess.check_output(["git", *a], cwd=ROOT, text=True).strip()
    receipt={"schema":1, "checkout_sha":git("rev-parse","HEAD"),
             "tree":git("rev-parse","HEAD^{tree}"), "controls":[],
             "production_protocol_enabled":False, "whole_package_qualified":False}

    def run(root, argv, label):
        with (out/(label+".log")).open("wb") as log:
            # Bounded witnesses never spin on a mutant: their second I/O call
            # returns an ordinary error. A timeout is failure, not a test kill.
            result=subprocess.run(argv, cwd=root, stdout=log, stderr=subprocess.STDOUT, timeout=600)
        return result.returncode, (out/(label+".log")).read_text(errors="replace")

    def suite(root, label):
        for crate, selection, name, count in SUITES:
            code,text=run(root,["cargo","+1.96.0","test","--locked","-p",crate,"--lib",selection,"--","--test-threads=1"],label+"-"+name)
            need(code==0 and re.search(r"test result: ok\. "+str(count)+r" passed; 0 failed; 0 ignored; 0 measured; \d+ filtered out",text),"missing/failed suite "+name)

    try:
        need(not git("status","--porcelain","--untracked-files=no"),"dirty tracked inputs")
        archive=subprocess.check_output(["git","archive","--format=zip","HEAD"],cwd=ROOT)
        (out/"source.zip").write_bytes(archive)
        with tempfile.TemporaryDirectory(prefix="si01-cancellation-controls-") as temp:
            root=Path(temp).resolve()
            with zipfile.ZipFile(io.BytesIO(archive)) as source:
                for member in source.infolist():
                    need((root/member.filename).resolve().is_relative_to(root),"archive path outside scratch")
                source.extractall(root)
            suite(root,"pristine")
            for label,crate,file,old,new,test,assertion in CASES:
                path=root/file; original=path.read_text()
                need(original.count(old)==1,"mutation seam drift: "+label)
                try:
                    path.write_text(original.replace(old,new,1))
                    code,_=run(root,["cargo","+1.96.0","test","--locked","-p",crate,"--lib","--no-run"],label+"-build")
                    need(code==0,"mutant compile failure: "+label)
                    code,text=run(root,["cargo","+1.96.0","test","--locked","-p",crate,"--lib",test,"--","--exact"],label+"-test")
                    need(code==101 and "test "+test+" ... FAILED" in text and assertion in text
                         and re.search(r"test result: FAILED\. 0 passed; 1 failed; 0 ignored; 0 measured; \d+ filtered out",text),"missing/wrong assertion: "+label)
                    receipt["controls"].append({"id":label,"test":test,"build_exit":0,"test_exit":101,"status":"KILLED_CAUSALLY"})
                finally:
                    path.write_text(original)
            suite(root,"restored")
        receipt["status"]="CANCELLATION_CONTROLS_PASSED"
    except Exception as exc:
        receipt.update(status="FAILED",error=str(exc))
    receipt["artifacts"]=[{"path":p.name,"sha256":hashlib.sha256(p.read_bytes()).hexdigest()} for p in sorted(out.iterdir()) if p.is_file()]
    (out/"receipt.json").write_text(json.dumps(receipt,indent=2)+"\n")
    print(json.dumps(receipt,indent=2))
    return 0 if receipt["status"]=="CANCELLATION_CONTROLS_PASSED" else 1

if __name__=="__main__":
    raise SystemExit(main())
