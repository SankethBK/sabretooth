use crate::board::Board;
use crate::search::SearchState;
use crate::types::{Move, Piece};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovePicker {
    moves: Vec<Move>,
    next_index: usize,
}

impl MovePicker {
    pub fn new(board: &mut Board, pv_hint: Option<Move>, state: &SearchState, ply: usize) -> Self {
        let mut moves = board.generate_pseudo_legal_moves();
        let mut scored_moves = moves
            .drain(..)
            .map(|mv| (score_move(board, mv, pv_hint, state, ply), mv))
            .collect::<Vec<_>>();
        scored_moves.sort_by(|a, b| b.0.cmp(&a.0));
        let moves = scored_moves.into_iter().map(|(_, mv)| mv).collect();

        Self {
            moves,
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
}
