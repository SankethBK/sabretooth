use crate::board::Board;
use crate::movegen::MoveGenType;
use crate::search::SearchState;
use crate::types::{Move, Piece};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovePicker {
    moves: Vec<Move>,
    next_index: usize,
}

impl MovePicker {
    pub fn new(
        board: &mut Board,
        pv_hint: Option<Move>,
        state: &SearchState,
        ply: usize,
    ) -> Self {
        let mut moves = Vec::new();

        if let Some(pv_hint) = pv_hint.filter(|&mv| is_legal_move(board, mv)) {
            moves.push(pv_hint);
        }

        let capture_moves = board.generate_legal_moves_by_type(MoveGenType::Captures);
        let quiet_moves = board.generate_legal_moves_by_type(MoveGenType::Quiets);

        let mut captures = score_moves(board, capture_moves, None, state, ply);
        let mut quiets = score_moves(board, quiet_moves, None, state, ply);

        captures.retain(|(_, mv)| Some(*mv) != pv_hint);
        quiets.retain(|(_, mv)| Some(*mv) != pv_hint);

        captures.sort_by(|a, b| b.0.cmp(&a.0));
        quiets.sort_by(|a, b| b.0.cmp(&a.0));

        moves.extend(captures.into_iter().map(|(_, mv)| mv));
        moves.extend(quiets.into_iter().map(|(_, mv)| mv));

        Self {
            moves,
            next_index: 0,
        }
    }

    pub fn captures_only(board: &mut Board, state: &SearchState, ply: usize) -> Self {
        let capture_moves = board.generate_legal_moves_by_type(MoveGenType::Captures);
        let mut captures = score_moves(board, capture_moves, None, state, ply);
        captures.sort_by(|a, b| b.0.cmp(&a.0));

        Self {
            moves: captures.into_iter().map(|(_, mv)| mv).collect(),
            next_index: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.moves.is_empty()
    }
}

impl Iterator for MovePicker {
    type Item = Move;

    fn next(&mut self) -> Option<Self::Item> {
        let mv = self.moves.get(self.next_index).copied()?;
        self.next_index += 1;
        Some(mv)
    }
}

fn score_moves(
    board: &Board,
    moves: Vec<Move>,
    pv_hint: Option<Move>,
    state: &SearchState,
    ply: usize,
) -> Vec<(i32, Move)> {
    moves
        .into_iter()
        .map(|mv| (score_move(board, mv, pv_hint, state, ply), mv))
        .collect()
}

fn is_legal_move(board: &mut Board, mv: Move) -> bool {
    let us = board.side_to_move();
    board.make_move(mv);
    let legal = !board.is_in_check(us);
    board.unmake_move();
    legal
}

fn score_move(
    board: &Board,
    mv: Move,
    pv_hint: Option<Move>,
    state: &SearchState,
    ply: usize,
) -> i32 {
    if Some(mv) == pv_hint {
        return 1_000_000;
    }

    if mv.is_capture() {
        return 100_000 + capture_score(board, mv);
    }

    let killers = state.killers(ply);
    if killers[0] == Some(mv) {
        return 90_000;
    }
    if killers[1] == Some(mv) {
        return 80_000;
    }

    state.history_score(mv)
}

fn capture_score(board: &Board, mv: Move) -> i32 {
    let attacker = board
        .piece_on(mv.from)
        .map(|(_, piece)| piece)
        .unwrap_or(Piece::Pawn);
    let victim = captured_piece(board, mv).unwrap_or(Piece::Pawn);

    victim.score() * 10 - attacker.score()
}

fn captured_piece(board: &Board, mv: Move) -> Option<Piece> {
    if matches!(mv.kind, crate::types::MoveKind::EnPassant) {
        return Some(Piece::Pawn);
    }

    board.piece_on(mv.to).map(|(_, piece)| piece)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Move, MoveKind, Square};

    #[test]
    fn move_picker_yields_pv_hint_first() {
        let mut board = Board::starting_position();
        let pv_hint = Move::new(
            Square::from_rf(1, 4),
            Square::from_rf(3, 4),
            MoveKind::DoublePawnPush,
        );
        let state = SearchState::new();

        let mut picker = MovePicker::new(&mut board, Some(pv_hint), &state, 0);

        assert_eq!(picker.next(), Some(pv_hint));
    }

    #[test]
    fn move_picker_iterates_all_legal_moves() {
        let mut board = Board::starting_position();
        let state = SearchState::new();
        let picker = MovePicker::new(&mut board, None, &state, 0);

        assert_eq!(picker.count(), 20);
    }

    #[test]
    fn move_picker_orders_captures_before_quiets_without_pv_hint() {
        let mut board = Board::from_fen("4k3/8/8/3p4/8/2N5/8/4K3 w - - 0 1").unwrap();
        let state = SearchState::new();
        let mut picker = MovePicker::new(&mut board, None, &state, 0);

        assert_eq!(
            picker.next(),
            Some(Move::new(
                Square::from_rf(2, 2),
                Square::from_rf(4, 3),
                MoveKind::Capture,
            ))
        );
    }

    #[test]
    fn capture_only_picker_filters_out_quiet_moves() {
        let mut board = Board::from_fen("4k3/8/8/3p4/8/2N5/8/4K3 w - - 0 1").unwrap();
        let state = SearchState::new();
        let moves = MovePicker::captures_only(&mut board, &state, 0).collect::<Vec<_>>();

        assert_eq!(
            moves,
            vec![Move::new(
                Square::from_rf(2, 2),
                Square::from_rf(4, 3),
                MoveKind::Capture,
            )]
        );
    }
}
