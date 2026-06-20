# Conformance timing harness — FreeMat (C++) vs FreeMat-rs

Measures per-test **execution time** for the shared conformance corpus on both
the original FreeMat (C++) interpreter and FreeMat-rs, then tabulates a
side-by-side speedup. It reuses the exact same `.m` test bodies the conformance
pass-rate already runs (`../data/tests`), so the two interpreters execute
identical code.

## Methodology

Both sides use the same criterion-style scheme so the numbers are comparable:

1. **Setup is excluded.** Building/initialising the interpreter, registering the
   standard library, and loading the directory's `.m` files happen once, *before*
   timing. Only the repeated test-function call is measured — never process
   startup or path loading.
2. **Budget-bounded repetition.** Each test body is invoked in a loop until a
   fixed wall-clock budget elapses (default 100 ms) or a rep cap is hit. Fast
   tests self-calibrate to many iterations, slow tests to few. The comparable
   figure is **mean nanoseconds per invocation** (`ns_per_iter`).
3. **Outcome classification matches the pass-rate harness.** The first call is
   classified `pass` / `fail` / `error` exactly as `time_test_file` and the
   FreeMat driver do (error/throw → `error`; empty or all-zero → `fail`;
   all-nonzero → `pass`). Tests that error on one side are not timed there and
   are reported as *uncompared* rather than silently dropped.
4. **Aggregation by geometric mean.** Per-directory and overall speedups use the
   geometric mean of per-test ratios — the correct average for ratios — plus the
   median. (The `total` columns are wall-clock sums and are dominated by a few
   slow linear-algebra tests; treat them as informational, not the headline.)

**Caveats.** Timer-overhead handling differs slightly (the FreeMat driver
amortises `tic`/`toc` with adaptive batching; the Rust side checks a cheap
`Instant` per iteration). Non-idempotent tests (file I/O, RNG) are repeated, so
their per-call time includes repeated side effects. These affect only a handful
of tests and not the geomean.

## Running

```sh
# 1. FreeMat-rs side  → rs_timings.csv  (release build for representative speed)
cargo build -p fm-conformance --release
./target/release/fm-conformance --timing --budget-ms 100 \
    > crates/fm-conformance/timing/rs_timings.csv

# 2. FreeMat (C++) side → freemat_timings.csv
#    FREEMAT_BIN / FREEMAT_TOOLBOX override the binary / toolbox locations.
crates/fm-conformance/timing/run_freemat_timing.sh 0.1 200000

# 3. Join + tabulate → comparison.csv and TIMING.md
python3 crates/fm-conformance/timing/tabulate_timing.py
```

Run the two timing passes **sequentially**, not concurrently — they are
CPU-sensitive and would skew each other.

## Files

| file | what it is |
|------|------------|
| `fm_timing_driver.m`     | FreeMat driver: times each test in a directory, writes a CSV chunk |
| `run_freemat_timing.sh`  | runs the driver once per covered directory, concatenates → `freemat_timings.csv` |
| `tabulate_timing.py`     | joins both CSVs on `(dir, name)`, emits `comparison.csv` + `TIMING.md` |
| `rs_timings.csv`         | FreeMat-rs per-test timings (`fm-conformance --timing`) |
| `freemat_timings.csv`    | FreeMat (C++) per-test timings |
| `comparison.csv`         | per-test join with both timings and the speedup |
| `TIMING.md`              | the report (per-directory summary + outlier tables) |

The CSV schema on both sides is `dir,name,outcome,reps,ns_per_iter`.
