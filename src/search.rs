use std::io::{self, Write};
use std::time::{Duration, Instant};

use crate::board::Board;
use crate::types::{Color, Move};
use crate::uci::move_to_uci;

pub const DEFAULT_SEARCH_DEPTH: u32 = 6;
pub const MAX_TIMED_SEARCH_DEPTH: u32 = 64;
const MATE_SCORE: i32 = 1_000_000;
const DEADLINE_CHECK_INTERVAL: u64 = 2048;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    pub best_move: Option<Move>,
    pub score: i32,
    pub nodes: u64,
    pub pv: Vec<Move>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SearchOptions {
    pub emit_info: bool,
    pub move_time: Option<Duration>,
}

pub fn find_best_move(board: &mut Board, depth: u32) -> Option<Move> {
    search(board, depth).best_move
}

pub fn search(board: &mut Board, depth: u32) -> SearchResult {
    search_with_options(board, depth, SearchOptions::default())
}

pub fn search_with_options(
    board: &mut Board,
    depth: u32,
    options: SearchOptions,
) -> SearchResult {
    let deadline = options.move_time.map(|move_time| Instant::now() + move_time);

    if depth == 0 {
        return search_with_pv_hint(board, 0, &[], deadline).0;
    }

    let (mut result, mut completed) = search_with_pv_hint(board, 1, &[], deadline);
    if !completed {
        return result;
    }

    let mut pv_hint = result.pv.clone();
    let mut nodes = result.nodes;
    emit_search_info(1, &result, options);

    for current_depth in 2..=depth {
        let (current_result, current_completed) =
            search_with_pv_hint(board, current_depth, &pv_hint, deadline);
        nodes += current_result.nodes;

        if !current_completed {
            break;
        }

        result = current_result;
        result.nodes = nodes;
        pv_hint = result.pv.clone();
        completed = current_completed;
        emit_search_info(current_depth, &result, options);
    }

    result.nodes = nodes;
    let _ = completed;
    result
}

pub fn search_with_pv_hint(
    board: &mut Board,
    depth: u32,
    pv_hint: &[Move],
    deadline: Option<Instant>,
) -> (SearchResult, bool) {
    let mut nodes = 1;
    let mut pv = PrincipalVariation::new();
    let mut legal_moves = board.generate_legal_moves();
    order_moves_by_hint(&mut legal_moves, pv_hint.first().copied());
    let mut timed_out = false;

    if legal_moves.is_empty() {
        let score = if board.is_in_check(board.side_to_move()) {
            -MATE_SCORE
        } else {
            0
        };
        return (
            SearchResult {
                best_move: None,
                score,
                nodes,
                pv: Vec::new(),
            },
            true,
        );
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
            &mut pv,
            pv_hint,
            deadline,
            &mut timed_out,
        );
        board.unmake_move();

        if timed_out {
            break;
        }

        if score > best_score {
            best_score = score;
            best_move = Some(mv);
            pv.update_from_child(0, mv);
        }

        alpha = alpha.max(score);
    }

    let result = SearchResult {
        best_move,
        score: best_score,
        nodes,
        pv: pv.line_at(0).to_vec(),
    };

    (result, !timed_out)
}

pub fn search_root_moves(board: &mut Board, depth: u32) -> Vec<RootMoveScore> {
    let legal_moves = board.generate_legal_moves();
    let mut scored_moves = Vec::with_capacity(legal_moves.len());

    for mv in legal_moves {
        board.make_move(mv);
        let mut nodes = 1;
        let mut pv = PrincipalVariation::new();
        let score = -negamax(
            board,
            depth.saturating_sub(1),
            i32::MIN + 1,
            i32::MAX,
            1,
            &mut nodes,
            &mut pv,
            &[],
            None,
            &mut false,
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
        let mut nodes = 1;
        let mut pv = PrincipalVariation::new();
        let child_score = negamax(
            board,
            depth.saturating_sub(1),
            i32::MIN + 1,
            i32::MAX,
            1,
            &mut nodes,
            &mut pv,
            &[],
            None,
            &mut false,
        );
        let (child_score, child_line) = (child_score, pv.line_at(1).to_vec());
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
    pv: &mut PrincipalVariation,
    pv_hint: &[Move],
    deadline: Option<Instant>,
    timed_out: &mut bool,
) -> i32 {
    *nodes += 1;
    if should_stop(*nodes, deadline) {
        *timed_out = true;
        return 0;
    }

    let in_check = board.is_in_check(board.side_to_move());
    if depth == 0 && !in_check {
        pv.clear_at(ply as usize);
        return side_to_move_eval(board);
    }

    let mut legal_moves = board.generate_legal_moves();
    if legal_moves.is_empty() {
        pv.clear_at(ply as usize);
        return terminal_score(in_check, ply);
    }
    order_moves_by_hint(&mut legal_moves, pv_hint.get(ply as usize).copied());

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
            pv,
            pv_hint,
            deadline,
            timed_out,
        );
        board.unmake_move();

        if *timed_out {
            return 0;
        }

        if score > best_score {
            best_score = score;
            pv.update_from_child(ply as usize, mv);
        }

        alpha = alpha.max(score);

        if alpha >= beta {
            break;
        }
    }

    best_score
}

fn should_stop(nodes: u64, deadline: Option<Instant>) -> bool {
    nodes % DEADLINE_CHECK_INTERVAL == 0
        && deadline.is_some_and(|deadline| Instant::now() >= deadline)
}

fn emit_search_info(depth: u32, result: &SearchResult, options: SearchOptions) {
    if !options.emit_info {
        return;
    }

    let pv_text = result
        .pv
        .iter()
        .copied()
        .map(move_to_uci)
        .collect::<Vec<_>>()
        .join(" ");

    let mut stdout = io::stdout().lock();
    let write_result = if pv_text.is_empty() {
        writeln!(
            stdout,
            "info depth {} score cp {} nodes {}",
            depth, result.score, result.nodes
        )
    } else {
        writeln!(
            stdout,
            "info depth {} score cp {} nodes {} pv {}",
            depth, result.score, result.nodes, pv_text
        )
    };
    let _ = write_result.and_then(|_| stdout.flush());
}

fn order_moves_by_hint(moves: &mut [Move], hint: Option<Move>) {
    let Some(hint) = hint else {
        return;
    };

    if let Some(idx) = moves.iter().position(|&mv| mv == hint) {
        moves.swap(0, idx);
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct PrincipalVariation {
    lines: Vec<Vec<Move>>,
}

impl PrincipalVariation {
    fn new() -> Self {
        Self::default()
    }

    fn line_at(&self, ply: usize) -> &[Move] {
        self.lines.get(ply).map(Vec::as_slice).unwrap_or(&[])
    }

    fn clear_at(&mut self, ply: usize) {
        self.ensure_ply(ply);
        self.lines[ply].clear();
    }

    fn update_from_child(&mut self, ply: usize, mv: Move) {
        self.ensure_ply(ply + 1);
        let child_line = self.lines[ply + 1].clone();

        self.lines[ply].clear();
        self.lines[ply].push(mv);
        self.lines[ply].extend(child_line);
    }

    fn ensure_ply(&mut self, ply: usize) {
        if self.lines.len() <= ply {
            self.lines.resize_with(ply + 1, Vec::new);
        }
    }
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
    use crate::types::{MoveKind, Square};
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

        let immediate_mate = negamax(
            &mut board,
            1,
            i32::MIN + 1,
            i32::MAX,
            1,
            &mut nodes,
            &mut PrincipalVariation::new(),
            &[],
            None,
            &mut false,
        );
        let slower_mate = negamax(
            &mut board,
            1,
            i32::MIN + 1,
            i32::MAX,
            3,
            &mut nodes,
            &mut PrincipalVariation::new(),
            &[],
            None,
            &mut false,
        );

        assert_eq!(immediate_mate, -MATE_SCORE + 1);
        assert_eq!(slower_mate, -MATE_SCORE + 3);
        assert!(immediate_mate < slower_mate);
    }

    #[test]
    fn principal_variation_starts_with_best_move() {
        let mut board = Board::starting_position();
        let result = search(&mut board, 2);

        assert_eq!(result.pv.first().copied(), result.best_move);
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
    fn order_moves_by_hint_moves_pv_move_to_front() {
        let mut moves = vec![
            Move::new(
                Square::from_rf(1, 3),
                Square::from_rf(3, 3),
                MoveKind::DoublePawnPush,
            ),
            Move::new(
                Square::from_rf(1, 4),
                Square::from_rf(3, 4),
                MoveKind::DoublePawnPush,
            ),
            Move::new(
                Square::from_rf(0, 6),
                Square::from_rf(2, 5),
                MoveKind::Quiet,
            ),
        ];
        let hint = moves[1];

        order_moves_by_hint(&mut moves, Some(hint));

        assert_eq!(moves[0], hint);
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

    #[test]
    #[ignore = "benchmark harness for measuring current tactical strength"]
    fn bk_suite_depth_5_report() {
        let (solved, misses) = bk_depth_report(5);

        for miss in misses {
            println!("{miss}");
        }

        println!("Solved {solved}/24 BK positions at depth 4");
    }
}
