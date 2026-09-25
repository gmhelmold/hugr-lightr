#[cfg(unix)]
fn main() {
    if !lightr_run::volume_gate_dispatch() {
        std::process::exit(127);
    }
}

#[cfg(not(unix))]
fn main() {
    std::process::exit(127);
}
