use crate::board::Board;
use crate::search::SearchState;
use crate::types::{Move, Piece};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovePicker {
    next_index: usize,
    buffer: Vec<(i32, Move)>,
    pv_move: Option<Move>,
    stage: Stage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Pv,
    Captures,
    Quiets,
    Done,
}

impl MovePicker {
    pub fn new(pv_hint: Option<Move>) -> Self {
        Self {
            next_index: 0,
            buffer: Vec::new(),
            pv_move: pv_hint,
            stage: Stage::Pv,
        }
    }

    fn fill_next_stage(&mut self, board: &Board, state: &SearchState, ply: usize) {
        self.buffer.clear();
        self.next_index = 0;

        match self.stage {
            Stage::Pv => {
                self.stage = Stage::Captures;

                if let Some(mv) = self.pv_move {
                    self.buffer.push((1_000_000, mv));
                }
            }

            Stage::Captures => {
                let moves = board.generate_pseudo_legal_captures();

                for mv in moves {
                    if Some(mv) != self.pv_move {
                        let score = score_move(board, mv, None, state, ply);
                        self.buffer.push((score, mv));
                    }
                }

                self.stage = Stage::Quiets;
            }

            Stage::Quiets => {
                let moves = board.generate_pseudo_legal_quiets();

                for mv in moves {
                    if Some(mv) != self.pv_move {
                        let score = score_move(board, mv, None, state, ply);
                        self.buffer.push((score, mv));
                    }
                }

                self.stage = Stage::Done;
            }

            Stage::Done => {}
        }
    }

    pub fn next_move(
        &mut self,
        board: &mut Board,
        state: &SearchState,
        ply: usize,
    ) -> Option<Move> {
        loop {
            if let Some(mv) = self.buffer.get(self.next_index).copied() {
                self.next_index += 1;
                return Some(mv.1);
            }

            if self.stage == Stage::Done {
                return None;
            }

            self.fill_next_stage(board, state, ply);
        }
    }

    pub fn is_empty(&mut self, board: &mut Board, state: &SearchState, ply: usize) -> bool {
        self.next_move(board, state, ply).is_none()
    }

    // Helper method to get all moves at once (for testing)
    pub fn generate_all_moves(&mut self, board: &mut Board) -> Vec<Move> {
        let mut moves = Vec::new();
        while let Some(mv) = self.next_move(board, &SearchState::new(), 0) {
            moves.push(mv);
        }
        moves
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

        let mut picker = MovePicker::new(Some(pv_hint));

        assert_eq!(picker.next_move(&mut board, &state, 0), Some(pv_hint));
    }

    #[test]
    fn move_picker_iterates_all_legal_moves() {
        let mut board = Board::starting_position();
        let mut picker = MovePicker::new(None);

        assert_eq!(picker.generate_all_moves(&mut board).len(), 20);
    }

    #[test]
    fn move_picker_orders_captures_before_quiets_without_pv_hint() {
        let mut board = Board::from_fen("4k3/8/8/3p4/8/2N5/8/4K3 w - - 0 1").unwrap();
        let state = SearchState::new();
        let mut picker = MovePicker::new(None);

        assert_eq!(
            picker.next_move(&mut board, &state, 0),
            Some(Move::new(
                Square::from_rf(2, 2),
                Square::from_rf(4, 3),
                MoveKind::Capture,
            ))
        );
    }
}
