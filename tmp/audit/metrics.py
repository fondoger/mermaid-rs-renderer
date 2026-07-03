#!/usr/bin/env python3
"""Compute layout-quality metrics from an mmdr --dumpLayout JSON.

Metrics per diagram:
  - edge_crossings: number of proper intersections between segments of
    different edges (shared endpoints within EPS are not counted).
  - total_edge_length: sum of polyline lengths over all edges.
  - bbox_area: layout width * height.
  - label_edge_distances: for each edge that has a label_anchor, the minimum
    distance from the anchor (label center) to its own edge polyline.
    Summarised as max/mean plus the per-edge list.
  - label_label_overlaps: count of overlapping pairs of edge-label rectangles.
  - label_node_overlaps: count of (label, node) pairs whose rectangles overlap
    (hidden nodes are skipped).

Usage:
  python3 metrics.py layout.json [--name NAME]      # single, prints JSON
  python3 metrics.py a.json b.json ... --out out.json
"""

from __future__ import annotations

import argparse
import json
import math
import os
import sys

EPS = 1e-6


def seg_length(p, q):
    return math.hypot(q[0] - p[0], q[1] - p[1])


def polyline_length(points):
    return sum(seg_length(points[i], points[i + 1]) for i in range(len(points) - 1))


def orient(a, b, c):
    v = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    if v > EPS:
        return 1
    if v < -EPS:
        return -1
    return 0


def on_segment(a, b, p):
    return (
        min(a[0], b[0]) - EPS <= p[0] <= max(a[0], b[0]) + EPS
        and min(a[1], b[1]) - EPS <= p[1] <= max(a[1], b[1]) + EPS
    )


def points_close(p, q, tol=0.5):
    return abs(p[0] - q[0]) <= tol and abs(p[1] - q[1]) <= tol


def segments_intersect(p1, p2, p3, p4):
    """Proper or improper intersection, ignoring shared endpoints."""
    # Shared endpoint (edges meeting at the same node port) is not a crossing.
    for a in (p1, p2):
        for b in (p3, p4):
            if points_close(a, b):
                return False
    d1 = orient(p3, p4, p1)
    d2 = orient(p3, p4, p2)
    d3 = orient(p1, p2, p3)
    d4 = orient(p1, p2, p4)
    if ((d1 > 0 and d2 < 0) or (d1 < 0 and d2 > 0)) and (
        (d3 > 0 and d4 < 0) or (d3 < 0 and d4 > 0)
    ):
        return True
    if d1 == 0 and on_segment(p3, p4, p1):
        return True
    if d2 == 0 and on_segment(p3, p4, p2):
        return True
    if d3 == 0 and on_segment(p1, p2, p3):
        return True
    if d4 == 0 and on_segment(p1, p2, p4):
        return True
    return False


def point_segment_distance(p, a, b):
    ax, ay = a
    bx, by = b
    px, py = p
    dx, dy = bx - ax, by - ay
    denom = dx * dx + dy * dy
    if denom < EPS:
        return math.hypot(px - ax, py - ay)
    t = max(0.0, min(1.0, ((px - ax) * dx + (py - ay) * dy) / denom))
    return math.hypot(px - (ax + t * dx), py - (ay + t * dy))


def point_polyline_distance(p, points):
    return min(
        point_segment_distance(p, points[i], points[i + 1])
        for i in range(len(points) - 1)
    )


def rects_overlap(r1, r2):
    """r = (x_min, y_min, x_max, y_max); touching is not overlapping."""
    return (
        r1[0] < r2[2] - EPS
        and r2[0] < r1[2] - EPS
        and r1[1] < r2[3] - EPS
        and r2[1] < r1[3] - EPS
    )


def label_rect(edge):
    anchor = edge.get("label_anchor")
    if anchor is None:
        return None
    w = edge.get("label_width") or 0.0
    h = edge.get("label_height") or 0.0
    cx, cy = anchor
    return (cx - w / 2.0, cy - h / 2.0, cx + w / 2.0, cy + h / 2.0)


def edge_key(edge, idx):
    return f"{idx}:{edge['from']}->{edge['to']}"


def compute_metrics(dump):
    edges = dump.get("edges", [])
    nodes = dump.get("nodes", [])

    # Edge crossings.
    crossings = 0
    crossing_pairs = []
    for i in range(len(edges)):
        pts_i = edges[i]["points"]
        for j in range(i + 1, len(edges)):
            pts_j = edges[j]["points"]
            hit = False
            for si in range(len(pts_i) - 1):
                for sj in range(len(pts_j) - 1):
                    if segments_intersect(
                        pts_i[si], pts_i[si + 1], pts_j[sj], pts_j[sj + 1]
                    ):
                        crossings += 1
                        hit = True
                        break
                if hit:
                    break
            if hit:
                crossing_pairs.append(
                    [edge_key(edges[i], i), edge_key(edges[j], j)]
                )

    total_edge_length = sum(
        polyline_length(e["points"]) for e in edges if len(e["points"]) >= 2
    )

    bbox_area = float(dump.get("width", 0.0)) * float(dump.get("height", 0.0))

    # Label -> own-edge distance.
    label_edge = {}
    for idx, edge in enumerate(edges):
        anchor = edge.get("label_anchor")
        if anchor is None or len(edge["points"]) < 2:
            continue
        label_edge[edge_key(edge, idx)] = round(
            point_polyline_distance(anchor, edge["points"]), 2
        )

    # Label-label overlaps.
    rects = []
    for idx, edge in enumerate(edges):
        r = label_rect(edge)
        if r is not None:
            rects.append((edge_key(edge, idx), r))
    label_label_overlaps = 0
    label_label_pairs = []
    for i in range(len(rects)):
        for j in range(i + 1, len(rects)):
            if rects_overlap(rects[i][1], rects[j][1]):
                label_label_overlaps += 1
                label_label_pairs.append([rects[i][0], rects[j][0]])

    # Label-node overlaps.
    label_node_overlaps = 0
    label_node_pairs = []
    for key, r in rects:
        for node in nodes:
            if node.get("hidden"):
                continue
            nr = (
                node["x"],
                node["y"],
                node["x"] + node["width"],
                node["y"] + node["height"],
            )
            if rects_overlap(r, nr):
                label_node_overlaps += 1
                label_node_pairs.append([key, node["id"]])

    dists = list(label_edge.values())
    return {
        "edge_count": len(edges),
        "node_count": len(nodes),
        "edge_crossings": crossings,
        "crossing_pairs": crossing_pairs,
        "total_edge_length": round(total_edge_length, 1),
        "bbox_area": round(bbox_area, 1),
        "width": dump.get("width"),
        "height": dump.get("height"),
        "labels_with_anchor": len(dists),
        "label_edge_dist_max": round(max(dists), 2) if dists else None,
        "label_edge_dist_mean": round(sum(dists) / len(dists), 2) if dists else None,
        "label_edge_distances": label_edge,
        "label_label_overlaps": label_label_overlaps,
        "label_label_pairs": label_label_pairs,
        "label_node_overlaps": label_node_overlaps,
        "label_node_pairs": label_node_pairs,
    }


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("layouts", nargs="+", help="--dumpLayout JSON file(s)")
    ap.add_argument("--name", help="diagram name for single-file mode")
    ap.add_argument("--out", help="write combined results JSON here")
    args = ap.parse_args()

    results = {}
    for path in args.layouts:
        with open(path) as f:
            dump = json.load(f)
        name = args.name if (args.name and len(args.layouts) == 1) else (
            os.path.splitext(os.path.basename(path))[0].removesuffix(".layout")
        )
        results[name] = compute_metrics(dump)

    out = json.dumps(results, indent=2)
    if args.out:
        with open(args.out, "w") as f:
            f.write(out + "\n")
        print(f"wrote {args.out} ({len(results)} diagrams)")
    else:
        print(out)


if __name__ == "__main__":
    main()
