# Benchmark

Machine: Apple M2, 8 cores, Darwin. vulture 2.16 on CPython 3.14.4; condor release build.
Mean wall-clock time over several runs after two warm-up runs (hyperfine), so
the file system cache is warm. Same corpus revisions as REPORT.md.
Load average when the run ended: 6.1 on 8 cores (busy machine: timings are slower than usual for both tools).

| Input | .py files | vulture (s) | condor (s) | Speed-up |
|---|---:|---:|---:|---:|
| one small file | 1 | 0.050 ± 0.002 | 0.003 ± 0.000 | 14× |
| requests | 36 | 0.174 ± 0.007 | 0.022 ± 0.000 | 8× |
| flask | 82 | 0.245 ± 0.002 | 0.031 ± 0.002 | 8× |
| rich | 190 | 0.448 ± 0.009 | 0.040 ± 0.001 | 11× |
| fastapi | 1283 | 1.397 ± 0.422 | 0.153 ± 0.013 | 9× |
| django | 2788 | 6.047 ± 0.109 | 0.399 ± 0.020 | 15× |
| saleor | 3524 | 6.249 ± 0.086 | 0.562 ± 0.006 | 11× |
