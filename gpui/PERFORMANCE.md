# Performance comparison

M8 measures release builds on the same machine and fixture database. Each latency result will report the median of at least five runs.

The first Linux idle sample used the deterministic visual fixture on 2026-08-18. Electron ran with its X11 backend in Xvfb. GPUI ran with its native Wayland backend under Niri. CPU was sampled for 10 seconds after initialization. Memory uses proportional set size (PSS) from `/proc/<pid>/smaps_rollup` across the full process tree.

| Measurement | Electron | GPUI | Difference | Method |
|---|---:|---:|---:|---|
| Cold start to usable window | Pending | Pending | Pending | Process start to explicit render-ready signal |
| Idle CPU | 0.90% | 0.00% | 0.90 percentage points lower | 10-second process-tree sample after initialization; repeat with a 60-second final sample |
| Idle resident memory | 387.2 MiB | 47.8 MiB | 339.4 MiB lower (87.7%) | Full process-tree PSS; Electron 7 processes, GPUI 1 process |
| Peak startup memory | Pending | Pending | Pending | Full process-tree peak resident memory |
| Database load | Pending | Pending | Pending | Same copied fixture database |
| Month switch | Pending | Pending | Pending | Loaded state to rendered state |
| Day editor open | Pending | Pending | Pending | Input action to rendered editor |
| XLSX export | Pending | Pending | Pending | Same report request and output storage |
| ODS export | Pending | Pending | Pending | Same report request and output storage |
| Installed application size | 452.9 MiB | 32.8 MiB | 420.1 MiB lower (92.8%) | Electron `dist/linux-unpacked`; GPUI release binary with embedded assets |

These values are provisional until the final 60-second run uses the same visible-window state and records five samples. The final report will also include summed resident set size (RSS) for tools that do not expose PSS.
