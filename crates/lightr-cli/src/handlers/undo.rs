//! `lightr undo` handler — revert a ref to its previous version (or a specific version).

use lightr_core::LightrError;
use lightr_index::undo_to;
use lightr_store::Store;
use serde::Serialize;

use crate::exit::die_lightr;

#[derive(Serialize)]
struct UndoJson {
    name: String,
    root: String,
    parent: Option<String>,
    created_at_unix: u64,
    tool_version: String,
}

pub fn run(name: &str, to: Option<&str>, json: bool) -> i32 {
    let store = match Store::open(Store::default_root()) {
        Ok(s) => s,
        Err(e) => return die_lightr(&e),
    };

    let rec = match undo_to(&store, name, to) {
        Ok(r) => r,
        Err(LightrError::RefNotFound(_)) | Err(LightrError::InvalidRef(_)) => {
            eprintln!("lightr: ref not found: {name}");
            return 2;
        }
        Err(e) => return die_lightr(&e),
    };

    if json {
        let out = UndoJson {
            name: rec.name.clone(),
            root: rec.root.to_hex(),
            parent: rec.parent.map(|d| d.to_hex()),
            created_at_unix: rec.created_at_unix,
            tool_version: rec.tool_version.clone(),
        };
        println!("{}", serde_json::to_string(&out).expect("serialize undo"));
    } else {
        let hex = rec.root.to_hex();
        let short = &hex[..16];
        if let Some(target) = to {
            println!("undo: {} restored to {} (target: {})", name, short, target);
        } else {
            println!("undo: {} restored to {}", name, short);
        }
    }

    0
}
