use crate::board::Board;
use crate::types::Move;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovePicker {
    moves: Vec<Move>,
    next_index: usize,
}

impl MovePicker {
    pub fn new(board: &mut Board, pv_hint: Option<Move>) -> Self {
        let mut moves = board.generate_legal_moves();
        prioritize_hint(&mut moves, pv_hint);

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

fn prioritize_hint(moves: &mut [Move], pv_hint: Option<Move>) {
    let Some(pv_hint) = pv_hint else {
        return;
    };

    if let Some(idx) = moves.iter().position(|&mv| mv == pv_hint) {
        moves.swap(0, idx);
    }
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

        let mut picker = MovePicker::new(&mut board, Some(pv_hint));

        assert_eq!(picker.next(), Some(pv_hint));
    }

    #[test]
    fn move_picker_iterates_all_legal_moves() {
        let mut board = Board::starting_position();
        let picker = MovePicker::new(&mut board, None);

        assert_eq!(picker.count(), 20);
    }
}
