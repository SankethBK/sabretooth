use crate::attacks::attack_table;
use crate::bitboard::pop_lsb;
use crate::board::Board;
use crate::types::{Color, Move, MoveKind, Piece, Square};

impl Board {
    pub fn generate_knight_moves(&self) -> Vec<Move> {
        let us = self.side_to_move;
        let our_occupancy = self.occupancy(us);
        let their_occupancy = self.occupancy(us.opposite());
        let attacks = attack_table();

        let mut knights = self.piece_bb(us, Piece::Knight);
        let mut moves = Vec::new();

        while knights != 0 {
            let from = pop_lsb(&mut knights);
            let mut targets = attacks.knight[from.index()] & !our_occupancy;

            while targets != 0 {
                let to = pop_lsb(&mut targets);
                let kind = if to.to_bb() & their_occupancy != 0 {
                    MoveKind::Capture
                } else {
                    MoveKind::Quiet
                };

                moves.push(Move::new(from, to, kind));
            }
        }

        moves
    }

    pub fn generate_king_moves(&self) -> Vec<Move> {
        let us = self.side_to_move;
        let our_occupancy = self.occupancy(us);
        let their_occupancy = self.occupancy(us.opposite());
        let attacks = attack_table();

        let mut kings = self.piece_bb(us, Piece::King);
        let mut moves = Vec::new();

        while kings != 0 {
            let from = pop_lsb(&mut kings);
            let mut targets = attacks.king[from.index()] & !our_occupancy;

            while targets != 0 {
                let to = pop_lsb(&mut targets);
                let kind = if to.to_bb() & their_occupancy != 0 {
                    MoveKind::Capture
                } else {
                    MoveKind::Quiet
                };

                moves.push(Move::new(from, to, kind));
            }
        }

        moves
    }

    pub fn generate_pawn_moves(&self) -> Vec<Move> {
        let us = self.side_to_move;
        let them = us.opposite();
        let all_occupancy = self.all_occupancy();
        let their_occupancy = self.occupancy(them);
        let attacks = attack_table();

        let mut pawns = self.piece_bb(us, Piece::Pawn);
        let mut moves = Vec::new();

        while pawns != 0 {
            let from = pop_lsb(&mut pawns);
            let rank = from.rank();

            match us {
                Color::White => {
                    let one_forward = Square::from_rf(rank + 1, from.file());
                    if rank < 7 && one_forward.to_bb() & all_occupancy == 0 {
                        if one_forward.rank() == 7 {
                            for piece in [Piece::Queen, Piece::Rook, Piece::Bishop, Piece::Knight] {
                                moves.push(Move::new(
                                    from,
                                    one_forward,
                                    MoveKind::Promotion(piece),
                                ));
                            }
                        } else {
                            moves.push(Move::new(from, one_forward, MoveKind::Quiet));

                            if rank == 1 {
                                let two_forward = Square::from_rf(rank + 2, from.file());
                                if two_forward.to_bb() & all_occupancy == 0 {
                                    moves.push(Move::new(
                                        from,
                                        two_forward,
                                        MoveKind::DoublePawnPush,
                                    ));
                                }
                            }
                        }
                    }
                }
                Color::Black => {
                    if rank > 0 {
                        let one_forward = Square::from_rf(rank - 1, from.file());
                        if one_forward.to_bb() & all_occupancy == 0 {
                            if one_forward.rank() == 0 {
                                for piece in
                                    [Piece::Queen, Piece::Rook, Piece::Bishop, Piece::Knight]
                                {
                                    moves.push(Move::new(
                                        from,
                                        one_forward,
                                        MoveKind::Promotion(piece),
                                    ));
                                }
                            } else {
                                moves.push(Move::new(from, one_forward, MoveKind::Quiet));

                                if rank == 6 {
                                    let two_forward = Square::from_rf(rank - 2, from.file());
                                    if two_forward.to_bb() & all_occupancy == 0 {
                                        moves.push(Move::new(
                                            from,
                                            two_forward,
                                            MoveKind::DoublePawnPush,
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }

            let mut targets = attacks.pawn[us.index()][from.index()];
            while targets != 0 {
                let to = pop_lsb(&mut targets);

                if to.to_bb() & their_occupancy != 0 {
                    if is_promotion_square(us, to) {
                        for piece in [Piece::Queen, Piece::Rook, Piece::Bishop, Piece::Knight] {
                            moves.push(Move::new(from, to, MoveKind::PromotionCapture(piece)));
                        }
                    } else {
                        moves.push(Move::new(from, to, MoveKind::Capture));
                    }
                } else if self.en_passant == Some(to) {
                    moves.push(Move::new(from, to, MoveKind::EnPassant));
                }
            }
        }

        moves
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starting_position_has_four_knight_moves() {
        let board = Board::starting_position();
        let moves = board.generate_knight_moves();

        assert_eq!(moves.len(), 4);
        assert!(moves.contains(&Move::new(
            Square::from_rf(0, 1),
            Square::from_rf(2, 0),
            MoveKind::Quiet
        )));
        assert!(moves.contains(&Move::new(
            Square::from_rf(0, 1),
            Square::from_rf(2, 2),
            MoveKind::Quiet
        )));
        assert!(moves.contains(&Move::new(
            Square::from_rf(0, 6),
            Square::from_rf(2, 5),
            MoveKind::Quiet
        )));
        assert!(moves.contains(&Move::new(
            Square::from_rf(0, 6),
            Square::from_rf(2, 7),
            MoveKind::Quiet
        )));
    }

    #[test]
    fn knight_move_generation_marks_captures() {
        let board = Board::from_fen("8/8/8/8/8/2p5/8/1N6 w - - 0 1").unwrap();
        let moves = board.generate_knight_moves();

        assert!(moves.contains(&Move::new(
            Square::from_rf(0, 1),
            Square::from_rf(2, 2),
            MoveKind::Capture
        )));
        assert!(moves.contains(&Move::new(
            Square::from_rf(0, 1),
            Square::from_rf(2, 0),
            MoveKind::Quiet
        )));
        assert!(moves.contains(&Move::new(
            Square::from_rf(0, 1),
            Square::from_rf(1, 3),
            MoveKind::Quiet
        )));
    }

    #[test]
    fn king_move_generation_respects_friendly_blockers() {
        let board = Board::from_fen("8/8/8/8/3K4/3P4/8/8 w - - 0 1").unwrap();
        let moves = board.generate_king_moves();

        assert_eq!(moves.len(), 7);
        assert!(!moves.contains(&Move::new(
            Square::from_rf(3, 3),
            Square::from_rf(2, 3),
            MoveKind::Quiet
        )));
        assert!(moves.contains(&Move::new(
            Square::from_rf(3, 3),
            Square::from_rf(4, 4),
            MoveKind::Quiet
        )));
    }

    #[test]
    fn king_move_generation_marks_captures() {
        let board = Board::from_fen("8/8/8/8/3Kp3/8/8/8 w - - 0 1").unwrap();
        let moves = board.generate_king_moves();

        assert!(moves.contains(&Move::new(
            Square::from_rf(3, 3),
            Square::from_rf(3, 4),
            MoveKind::Capture
        )));
        assert!(moves.contains(&Move::new(
            Square::from_rf(3, 3),
            Square::from_rf(2, 2),
            MoveKind::Quiet
        )));
    }

    #[test]
    fn starting_position_has_sixteen_pawn_moves() {
        let board = Board::starting_position();
        let moves = board.generate_pawn_moves();

        assert_eq!(moves.len(), 16);
        assert!(moves.contains(&Move::new(
            Square::from_rf(1, 4),
            Square::from_rf(2, 4),
            MoveKind::Quiet
        )));
        assert!(moves.contains(&Move::new(
            Square::from_rf(1, 4),
            Square::from_rf(3, 4),
            MoveKind::DoublePawnPush
        )));
    }

    #[test]
    fn pawn_move_generation_handles_captures_and_en_passant() {
        let board = Board::from_fen("8/8/8/3pP3/8/8/8/8 w - d6 0 1").unwrap();
        let moves = board.generate_pawn_moves();

        assert!(moves.contains(&Move::new(
            Square::from_rf(4, 4),
            Square::from_rf(5, 3),
            MoveKind::EnPassant
        )));
        assert!(moves.contains(&Move::new(
            Square::from_rf(4, 4),
            Square::from_rf(5, 4),
            MoveKind::Quiet
        )));
    }

    #[test]
    fn pawn_move_generation_handles_promotions() {
        let board = Board::from_fen("1r6/P7/8/8/8/8/8/8 w - - 0 1").unwrap();
        let moves = board.generate_pawn_moves();

        for piece in [Piece::Queen, Piece::Rook, Piece::Bishop, Piece::Knight] {
            assert!(moves.contains(&Move::new(
                Square::from_rf(6, 0),
                Square::from_rf(7, 0),
                MoveKind::Promotion(piece)
            )));
            assert!(moves.contains(&Move::new(
                Square::from_rf(6, 0),
                Square::from_rf(7, 1),
                MoveKind::PromotionCapture(piece)
            )));
        }
    }
}

pub fn is_promotion_square(color: Color, square: Square) -> bool {
    match color {
        Color::White => square.rank() == 7,
        Color::Black => square.rank() == 0,
    }
}
