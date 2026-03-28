use std::io::{self, BufRead, Write};

use crate::board::Board;
use crate::search::{
    DEFAULT_SEARCH_DEPTH, SearchResult, approximate_principal_variation, parse_go_depth, search,
};
use crate::types::{Color, Move, MoveKind, Piece, Square};

pub struct Engine {
    board: Board,
    last_search: Option<(u32, SearchResult)>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            board: Board::starting_position(),
            last_search: None,
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
                self.last_search = None;
            }
            "eval" => {
                self.handle_eval(out)?;
            }
            "lasteval" => {
                self.handle_last_eval(out)?;
            }
            _ if trimmed.starts_with("pv") => {
                self.handle_pv(trimmed, out)?;
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
        self.last_search = None;
        Ok(())
    }

    fn handle_go<W: Write>(&mut self, command: &str, out: &mut W) -> io::Result<()> {
        let depth = parse_go_depth(command).unwrap_or(DEFAULT_SEARCH_DEPTH);
        let result = search(&mut self.board, depth);
        self.last_search = Some((depth, result));

        if let Some(bestmove) = result.best_move {
            writeln!(out, "bestmove {}", move_to_uci(bestmove))?;
        } else {
            writeln!(out, "bestmove 0000")?;
        }

        Ok(())
    }

    fn handle_eval<W: Write>(&self, out: &mut W) -> io::Result<()> {
        let eval = self.board.evaluate_breakdown();
        let stm = match self.board.side_to_move() {
            Color::White => eval.total,
            Color::Black => -eval.total,
        };

        writeln!(out, "material {}", eval.material)?;
        writeln!(out, "pst {}", eval.pst)?;
        writeln!(out, "castling {}", eval.castling)?;
        writeln!(out, "total {}", eval.total)?;
        writeln!(out, "stm {}", stm)?;

        Ok(())
    }

    fn handle_last_eval<W: Write>(&self, out: &mut W) -> io::Result<()> {
        let Some((depth, result)) = self.last_search else {
            writeln!(out, "no search has been run")?;
            return Ok(());
        };

        writeln!(out, "depth {}", depth)?;
        writeln!(out, "search_score {}", result.score)?;

        if let Some(bestmove) = result.best_move {
            writeln!(out, "bestmove {}", move_to_uci(bestmove))?;
        } else {
            writeln!(out, "bestmove 0000")?;
        }

        Ok(())
    }

    fn handle_pv<W: Write>(&mut self, command: &str, out: &mut W) -> io::Result<()> {
        let depth = parse_go_depth(command).unwrap_or(DEFAULT_SEARCH_DEPTH);
        let (result, pv) = approximate_principal_variation(&self.board, depth);
        self.last_search = Some((depth, result));
        let pv_text = pv
            .into_iter()
            .map(move_to_uci)
            .collect::<Vec<_>>()
            .join(" ");

        writeln!(out, "depth {}", depth)?;
        writeln!(out, "search_score {}", result.score)?;

        if let Some(bestmove) = result.best_move {
            writeln!(out, "bestmove {}", move_to_uci(bestmove))?;
        } else {
            writeln!(out, "bestmove 0000")?;
        }

        writeln!(out, "pv {}", pv_text)?;
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

    #[test]
    fn eval_command_prints_breakdown() {
        let engine = Engine {
            board: Board::starting_position(),
            last_search: None,
        };
        let mut output = Vec::new();

        engine.handle_eval(&mut output).unwrap();

        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("material 0"));
        assert!(text.contains("pst 0"));
        assert!(text.contains("castling 0"));
        assert!(text.contains("total 0"));
        assert!(text.contains("stm 0"));
    }

    #[test]
    fn lasteval_prints_last_search_result() {
        let engine = Engine {
            board: Board::starting_position(),
            last_search: Some((
                4,
                SearchResult {
                    best_move: Some(Move::new(
                        Square::from_rf(1, 4),
                        Square::from_rf(3, 4),
                        MoveKind::DoublePawnPush,
                    )),
                    score: 123,
                },
            )),
        };
        let mut output = Vec::new();

        engine.handle_last_eval(&mut output).unwrap();

        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("depth 4"));
        assert!(text.contains("search_score 123"));
        assert!(text.contains("bestmove e2e4"));
    }

    #[test]
    fn pv_command_prints_line() {
        let mut engine = Engine {
            board: Board::starting_position(),
            last_search: None,
        };
        let mut output = Vec::new();

        engine.handle_pv("pv depth 2", &mut output).unwrap();

        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("depth 2"));
        assert!(text.contains("bestmove"));
        assert!(text.contains("pv "));
    }
}
