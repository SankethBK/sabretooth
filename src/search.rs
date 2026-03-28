use crate::board::Board;
use crate::types::{Color, Move};

pub const DEFAULT_SEARCH_DEPTH: u32 = 6;
const MATE_SCORE: i32 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchResult {
    pub best_move: Option<Move>,
    pub score: i32,
}

pub fn find_best_move(board: &mut Board, depth: u32) -> Option<Move> {
    search(board, depth).best_move
}

pub fn search(board: &mut Board, depth: u32) -> SearchResult {
    let legal_moves = board.generate_legal_moves();

    if legal_moves.is_empty() {
        let score = if board.is_in_check(board.side_to_move()) {
            -MATE_SCORE
        } else {
            0
        };
        return SearchResult {
            best_move: None,
            score,
        };
    }

    let mut best_move = None;
    let mut best_score = i32::MIN;
    let mut alpha = i32::MIN + 1;
    let beta = i32::MAX;

    for mv in legal_moves {
        board.make_move(mv);
        let score = -negamax(board, depth.saturating_sub(1), -beta, -alpha, 1);
        board.unmake_move();

        if score > best_score {
            best_score = score;
            best_move = Some(mv);
        }

        alpha = alpha.max(score);
    }

    SearchResult {
        best_move,
        score: best_score,
    }
}

pub fn approximate_principal_variation(board: &Board, depth: u32) -> (SearchResult, Vec<Move>) {
    let mut board = board.clone();
    let root = search(&mut board, depth);
    let mut pv = Vec::new();

    if root.best_move.is_none() {
        return (root, pv);
    }

    for remaining_depth in (1..=depth).rev() {
        let result = search(&mut board, remaining_depth);
        let Some(best_move) = result.best_move else {
            break;
        };

        pv.push(best_move);
        board.make_move(best_move);
    }

    (root, pv)
}

pub fn parse_go_depth(command: &str) -> Option<u32> {
    let tokens: Vec<&str> = command.split_whitespace().collect();
    let depth_idx = tokens.iter().position(|&token| token == "depth")?;
    tokens.get(depth_idx + 1)?.parse::<u32>().ok()
}

fn negamax(board: &mut Board, depth: u32, mut alpha: i32, beta: i32, ply: u32) -> i32 {
    if depth == 0 {
        return side_to_move_eval(board);
    }

    let legal_moves = board.generate_legal_moves();
    if legal_moves.is_empty() {
        return if board.is_in_check(board.side_to_move()) {
            -MATE_SCORE + ply as i32
        } else {
            0
        };
    }

    let mut best_score = i32::MIN;

    for mv in legal_moves {
        board.make_move(mv);
        let score = -negamax(board, depth - 1, -beta, -alpha, ply + 1);
        board.unmake_move();
        best_score = best_score.max(score);
        alpha = alpha.max(score);

        if alpha >= beta {
            break;
        }
    }

    best_score
}

fn side_to_move_eval(board: &Board) -> i32 {
    let eval = board.evaluate();
    match board.side_to_move() {
        Color::White => eval,
        Color::Black => -eval,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;
    use crate::uci::move_to_uci;

    #[test]
    fn depth_one_search_prefers_winning_material() {
        let mut board = Board::from_fen("4k3/8/8/8/2q5/8/1N6/4K3 w - - 0 1").unwrap();
        let best = find_best_move(&mut board, 1).expect("expected at least one legal move");

        assert_eq!(move_to_uci(best), "b2c4");
    }

    #[test]
    fn depth_two_search_finds_mate_in_one() {
        let mut board = Board::from_fen("6k1/5Q2/6K1/8/8/8/8/8 w - - 0 1").unwrap();
        let best = find_best_move(&mut board, 2).expect("expected at least one legal move");

        assert_eq!(move_to_uci(best), "f7g7");
    }

    #[test]
    fn tactical_position_avoids_knight_blunder() {
        let mut board =
            Board::from_fen("2k4r/pp2pQ2/2qp2p1/2p5/4P2r/P2P3N/1PP2P2/R3R1K1 w - - 0 24").unwrap();
        let best = find_best_move(&mut board, 2).expect("expected at least one legal move");

        assert_eq!(move_to_uci(best), "f7e6");
    }

    #[test]
    fn checkmate_score_prefers_faster_mates() {
        let mut board = Board::from_fen("6k1/6Q1/6K1/8/8/8/8/8 b - - 0 1").unwrap();

        let immediate_mate = negamax(&mut board, 1, i32::MIN + 1, i32::MAX, 1);
        let slower_mate = negamax(&mut board, 1, i32::MIN + 1, i32::MAX, 3);

        assert_eq!(immediate_mate, -MATE_SCORE + 1);
        assert_eq!(slower_mate, -MATE_SCORE + 3);
        assert!(immediate_mate < slower_mate);
    }

    #[test]
    fn principal_variation_starts_with_best_move() {
        let board = Board::starting_position();
        let (result, pv) = approximate_principal_variation(&board, 2);

        assert_eq!(pv.first().copied(), result.best_move);
    }

    #[test]
    fn parse_go_depth_reads_requested_depth() {
        assert_eq!(parse_go_depth("go depth 2"), Some(2));
        assert_eq!(parse_go_depth("go movetime 100"), None);
    }
}
