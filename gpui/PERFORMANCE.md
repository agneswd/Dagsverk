# Performance comparison

M8 will measure release builds on the same machine and fixture database. Each latency result will report the median of at least five runs.

| Measurement | Electron | GPUI | Difference | Method |
|---|---:|---:|---:|---|
| Cold start to usable window | Pending | Pending | Pending | Process start to explicit render-ready signal |
| Idle CPU | Pending | Pending | Pending | 60-second sample after initialization |
| Idle resident memory | Pending | Pending | Pending | Full process-tree proportional set size where available |
| Peak startup memory | Pending | Pending | Pending | Full process-tree peak resident memory |
| Database load | Pending | Pending | Pending | Same copied fixture database |
| Month switch | Pending | Pending | Pending | Loaded state to rendered state |
| Day editor open | Pending | Pending | Pending | Input action to rendered editor |
| XLSX export | Pending | Pending | Pending | Same report request and output storage |
| ODS export | Pending | Pending | Pending | Same report request and output storage |
| Installed application size | Pending | Pending | Pending | Runtime files required for each preview |
