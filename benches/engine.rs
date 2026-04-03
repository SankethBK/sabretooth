use std::time::Duration;

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use sabretooth::board::Board;
use sabretooth::search::search;

fn bench_movegen(c: &mut Criterion) {
    let mut group = c.benchmark_group("movegen");

    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(300));
    group.measurement_time(Duration::from_secs(1));

    group.bench_function("startpos_generate_legal_moves", |b| {
        b.iter(|| {
            let mut board = Board::starting_position();
            black_box(board.generate_legal_moves())
        });
    });

    group.bench_function("kiwipete_generate_legal_moves", |b| {
        b.iter(|| {
            let mut board = Board::from_fen(
                "r3k2r/p1ppqpb1/bn2pnp1/2pPN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            )
            .unwrap();
            black_box(board.generate_legal_moves())
        });
    });

    group.finish();
}

fn bench_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("search");

    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(1000));
    group.measurement_time(Duration::from_secs(15));

    group.bench_function("startpos_depth_4", |b| {
        b.iter(|| {
            let mut board = Board::starting_position();
            black_box(search(&mut board, 4))
        });
    });

    group.bench_function("bk01_depth_5", |b| {
        b.iter(|| {
            let mut board =
                Board::from_fen("1k1r4/pp1b1R2/3q2pp/4p3/2B5/4Q3/PPP2B2/2K5 b - - 0 1").unwrap();
            black_box(search(&mut board, 5))
        });
    });

    group.bench_function("bk05_depth_5", |b| {
        b.iter(|| {
            let mut board =
                Board::from_fen("r1b2rk1/2q1b1pp/p2ppn2/1p6/3QP3/1BN1B3/PPP3PP/R4RK1 w - - 0 1")
                    .unwrap();
            black_box(search(&mut board, 5))
        });
    });

    group.bench_function("bk11_depth_5", |b| {
        b.iter(|| {
            let mut board = Board::from_fen(
                "2r1nrk1/p2q1ppp/bp1p4/n1pPp3/P1P1P3/2PBB1N1/4QPPP/R4RK1 w - - 0 1",
            )
            .unwrap();
            black_box(search(&mut board, 5))
        });
    });

    //

    group.bench_function("bk14_depth_5", |b| {
        b.iter(|| {
            let mut board =
                Board::from_fen("rnb2r1k/pp2p2p/2pp2p1/q2P1p2/8/1Pb2NP1/PB2PPBP/R2Q1RK1 w - - 0 1")
                    .unwrap();
            black_box(search(&mut board, 5))
        });
    });

    group.bench_function("st01_depth_5", |b| {
        b.iter(|| {
            let mut board =
                Board::from_fen("2k4r/pp2pQ2/2qp2p1/2p5/4P2r/P2P3N/1PP2P2/R3R1K1 w - - 0 24")
                    .unwrap();
            black_box(search(&mut board, 4))
        });
    });

    group.finish();
}

criterion_group!(benches, bench_movegen, bench_search);
criterion_main!(benches);
