use crate::board::Board;
use crate::types::{Color, Move};

pub const DEFAULT_SEARCH_DEPTH: u32 = 2;
const MATE_SCORE: i32 = 1_000_000;

pub fn find_best_move(board: &mut Board, depth: u32) -> Option<Move> {
    let legal_moves = board.generate_legal_moves();
    let mut best_move = None;
    let mut best_score = i32::MIN;

    for mv in legal_moves {
        board.make_move(mv);
        let score = -negamax(board, depth.saturating_sub(1));
        board.unmake_move();

        if score > best_score {
            best_score = score;
            best_move = Some(mv);
        }
    }

    best_move
}

pub fn parse_go_depth(command: &str) -> Option<u32> {
    let tokens: Vec<&str> = command.split_whitespace().collect();
    let depth_idx = tokens.iter().position(|&token| token == "depth")?;
    tokens.get(depth_idx + 1)?.parse::<u32>().ok()
}

fn negamax(board: &mut Board, depth: u32) -> i32 {
    if depth == 0 {
        return side_to_move_eval(board);
    }

    let legal_moves = board.generate_legal_moves();
    if legal_moves.is_empty() {
        return if board.is_in_check(board.side_to_move()) {
            -MATE_SCORE
        } else {
            0
        };
    }

    let mut best_score = i32::MIN;

    for mv in legal_moves {
        board.make_move(mv);
        let score = -negamax(board, depth - 1);
        board.unmake_move();
        best_score = best_score.max(score);
    }

    best_score
}

fn side_to_move_eval(board: &Board) -> i32 {
    let eval = board.evaluate_material();
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
    fn parse_go_depth_reads_requested_depth() {
        assert_eq!(parse_go_depth("go depth 2"), Some(2));
        assert_eq!(parse_go_depth("go movetime 100"), None);
    }
}
