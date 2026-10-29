# Benchmark

Machine: Apple M2, 8 cores, Darwin. vulture 2.16 on CPython 3.14.4; condor release build.
Mean wall-clock time over several runs after two warm-up runs (hyperfine), so
the file system cache is warm. Same corpus revisions as REPORT.md.
Load average when the run ended: about 23 on 8 cores (busy machine: timings are slower than usual for both tools).

| Input | .py files | vulture (s) | condor (s) | Speed-up |
|---|---:|---:|---:|---:|
| one small file | 1 | 0.051 ± 0.001 | 0.004 ± 0.000 | 13× |
| requests | 36 | 0.173 ± 0.001 | 0.022 ± 0.001 | 8× |
| flask | 82 | 0.248 ± 0.003 | 0.030 ± 0.001 | 8× |
| rich | 190 | 0.456 ± 0.014 | 0.045 ± 0.001 | 10× |
| fastapi | 1283 | 1.065 ± 0.011 | 0.124 ± 0.001 | 9× |
| django | 2788 | 6.733 ± 1.575 | 0.368 ± 0.003 | 18× |
| saleor | 3524 | 11.604 ± 3.022 | 0.624 ± 0.056 | 19× |
