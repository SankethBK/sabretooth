use std::sync::OnceLock;

static OPENING_BOOK: OnceLock<Vec<Vec<String>>> = OnceLock::new();

pub fn next_book_move(history: &[String]) -> Option<String> {
    opening_book()
        .iter()
        .find(|line| {
            line.len() > history.len()
                && line
                    .iter()
                    .take(history.len())
                    .zip(history.iter())
                    .all(|(book_move, played_move)| book_move == played_move)
        })
        .map(|line| line[history.len()].clone())
}

fn opening_book() -> &'static [Vec<String>] {
    OPENING_BOOK
        .get_or_init(|| {
            include_str!("../book/tscp.txt")
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(|line| line.split_whitespace().map(str::to_string).collect())
                .collect()
        })
        .as_slice()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_first_matching_book_move() {
        assert_eq!(next_book_move(&[]), Some("g1f3".to_string()));
        assert_eq!(
            next_book_move(&["g1f3".to_string(), "g8f6".to_string()]),
            Some("c2c4".to_string())
        );
    }
}
