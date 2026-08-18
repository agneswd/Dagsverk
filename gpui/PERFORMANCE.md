# Performance comparison

The resource comparison uses the deterministic August 2026 fixture. Each application receives a copied database in a temporary directory. The benchmark never opens the production database.

## Linux idle resource use

The table reports the median of five 60-second samples after a five-second warm-up. The harness reads each complete process tree once per second. PSS means proportional set size. RSS means resident set size.

| Measurement | Electron | GPUI | GPUI difference |
|---|---:|---:|---:|
| Idle CPU | 0.117% | below 0.017% sampling resolution | at least 0.100 percentage points lower |
| Mean PSS | 322.9 MiB | 58.7 MiB | 264.2 MiB lower (81.8%) |
| Peak sampled PSS | 354.4 MiB | 58.7 MiB | 295.7 MiB lower (83.4%) |
| Mean summed RSS | 738.5 MiB | 74.0 MiB | 664.4 MiB lower (90.0%) |
| Peak sampled summed RSS | 775.5 MiB | 74.0 MiB | 701.5 MiB lower (90.5%) |
| Process count | 8 | 1 | 7 fewer |
| Thread count | 74 | 24 | 50 fewer (67.6%) |
| Open file descriptors | 255 | 26 | 229 fewer (89.8%) |
| Process-tree ready probe | 204 ms | 23 ms | 181 ms lower (88.7%) |
| Installed application size | 453.0 MiB | 33.2 MiB | 419.8 MiB lower (92.7%) |

The ready probe ends when the expected process tree exists. It does not claim that the first application frame is usable.

## Method

- Host: AMD Ryzen 7 7800X3D, 16 threads, 30 GiB RAM.
- Kernel: Linux 7.1.6-1-cachyos x86_64.
- Electron: packaged release renderer under Electron 43.4.0, X11 in Xvfb, GPU disabled.
- GPUI: release binary, GPUI 0.2.2, native Wayland under Niri 26.04.
- Window state: ledger, 1366 x 820, 100% interface scale, fixed date 2026-08-18.
- Raw samples: `resource-comparison.csv`.

Run the comparison with:

```bash
cd gpui
tools/performance/compare-resources.sh resource-comparison.csv 5 60 5
```

The Electron benchmark mode disables update checks and loads the production bundle. It does not change normal application startup.

## Interaction latency

| Measurement | Electron | GPUI | Status |
|---|---:|---:|---|
| Cold start to usable window | Pending | Pending | Requires the same explicit render-ready signal in both apps |
| Database load | Pending | Pending | Requires matching instrumentation |
| Month switch | Pending | Pending | Requires matching rendered-state instrumentation |
| Day editor open | Pending | Pending | Requires matching rendered-state instrumentation |
| XLSX export | Pending | Pending | Requires the same report request and storage |
| ODS export | Pending | Pending | Requires the same report request and storage |
