use crate::attacks::{attack_table, bishop_attacks, queen_attacks, rook_attacks};
use crate::bitboard::pop_lsb;
use crate::board::Board;
use crate::types::{CastlingRights, Color, Move, MoveKind, Piece, Square};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveGenType {
    Captures,
    Quiets,
    NonEvasions,
}

impl Board {
    pub fn generate_legal_moves(&mut self) -> Vec<Move> {
        self.generate_legal_moves_by_type(MoveGenType::NonEvasions)
    }

    pub fn generate_legal_moves_by_type(&mut self, gen_type: MoveGenType) -> Vec<Move> {
        let us = self.side_to_move;
        let pseudo = self.generate_pseudo_legal_moves_by_type(gen_type);
        let mut legal = Vec::new();

        for mv in pseudo {
            self.make_move(mv);
            if !self.is_in_check(us) {
                legal.push(mv);
            }
            self.unmake_move();
        }

        legal
    }

    pub fn generate_pseudo_legal_moves(&self) -> Vec<Move> {
        self.generate_pseudo_legal_moves_by_type(MoveGenType::NonEvasions)
    }

    pub fn generate_pseudo_legal_moves_by_type(&self, gen_type: MoveGenType) -> Vec<Move> {
        let mut moves = Vec::new();
        moves.extend(self.generate_pawn_moves(gen_type));
        moves.extend(self.generate_knight_moves(gen_type));
        moves.extend(self.generate_bishop_moves(gen_type));
        moves.extend(self.generate_rook_moves(gen_type));
        moves.extend(self.generate_queen_moves(gen_type));
        moves.extend(self.generate_king_moves(gen_type));
        if gen_type != MoveGenType::Captures {
            moves.extend(self.generate_castling_moves());
        }
        moves
    }

    pub fn generate_knight_moves(&self, gen_type: MoveGenType) -> Vec<Move> {
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
                    if self.piece_on(to) == Some((us.opposite(), Piece::King)) {
                        continue;
                    }
                    MoveKind::Capture
                } else {
                    MoveKind::Quiet
                };

                if matches_movegen(kind, gen_type) {
                    moves.push(Move::new(from, to, kind));
                }
            }
        }

        moves
    }

    pub fn generate_king_moves(&self, gen_type: MoveGenType) -> Vec<Move> {
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
                    if self.piece_on(to) == Some((us.opposite(), Piece::King)) {
                        continue;
                    }
                    MoveKind::Capture
                } else {
                    MoveKind::Quiet
                };

                if matches_movegen(kind, gen_type) {
                    moves.push(Move::new(from, to, kind));
                }
            }
        }

        moves
    }

    pub fn generate_castling_moves(&self) -> Vec<Move> {
        let us = self.side_to_move;
        let them = us.opposite();
        let rights = self.castling_rights();
        let all_occupancy = self.all_occupancy();
        let mut moves = Vec::new();

        match us {
            Color::White => {
                let king_from = Square::from_rf(0, 4);

                if self.piece_on(king_from) == Some((Color::White, Piece::King))
                    && !self.is_square_attacked(king_from, them)
                {
                    if rights & CastlingRights::WHITE_KINGSIDE != 0
                        && self.piece_on(Square::from_rf(0, 7)) == Some((Color::White, Piece::Rook))
                    {
                        let f1 = Square::from_rf(0, 5);
                        let g1 = Square::from_rf(0, 6);

                        if (f1.to_bb() | g1.to_bb()) & all_occupancy == 0
                            && !self.is_square_attacked(f1, them)
                            && !self.is_square_attacked(g1, them)
                        {
                            moves.push(Move::new(king_from, g1, MoveKind::CastleKingside));
                        }
                    }

                    if rights & CastlingRights::WHITE_QUEENSIDE != 0
                        && self.piece_on(Square::from_rf(0, 0)) == Some((Color::White, Piece::Rook))
                    {
                        let b1 = Square::from_rf(0, 1);
                        let c1 = Square::from_rf(0, 2);
                        let d1 = Square::from_rf(0, 3);

                        if (b1.to_bb() | c1.to_bb() | d1.to_bb()) & all_occupancy == 0
                            && !self.is_square_attacked(d1, them)
                            && !self.is_square_attacked(c1, them)
                        {
                            moves.push(Move::new(king_from, c1, MoveKind::CastleQueenside));
                        }
                    }
                }
            }
            Color::Black => {
                let king_from = Square::from_rf(7, 4);

                if self.piece_on(king_from) == Some((Color::Black, Piece::King))
                    && !self.is_square_attacked(king_from, them)
                {
                    if rights & CastlingRights::BLACK_KINGSIDE != 0
                        && self.piece_on(Square::from_rf(7, 7)) == Some((Color::Black, Piece::Rook))
                    {
                        let f8 = Square::from_rf(7, 5);
                        let g8 = Square::from_rf(7, 6);

                        if (f8.to_bb() | g8.to_bb()) & all_occupancy == 0
                            && !self.is_square_attacked(f8, them)
                            && !self.is_square_attacked(g8, them)
                        {
                            moves.push(Move::new(king_from, g8, MoveKind::CastleKingside));
                        }
                    }

                    if rights & CastlingRights::BLACK_QUEENSIDE != 0
                        && self.piece_on(Square::from_rf(7, 0)) == Some((Color::Black, Piece::Rook))
                    {
                        let b8 = Square::from_rf(7, 1);
                        let c8 = Square::from_rf(7, 2);
                        let d8 = Square::from_rf(7, 3);

                        if (b8.to_bb() | c8.to_bb() | d8.to_bb()) & all_occupancy == 0
                            && !self.is_square_attacked(d8, them)
                            && !self.is_square_attacked(c8, them)
                        {
                            moves.push(Move::new(king_from, c8, MoveKind::CastleQueenside));
                        }
                    }
                }
            }
        }

        moves
    }

    pub fn generate_pawn_moves(&self, gen_type: MoveGenType) -> Vec<Move> {
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
                                let kind = MoveKind::Promotion(piece);
                                if matches_movegen(kind, gen_type) {
                                    moves.push(Move::new(from, one_forward, kind));
                                }
                            }
                        } else {
                            if matches_movegen(MoveKind::Quiet, gen_type) {
                                moves.push(Move::new(from, one_forward, MoveKind::Quiet));
                            }

                            if rank == 1 {
                                let two_forward = Square::from_rf(rank + 2, from.file());
                                if two_forward.to_bb() & all_occupancy == 0 {
                                    if matches_movegen(MoveKind::DoublePawnPush, gen_type) {
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
                Color::Black => {
                    if rank > 0 {
                        let one_forward = Square::from_rf(rank - 1, from.file());
                        if one_forward.to_bb() & all_occupancy == 0 {
                            if one_forward.rank() == 0 {
                                for piece in
                                    [Piece::Queen, Piece::Rook, Piece::Bishop, Piece::Knight]
                                {
                                    let kind = MoveKind::Promotion(piece);
                                    if matches_movegen(kind, gen_type) {
                                        moves.push(Move::new(from, one_forward, kind));
                                    }
                                }
                            } else {
                                if matches_movegen(MoveKind::Quiet, gen_type) {
                                    moves.push(Move::new(from, one_forward, MoveKind::Quiet));
                                }

                                if rank == 6 {
                                    let two_forward = Square::from_rf(rank - 2, from.file());
                                    if two_forward.to_bb() & all_occupancy == 0 {
                                        if matches_movegen(MoveKind::DoublePawnPush, gen_type) {
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
            }

            let mut targets = attacks.pawn[us.index()][from.index()];
            while targets != 0 {
                let to = pop_lsb(&mut targets);

                if to.to_bb() & their_occupancy != 0 {
                    if self.piece_on(to) == Some((them, Piece::King)) {
                        continue;
                    }
                    if is_promotion_square(us, to) {
                        for piece in [Piece::Queen, Piece::Rook, Piece::Bishop, Piece::Knight] {
                            let kind = MoveKind::PromotionCapture(piece);
                            if matches_movegen(kind, gen_type) {
                                moves.push(Move::new(from, to, kind));
                            }
                        }
                    } else if matches_movegen(MoveKind::Capture, gen_type) {
                        moves.push(Move::new(from, to, MoveKind::Capture));
                    }
                } else if self.en_passant == Some(to) && matches_movegen(MoveKind::EnPassant, gen_type) {
                    moves.push(Move::new(from, to, MoveKind::EnPassant));
                }
            }
        }

        moves
    }

    pub fn generate_bishop_moves(&self, gen_type: MoveGenType) -> Vec<Move> {
        self.generate_slider_moves(Piece::Bishop, bishop_attacks, gen_type)
    }

    pub fn generate_rook_moves(&self, gen_type: MoveGenType) -> Vec<Move> {
        self.generate_slider_moves(Piece::Rook, rook_attacks, gen_type)
    }

    pub fn generate_queen_moves(&self, gen_type: MoveGenType) -> Vec<Move> {
        self.generate_slider_moves(Piece::Queen, queen_attacks, gen_type)
    }

    fn generate_slider_moves(
        &self,
        piece: Piece,
        attack_fn: fn(Square, u64) -> u64,
        gen_type: MoveGenType,
    ) -> Vec<Move> {
        let us = self.side_to_move;
        let our_occupancy = self.occupancy(us);
        let their_occupancy = self.occupancy(us.opposite());
        let all_occupancy = self.all_occupancy();

        let mut pieces = self.piece_bb(us, piece);
        let mut moves = Vec::new();

        while pieces != 0 {
            let from = pop_lsb(&mut pieces);
            let mut targets = attack_fn(from, all_occupancy) & !our_occupancy;

            while targets != 0 {
                let to = pop_lsb(&mut targets);
                let kind = if to.to_bb() & their_occupancy != 0 {
                    if self.piece_on(to) == Some((us.opposite(), Piece::King)) {
                        continue;
                    }
                    MoveKind::Capture
                } else {
                    MoveKind::Quiet
                };

                if matches_movegen(kind, gen_type) {
                    moves.push(Move::new(from, to, kind));
                }
            }
        }

        moves
    }

    pub fn perft(&mut self, depth: u32) -> u64 {
        if depth == 0 {
            return 1;
        }

        let moves = self.generate_legal_moves();
        if depth == 1 {
            return moves.len() as u64;
        }

        let mut nodes = 0;
        for mv in moves {
            self.make_move(mv);
            nodes += self.perft(depth - 1);
            self.unmake_move();
        }

        nodes
    }
}

fn matches_movegen(kind: MoveKind, gen_type: MoveGenType) -> bool {
    match gen_type {
        MoveGenType::Captures => matches!(
            kind,
            MoveKind::Capture | MoveKind::EnPassant | MoveKind::PromotionCapture(_)
        ),
        MoveGenType::Quiets => matches!(
            kind,
            MoveKind::Quiet
                | MoveKind::DoublePawnPush
                | MoveKind::CastleKingside
                | MoveKind::CastleQueenside
                | MoveKind::Promotion(_)
        ),
        MoveGenType::NonEvasions => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starting_position_has_four_knight_moves() {
        let board = Board::starting_position();
        let moves = board.generate_knight_moves(MoveGenType::NonEvasions);

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
        let moves = board.generate_knight_moves(MoveGenType::NonEvasions);

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
        let moves = board.generate_king_moves(MoveGenType::NonEvasions);

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
        let moves = board.generate_king_moves(MoveGenType::NonEvasions);

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
        let moves = board.generate_pawn_moves(MoveGenType::NonEvasions);

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
        let moves = board.generate_pawn_moves(MoveGenType::NonEvasions);

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
        let moves = board.generate_pawn_moves(MoveGenType::NonEvasions);

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

    #[test]
    fn starting_position_has_no_bishop_rook_or_queen_moves() {
        let board = Board::starting_position();

        assert_eq!(board.generate_bishop_moves(MoveGenType::NonEvasions).len(), 0);
        assert_eq!(board.generate_rook_moves(MoveGenType::NonEvasions).len(), 0);
        assert_eq!(board.generate_queen_moves(MoveGenType::NonEvasions).len(), 0);
    }

    #[test]
    fn bishop_move_generation_handles_quiets_and_captures() {
        let board = Board::from_fen("8/8/5p2/8/3B4/8/1P6/8 w - - 0 1").unwrap();
        let moves = board.generate_bishop_moves(MoveGenType::NonEvasions);

        assert!(moves.contains(&Move::new(
            Square::from_rf(3, 3),
            Square::from_rf(4, 4),
            MoveKind::Quiet
        )));
        assert!(moves.contains(&Move::new(
            Square::from_rf(3, 3),
            Square::from_rf(5, 5),
            MoveKind::Capture
        )));
        assert!(!moves.contains(&Move::new(
            Square::from_rf(3, 3),
            Square::from_rf(1, 1),
            MoveKind::Quiet
        )));
    }

    #[test]
    fn rook_move_generation_handles_quiets_and_captures() {
        let board = Board::from_fen("8/8/3p4/8/3R1P2/8/8/8 w - - 0 1").unwrap();
        let moves = board.generate_rook_moves(MoveGenType::NonEvasions);

        assert!(moves.contains(&Move::new(
            Square::from_rf(3, 3),
            Square::from_rf(4, 3),
            MoveKind::Quiet
        )));
        assert!(moves.contains(&Move::new(
            Square::from_rf(3, 3),
            Square::from_rf(5, 3),
            MoveKind::Capture
        )));
        assert!(!moves.contains(&Move::new(
            Square::from_rf(3, 3),
            Square::from_rf(3, 6),
            MoveKind::Quiet
        )));
    }

    #[test]
    fn queen_move_generation_combines_rook_and_bishop_patterns() {
        let board = Board::from_fen("8/8/8/2p5/3Q4/5P2/8/8 w - - 0 1").unwrap();
        let moves = board.generate_queen_moves(MoveGenType::NonEvasions);

        assert!(moves.contains(&Move::new(
            Square::from_rf(3, 3),
            Square::from_rf(4, 3),
            MoveKind::Quiet
        )));
        assert!(moves.contains(&Move::new(
            Square::from_rf(3, 3),
            Square::from_rf(4, 2),
            MoveKind::Capture
        )));
        assert!(!moves.contains(&Move::new(
            Square::from_rf(3, 3),
            Square::from_rf(2, 5),
            MoveKind::Quiet
        )));
    }

    #[test]
    fn castling_generation_requires_clear_paths_and_rights() {
        let board = Board::from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();
        let moves = board.generate_castling_moves();

        assert!(moves.contains(&Move::new(
            Square::from_rf(0, 4),
            Square::from_rf(0, 6),
            MoveKind::CastleKingside
        )));
        assert!(moves.contains(&Move::new(
            Square::from_rf(0, 4),
            Square::from_rf(0, 2),
            MoveKind::CastleQueenside
        )));
    }

    #[test]
    fn castling_generation_rejects_attacked_transit_square() {
        let board = Board::from_fen("4kr2/8/8/8/8/8/8/R3K2R w KQ - 0 1").unwrap();
        let moves = board.generate_castling_moves();

        assert!(!moves.contains(&Move::new(
            Square::from_rf(0, 4),
            Square::from_rf(0, 6),
            MoveKind::CastleKingside
        )));
        assert!(moves.contains(&Move::new(
            Square::from_rf(0, 4),
            Square::from_rf(0, 2),
            MoveKind::CastleQueenside
        )));
    }

    #[test]
    fn castling_generation_rejects_missing_rook_even_with_rights() {
        let board = Board::from_fen("4k3/8/8/8/8/8/8/4K3 w KQ - 0 1").unwrap();
        let moves = board.generate_castling_moves();

        assert!(moves.is_empty());
    }

    #[test]
    fn starting_position_has_twenty_pseudo_legal_moves() {
        let board = Board::starting_position();
        let moves = board.generate_pseudo_legal_moves();

        assert_eq!(moves.len(), 20);
    }

    #[test]
    fn pseudo_legal_generation_includes_castling_when_available() {
        let board = Board::from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();
        let moves = board.generate_pseudo_legal_moves();

        assert!(moves.contains(&Move::new(
            Square::from_rf(0, 4),
            Square::from_rf(0, 6),
            MoveKind::CastleKingside
        )));
        assert!(moves.contains(&Move::new(
            Square::from_rf(0, 4),
            Square::from_rf(0, 2),
            MoveKind::CastleQueenside
        )));
    }

    #[test]
    fn legal_move_generation_matches_starting_position_count() {
        let mut board = Board::starting_position();
        let moves = board.generate_legal_moves();

        assert_eq!(moves.len(), 20);
    }

    #[test]
    fn legal_move_generation_filters_out_pinned_rook_moves() {
        let mut board = Board::from_fen("4r3/8/8/8/8/8/4R3/4K3 w - - 0 1").unwrap();
        let moves = board.generate_legal_moves();

        assert!(!moves.contains(&Move::new(
            Square::from_rf(1, 4),
            Square::from_rf(1, 3),
            MoveKind::Quiet
        )));
        assert!(moves.contains(&Move::new(
            Square::from_rf(1, 4),
            Square::from_rf(2, 4),
            MoveKind::Quiet
        )));
    }

    #[test]
    fn legal_move_generation_filters_out_king_move_into_check() {
        let mut board = Board::from_fen("4r3/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        let moves = board.generate_legal_moves();

        assert!(!moves.contains(&Move::new(
            Square::from_rf(0, 4),
            Square::from_rf(1, 4),
            MoveKind::Quiet
        )));
    }

    #[test]
    #[ignore]
    fn perft_matches_initial_position_reference_counts() {
        let mut board = Board::starting_position();

        assert_eq!(board.perft(0), 1);
        assert_eq!(board.perft(1), 20);
        assert_eq!(board.perft(2), 400);
        assert_eq!(board.perft(3), 8_902);
        assert_eq!(board.perft(4), 197_281);
        assert_eq!(board.perft(5), 4_865_609);
        assert_eq!(board.perft(6), 119_060_324);
    }

    #[test]
    #[ignore]
    fn perft_kiwipete() {
        let mut board =
            Board::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1")
                .unwrap();

        assert_eq!(board.perft(1), 48);
        assert_eq!(board.perft(2), 2039);
        assert_eq!(board.perft(3), 97_862);
        assert_eq!(board.perft(4), 4_085_603);
        assert_eq!(board.perft(5), 193_690_690);
    }

    #[test]
    fn perft_position_3() {
        let mut board = Board::from_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1").unwrap();

        assert_eq!(board.perft(1), 14);
        assert_eq!(board.perft(2), 191);
        assert_eq!(board.perft(3), 2812);
        assert_eq!(board.perft(4), 43_238);
        assert_eq!(board.perft(5), 674_624);
    }

    #[test]
    #[ignore]
    fn perft_position_4() {
        let mut board =
            Board::from_fen("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1")
                .unwrap();

        assert_eq!(board.perft(1), 6);
        assert_eq!(board.perft(2), 264);
        assert_eq!(board.perft(3), 9467);
        assert_eq!(board.perft(4), 422_333);
        assert_eq!(board.perft(5), 15_833_292);
    }

    #[test]
    #[ignore]
    fn perft_position_5() {
        let mut board =
            Board::from_fen("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8").unwrap();

        assert_eq!(board.perft(1), 44);
        assert_eq!(board.perft(2), 1_486);
        assert_eq!(board.perft(3), 62_379);
        assert_eq!(board.perft(4), 2_103_487);
    }
}

pub fn is_promotion_square(color: Color, square: Square) -> bool {
    match color {
        Color::White => square.rank() == 7,
        Color::Black => square.rank() == 0,
    }
}
