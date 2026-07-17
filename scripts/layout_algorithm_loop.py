#!/usr/bin/env python3
"""Deterministic placement-only hill-climb loop for layered flowchart engines.

The baseline and candidate receive identical Mermaid input and configuration.
Placement metrics come from --dumpLayeredLayout before routing. The full layout
is scored afterward only as a downstream regression veto.
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_FIXTURES = [
    ROOT / "tests/fixtures/flowchart/ports_arrow_pathing_regression.mmd",
    ROOT / "tests/fixtures/flowchart/complex.mmd",
    ROOT / "tests/fixtures/flowchart/cycles.mmd",
    ROOT / "tests/fixtures/flowchart/subgraph_direction.mmd",
    ROOT / "benches/fixtures/flowchart_ports_heavy.mmd",
    ROOT / "benches/fixtures/flowchart_long_edge_labels.mmd",
    ROOT / "benches/fixtures/flowchart_path_occlusion_maze.mmd",
]

STAGE_STRICT = (
    "node_overlaps",
    "adjacent_rank_crossings",
    "straight_line_crossings",
    "feedback_edges",
)
STAGE_RELATIVE = (
    "total_rank_span",
    "centerline_manhattan",
    "layout_area",
)
DOWNSTREAM_HARD = (
    "node_overlap_count",
    "edge_node_crossings",
    "endpoint_off_boundary_count",
    "subgraph_boundary_intrusion_pairs",
    "containment_foreign_node_count",
    "containment_member_escape_count",
    "label_overflow_count",
    "canvas_overflow_count",
)
DOWNSTREAM_STRICT = (
    "edge_crossings",
    "svg_edge_crossings",
    "edge_node_crossings",
    "arrow_path_intersections",
    "node_overlap_count",
    "label_overlap_count",
    "label_edge_overlap_count",
    "label_out_of_bounds_count",
    "edge_label_alignment_bad_count",
    "edge_label_path_gap_bad_count",
    "edge_label_owned_path_non_touch_ratio",
    "edge_label_owned_path_gap_bad_ratio",
    "edge_label_owned_anchor_offset_bad_ratio",
)


def load_layout_score_module():
    path = ROOT / "scripts/layout_score.py"
    spec = importlib.util.spec_from_file_location("layout_score", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def binary_needs_rebuild(binary: Path) -> bool:
    if not binary.exists():
        return True
    stamp = binary.stat().st_mtime
    candidates = [ROOT / "Cargo.toml", ROOT / "Cargo.lock"]
    candidates.extend((ROOT / "src").rglob("*.rs"))
    return any(path.exists() and path.stat().st_mtime > stamp for path in candidates)


def ensure_binary(binary: Path) -> None:
    if not binary_needs_rebuild(binary):
        return
    cargo = os.environ.get("CARGO") or shutil.which("cargo")
    if not cargo:
        fallback = Path.home() / ".cargo/bin/cargo"
        cargo = str(fallback) if fallback.exists() else "cargo"
    cmd = [cargo, "build", "--locked", "--profile", "release-fast"]
    result = subprocess.run(cmd, cwd=ROOT, text=True, capture_output=True)
    if result.returncode != 0:
        raise RuntimeError(result.stderr.strip() or "cargo build failed")


def fixture_key(path: Path) -> str:
    try:
        rel = path.resolve().relative_to(ROOT)
    except ValueError:
        rel = Path(path.name)
    return "__".join(rel.with_suffix("").parts)


def render_once(
    binary: Path,
    fixture: Path,
    engine: str,
    output_dir: Path,
    run_index: int,
    layout_score: Any,
) -> dict[str, Any]:
    key = fixture_key(fixture)
    prefix = output_dir / f"{key}--{engine}--{run_index}"
    svg = prefix.with_suffix(".svg")
    stage = Path(f"{prefix}.stage.json")
    layout = Path(f"{prefix}.layout.json")
    cmd = [
        str(binary),
        "-i",
        str(fixture),
        "-o",
        str(svg),
        "-e",
        "svg",
        "--fastText",
        "--layoutEngine",
        engine,
        "--dumpLayeredLayout",
        str(stage),
        "--dumpLayout",
        str(layout),
    ]
    result = subprocess.run(cmd, cwd=ROOT, text=True, capture_output=True)
    if result.returncode != 0:
        raise RuntimeError(
            f"render failed for {fixture} ({engine}): "
            f"{result.stderr.strip() or result.stdout.strip()}"
        )
    stage_data = json.loads(stage.read_text())
    layout_data, nodes, edges = layout_score.load_layout(layout)
    downstream = layout_score.compute_metrics(layout_data, nodes, edges)
    downstream["score"] = layout_score.weighted_score(downstream)
    return {
        "stage_path": stage,
        "stage_bytes": stage.read_bytes(),
        "stage": stage_data,
        "layout_path": layout,
        "svg_path": svg,
        "svg_bytes": svg.read_bytes(),
        "downstream": downstream,
    }


def increased(candidate: float, baseline: float, tolerance: float = 1e-6) -> bool:
    return candidate > baseline + tolerance


def relative_improvement(candidate: float, baseline: float) -> float:
    if baseline <= 1e-9:
        return 0.0
    return (baseline - candidate) / baseline


def compare_fixture(
    fixture: Path,
    baseline_runs: list[dict[str, Any]],
    candidate_runs: list[dict[str, Any]],
) -> dict[str, Any]:
    baseline = baseline_runs[0]
    candidate = candidate_runs[0]
    regressions: list[str] = []
    improvements: list[str] = []

    if baseline_runs[0]["stage_bytes"] != baseline_runs[1]["stage_bytes"]:
        regressions.append("baseline stage dump is nondeterministic")
    if candidate_runs[0]["stage_bytes"] != candidate_runs[1]["stage_bytes"]:
        regressions.append("candidate stage dump is nondeterministic")
    if baseline_runs[0]["svg_bytes"] != baseline_runs[1]["svg_bytes"]:
        regressions.append("baseline SVG is nondeterministic")
    if candidate_runs[0]["svg_bytes"] != candidate_runs[1]["svg_bytes"]:
        regressions.append("candidate SVG is nondeterministic")

    base_stage = baseline["stage"]["metrics"]
    cand_stage = candidate["stage"]["metrics"]
    for metric in STAGE_STRICT:
        if increased(float(cand_stage[metric]), float(base_stage[metric])):
            regressions.append(
                f"stage {metric}: {base_stage[metric]} -> {cand_stage[metric]}"
            )
        elif float(cand_stage[metric]) < float(base_stage[metric]):
            improvements.append(
                f"stage {metric}: {base_stage[metric]} -> {cand_stage[metric]}"
            )
    for metric in STAGE_RELATIVE:
        delta = relative_improvement(float(cand_stage[metric]), float(base_stage[metric]))
        if delta >= 0.01:
            improvements.append(
                f"stage {metric}: {base_stage[metric]:.3f} -> "
                f"{cand_stage[metric]:.3f} ({delta * 100:.2f}%)"
            )

    base_downstream = baseline["downstream"]
    cand_downstream = candidate["downstream"]
    for metric in DOWNSTREAM_HARD:
        if increased(float(cand_downstream.get(metric, 0)), float(base_downstream.get(metric, 0))):
            regressions.append(
                f"downstream hard {metric}: {base_downstream.get(metric, 0)} -> "
                f"{cand_downstream.get(metric, 0)}"
            )
    for metric in DOWNSTREAM_STRICT:
        if increased(float(cand_downstream.get(metric, 0)), float(base_downstream.get(metric, 0))):
            regressions.append(
                f"downstream strict {metric}: {base_downstream.get(metric, 0)} -> "
                f"{cand_downstream.get(metric, 0)}"
            )

    changed = baseline["svg_bytes"] != candidate["svg_bytes"]
    selected = candidate["stage"].get("selected_engine", candidate["stage"].get("engine"))
    return {
        "fixture": str(fixture.relative_to(ROOT) if fixture.is_relative_to(ROOT) else fixture),
        "selected_engine": selected,
        "changed": changed,
        "accepted": not regressions,
        "regressions": regressions,
        "improvements": improvements,
        "baseline_stage": base_stage,
        "candidate_stage": cand_stage,
        "baseline_downstream_score": base_downstream.get("score", 0),
        "candidate_downstream_score": cand_downstream.get("score", 0),
    }


def collect_fixtures(paths: list[str], patterns: list[str]) -> list[Path]:
    fixtures = [Path(path) for path in paths] if paths else list(DEFAULT_FIXTURES)
    resolved = [(path if path.is_absolute() else ROOT / path).resolve() for path in fixtures]
    if patterns:
        regexes = [re.compile(pattern) for pattern in patterns]
        resolved = [
            path
            for path in resolved
            if any(regex.search(str(path.relative_to(ROOT))) for regex in regexes)
        ]
    missing = [path for path in resolved if not path.exists()]
    if missing:
        raise FileNotFoundError(f"missing fixtures: {', '.join(map(str, missing))}")
    return sorted(dict.fromkeys(resolved))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("fixtures", nargs="*", help="fixture paths; defaults to the visual suite")
    parser.add_argument("--pattern", action="append", default=[], help="regex fixture filter")
    parser.add_argument(
        "--bin", default=str(ROOT / "target/release-fast/mmdr"), help="mmdr binary"
    )
    parser.add_argument("--baseline-engine", default="current")
    parser.add_argument("--candidate-engine", default="dagre")
    parser.add_argument("--out-dir", default=str(ROOT / "tmp/layout-algorithm-loop"))
    parser.add_argument("--output-json", default="")
    parser.add_argument("--allow-neutral", action="store_true")
    args = parser.parse_args()

    binary = Path(args.bin)
    ensure_binary(binary)
    fixtures = collect_fixtures(args.fixtures, args.pattern)
    if not fixtures:
        print("No fixtures selected", file=sys.stderr)
        return 2

    output_dir = Path(args.out_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    layout_score = load_layout_score_module()
    rows = []
    for index, fixture in enumerate(fixtures, 1):
        baseline_runs = [
            render_once(binary, fixture, args.baseline_engine, output_dir, run, layout_score)
            for run in (1, 2)
        ]
        candidate_runs = [
            render_once(binary, fixture, args.candidate_engine, output_dir, run, layout_score)
            for run in (1, 2)
        ]
        row = compare_fixture(fixture, baseline_runs, candidate_runs)
        rows.append(row)
        print(
            f"[{index}/{len(fixtures)}] {row['fixture']}: "
            f"selected={row['selected_engine']} changed={row['changed']} "
            f"regressions={len(row['regressions'])} improvements={len(row['improvements'])}"
        )

    regressions = [
        (row["fixture"], item) for row in rows for item in row["regressions"]
    ]
    improvements = [
        (row["fixture"], item) for row in rows for item in row["improvements"]
    ]
    changed = sum(bool(row["changed"]) for row in rows)
    if args.candidate_engine == "auto":
        selected = sum(row["selected_engine"] != args.baseline_engine for row in rows)
    else:
        selected = sum(row["selected_engine"] == args.candidate_engine for row in rows)
    accepted = not regressions and (bool(improvements) or args.allow_neutral)
    report = {
        "baseline_engine": args.baseline_engine,
        "candidate_engine": args.candidate_engine,
        "accepted": accepted,
        "fixture_count": len(rows),
        "changed_fixture_count": changed,
        "candidate_selected_count": selected,
        "regression_count": len(regressions),
        "improvement_count": len(improvements),
        "rows": rows,
    }
    output_json = Path(args.output_json) if args.output_json else output_dir / "report.json"
    output_json.write_text(json.dumps(report, indent=2) + "\n")

    if regressions:
        print("\nRegressions:")
        for fixture, item in regressions:
            print(f"  {fixture}: {item}")
    if improvements:
        print("\nImprovements:")
        for fixture, item in improvements:
            print(f"  {fixture}: {item}")
    print(
        f"\naccepted={accepted} changed={changed}/{len(rows)} "
        f"selected={selected}/{len(rows)} report={output_json}"
    )
    return 0 if accepted else 1


if __name__ == "__main__":
    raise SystemExit(main())
