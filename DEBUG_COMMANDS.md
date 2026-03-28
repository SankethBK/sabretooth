# Sabretooth Debug Commands

This document covers the local debug commands currently supported by the engine.

These are not standard UCI commands. They are intended for terminal debugging while
developing the engine.

## Basic Flow

Most debug commands work after setting a position:

```text
position fen <fen>
<debug command>
```

You can also apply moves from that position:

```text
position fen <fen> moves e2e4 e7e5
<debug command>
```

If you want to explore incrementally, you can also apply one move at a time:

```text
position fen <fen>
move e2e4
move e7e5
<debug command>
```

You can print the current board at any time with:

```text
board
```

or the short alias:

```text
d
```

## `go depth N`

Runs the normal search and prints the best move.

Example:

```text
go depth 4
```

Output:

```text
bestmove f7e6
```

## `move <uci>`

Applies one legal move to the current board.

This is a local debug convenience command. It is useful when exploring a position
move by move without rewriting the full `position ... moves ...` command.

Example:

```text
position fen <fen>
move h3f4
move h4h1
eval
```

Notes:
- the move must be legal in the current position
- applying a move clears `lasteval`, because the board has changed

## `board` / `d`

Prints the current board, side to move, castling rights, en passant square, and move counters.

Examples:

```text
board
```

```text
d
```

## `eval`

Prints the static evaluation of the current position.

Fields:
- `material`: material balance from White's point of view
- `pst`: piece-square-table contribution from White's point of view
- `castling`: castling bonus contribution from White's point of view
- `total`: full static eval from White's point of view

Scores are always printed from White's point of view:
- `+` means White is better
- `-` means Black is better

Example:

```text
eval
```

Output:

```text
material +320
pst +40
castling +15
total +375
```

## `lasteval`

Prints the result of the most recent search-like command.

Currently updated by:
- `go depth N`
- `pv depth N`
- `rootmoves depth N`

Fields:
- `depth`: search depth used
- `search_score`: searched score from the side-to-move's point of view at the root
- `bestmove`: best move found by that search

Example:

```text
lasteval
```

Output:

```text
depth 4
search_score 305
bestmove f7e6
```

## `pv depth N`

Prints the engine's approximate principal variation from the current position.

Fields:
- `depth`: search depth used
- `search_score`: searched score for the root position
- `bestmove`: best root move
- `pv`: approximate best line

Example:

```text
pv depth 4
```

Output:

```text
depth 4
search_score 305
bestmove f7e6
pv f7e6 c8b8 h3g5 h4e4
```

Note:
- This is an approximate PV, not a full transposition-table PV.
- Later moves in the line are less reliable than earlier ones.

## `rootmoves depth N`

Prints every legal root move and its searched score, sorted best first.

This is the best command for answering:
- Why did the engine prefer move A over move B?
- How much worse did it think a candidate move was?

Example:

```text
rootmoves depth 4
```

Output:

```text
depth 4
f7e6 score 305
f7g6 score 180
f7e7 score 140
h3f4 score -100
```

## Suggested Debug Workflow

### Compare two candidate moves

```text
position fen <fen>
rootmoves depth 4
```

Then inspect a candidate move directly:

```text
position fen <fen> moves h3f4
eval
go depth 4
lasteval
```

### Inspect the engine's preferred line

```text
position fen <fen>
pv depth 4
lasteval
```

### Inspect the position after a forcing reply

```text
position fen <fen> moves h3f4 h4h1
eval
rootmoves depth 4
pv depth 4
```

## Current Limitations

- `pv depth N` is approximate.
- Static eval can disagree with search in tactical positions.
- Deeper `rootmoves` commands can be slow in sharp positions.
- These commands are local developer utilities, not standard UCI protocol.
