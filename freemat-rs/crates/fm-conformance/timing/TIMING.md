# Conformance timing: FreeMat (C++) vs FreeMat-rs

Per-test execution time on the shared conformance corpus. Each test's body is invoked repeatedly under a fixed wall-clock budget (interpreter startup excluded) and the mean per-invocation time is reported; see `timing/README.md` for the methodology.

**Speedup** = FreeMat (C++) time ÷ FreeMat-rs time (>1 ⇒ Rust is faster). Aggregates are the geometric mean over the 659 tests that timed cleanly on both sides (18 uncompared — errored/missing on one side).

## Per-directory summary

| directory | tests | compared | geomean ×  | median × | FreeMat total | rs total |
|-----------|------:|---------:|----------:|---------:|--------------:|---------:|
| array | 68 | 68 | 14.3 | 21.1 | 2.05 s | 1.15 s |
| binary | 3 | 3 | 37.6 | 37.7 | 227.27 µs | 6.04 µs |
| constants | 1 | 1 | 38.4 | 38.4 | 99.07 µs | 2.58 µs |
| elementary | 7 | 7 | 111.1 | 79.1 | 2.75 ms | 11.24 µs |
| flow | 22 | 22 | 32.6 | 29.3 | 1.20 ms | 45.08 µs |
| freemat | 11 | 11 | 36.4 | 34.8 | 3.06 ms | 65.02 µs |
| functions | 9 | 8 | 46.0 | 46.5 | 835.97 µs | 18.55 µs |
| handle | 3 | 0 | — | — | 0 ns | 0 ns |
| inspection | 20 | 20 | 79.7 | 51.5 | 6.61 ms | 42.17 µs |
| io | 6 | 4 | 6.3 | 3.6 | 624.35 µs | 168.26 µs |
| operators | 66 | 65 | 16.3 | 13.9 | 63.00 s | 1.96 s |
| random | 1 | 1 | 31.7 | 31.7 | 54.76 µs | 1.73 µs |
| signal | 1 | 1 | 11.5 | 11.5 | 165.62 µs | 14.45 µs |
| sparse | 37 | 36 | 10.4 | 13.3 | 3.30 s | 822.18 ms |
| string | 3 | 3 | 52.8 | 57.9 | 182.39 µs | 3.31 µs |
| suite | 337 | 328 | 19.9 | 21.7 | 97.84 s | 14.80 s |
| transforms | 34 | 33 | 6.1 | 4.7 | 28.73 s | 10.10 s |
| typecast | 5 | 5 | 17.1 | 23.9 | 591.27 ms | 434.32 ms |
| variables | 43 | 43 | 27.3 | 26.1 | 4.71 ms | 179.59 µs |
| **overall** | **677** | **659** | **19.3** | **21.5** | **195.53 s** | **29.26 s** |

## Largest FreeMat-rs speedups

| test | FreeMat | rs | speedup × |
|------|--------:|---:|----------:|
| suite/test_test5 | 3.43 ms | 845 ns | 4061.2 |
| suite/test_clear1 | 3.33 ms | 947 ns | 3518.4 |
| suite/test_clear2 | 3.37 ms | 1.15 µs | 2915.6 |
| elementary/test_test5 | 2.04 ms | 883 ns | 2308.3 |
| inspection/test_clear1 | 2.08 ms | 960 ns | 2166.9 |
| inspection/test_clear2 | 2.04 ms | 1.16 µs | 1757.4 |
| suite/test_eval3 | 3.37 ms | 1.95 µs | 1723.9 |
| freemat/test_eval3 | 2.04 ms | 2.01 µs | 1014.8 |
| suite/test_fitfun1 | 3.43 ms | 4.70 µs | 731.3 |
| suite/test_isfield1 | 772.73 µs | 1.69 µs | 457.9 |
| inspection/test_isfield1 | 748.25 µs | 1.68 µs | 445.4 |
| suite/test_isfloat1 | 238.64 µs | 628 ns | 379.9 |
| suite/test_isinteger1 | 283.85 µs | 774 ns | 366.7 |
| array/test_isfloat1 | 232.56 µs | 638 ns | 364.6 |
| array/test_isinteger1 | 276.04 µs | 771 ns | 357.8 |

## Smallest speedups (where rs is relatively weakest)

| test | FreeMat | rs | speedup × |
|------|--------:|---:|----------:|
| array/test_sparse69 | 2.43 ms | 7.77 ms | 0.3 |
| suite/test_sparse69 | 2.43 ms | 7.75 ms | 0.3 |
| transforms/test_qr9 | 550.00 ms | 1.05 s | 0.5 |
| suite/test_eig5 | 1.51 s | 2.00 s | 0.8 |
| transforms/test_eig5 | 1.51 s | 1.56 s | 1.0 |
| suite/test_qr10 | 590.00 ms | 577.47 ms | 1.0 |
| suite/test_qr9 | 548.00 ms | 458.96 ms | 1.2 |
| typecast/test_sparse58 | 591.00 ms | 434.31 ms | 1.4 |
| suite/test_sparse58 | 580.00 ms | 423.30 ms | 1.4 |
| array/test_sparse71 | 168.00 ms | 122.10 ms | 1.4 |
| suite/test_sparse57 | 202.00 ms | 145.61 ms | 1.4 |
| sparse/test_sparse57 | 201.00 ms | 144.58 ms | 1.4 |
| transforms/test_qr10 | 587.00 ms | 388.56 ms | 1.5 |
| sparse/test_sparse26 | 239.00 ms | 155.64 ms | 1.5 |
| array/test_sparse72 | 188.00 ms | 122.09 ms | 1.5 |
