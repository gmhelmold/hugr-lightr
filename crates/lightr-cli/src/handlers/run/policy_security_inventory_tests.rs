use super::*;
use crate::handlers::run::flags::RawRcFlags;

struct ControlMap {
    control: &'static str,
    cli: &'static str,
    parser: &'static str,
    run_config: &'static str,
    exec_spec: Option<&'static str>,
    witness: &'static str,
}

// Closed parser -> RcConfig -> RunSpec/ExecSpec lowering inventory. Keep this
// beside lowering code: changing either side must break this test.
const CONTROLS: &[ControlMap] = &[
    ControlMap {
        control: "user",
        cli: "--user",
        parser: "RunArgs.user",
        run_config: "user",
        exec_spec: Some("user"),
        witness: "crates/lightr-cli/src/handlers/run/paths.rs::eff_user",
    },
    ControlMap {
        control: "hostname",
        cli: "--hostname",
        parser: "RawRcFlags.hostname",
        run_config: "hostname",
        exec_spec: None,
        witness: "crates/lightr-cli/src/handlers/run/policy.rs::build_detached_spec",
    },
    ControlMap {
        control: "labels",
        cli: "--label",
        parser: "RawRcFlags.label",
        run_config: "labels",
        exec_spec: None,
        witness: "crates/lightr-cli/src/handlers/run/policy.rs::build_detached_spec",
    },
    ControlMap {
        control: "tty",
        cli: "--tty",
        parser: "RawRcFlags.tty",
        run_config: "tty",
        exec_spec: None,
        witness: "crates/lightr-cli/src/handlers/run/policy.rs::build_detached_spec",
    },
    ControlMap {
        control: "init",
        cli: "--init",
        parser: "RawRcFlags.init",
        run_config: "init",
        exec_spec: Some("init"),
        witness: "crates/lightr-cli/src/handlers/run/paths.rs::run_engine",
    },
    ControlMap {
        control: "privileged",
        cli: "--privileged",
        parser: "RawRcFlags.privileged",
        run_config: "privileged",
        exec_spec: None,
        witness: "crates/lightr-cli/src/handlers/run/policy.rs::rc_privileged_policy",
    },
    ControlMap {
        control: "read_only",
        cli: "--read-only",
        parser: "RawRcFlags.read_only",
        run_config: "read_only",
        exec_spec: Some("read_only"),
        witness: "crates/lightr-cli/src/handlers/run/paths.rs::run_engine",
    },
    ControlMap {
        control: "cap_add",
        cli: "--cap-add",
        parser: "RawRcFlags.cap_add",
        run_config: "cap_add",
        exec_spec: Some("cap_add"),
        witness: "crates/lightr-cli/src/handlers/run/paths.rs::run_engine",
    },
    ControlMap {
        control: "cap_drop",
        cli: "--cap-drop",
        parser: "RawRcFlags.cap_drop",
        run_config: "cap_drop",
        exec_spec: Some("cap_drop"),
        witness: "crates/lightr-cli/src/handlers/run/paths.rs::run_engine",
    },
    ControlMap {
        control: "seccomp",
        cli: "--seccomp",
        parser: "RawRcFlags.seccomp",
        run_config: "seccomp",
        exec_spec: Some("seccomp"),
        witness: "crates/lightr-cli/src/handlers/run/paths.rs::run_engine",
    },
    ControlMap {
        control: "apparmor",
        cli: "--apparmor",
        parser: "RawRcFlags.apparmor",
        run_config: "apparmor",
        exec_spec: Some("apparmor"),
        witness: "crates/lightr-cli/src/handlers/run/paths.rs::run_engine",
    },
    ControlMap {
        control: "memory_limit",
        cli: "--memory",
        parser: "RunArgs.memory",
        run_config: "limits.memory_bytes",
        exec_spec: Some("limits"),
        witness: "crates/lightr-cli/src/handlers/run/mod.rs::ResourceLimits::parse",
    },
    ControlMap {
        control: "cpu_limit",
        cli: "--cpus",
        parser: "RunArgs.cpus",
        run_config: "limits.cpu_millis",
        exec_spec: Some("limits"),
        witness: "crates/lightr-cli/src/handlers/run/mod.rs::ResourceLimits::parse",
    },
    ControlMap {
        control: "pids_limit",
        cli: "--pids-limit",
        parser: "RawRcFlags.pids_limit",
        run_config: "limits.pids_max",
        exec_spec: Some("limits"),
        witness: "crates/lightr-cli/src/handlers/run/mod.rs::with_pids",
    },
    ControlMap {
        control: "ulimit",
        cli: "--ulimit",
        parser: "RawRunFlags.ulimit",
        run_config: "ulimits",
        exec_spec: Some("ulimits"),
        witness: "crates/lightr-cli/src/handlers/run/mod.rs::parse_ulimits",
    },
    ControlMap {
        control: "oom_score_adj",
        cli: "--oom-score-adj",
        parser: "RawRcFlags.oom_score_adj",
        run_config: "oom_score_adj",
        exec_spec: Some("oom_score_adj"),
        witness: "crates/lightr-cli/src/handlers/run/paths.rs::run_engine",
    },
    ControlMap {
        control: "shm_size",
        cli: "--shm-size",
        parser: "RawRcFlags.shm_size",
        run_config: "shm_size",
        exec_spec: Some("shm_size"),
        witness: "crates/lightr-cli/src/handlers/run/paths.rs::run_engine",
    },
    ControlMap {
        control: "tmpfs",
        cli: "--tmpfs",
        parser: "RawRunFlags.tmpfs",
        run_config: "tmpfs",
        exec_spec: Some("tmpfs"),
        witness: "crates/lightr-cli/src/handlers/run/mod.rs::parse_tmpfs",
    },
    ControlMap {
        control: "network_mode",
        cli: "--net",
        parser: "RunArgs.net",
        run_config: "net_isolate",
        exec_spec: Some("net_isolate"),
        witness: "crates/lightr-cli/src/handlers/run/flags.rs::resolve_net_isolate",
    },
    ControlMap {
        control: "add_host",
        cli: "--add-host",
        parser: "RawRunFlags.add_host",
        run_config: "add_host",
        exec_spec: Some("add_host"),
        witness: "crates/lightr-cli/src/handlers/run/policy.rs::resolve_add_host_pairs",
    },
    ControlMap {
        control: "healthcheck",
        cli: "--health-*",
        parser: "HealthFlags",
        run_config: "healthcheck",
        exec_spec: None,
        witness: "crates/lightr-cli/src/handlers/run/flags.rs::HealthFlags",
    },
    ControlMap {
        control: "secret",
        cli: "--secret",
        parser: "RunArgs.secret",
        run_config: "secrets",
        exec_spec: None,
        witness: "crates/lightr-cli/src/handlers/run/policy.rs::resolve_store_files",
    },
    ControlMap {
        control: "config",
        cli: "--config",
        parser: "RunArgs.config",
        run_config: "configs",
        exec_spec: None,
        witness: "crates/lightr-cli/src/handlers/run/policy.rs::resolve_store_files",
    },
];

fn source(path: &str) -> Option<&'static str> {
    match path {
        "crates/lightr-cli/src/handlers/run/policy.rs" => Some(include_str!("policy.rs")),
        "crates/lightr-cli/src/handlers/run/paths.rs" => Some(include_str!("paths.rs")),
        "crates/lightr-cli/src/handlers/run/mod.rs" => Some(include_str!("mod.rs")),
        "crates/lightr-cli/src/handlers/run/flags.rs" => Some(include_str!("flags.rs")),
        _ => None,
    }
}

fn validate_inventory(inventory: &serde_json::Value) -> Result<(), String> {
    let rows = inventory["controls"].as_array().ok_or("missing controls")?;
    if rows.len() != CONTROLS.len() {
        return Err(format!(
            "expected {} controls, got {}",
            CONTROLS.len(),
            rows.len()
        ));
    }
    for map in CONTROLS {
        let row = rows
            .iter()
            .find(|row| row["control"] == map.control)
            .ok_or_else(|| format!("missing control: {}", map.control))?;
        for (key, expected) in [
            ("cli", map.cli),
            ("parser", map.parser),
            ("run_config", map.run_config),
            ("witness", map.witness),
        ] {
            if row[key].as_str() != Some(expected) {
                return Err(format!("{} has wrong {key}", map.control));
            }
        }
        if row["exec_spec"].as_str() != map.exec_spec {
            return Err(format!("{} has wrong exec_spec", map.control));
        }
        for field in ["enforcement", "platform", "oracle", "fixture", "mutation"] {
            if !row[field].as_str().is_some_and(|value| !value.is_empty()) {
                return Err(format!("{} missing {field}", map.control));
            }
        }
        let (path, symbol) = map
            .witness
            .split_once("::")
            .ok_or_else(|| format!("{} witness has no symbol", map.control))?;
        let contents = source(path)
            .ok_or_else(|| format!("{} witness path is not allowlisted", map.control))?;
        if !contents.contains(symbol) {
            return Err(format!("{} witness symbol is not resolvable", map.control));
        }
        for engine in ["native", "ns", "vz"] {
            if !matches!(
                row[engine].as_str(),
                Some("enforced" | "refused" | "unsupported")
            ) {
                return Err(format!("{}/{engine} has no typed outcome", map.control));
            }
        }
    }
    Ok(())
}

#[test]
fn inventory_matches_closed_lowering_map_and_resolvable_witnesses() {
    let inventory: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../benchmarks/s3/security/inventory.json"
    )))
    .unwrap();
    validate_inventory(&inventory).unwrap();
}

#[test]
fn inventory_validator_rejects_mapping_and_witness_mutations() {
    let mut inventory: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../benchmarks/s3/security/inventory.json"
    )))
    .unwrap();
    inventory["controls"][0]["cli"] = serde_json::Value::String("--mutated".into());
    assert!(validate_inventory(&inventory).is_err());
    inventory["controls"][0]["cli"] = serde_json::Value::String("--user".into());
    inventory["controls"][0]["witness"] =
        serde_json::Value::String("crates/lightr-cli/src/handlers/run/paths.rs::gone".into());
    assert!(validate_inventory(&inventory).is_err());
}

#[test]
fn raw_rc_flags_lower_to_runspec_and_engine_policy_consumes_lsm_fields() {
    let rc = RawRcFlags {
        hostname: Some("host".to_string()),
        label: vec!["key=value".to_string()],
        cap_add: vec!["NET_BIND_SERVICE".to_string()],
        cap_drop: vec!["ALL".to_string()],
        privileged: true,
        tty: true,
        init: true,
        read_only: true,
        oom_score_adj: Some(100),
        pids_limit: Some(16),
        shm_size: Some("64m".to_string()),
        apparmor: Some("profile".to_string()),
        seccomp: Some("profile.json".to_string()),
    }
    .resolve()
    .unwrap();
    let spec = build_detached_spec(
        std::path::PathBuf::from("/work"),
        &[],
        &[],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        None,
        None,
        None,
        None,
        ResourceLimits::default(),
        &rc,
        &RunFlags::default(),
    );
    assert_eq!(spec.hostname.as_deref(), Some("host"));
    assert_eq!(spec.labels, vec![("key".to_string(), "value".to_string())]);
    assert_eq!(spec.cap_add, ["NET_BIND_SERVICE"]);
    assert_eq!(spec.cap_drop, ["ALL"]);
    assert!(spec.privileged && spec.tty && spec.init && spec.read_only);
    assert_eq!(spec.oom_score_adj, Some(100));
    assert_eq!(spec.pids_limit, Some(16));
    assert_eq!(spec.shm_size, Some(64 * 1024 * 1024));
    assert_eq!(engine_capability_policy(EngineKind::Ns, &rc), None);
    assert_eq!(engine_capability_policy(EngineKind::Native, &rc), Some(2));
}
