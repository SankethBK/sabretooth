mod attacks;
mod bitboard;
mod book;
mod board;
mod movegen;
mod search;
mod types;
mod uci;
#[cfg(test)]
mod testsuite;

fn main() {
    let mut engine = uci::Engine::new();
    if let Err(err) = engine.run() {
        eprintln!("uci error: {err}");
    }
}
