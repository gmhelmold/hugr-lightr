//! `lightr diff` handler — diff a ref against a previous version.

use lightr_core::LightrError;
use lightr_index::diff_manifests;
use lightr_store::Store;
use serde::Serialize;

use crate::exit::die_lightr;

#[derive(Serialize)]
struct DiffJson {
    added: Vec<String>,
    removed: Vec<String>,
    changed: Vec<String>,
}

pub fn run(name: &str, at: usize, to: Option<&str>, dir_opt: Option<&str>, json: bool) -> i32 {
    let store = match Store::open(Store::default_root()) {
        Ok(s) => s,
        Err(e) => return die_lightr(&e),
    };

    // Get ref log for this name
    let ref_log = match store.ref_log(name) {
        Ok(log) if !log.is_empty() => log,
        Ok(_) => {
            eprintln!("lightr: ref not found: {name}");
            return 2;
        }
        Err(LightrError::RefNotFound(_)) | Err(LightrError::InvalidRef(_)) => {
            eprintln!("lightr: ref not found: {name}");
            return 2;
        }
        Err(e) => return die_lightr(&e),
    };

    // Current manifest (index 0)
    let current_manifest = match store.get_bytes(&ref_log[0].root) {
        Ok(bytes) => match lightr_core::Manifest::decode(&bytes) {
            Ok(m) => m,
            Err(e) => return die_lightr(&e),
        },
        Err(e) => return die_lightr(&e),
    };

    let report = if let Some(dir) = dir_opt {
        // Diff against local directory
        let dir_path = std::path::Path::new(dir);
        let mut index = match lightr_index::Index::load_for(dir_path) {
            Ok(i) => i,
            Err(e) => return die_lightr(&e),
        };
        let walk = match lightr_index::scan(dir_path, &mut index) {
            Ok(r) => r,
            Err(e) => return die_lightr(&e),
        };
        diff_manifests(&current_manifest, &walk.manifest)
    } else if let Some(to_ref) = to {
        // Diff against another ref@version (e.g., @ref@v1)
        let (to_name, _to_version): (String, &str) = if let Some(rest) = to_ref.strip_prefix('@') {
            // Parse @ref@version or just @ref
            if let Some(at_pos) = rest.rfind('@') {
                let name = &rest[..at_pos];
                let version = &rest[at_pos + 1..];
                (format!("@{}", name), version)
            } else {
                (to_ref.to_string(), "0") // default to latest
            }
        } else {
            (to_ref.to_string(), "0")
        };
        let to_log = match store.ref_log(&to_name) {
            Ok(log) if !log.is_empty() => log,
            Ok(_) => {
                eprintln!("lightr: ref not found: {to_name}");
                return 2;
            }
            Err(LightrError::RefNotFound(_)) | Err(LightrError::InvalidRef(_)) => {
                eprintln!("lightr: ref not found: {to_name}");
                return 2;
            }
            Err(e) => return die_lightr(&e),
        };
        // Default to latest (index 0) of the target ref
        let old_manifest = match store.get_bytes(&to_log[0].root) {
            Ok(bytes) => match lightr_core::Manifest::decode(&bytes) {
                Ok(m) => m,
                Err(e) => return die_lightr(&e),
            },
            Err(e) => return die_lightr(&e),
        };
        diff_manifests(&old_manifest, &current_manifest)
    } else {
        // Diff against historical ref entry (--at)
        if ref_log.len() <= at {
            eprintln!(
                "lightr: not enough history (need index {at}, have {})",
                ref_log.len()
            );
            return 2;
        }
        let old_manifest = match store.get_bytes(&ref_log[at].root) {
            Ok(bytes) => match lightr_core::Manifest::decode(&bytes) {
                Ok(m) => m,
                Err(e) => return die_lightr(&e),
            },
            Err(e) => return die_lightr(&e),
        };
        diff_manifests(&old_manifest, &current_manifest)
    };

    if json {
        let out = DiffJson {
            added: report.added.clone(),
            removed: report.removed.clone(),
            changed: report.changed.clone(),
        };
        println!("{}", serde_json::to_string(&out).expect("serialize diff"));
    } else {
        for path in &report.added {
            println!("+{path}");
        }
        for path in &report.removed {
            println!("-{path}");
        }
        for path in &report.changed {
            println!("~{path}");
        }
    }

    // Exit 0 if no differences, 1 if there are differences
    if report.added.is_empty() && report.removed.is_empty() && report.changed.is_empty() {
        0
    } else {
        1
    }
}
