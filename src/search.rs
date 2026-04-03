use crate::board::Board;
use crate::types::{Color, Move};

pub const DEFAULT_SEARCH_DEPTH: u32 = 6;
const MATE_SCORE: i32 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchResult {
    pub best_move: Option<Move>,
    pub score: i32,
    pub nodes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RootMoveScore {
    pub mv: Move,
    pub score: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootMoveLine {
    pub mv: Move,
    pub score: i32,
    pub line: Vec<Move>,
}

pub fn find_best_move(board: &mut Board, depth: u32) -> Option<Move> {
    search(board, depth).best_move
}

pub fn search(board: &mut Board, depth: u32) -> SearchResult {
    let mut nodes = 1;
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
            nodes,
        };
    }

    let mut best_move = None;
    let mut best_score = i32::MIN;
    let mut alpha = i32::MIN + 1;
    let beta = i32::MAX;

    for mv in legal_moves {
        board.make_move(mv);
        let score = -negamax(
            board,
            depth.saturating_sub(1),
            -beta,
            -alpha,
            1,
            &mut nodes,
        );
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
        nodes,
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

pub fn search_root_moves(board: &mut Board, depth: u32) -> Vec<RootMoveScore> {
    let legal_moves = board.generate_legal_moves();
    let mut scored_moves = Vec::with_capacity(legal_moves.len());

    for mv in legal_moves {
        board.make_move(mv);
        let mut nodes = 1;
        let score = -negamax(
            board,
            depth.saturating_sub(1),
            i32::MIN + 1,
            i32::MAX,
            1,
            &mut nodes,
        );
        board.unmake_move();
        scored_moves.push(RootMoveScore { mv, score });
    }

    scored_moves.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.mv.from.0.cmp(&b.mv.from.0))
            .then_with(|| a.mv.to.0.cmp(&b.mv.to.0))
    });
    scored_moves
}

pub fn search_root_move_lines(board: &mut Board, depth: u32) -> Vec<RootMoveLine> {
    let legal_moves = board.generate_legal_moves();
    let mut lines = Vec::with_capacity(legal_moves.len());

    for mv in legal_moves {
        board.make_move(mv);
        let (child_score, child_line) =
            negamax_line(board, depth.saturating_sub(1), i32::MIN + 1, i32::MAX, 1);
        board.unmake_move();
        let score = -child_score;

        let mut line = Vec::with_capacity(child_line.len() + 1);
        line.push(mv);
        line.extend(child_line);

        lines.push(RootMoveLine { mv, score, line });
    }

    lines.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.mv.from.0.cmp(&b.mv.from.0))
            .then_with(|| a.mv.to.0.cmp(&b.mv.to.0))
    });
    lines
}

pub fn parse_go_depth(command: &str) -> Option<u32> {
    let tokens: Vec<&str> = command.split_whitespace().collect();
    let depth_idx = tokens.iter().position(|&token| token == "depth")?;
    tokens.get(depth_idx + 1)?.parse::<u32>().ok()
}

fn negamax(
    board: &mut Board,
    depth: u32,
    mut alpha: i32,
    beta: i32,
    ply: u32,
    nodes: &mut u64,
) -> i32 {
    *nodes += 1;

    let in_check = board.is_in_check(board.side_to_move());
    if depth == 0 && !in_check {
        return side_to_move_eval(board);
    }

    let legal_moves = board.generate_legal_moves();
    if legal_moves.is_empty() {
        return terminal_score(in_check, ply);
    }

    let mut best_score = i32::MIN;

    for mv in legal_moves {
        board.make_move(mv);
        let score = -negamax(
            board,
            depth.saturating_sub(1),
            -beta,
            -alpha,
            ply + 1,
            nodes,
        );
        board.unmake_move();
        best_score = best_score.max(score);
        alpha = alpha.max(score);

        if alpha >= beta {
            break;
        }
    }

    best_score
}

// Negamax that also returns the principal variation line for the given depth.
// Currently not used for main search, but only as a utlity for debugging
// This will be removed in future once PV is added to main search
fn negamax_line(
    board: &mut Board,
    depth: u32,
    mut alpha: i32,
    beta: i32,
    ply: u32,
) -> (i32, Vec<Move>) {
    let in_check = board.is_in_check(board.side_to_move());
    if depth == 0 && !in_check {
        return (side_to_move_eval(board), Vec::new());
    }

    let legal_moves = board.generate_legal_moves();
    if legal_moves.is_empty() {
        return (terminal_score(in_check, ply), Vec::new());
    }

    let mut best_score = i32::MIN;
    let mut best_line = Vec::new();

    for mv in legal_moves {
        board.make_move(mv);
        let (child_score, child_line) = negamax_line(
            board,
            depth.saturating_sub(1),
            -beta,
            -alpha,
            ply + 1,
        );
        board.unmake_move();

        let score = -child_score;
        if score > best_score {
            best_score = score;
            best_line.clear();
            best_line.push(mv);
            best_line.extend(child_line);
        }

        alpha = alpha.max(score);
        if alpha >= beta {
            break;
        }
    }

    (best_score, best_line)
}

fn terminal_score(in_check: bool, ply: u32) -> i32 {
    if in_check {
        -MATE_SCORE + ply as i32
    } else {
        0
    }
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
    use crate::testsuite::bk_depth_report;
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
        let mut nodes = 1;

        let immediate_mate = negamax(&mut board, 1, i32::MIN + 1, i32::MAX, 1, &mut nodes);
        let slower_mate = negamax(&mut board, 1, i32::MIN + 1, i32::MAX, 3, &mut nodes);

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
    fn root_move_scores_are_sorted_best_first() {
        let mut board = Board::starting_position();
        let scored = search_root_moves(&mut board, 1);

        assert!(!scored.is_empty());
        assert!(scored.windows(2).all(|w| w[0].score >= w[1].score));
    }

    #[test]
    fn root_move_lines_include_the_root_move_first() {
        let mut board = Board::starting_position();
        let lines = search_root_move_lines(&mut board, 2);

        assert!(!lines.is_empty());
        assert!(
            lines
                .iter()
                .all(|line| line.line.first().copied() == Some(line.mv))
        );
    }

    #[test]
    fn parse_go_depth_reads_requested_depth() {
        assert_eq!(parse_go_depth("go depth 2"), Some(2));
        assert_eq!(parse_go_depth("go movetime 100"), None);
    }

    #[test]
    fn search_reports_visited_nodes() {
        let mut board = Board::starting_position();
        let result = search(&mut board, 2);

        assert!(result.nodes > 1);
    }

    #[test]
    #[ignore = "benchmark harness for measuring current tactical strength"]
    fn bk_suite_depth_4_report() {
        let (solved, misses) = bk_depth_report(4);

        for miss in misses {
            println!("{miss}");
        }

        println!("Solved {solved}/24 BK positions at depth 4");
    }
}
