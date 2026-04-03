use crate::board::Board;
use crate::search::find_best_move;
use crate::types::{Move, MoveKind, Piece, Square};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpdCase {
    pub id: String,
    pub fen: String,
    pub best_moves_san: Vec<String>,
}

pub fn parse_epd_cases(input: &str) -> Vec<EpdCase> {
    input.lines().filter_map(parse_epd_case).collect()
}

pub fn parse_epd_case(line: &str) -> Option<EpdCase> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }

    let mut segments = line.split(';').map(str::trim).filter(|segment| !segment.is_empty());
    let head = segments.next()?;
    let mut tokens = head.split_whitespace();

    let placement = tokens.next()?;
    let side_to_move = tokens.next()?;
    let castling = tokens.next()?;
    let en_passant = tokens.next()?;

    let fen = format!("{placement} {side_to_move} {castling} {en_passant} 0 1");
    let mut id = None;
    let mut best_moves_san = Vec::new();

    let head_ops = tokens.collect::<Vec<_>>().join(" ");
    parse_epd_ops(&head_ops, &mut id, &mut best_moves_san);

    for segment in segments {
        parse_epd_ops(segment, &mut id, &mut best_moves_san);
    }

    Some(EpdCase {
        id: id.unwrap_or_default(),
        fen,
        best_moves_san,
    })
}

pub fn move_to_san(board: &mut Board, mv: Move) -> String {
    match mv.kind {
        MoveKind::CastleKingside => return append_check_suffix(board, mv, "O-O".to_string()),
        MoveKind::CastleQueenside => {
            return append_check_suffix(board, mv, "O-O-O".to_string());
        }
        _ => {}
    }

    let (_, piece) = board
        .piece_on(mv.from)
        .expect("SAN conversion requires a piece on the from square");

    let mut san = String::new();

    if piece != Piece::Pawn {
        san.push(piece_letter(piece));
        san.push_str(&disambiguation(board, mv, piece));
    } else if mv.is_capture() {
        san.push(file_char(mv.from.file()));
    }

    if mv.is_capture() {
        san.push('x');
    }

    san.push_str(&square_to_algebraic(mv.to));

    if let MoveKind::Promotion(promoted) | MoveKind::PromotionCapture(promoted) = mv.kind {
        san.push('=');
        san.push(piece_letter(promoted));
    }

    append_check_suffix(board, mv, san)
}

pub fn bk_depth_report(depth: u32) -> (usize, Vec<String>) {
    let cases = parse_epd_cases(include_str!("../testdata/bk.epd"));
    let mut solved = 0usize;
    let mut misses = Vec::new();

    for case in &cases {
        let mut board = Board::from_fen(&case.fen).expect("BK case FEN must parse");
        let best = find_best_move(&mut board, depth).expect("expected a legal move");
        let best_san = move_to_san(&mut board, best);

        if case.best_moves_san.iter().any(|expected| expected == &best_san) {
            solved += 1;
        } else {
            misses.push(format!(
                "{} miss: expected {:?}, got {}",
                case.id, case.best_moves_san, best_san
            ));
        }
    }

    (solved, misses)
}

fn parse_epd_ops(segment: &str, id: &mut Option<String>, best_moves_san: &mut Vec<String>) {
    if let Some(rest) = segment.strip_prefix("bm ") {
        *best_moves_san = rest.split_whitespace().map(str::to_string).collect();
    } else if let Some(rest) = segment.strip_prefix("id ") {
        *id = Some(rest.trim().trim_matches('"').to_string());
    }
}

fn append_check_suffix(board: &mut Board, mv: Move, mut san: String) -> String {
    board.make_move(mv);
    let side_to_move = board.side_to_move();
    let suffix = if board.is_in_check(side_to_move) {
        if board.generate_legal_moves().is_empty() {
            "#"
        } else {
            "+"
        }
    } else {
        ""
    };
    board.unmake_move();
    san.push_str(suffix);
    san
}

fn disambiguation(board: &mut Board, mv: Move, piece: Piece) -> String {
    let color = board
        .piece_on(mv.from)
        .map(|(color, _)| color)
        .expect("disambiguation requires moving piece");

    let competing_moves = board
        .generate_legal_moves()
        .into_iter()
        .filter(|candidate| candidate.to == mv.to && candidate.from != mv.from)
        .filter(|candidate| board.piece_on(candidate.from) == Some((color, piece)))
        .collect::<Vec<_>>();

    if competing_moves.is_empty() {
        return String::new();
    }

    let same_file = competing_moves
        .iter()
        .any(|candidate| candidate.from.file() == mv.from.file());
    let same_rank = competing_moves
        .iter()
        .any(|candidate| candidate.from.rank() == mv.from.rank());

    let mut out = String::new();
    if !same_file {
        out.push(file_char(mv.from.file()));
    } else if !same_rank {
        out.push(rank_char(mv.from.rank()));
    } else {
        out.push(file_char(mv.from.file()));
        out.push(rank_char(mv.from.rank()));
    }
    out
}

fn square_to_algebraic(square: Square) -> String {
    format!("{}{}", file_char(square.file()), rank_char(square.rank()))
}

fn piece_letter(piece: Piece) -> char {
    match piece {
        Piece::Pawn => panic!("pawns do not use a SAN piece letter"),
        Piece::Knight => 'N',
        Piece::Bishop => 'B',
        Piece::Rook => 'R',
        Piece::Queen => 'Q',
        Piece::King => 'K',
    }
}

fn file_char(file: u8) -> char {
    (b'a' + file) as char
}

fn rank_char(rank: u8) -> char {
    (b'1' + rank) as char
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_epd_case_with_single_best_move() {
        let case = parse_epd_case(r#"1k1r4/pp1b1R2/3q2pp/4p3/2B5/4Q3/PPP2B2/2K5 b - - bm Qd1+; id "BK.01";"#)
            .expect("expected case");

        assert_eq!(case.id, "BK.01");
        assert_eq!(
            case.fen,
            "1k1r4/pp1b1R2/3q2pp/4p3/2B5/4Q3/PPP2B2/2K5 b - - 0 1"
        );
        assert_eq!(case.best_moves_san, vec!["Qd1+".to_string()]);
    }

    #[test]
    fn parses_epd_case_with_multiple_best_moves() {
        let case = parse_epd_case(
            r#"r1b2rk1/2q1b1pp/p2ppn2/1p6/3QP3/1BN1B3/PPP3PP/R4RK1 w - - bm Nd5 a4; id "BK.05";"#,
        )
        .expect("expected case");

        assert_eq!(case.id, "BK.05");
        assert_eq!(
            case.best_moves_san,
            vec!["Nd5".to_string(), "a4".to_string()]
        );
    }

    #[test]
    fn formats_simple_pawn_push() {
        let mut board = Board::starting_position();
        let mv = board
            .generate_legal_moves()
            .into_iter()
            .find(|mv| mv.from == Square::from_rf(1, 4) && mv.to == Square::from_rf(3, 4))
            .unwrap();

        assert_eq!(move_to_san(&mut board, mv), "e4");
    }

    #[test]
    fn formats_checking_queen_move() {
        let mut board = Board::from_fen("1k1r4/pp1b1R2/3q2pp/4p3/2B5/4Q3/PPP2B2/2K5 b - - 0 1")
            .unwrap();
        let mv = board
            .generate_legal_moves()
            .into_iter()
            .find(|mv| mv.from == Square::from_rf(5, 3) && mv.to == Square::from_rf(0, 3))
            .unwrap();

        assert_eq!(move_to_san(&mut board, mv), "Qd1+");
    }

    #[test]
    fn bk_suite_positions_match_legal_san_moves() {
        let cases = parse_epd_cases(include_str!("../testdata/bk.epd"));
        assert_eq!(cases.len(), 25);

        for case in cases {
            let mut board = Board::from_fen(&case.fen).expect("BK case FEN must parse");
            let legal_san = board
                .generate_legal_moves()
                .into_iter()
                .map(|mv| move_to_san(&mut board, mv))
                .collect::<Vec<_>>();

            assert!(
                case.best_moves_san
                    .iter()
                    .any(|expected| legal_san.iter().any(|actual| actual == expected)),
                "case {} had no matching SAN move. expected one of {:?}, got {:?}",
                case.id,
                case.best_moves_san,
                legal_san
            );
        }
    }
}
