#!/usr/bin/env python3

import argparse
import json
import platform
import socket
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from uuid import uuid4


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


def run_bench(repo_root: Path, bench_name: str, bench_filter: str | None) -> None:
    command = ["cargo", "bench", "--bench", bench_name]
    if bench_filter:
        command.extend(["--", bench_filter])

    subprocess.run(command, cwd=repo_root, check=True)


def collect_run_record(
    repo_root: Path,
    machine_label: str,
    bench_filter: str | None,
) -> dict:
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
            {
                "benchmark": benchmark_name,
                "mean_ns": mean["point_estimate"],
                "mean_ci_low_ns": mean["confidence_interval"]["lower_bound"],
                "mean_ci_high_ns": mean["confidence_interval"]["upper_bound"],
                "median_ns": median["point_estimate"],
                "median_ci_low_ns": median["confidence_interval"]["lower_bound"],
                "median_ci_high_ns": median["confidence_interval"]["upper_bound"],
            }
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


def append_run_record(path: Path, run_record: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a") as f:
        f.write(json.dumps(run_record, sort_keys=True))
        f.write("\n")


def git_output(repo_root: Path, command: list[str]) -> str:
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
