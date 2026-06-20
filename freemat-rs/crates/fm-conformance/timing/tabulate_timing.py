#!/usr/bin/env python3
"""Join FreeMat (C++) and FreeMat-rs per-test timings and tabulate the result.

Reads the two CSVs produced by the timing harnesses
(`dir,name,outcome,reps,ns_per_iter`), joins them on (dir, name), and writes:

  * comparison.csv  — one row per test with both timings and the speedup, and
  * TIMING.md       — a per-directory + overall report (Markdown).

The headline figure is the per-test **speedup** = freemat_ns / rs_ns (>1 means
FreeMat-rs is faster). Aggregates use the geometric mean, the correct average
for ratios. Only tests that produced a real timing on *both* sides (outcome not
`error`, reps > 0) are included in speedup stats; the rest are reported as
"uncompared" so nothing is silently dropped.
"""
import csv
import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))


def load(path):
    rows = {}
    with open(path) as f:
        for r in csv.DictReader(f):
            rows[(r["dir"], r["name"])] = {
                "outcome": r["outcome"],
                "reps": int(r["reps"]),
                "ns": float(r["ns_per_iter"]),
            }
    return rows


def geomean(xs):
    return math.exp(sum(math.log(x) for x in xs) / len(xs)) if xs else float("nan")


def median(xs):
    if not xs:
        return float("nan")
    s = sorted(xs)
    n = len(s)
    return s[n // 2] if n % 2 else 0.5 * (s[n // 2 - 1] + s[n // 2])


def fmt_ns(ns):
    if ns >= 1e9:
        return f"{ns/1e9:.2f} s"
    if ns >= 1e6:
        return f"{ns/1e6:.2f} ms"
    if ns >= 1e3:
        return f"{ns/1e3:.2f} µs"
    return f"{ns:.0f} ns"


def main():
    fm_path = sys.argv[1] if len(sys.argv) > 1 else os.path.join(HERE, "freemat_timings.csv")
    rs_path = sys.argv[2] if len(sys.argv) > 2 else os.path.join(HERE, "rs_timings.csv")
    fm, rs = load(fm_path), load(rs_path)

    keys = sorted(set(fm) | set(rs))
    rows = []  # comparison rows (all keys, joined)
    for k in keys:
        a, b = fm.get(k), rs.get(k)
        comparable = (
            a and b
            and a["outcome"] != "error" and b["outcome"] != "error"
            and a["reps"] > 0 and b["reps"] > 0 and a["ns"] > 0 and b["ns"] > 0
        )
        rows.append({
            "dir": k[0], "name": k[1],
            "fm_outcome": a["outcome"] if a else "missing",
            "rs_outcome": b["outcome"] if b else "missing",
            "fm_ns": a["ns"] if a else "",
            "rs_ns": b["ns"] if b else "",
            "speedup": (a["ns"] / b["ns"]) if comparable else "",
        })

    # --- comparison.csv ---
    with open(os.path.join(HERE, "comparison.csv"), "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=["dir", "name", "fm_outcome", "rs_outcome", "fm_ns", "rs_ns", "speedup"])
        w.writeheader()
        for r in rows:
            w.writerow(r)

    # --- per-directory aggregates ---
    dirs = sorted({r["dir"] for r in rows})
    per_dir = []
    all_speedups = []
    for d in dirs:
        drows = [r for r in rows if r["dir"] == d]
        sp = [r["speedup"] for r in drows if r["speedup"] != ""]
        all_speedups += sp
        fm_tot = sum(r["fm_ns"] for r in drows if r["speedup"] != "")
        rs_tot = sum(r["rs_ns"] for r in drows if r["speedup"] != "")
        per_dir.append({
            "dir": d,
            "n": len(drows),
            "cmp": len(sp),
            "geomean": geomean(sp),
            "median": median(sp),
            "fm_tot": fm_tot,
            "rs_tot": rs_tot,
        })

    n_total = len(rows)
    n_cmp = len(all_speedups)
    n_uncmp = n_total - n_cmp

    # --- TIMING.md ---
    out = []
    out.append("# Conformance timing: FreeMat (C++) vs FreeMat-rs\n")
    out.append(
        "Per-test execution time on the shared conformance corpus. Each test's "
        "body is invoked repeatedly under a fixed wall-clock budget "
        "(interpreter startup excluded) and the mean per-invocation time is "
        "reported; see `timing/README.md` for the methodology.\n"
    )
    out.append(
        f"**Speedup** = FreeMat (C++) time ÷ FreeMat-rs time (>1 ⇒ Rust is "
        f"faster). Aggregates are the geometric mean over the "
        f"{n_cmp} tests that timed cleanly on both sides "
        f"({n_uncmp} uncompared — errored/missing on one side).\n"
    )

    out.append("## Per-directory summary\n")
    out.append("| directory | tests | compared | geomean ×  | median × | FreeMat total | rs total |")
    out.append("|-----------|------:|---------:|----------:|---------:|--------------:|---------:|")
    for p in per_dir:
        gm = f"{p['geomean']:.1f}" if p["cmp"] else "—"
        md = f"{p['median']:.1f}" if p["cmp"] else "—"
        out.append(
            f"| {p['dir']} | {p['n']} | {p['cmp']} | {gm} | {md} | "
            f"{fmt_ns(p['fm_tot'])} | {fmt_ns(p['rs_tot'])} |"
        )
    out.append(
        f"| **overall** | **{n_total}** | **{n_cmp}** | "
        f"**{geomean(all_speedups):.1f}** | **{median(all_speedups):.1f}** | "
        f"**{fmt_ns(sum(p['fm_tot'] for p in per_dir))}** | "
        f"**{fmt_ns(sum(p['rs_tot'] for p in per_dir))}** |"
    )

    cmp_rows = [r for r in rows if r["speedup"] != ""]
    fastest = sorted(cmp_rows, key=lambda r: -r["speedup"])[:15]
    slowest = sorted(cmp_rows, key=lambda r: r["speedup"])[:15]

    def detail_table(title, rs):
        out.append(f"\n## {title}\n")
        out.append("| test | FreeMat | rs | speedup × |")
        out.append("|------|--------:|---:|----------:|")
        for r in rs:
            out.append(
                f"| {r['dir']}/{r['name']} | {fmt_ns(r['fm_ns'])} | "
                f"{fmt_ns(r['rs_ns'])} | {r['speedup']:.1f} |"
            )

    detail_table("Largest FreeMat-rs speedups", fastest)
    detail_table("Smallest speedups (where rs is relatively weakest)", slowest)

    with open(os.path.join(HERE, "TIMING.md"), "w") as f:
        f.write("\n".join(out) + "\n")

    print(f"compared {n_cmp}/{n_total} tests; "
          f"overall geomean speedup {geomean(all_speedups):.1f}×, "
          f"median {median(all_speedups):.1f}×")
    print(f"wrote comparison.csv and TIMING.md in {HERE}")


if __name__ == "__main__":
    main()
