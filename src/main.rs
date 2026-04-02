fn main() {
    let mut engine = sabretooth::uci::Engine::new();
    if let Err(err) = engine.run() {
        eprintln!("uci error: {err}");
    }
}
