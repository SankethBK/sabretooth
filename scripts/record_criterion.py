#!/usr/bin/env python3

import argparse
import json
import platform
import socket
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional
from uuid import uuid4


SEARCH_BENCHMARKS = {
    "search/startpos_depth_4": {
        "fen": "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        "depth": 4,
    },
    "search/bk01_depth_5": {
        "fen": "1k1r4/pp1b1R2/3q2pp/4p3/2B5/4Q3/PPP2B2/2K5 b - - 0 1",
        "depth": 5,
    },
    "search/bk05_depth_5": {
        "fen": "r1b2rk1/2q1b1pp/p2ppn2/1p6/3QP3/1BN1B3/PPP3PP/R4RK1 w - - 0 1",
        "depth": 5,
    },
    "search/bk11_depth_5": {
        "fen": "2r1nrk1/p2q1ppp/bp1p4/n1pPp3/P1P1P3/2PBB1N1/4QPPP/R4RK1 w - - 0 1",
        "depth": 5,
    },
    "search/bk14_depth_5": {
        "fen": "rnb2r1k/pp2p2p/2pp2p1/q2P1p2/8/1Pb2NP1/PB2PPBP/R2Q1RK1 w - - 0 1",
        "depth": 5,
    },
    "search/st01_depth_5": {
        "fen": "2k4r/pp2pQ2/2qp2p1/2p5/4P2r/P2P3N/1PP2P2/R3R1K1 w - - 0 24",
        "depth": 5,
    },
}


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Run Criterion and append benchmark results to a JSONL history file."
    )
    parser.add_argument(
        "--machine",
        default=socket.gethostname(),
        help="Stable machine label, e.g. m2-mac or x86-laptop.",
    )
    parser.add_argument(
        "--output",
        default="benchmarks/history.jsonl",
        help="Path to the JSONL history file to append results to.",
    )
    parser.add_argument(
        "--bench",
        default="engine",
        help="Criterion bench target to run.",
    )
    parser.add_argument(
        "bench_filter",
        nargs="?",
        default=None,
        help="Optional Criterion benchmark filter, e.g. search/st01_depth_5.",
    )

    args = parser.parse_args()
    repo_root = Path(__file__).resolve().parents[1]

    run_bench(repo_root, args.bench, args.bench_filter)
    run_record = collect_run_record(repo_root, args.machine, args.bench_filter)
    append_run_record(repo_root / args.output, run_record)

    print(
        f"Recorded {len(run_record['benchmarks'])} benchmark results to {args.output}"
    )
    return 0


def run_bench(
    repo_root: Path,
    bench_name: str,
    bench_filter: Optional[str],
) -> None:
    command = ["cargo", "bench", "--bench", bench_name]
    if bench_filter:
        command.extend(["--", bench_filter])

    subprocess.run(command, cwd=repo_root, check=True)


def collect_run_record(
    repo_root: Path,
    machine_label: str,
    bench_filter: Optional[str],
) -> Dict[str, Any]:
    criterion_root = repo_root / "target" / "criterion"
    benchmark_results = []

    for estimates_path in sorted(criterion_root.glob("*/*/new/estimates.json")):
        with estimates_path.open() as f:
            estimates = json.load(f)

        group = estimates_path.parents[2].name
        benchmark = estimates_path.parents[1].name
        benchmark_name = f"{group}/{benchmark}"

        if bench_filter and bench_filter not in benchmark_name:
            continue

        mean = estimates["mean"]
        median = estimates["median"]

        benchmark_results.append(
            enrich_benchmark_result(
                repo_root,
                {
                "benchmark": benchmark_name,
                "mean_ns": mean["point_estimate"],
                "mean_ci_low_ns": mean["confidence_interval"]["lower_bound"],
                "mean_ci_high_ns": mean["confidence_interval"]["upper_bound"],
                "median_ns": median["point_estimate"],
                "median_ci_low_ns": median["confidence_interval"]["lower_bound"],
                "median_ci_high_ns": median["confidence_interval"]["upper_bound"],
                },
            )
        )

    if not benchmark_results:
        raise RuntimeError("No Criterion estimate files found under target/criterion/*/*/new")

    return {
        "run_id": str(uuid4()),
        "recorded_at": datetime.now(timezone.utc).isoformat(),
        "machine_label": machine_label,
        "hostname": socket.gethostname(),
        "system": platform.system(),
        "arch": platform.machine(),
        "cpu_model": detect_cpu_model(),
        "branch": git_output(repo_root, ["git", "branch", "--show-current"]),
        "commit_id": git_output(repo_root, ["git", "rev-parse", "HEAD"]),
        "dirty": bool(git_output(repo_root, ["git", "status", "--short"])),
        "benchmarks": benchmark_results,
    }


def append_run_record(path: Path, run_record: Dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a") as f:
        f.write(json.dumps(run_record, sort_keys=True))
        f.write("\n")


def enrich_benchmark_result(
    repo_root: Path,
    benchmark_result: Dict[str, Any],
) -> Dict[str, Any]:
    search_config = SEARCH_BENCHMARKS.get(benchmark_result["benchmark"])
    if not search_config:
        return benchmark_result

    search_stats = probe_search_stats(
        repo_root,
        search_config["fen"],
        search_config["depth"],
    )
    benchmark_result.update(search_stats)

    mean_ns = benchmark_result["mean_ns"]
    if mean_ns > 0:
        benchmark_result["nps"] = int(
            benchmark_result["nodes"] * 1_000_000_000 / mean_ns
        )

    return benchmark_result


def probe_search_stats(
    repo_root: Path,
    fen: str,
    depth: int,
) -> Dict[str, Any]:
    command = ["cargo", "run", "--release", "--quiet"]
    uci_script = (
        f"position fen {fen}\n"
        f"go depth {depth}\n"
        "lasteval\n"
        "quit\n"
    )

    result = subprocess.run(
        command,
        cwd=repo_root,
        input=uci_script,
        capture_output=True,
        text=True,
        check=True,
    )

    search_stats = {
        "depth": depth,
        "score": None,
        "nodes": None,
        "bestmove": "0000",
    }

    for line in result.stdout.splitlines():
        if line.startswith("search_score "):
            search_stats["score"] = int(line.split()[1])
        elif line.startswith("nodes "):
            search_stats["nodes"] = int(line.split()[1])
        elif line.startswith("bestmove "):
            search_stats["bestmove"] = line.split()[1]

    if search_stats["score"] is None or search_stats["nodes"] is None:
        raise RuntimeError(f"Failed to parse search stats from engine output:\n{result.stdout}")

    return search_stats


def git_output(repo_root: Path, command: List[str]) -> str:
    return subprocess.run(
        command,
        cwd=repo_root,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()


def detect_cpu_model() -> str:
    if platform.system() == "Darwin":
        result = subprocess.run(
            ["sysctl", "-n", "machdep.cpu.brand_string"],
            check=False,
            capture_output=True,
            text=True,
        )
        if result.returncode == 0:
            model = result.stdout.strip()
            if model:
                return model

    if platform.system() == "Linux":
        cpuinfo = Path("/proc/cpuinfo")
        if cpuinfo.exists():
            for line in cpuinfo.read_text().splitlines():
                if line.startswith("model name"):
                    return line.split(":", 1)[1].strip()

    return platform.processor() or "unknown"


if __name__ == "__main__":
    raise SystemExit(main())
