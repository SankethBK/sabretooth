use crate::board::Board;
use crate::movegen::MoveGenType;
use crate::search::SearchState;
use crate::types::{Move, MoveKind, Piece};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Pv,
    Captures,
    Quiets,
    Done,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovePicker {
    pv_move: Option<Move>,
    stage: Stage,
    buffer: Vec<Move>,
    next_index: usize,
    captures_only: bool,
}

impl MovePicker {
    pub fn new(
        board: &mut Board,
        pv_hint: Option<Move>,
        state: &SearchState,
        ply: usize,
    ) -> Self {
        let pv_move = pv_hint.filter(|&mv| is_legal_move(board, mv));
        let mut picker = Self {
            pv_move,
            stage: Stage::Pv,
            buffer: Vec::new(),
            next_index: 0,
            captures_only: false,
        };
        picker.fill_next_stage(board, state, ply);
        picker
    }

    pub fn captures_only(board: &mut Board, state: &SearchState, ply: usize) -> Self {
        let mut picker = Self {
            pv_move: None,
            stage: Stage::Captures,
            buffer: Vec::new(),
            next_index: 0,
            captures_only: true,
        };
        picker.fill_next_stage(board, state, ply);
        picker
    }

    pub fn is_empty(&self) -> bool {
        self.stage == Stage::Done && self.buffer.is_empty()
    }

    fn fill_next_stage(&mut self, board: &mut Board, state: &SearchState, ply: usize) {
        loop {
            self.buffer.clear();
            self.next_index = 0;

            match self.stage {
                Stage::Pv => {
                    self.stage = Stage::Captures;
                    if let Some(pv_move) = self.pv_move {
                        self.buffer.push(pv_move);
                        return;
                    }
                }
                Stage::Captures => {
                    self.buffer = generate_scored_stage(
                        board,
                        MoveGenType::Captures,
                        self.pv_move,
                        state,
                        ply,
                    );
                    self.stage = if self.captures_only {
                        Stage::Done
                    } else {
                        Stage::Quiets
                    };

                    if !self.buffer.is_empty() {
                        return;
                    }
                }
                Stage::Quiets => {
                    self.buffer = generate_scored_stage(
                        board,
                        MoveGenType::Quiets,
                        self.pv_move,
                        state,
                        ply,
                    );
                    self.stage = Stage::Done;

                    if !self.buffer.is_empty() {
                        return;
                    }
                }
                Stage::Done => return,
            }
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
                return Some(mv);
            }

            if self.stage == Stage::Done {
                return None;
            }

            self.fill_next_stage(board, state, ply);
        }
    }
}

fn generate_scored_stage(
    board: &mut Board,
    gen_type: MoveGenType,
    pv_move: Option<Move>,
    state: &SearchState,
    ply: usize,
) -> Vec<Move> {
    let mut scored_moves = board
        .generate_legal_moves_by_type(gen_type)
        .into_iter()
        .filter(|&mv| Some(mv) != pv_move)
        .map(|mv| (score_move(board, mv, None, state, ply), mv))
        .collect::<Vec<_>>();

    scored_moves.sort_by(|a, b| b.0.cmp(&a.0));
    scored_moves.into_iter().map(|(_, mv)| mv).collect()
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
    if matches!(mv.kind, MoveKind::EnPassant) {
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

        assert_eq!(picker.next_move(&mut board, &state, 0), Some(pv_hint));
    }

    #[test]
    fn move_picker_iterates_all_legal_moves() {
        let mut board = Board::starting_position();
        let state = SearchState::new();
        let mut picker = MovePicker::new(&mut board, None, &state, 0);
        let mut count = 0;

        while picker.next_move(&mut board, &state, 0).is_some() {
            count += 1;
        }

        assert_eq!(count, 20);
    }

    #[test]
    fn move_picker_orders_captures_before_quiets_without_pv_hint() {
        let mut board = Board::from_fen("4k3/8/8/3p4/8/2N5/8/4K3 w - - 0 1").unwrap();
        let state = SearchState::new();
        let mut picker = MovePicker::new(&mut board, None, &state, 0);

        assert_eq!(
            picker.next_move(&mut board, &state, 0),
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
        let mut picker = MovePicker::captures_only(&mut board, &state, 0);
        let mut moves = Vec::new();

        while let Some(mv) = picker.next_move(&mut board, &state, 0) {
            moves.push(mv);
        }

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
