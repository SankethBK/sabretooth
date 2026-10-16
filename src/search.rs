use std::io::{self, Write};
use std::time::{Duration, Instant};

use crate::board::Board;
use crate::movepicker::MovePicker;
use crate::types::{Color, Move};
use crate::uci::move_to_uci;

pub const DEFAULT_SEARCH_DEPTH: u32 = 6;
pub const MAX_TIMED_SEARCH_DEPTH: u32 = 64;
const MAX_SEARCH_PLY: usize = 64;
const MAX_QUIESCENCE_PLY: u32 = 32;
const MATE_SCORE: i32 = 1_000_000;
const DEADLINE_CHECK_INTERVAL: u64 = 2048;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    pub best_move: Option<Move>,
    pub score: i32,
    pub nodes: u64,
    pub qnodes: u64,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchState {
    killers: [[Option<Move>; 2]; MAX_SEARCH_PLY],
    history: [[i32; 64]; 64],
}

impl SearchState {
    pub fn new() -> Self {
        Self {
            killers: [[None; 2]; MAX_SEARCH_PLY],
            history: [[0; 64]; 64],
        }
    }

    pub fn reset_for_new_search(&mut self) {
        self.killers = [[None; 2]; MAX_SEARCH_PLY];

        for from in &mut self.history {
            for score in from {
                *score /= 2;
            }
        }
    }

    pub fn killers(&self, ply: usize) -> [Option<Move>; 2] {
        self.killers.get(ply).copied().unwrap_or([None; 2])
    }

    pub fn history_score(&self, mv: Move) -> i32 {
        self.history[mv.from.index()][mv.to.index()]
    }

    fn record_beta_cutoff(&mut self, mv: Move, ply: usize, depth: u32) {
        if mv.is_capture() || mv.is_promotion() {
            return;
        }

        if let Some(killers) = self.killers.get_mut(ply) {
            if killers[0] != Some(mv) {
                killers[1] = killers[0];
                killers[0] = Some(mv);
            }
        }

        let bonus = (depth as i32).saturating_mul(depth as i32).max(1);
        let history = &mut self.history[mv.from.index()][mv.to.index()];
        *history = history.saturating_add(bonus);
    }
}

pub fn find_best_move(board: &mut Board, depth: u32) -> Option<Move> {
    search(board, depth).best_move
}

pub fn search(board: &mut Board, depth: u32) -> SearchResult {
    let mut state = SearchState::new();
    search_with_options(board, depth, SearchOptions::default(), &mut state)
}

pub fn search_with_options(
    board: &mut Board,
    depth: u32,
    options: SearchOptions,
    state: &mut SearchState,
) -> SearchResult {
    let deadline = options
        .move_time
        .map(|move_time| Instant::now() + move_time);

    if depth == 0 {
        return search_with_pv_hint(board, 0, &[], deadline, state).0;
    }

    let (mut result, mut completed) = search_with_pv_hint(board, 1, &[], deadline, state);
    if !completed {
        return result;
    }

    let mut pv_hint = result.pv.clone();
    let mut nodes = result.nodes;
    let mut qnodes = result.qnodes;
    emit_search_info(1, &result, options);

    for current_depth in 2..=depth {
        let (current_result, current_completed) =
            search_with_pv_hint(board, current_depth, &pv_hint, deadline, state);
        nodes += current_result.nodes;
        qnodes += current_result.qnodes;

        if !current_completed {
            break;
        }

        result = current_result;
        result.nodes = nodes;
        result.qnodes = qnodes;
        pv_hint = result.pv.clone();
        completed = current_completed;
        emit_search_info(current_depth, &result, options);
    }

    result.nodes = nodes;
    result.qnodes = qnodes;
    let _ = completed;
    result
}

pub fn search_with_pv_hint(
    board: &mut Board,
    depth: u32,
    pv_hint: &[Move],
    deadline: Option<Instant>,
    state: &mut SearchState,
) -> (SearchResult, bool) {
    let mut nodes = 1;
    let mut qnodes = 0;
    let mut pv = PrincipalVariation::new();
    let mut move_picker = MovePicker::new(pv_hint.first().copied());
    let mut timed_out = false;

    let mut best_move = None;
    let mut best_score = i32::MIN;
    let mut alpha = i32::MIN + 1;
    let beta = i32::MAX;

    let us = board.side_to_move();

    let mut legal_moves: u32 = 0;

    while let Some(mv) = move_picker.next_move(board, state, 0) {
        debug_assert!(
            board.is_pseudo_legal(mv),
            "move picker produced pseudo-illegal root move {mv} ({:?}) for {:?}; from={:?}, to={:?}",
            mv.kind,
            board.side_to_move(),
            board.piece_on(mv.from),
            board.piece_on(mv.to),
        );
        board.make_move(mv);

        if board.is_in_check(us) {
            board.unmake_move();
            continue;
        }

        legal_moves += 1;

        let score = -negamax(
            board,
            depth.saturating_sub(1),
            -beta,
            -alpha,
            1,
            &mut nodes,
            &mut qnodes,
            &mut pv,
            pv_hint,
            deadline,
            &mut timed_out,
            state,
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

    if legal_moves == 0 {
        let score = if board.is_in_check(us) {
            terminal_score(true, 0)
        } else {
            0
        };
        return (
            SearchResult {
                best_move: None,
                score,
                nodes,
                qnodes,
                pv: Vec::new(),
            },
            true,
        );
    }

    let result = SearchResult {
        best_move,
        score: best_score,
        nodes,
        qnodes,
        pv: pv.line_at(0).to_vec(),
    };

    (result, !timed_out)
}

pub fn search_root_moves(board: &mut Board, depth: u32) -> Vec<RootMoveScore> {
    let legal_moves = board.generate_legal_moves();
    let mut scored_moves = Vec::with_capacity(legal_moves.len());

    let us = board.side_to_move();

    for mv in legal_moves {
        board.make_move(mv);
        let mut nodes = 1;
        let mut qnodes = 0;
        let mut pv = PrincipalVariation::new();
        let mut state = SearchState::new();

        if board.is_in_check(us) {
            board.unmake_move();
            continue;
        }

        let score = -negamax(
            board,
            depth.saturating_sub(1),
            i32::MIN + 1,
            i32::MAX,
            1,
            &mut nodes,
            &mut qnodes,
            &mut pv,
            &[],
            None,
            &mut false,
            &mut state,
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
        let mut qnodes = 0;
        let mut pv = PrincipalVariation::new();
        let mut state = SearchState::new();
        let child_score = negamax(
            board,
            depth.saturating_sub(1),
            i32::MIN + 1,
            i32::MAX,
            1,
            &mut nodes,
            &mut qnodes,
            &mut pv,
            &[],
            None,
            &mut false,
            &mut state,
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
    qnodes: &mut u64,
    pv: &mut PrincipalVariation,
    pv_hint: &[Move],
    deadline: Option<Instant>,
    timed_out: &mut bool,
    state: &mut SearchState,
) -> i32 {
    if depth == 0 {
        pv.clear_at(ply as usize);
        return quiescence_search(
            board, alpha, beta, ply, 0, nodes, qnodes, deadline, timed_out,
        );
    }

    *nodes += 1;
    if should_stop(*nodes, deadline) {
        *timed_out = true;
        return 0;
    }

    let us = board.side_to_move();

    let in_check = board.is_in_check(us);
    let mut move_picker = MovePicker::new(pv_hint.get(ply as usize).copied());
    if move_picker.is_empty(board, state, ply as usize) {
        pv.clear_at(ply as usize);
        return terminal_score(in_check, ply);
    }

    let mut best_score = i32::MIN;
    let mut legal_moves: u32 = 0;

    while let Some(mv) = move_picker.next_move(board, state, ply as usize) {
        debug_assert!(
            board.is_pseudo_legal(mv),
            "move picker produced pseudo-illegal move {mv} ({:?}) at ply {ply} for {:?}; from={:?}, to={:?}",
            mv.kind,
            board.side_to_move(),
            board.piece_on(mv.from),
            board.piece_on(mv.to),
        );
        board.make_move(mv);
        if board.is_in_check(us) {
            board.unmake_move();
            continue;
        }

        legal_moves += 1;

        let score = -negamax(
            board,
            depth.saturating_sub(1),
            -beta,
            -alpha,
            ply + 1,
            nodes,
            qnodes,
            pv,
            pv_hint,
            deadline,
            timed_out,
            state,
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
            state.record_beta_cutoff(mv, ply as usize, depth);
            break;
        }
    }

    if legal_moves == 0 {
        pv.clear_at(ply as usize);

        if in_check {
            return terminal_score(in_check, ply);
        } else {
            return 0;
        }
    }

    best_score
}

fn quiescence_search(
    board: &mut Board,
    mut alpha: i32,
    beta: i32,
    ply: u32,
    qply: u32,
    nodes: &mut u64,
    qnodes: &mut u64,
    deadline: Option<Instant>,
    timed_out: &mut bool,
) -> i32 {
    *nodes += 1;
    *qnodes += 1;
    if should_stop(*nodes, deadline) {
        *timed_out = true;
        return 0;
    }

    let us = board.side_to_move();
    let in_check = board.is_in_check(us);
    let mut best_score = if in_check {
        i32::MIN + 1
    } else {
        let stand_pat = side_to_move_eval(board);
        if qply >= MAX_QUIESCENCE_PLY {
            return if has_any_legal_move(board) {
                stand_pat
            } else {
                0
            };
        }
        if stand_pat >= beta {
            return if has_any_legal_move(board) {
                stand_pat
            } else {
                0
            };
        }
        alpha = alpha.max(stand_pat);
        stand_pat
    };

    let mut moves = Vec::with_capacity(32);
    if in_check {
        moves = board.generate_pseudo_legal_moves();
    } else {
        board.generate_pseudo_legal_captures_into(&mut moves);
        board.generate_pseudo_legal_quiet_promotions_into(&mut moves);
        moves.sort_unstable_by(|a, b| {
            capture_order_score(board, *b).cmp(&capture_order_score(board, *a))
        });
    }

    let mut legal_moves = 0;
    for mv in moves {
        debug_assert!(
            board.is_pseudo_legal(mv),
            "quiescence generated pseudo-illegal move {mv} ({:?}) for {:?}",
            mv.kind,
            us,
        );
        let capture_gain = if mv.is_capture() {
            captured_piece_value(board, mv)
        } else {
            0
        };
        board.make_move(mv);
        if board.is_in_check(us) {
            board.unmake_move();
            continue;
        }
        legal_moves += 1;

        // Don't spend qsearch nodes on losing captures. Keep promotions and
        // every move made while in check; both need full tactical treatment.
        if !in_check
            && mv.is_capture()
            && !mv.is_promotion()
            && !board.is_in_check(board.side_to_move())
        {
            let exchange_gain = capture_gain
                - best_exchange_gain(
                    board,
                    mv.to,
                    board.piece_on(mv.to).map_or(0, |(_, piece)| piece.score()),
                );
            if exchange_gain < 0 {
                board.unmake_move();
                continue;
            }
        }

        let score = if qply >= MAX_QUIESCENCE_PLY {
            -side_to_move_eval(board)
        } else {
            -quiescence_search(
                board,
                -beta,
                -alpha,
                ply + 1,
                qply + 1,
                nodes,
                qnodes,
                deadline,
                timed_out,
            )
        };
        board.unmake_move();

        if *timed_out {
            return 0;
        }
        best_score = best_score.max(score);
        alpha = alpha.max(score);
        if alpha >= beta {
            break;
        }
    }

    if in_check && legal_moves == 0 {
        return terminal_score(true, ply);
    }

    if !in_check && legal_moves == 0 && !has_legal_quiet_move(board) {
        return 0;
    }

    // Captures and promotions can all be illegal (for example, pinned
    // attackers); in that case, retain stand-pat if a quiet move exists.
    best_score
}

fn captured_piece_value(board: &Board, mv: Move) -> i32 {
    if matches!(mv.kind, crate::types::MoveKind::EnPassant) {
        crate::types::Piece::Pawn.score()
    } else {
        board.piece_on(mv.to).map_or(0, |(_, piece)| piece.score())
    }
}

fn has_any_legal_move(board: &mut Board) -> bool {
    let us = board.side_to_move();
    for mv in board.generate_pseudo_legal_moves() {
        board.make_move(mv);
        let legal = !board.is_in_check(us);
        board.unmake_move();
        if legal {
            return true;
        }
    }
    false
}

fn has_legal_quiet_move(board: &mut Board) -> bool {
    let us = board.side_to_move();
    for mv in board.generate_pseudo_legal_quiets() {
        board.make_move(mv);
        let legal = !board.is_in_check(us);
        board.unmake_move();
        if legal {
            return true;
        }
    }
    false
}

// Called after the candidate capture has been made. Follow a least-valuable
// legal-attacker exchange sequence on the destination square. This is a
// conservative SEE: promotions and captures made in check are not pruned.
fn best_exchange_gain(board: &mut Board, target: crate::types::Square, victim_value: i32) -> i32 {
    let us = board.side_to_move();
    let mut least_attacker: Option<(i32, Move)> = None;

    for mv in board.generate_pseudo_legal_captures() {
        if mv.to != target {
            continue;
        }
        let Some((_, attacker)) = board.piece_on(mv.from) else {
            continue;
        };

        board.make_move(mv);
        let legal = !board.is_in_check(us);
        board.unmake_move();
        if !legal {
            continue;
        }

        let value = attacker.score();
        if least_attacker.is_none_or(|(least, _)| value < least) {
            least_attacker = Some((value, mv));
        }
    }

    let Some((_, mv)) = least_attacker else {
        return 0;
    };

    board.make_move(mv);
    let promoted_value = match mv.kind {
        crate::types::MoveKind::PromotionCapture(piece) => {
            piece.score() - crate::types::Piece::Pawn.score()
        }
        _ => 0,
    };
    let occupant_value = board.piece_on(target).map_or(0, |(_, piece)| piece.score());
    let reply_gain = best_exchange_gain(board, target, occupant_value);
    board.unmake_move();

    (victim_value + promoted_value - reply_gain).max(0)
}

fn capture_order_score(board: &Board, mv: Move) -> i32 {
    let attacker = board
        .piece_on(mv.from)
        .map_or(0, |(_, piece)| piece.score());
    let victim = if matches!(mv.kind, crate::types::MoveKind::EnPassant) {
        crate::types::Piece::Pawn.score()
    } else {
        board.piece_on(mv.to).map_or(0, |(_, piece)| piece.score())
    };
    let promotion = match mv.kind {
        crate::types::MoveKind::Promotion(piece)
        | crate::types::MoveKind::PromotionCapture(piece) => {
            piece.score() - crate::types::Piece::Pawn.score()
        }
        _ => 0,
    };
    victim * 16 - attacker + promotion
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
    use crate::uci::move_to_uci;

    #[test]
    fn depth_one_search_prefers_winning_material() {
        let mut board = Board::from_fen("4k3/8/8/8/2q5/8/1N6/4K3 w - - 0 1").unwrap();
        let best = find_best_move(&mut board, 1).expect("expected at least one legal move");

        assert_eq!(move_to_uci(best), "b2c4");
    }

    #[test]
    fn depth_two_search_finds_mate_in_one() {
        let mut board = Board::from_fen("7k/5Q2/6K1/8/8/8/8/8 w - - 0 1").unwrap();
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
        let mut qnodes = 0;

        let immediate_mate = negamax(
            &mut board,
            1,
            i32::MIN + 1,
            i32::MAX,
            1,
            &mut nodes,
            &mut qnodes,
            &mut PrincipalVariation::new(),
            &[],
            None,
            &mut false,
            &mut SearchState::new(),
        );
        let slower_mate = negamax(
            &mut board,
            1,
            i32::MIN + 1,
            i32::MAX,
            3,
            &mut nodes,
            &mut qnodes,
            &mut PrincipalVariation::new(),
            &[],
            None,
            &mut false,
            &mut SearchState::new(),
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
        assert!(lines
            .iter()
            .all(|line| line.line.first().copied() == Some(line.mv)));
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
        assert!(result.qnodes > 0);
        assert!(result.nodes >= result.qnodes);
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
