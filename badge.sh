#!/usr/bin/env sh
# Regenerates badges/{functions,lines,regions,branches}.svg from a fresh
# cargo-llvm-cov run. Commit the badges/ dir so the README renders offline.
set -eu

# --branch needs nightly's coverage instrumentation
cargo +nightly llvm-cov nextest --branch --json --output-path target/llvm-cov/coverage.json >/dev/null
python3 - <<'PY'
import json, os

totals = json.load(open("target/llvm-cov/coverage.json"))["data"][0]["totals"]
os.makedirs("badges", exist_ok=True)

def color(pct):
    if pct >= 100:
        return "#4c1"
    if pct >= 99:
        return "#97ca00"
    if pct >= 95:
        return "#a4a61d"
    if pct >= 90:
        return "#dfb317"
    return "#e05d44"

def badge(label, value, fill):
    lw = 10 + 7 * len(label)
    vw = 10 + 7 * len(value)
    w = lw + vw
    return f'''<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="20" role="img" aria-label="{label}: {value}">
  <linearGradient id="s" x2="0" y2="100%"><stop offset="0" stop-color="#bbb" stop-opacity=".1"/><stop offset="1" stop-opacity=".1"/></linearGradient>
  <clipPath id="r"><rect width="{w}" height="20" rx="3" fill="#fff"/></clipPath>
  <g clip-path="url(#r)">
    <rect width="{lw}" height="20" fill="#555"/>
    <rect x="{lw}" width="{vw}" height="20" fill="{fill}"/>
    <rect width="{w}" height="20" fill="url(#s)"/>
  </g>
  <g fill="#fff" text-anchor="middle" font-family="Verdana,Geneva,DejaVu Sans,sans-serif" font-size="11">
    <text x="{lw / 2}" y="14">{label}</text>
    <text x="{lw + vw / 2}" y="14">{value}</text>
  </g>
</svg>
'''

for metric in ("functions", "lines", "regions", "branches"):
    total = totals[metric]
    if total["count"] == 0:
        value, fill = "n/a", "#9f9f9f"
    else:
        pct = total["percent"]
        value = f"{pct:.2f}%".replace(".00%", "%")
        fill = color(pct)
    with open(f"badges/{metric}.svg", "w") as svg:
        svg.write(badge(metric, value, fill))
    print(f"badges/{metric}.svg -> {value}")
PY
