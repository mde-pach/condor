# Results

condor and vulture 2.16 run on the same real projects.

## Findings

Fewer findings means fewer false positives: framework code, model fields and
stub parameters are no longer reported.

--8<-- "harness/SUMMARY.md"

## Speed

![Scanning Django: condor versus vulture](assets/bench-dark.svg#only-dark)
![Scanning Django: condor versus vulture](assets/bench-light.svg#only-light)

![How many times faster condor is on each input](assets/speedup-dark.svg#only-dark)
![How many times faster condor is on each input](assets/speedup-light.svg#only-light)

--8<-- "harness/BENCHMARK.md:3:6"

Every run, option set and sample is in the
[full report](https://github.com/mde-pach/condor/blob/main/harness/REPORT.md).
