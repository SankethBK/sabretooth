use std::io::{self, BufRead, Write};

use crate::board::Board;
use crate::search::{DEFAULT_SEARCH_DEPTH, find_best_move, parse_go_depth};
use crate::types::{Move, MoveKind, Piece, Square};

pub struct Engine {
    board: Board,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            board: Board::starting_position(),
        }
    }

    pub fn run(&mut self) -> io::Result<()> {
        let stdin = io::stdin();
        let mut stdout = io::stdout().lock();

        for line in stdin.lock().lines() {
            let line = line?;
            if !self.handle_command(&line, &mut stdout)? {
                break;
            }
        }

        Ok(())
    }

    fn handle_command<W: Write>(&mut self, line: &str, out: &mut W) -> io::Result<bool> {
        let trimmed = line.trim();

        match trimmed {
            "uci" => {
                writeln!(out, "id name sabretooth")?;
                writeln!(out, "id author sanketh+codex")?;
                writeln!(out, "uciok")?;
            }
            "isready" => {
                writeln!(out, "readyok")?;
            }
            "ucinewgame" => {
                self.board = Board::starting_position();
            }
            "quit" => return Ok(false),
            _ if trimmed.starts_with("position ") => {
                self.handle_position(trimmed)?;
            }
            _ if trimmed == "position" => {}
            _ if trimmed.starts_with("go") => {
                self.handle_go(trimmed, out)?;
            }
            _ => {}
        }

        out.flush()?;
        Ok(true)
    }

    fn handle_position(&mut self, command: &str) -> io::Result<()> {
        let rest = command
            .strip_prefix("position ")
            .expect("position prefix already checked");

        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if tokens.is_empty() {
            return Ok(());
        }

        let mut moves_start = tokens.len();

        let mut board = if tokens[0] == "startpos" {
            if let Some(idx) = tokens.iter().position(|&token| token == "moves") {
                moves_start = idx + 1;
            }
            Board::starting_position()
        } else if tokens[0] == "fen" {
            let Some(moves_idx) = tokens.iter().position(|&token| token == "moves") else {
                let fen = tokens[1..].join(" ");
                self.board = Board::from_fen(&fen).map_err(invalid_input)?;
                return Ok(());
            };

            let fen = tokens[1..moves_idx].join(" ");
            moves_start = moves_idx + 1;
            Board::from_fen(&fen).map_err(invalid_input)?
        } else {
            return Ok(());
        };

        if moves_start < tokens.len() {
            for mv_str in &tokens[moves_start..] {
                let mv = parse_uci_move(&mut board, mv_str).ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("invalid UCI move in position command: {mv_str}"),
                    )
                })?;
                board.make_move(mv);
            }
        }

        self.board = board;
        Ok(())
    }

    fn handle_go<W: Write>(&mut self, command: &str, out: &mut W) -> io::Result<()> {
        let depth = parse_go_depth(command).unwrap_or(DEFAULT_SEARCH_DEPTH);

        if let Some(bestmove) = find_best_move(&mut self.board, depth) {
            writeln!(out, "bestmove {}", move_to_uci(bestmove))?;
        } else {
            writeln!(out, "bestmove 0000")?;
        }

        Ok(())
    }
}

fn invalid_input(err: impl std::fmt::Debug) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, format!("{err:?}"))
}

pub fn move_to_uci(mv: Move) -> String {
    let mut uci = format!("{}{}", square_to_uci(mv.from), square_to_uci(mv.to));
    if let MoveKind::Promotion(piece) | MoveKind::PromotionCapture(piece) = mv.kind {
        uci.push(promotion_suffix(piece));
    }
    uci
}

fn parse_uci_move(board: &mut Board, uci: &str) -> Option<Move> {
    board
        .generate_legal_moves()
        .into_iter()
        .find(|mv| move_to_uci(*mv) == uci)
}

fn square_to_uci(square: Square) -> String {
    let file = (b'a' + square.file()) as char;
    let rank = (b'1' + square.rank()) as char;
    format!("{file}{rank}")
}

fn promotion_suffix(piece: Piece) -> char {
    match piece {
        Piece::Knight => 'n',
        Piece::Bishop => 'b',
        Piece::Rook => 'r',
        Piece::Queen => 'q',
        Piece::Pawn | Piece::King => panic!("invalid promotion piece"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;

    #[test]
    fn move_to_uci_formats_quiet_and_promotion_moves() {
        assert_eq!(
            move_to_uci(Move::new(
                Square::from_rf(1, 4),
                Square::from_rf(3, 4),
                MoveKind::DoublePawnPush
            )),
            "e2e4"
        );

        assert_eq!(
            move_to_uci(Move::new(
                Square::from_rf(6, 0),
                Square::from_rf(7, 0),
                MoveKind::Promotion(Piece::Queen)
            )),
            "a7a8q"
        );
    }

    #[test]
    fn parse_uci_move_matches_legal_moves() {
        let mut board = Board::starting_position();
        let mv = parse_uci_move(&mut board, "e2e4").expect("e2e4 should be legal");

        assert_eq!(
            mv,
            Move::new(
                Square::from_rf(1, 4),
                Square::from_rf(3, 4),
                MoveKind::DoublePawnPush
            )
        );
    }
}
