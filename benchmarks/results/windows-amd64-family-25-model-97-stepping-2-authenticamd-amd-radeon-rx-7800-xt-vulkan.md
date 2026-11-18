# Benchmarks

windows, AMD64 Family 25 Model 97 Stepping 2, AuthenticAMD with 16 threads, AMD Radeon RX 7800 XT on vulkan, driver AMD proprietary driver 26.10.1 (LLPC). Commit 7630e2e.

The time from an edit until its result is computed, the median of many edits. Below each table, where it goes: setting the new value (which hashes it) and planning are the same for every backend, `flush` is the executor, then waiting for the device. The device column is the time the GPU spends on compute passes, by its timestamps.

## nurbs

NURBS Curve, the control points move.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 38 µs | 38 µs | 192 µs | 5.0× slower | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 16 × 32 | 600 µs | 108 µs | 207 µs | 1.9× slower | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 32 | 9.58 ms | 1.14 ms | 237 µs | 4.8× faster | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 4096 × 32 | 158.21 ms | 19.86 ms | 1.21 ms | 16.4× faster | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 1 × 128 | 161 µs | 163 µs | 243 µs | 1.5× slower | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 16 × 128 | 2.57 ms | 318 µs | 263 µs | 1.2× faster | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 128 | 42.20 ms | 5.12 ms | 491 µs | 10.4× faster | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 1024 × 128 | 175.39 ms | 22.73 ms | 1.29 ms | 17.7× faster | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device |
|---|---|---:|---:|---:|---|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 0.4 µs | 38 µs | kernels 37 µs, store 0.1 µs, rest 0.3 µs | 0.0 µs | – |
| 1 × 32 | cpu | 0.2 µs | 0.4 µs | 38 µs | kernels 38 µs, store 0.2 µs, rest 0.3 µs | 0.0 µs | – |
| 1 × 32 | gpu | 0.4 µs | 0.9 µs | 52 µs | upload 4.9 µs, record 3.0 µs, submit 40 µs, download 0.1 µs, rest 3.8 µs | 137 µs | 72 µs |
| 16 × 32 | cpu-1 | 0.5 µs | 0.5 µs | 599 µs | kernels 598 µs, store 0.8 µs, rest 0.3 µs | 0.0 µs | – |
| 16 × 32 | cpu | 0.9 µs | 1.2 µs | 106 µs | kernels 104 µs, store 1.2 µs, rest 0.5 µs | 0.0 µs | – |
| 16 × 32 | gpu | 0.7 µs | 1.1 µs | 53 µs | upload 5.5 µs, record 3.1 µs, submit 40 µs, download 0.1 µs, rest 5.0 µs | 148 µs | 79 µs |
| 256 × 32 | cpu-1 | 6.1 µs | 3.1 µs | 9.57 ms | kernels 9.56 ms, store 18 µs, rest 0.5 µs | 0.0 µs | – |
| 256 × 32 | cpu | 6.8 µs | 4.9 µs | 1.13 ms | kernels 1.11 ms, store 14 µs, rest 0.9 µs | 0.0 µs | – |
| 256 × 32 | gpu | 6.0 µs | 3.1 µs | 68 µs | upload 17 µs, record 3.0 µs, submit 39 µs, download 0.1 µs, rest 9.4 µs | 158 µs | 89 µs |
| 4096 × 32 | cpu-1 | 315 µs | 132 µs | 157.83 ms | kernels 157.34 ms, store 517 µs, rest 4.0 µs | 0.0 µs | – |
| 4096 × 32 | cpu | 310 µs | 135 µs | 19.42 ms | kernels 18.88 ms, store 559 µs, rest 2.7 µs | 0.0 µs | – |
| 4096 × 32 | gpu | 297 µs | 134 µs | 258 µs | upload 137 µs, record 3.6 µs, submit 52 µs, download 0.1 µs, rest 66 µs | 518 µs | 348 µs |
| 1 × 128 | cpu-1 | 0.2 µs | 0.4 µs | 160 µs | kernels 159 µs, store 0.3 µs, rest 0.3 µs | 0.0 µs | – |
| 1 × 128 | cpu | 0.2 µs | 0.4 µs | 162 µs | kernels 160 µs, store 2.4 µs, rest 0.2 µs | 0.0 µs | – |
| 1 × 128 | gpu | 0.4 µs | 0.9 µs | 49 µs | upload 4.9 µs, record 2.9 µs, submit 38 µs, download 0.1 µs, rest 3.7 µs | 192 µs | 128 µs |
| 16 × 128 | cpu-1 | 1.8 µs | 1.1 µs | 2.57 ms | kernels 2.57 ms, store 3.1 µs, rest 0.3 µs | 0.0 µs | – |
| 16 × 128 | cpu | 2.1 µs | 2.0 µs | 314 µs | kernels 309 µs, store 4.1 µs, rest 0.6 µs | 0.0 µs | – |
| 16 × 128 | gpu | 1.9 µs | 1.5 µs | 54 µs | upload 6.7 µs, record 3.0 µs, submit 39 µs, download 0.1 µs, rest 5.2 µs | 204 µs | 131 µs |
| 256 × 128 | cpu-1 | 25 µs | 16 µs | 42.16 ms | kernels 42.01 ms, store 128 µs, rest 1.0 µs | 0.0 µs | – |
| 256 × 128 | cpu | 27 µs | 12 µs | 5.08 ms | kernels 4.94 ms, store 121 µs, rest 1.1 µs | 0.0 µs | – |
| 256 × 128 | gpu | 23 µs | 13 µs | 98 µs | upload 40 µs, record 3.1 µs, submit 40 µs, download 0.1 µs, rest 14 µs | 357 µs | 185 µs |
| 1024 × 128 | cpu-1 | 345 µs | 135 µs | 174.98 ms | kernels 174.37 ms, store 600 µs, rest 5.4 µs | 0.1 µs | – |
| 1024 × 128 | cpu | 339 µs | 127 µs | 22.26 ms | kernels 21.61 ms, store 578 µs, rest 4.1 µs | 0.0 µs | – |
| 1024 × 128 | gpu | 335 µs | 84 µs | 241 µs | upload 138 µs, record 4.1 µs, submit 52 µs, download 0.1 µs, rest 44 µs | 630 µs | 471 µs |

## translate

Translate Curve of the curves, the offset changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 1.2 µs | 4.5 µs | 90 µs | 20.0× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 32 | 3.5 µs | 13 µs | 93 µs | 7.2× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 32 | 42 µs | 97 µs | 107 µs | 1.1× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 4096 × 32 | 2.51 ms | 2.32 ms | 201 µs | 11.6× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 1 × 128 | 1.5 µs | 8.0 µs | 102 µs | 12.8× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 128 | 12 µs | 26 µs | 90 µs | 3.4× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 128 | 671 µs | 665 µs | 123 µs | 5.4× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 1024 × 128 | 2.54 ms | 2.19 ms | 194 µs | 11.3× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device |
|---|---|---:|---:|---:|---|---:|---:|
| 1 × 32 | cpu-1 | 0.1 µs | 0.5 µs | 0.6 µs | kernels 0.2 µs, store 0.1 µs, rest 0.2 µs | 0.0 µs | – |
| 1 × 32 | cpu | 0.2 µs | 0.7 µs | 3.6 µs | kernels 3.0 µs, store 0.2 µs, rest 0.4 µs | 0.0 µs | – |
| 1 × 32 | gpu | 0.3 µs | 0.9 µs | 37 µs | upload 3.0 µs, record 1.7 µs, submit 31 µs, rest 1.7 µs | 52 µs | 1.7 µs |
| 16 × 32 | cpu-1 | 0.1 µs | 0.4 µs | 3.0 µs | kernels 2.1 µs, store 0.7 µs, rest 0.2 µs | 0.0 µs | – |
| 16 × 32 | cpu | 0.2 µs | 0.7 µs | 12 µs | kernels 10 µs, store 0.9 µs, rest 0.4 µs | 0.0 µs | – |
| 16 × 32 | gpu | 0.3 µs | 0.9 µs | 38 µs | upload 3.1 µs, record 1.7 µs, submit 31 µs, rest 1.7 µs | 52 µs | 2.3 µs |
| 256 × 32 | cpu-1 | 0.2 µs | 0.6 µs | 41 µs | kernels 30 µs, store 10 µs, rest 0.4 µs | 0.0 µs | – |
| 256 × 32 | cpu | 0.3 µs | 0.8 µs | 96 µs | kernels 81 µs, store 13 µs, rest 0.6 µs | 0.0 µs | – |
| 256 × 32 | gpu | 0.3 µs | 0.9 µs | 39 µs | upload 3.1 µs, record 1.7 µs, submit 32 µs, rest 1.8 µs | 64 µs | 6.3 µs |
| 4096 × 32 | cpu-1 | 0.5 µs | 1.4 µs | 2.51 ms | kernels 1.95 ms, store 554 µs, rest 1.6 µs | 0.0 µs | – |
| 4096 × 32 | cpu | 1.2 µs | 2.8 µs | 2.32 ms | kernels 1.75 ms, store 550 µs, rest 3.0 µs | 0.0 µs | – |
| 4096 × 32 | gpu | 0.4 µs | 1.3 µs | 55 µs | upload 4.8 µs, record 2.4 µs, submit 45 µs, rest 2.7 µs | 141 µs | 72 µs |
| 1 × 128 | cpu-1 | 0.1 µs | 0.4 µs | 1.0 µs | kernels 0.5 µs, store 0.2 µs, rest 0.2 µs | 0.0 µs | – |
| 1 × 128 | cpu | 0.2 µs | 0.6 µs | 7.1 µs | kernels 6.5 µs, store 0.3 µs, rest 0.4 µs | 0.0 µs | – |
| 1 × 128 | gpu | 0.3 µs | 1.1 µs | 46 µs | upload 3.7 µs, record 2.2 µs, submit 37 µs, rest 2.2 µs | 54 µs | 2.2 µs |
| 16 × 128 | cpu-1 | 0.2 µs | 0.5 µs | 12 µs | kernels 8.4 µs, store 2.8 µs, rest 0.3 µs | 0.0 µs | – |
| 16 × 128 | cpu | 0.2 µs | 0.7 µs | 25 µs | kernels 22 µs, store 3.2 µs, rest 0.5 µs | 0.0 µs | – |
| 16 × 128 | gpu | 0.3 µs | 0.9 µs | 38 µs | upload 3.1 µs, record 1.7 µs, submit 31 µs, rest 1.8 µs | 50 µs | 2.9 µs |
| 256 × 128 | cpu-1 | 0.2 µs | 0.8 µs | 670 µs | kernels 483 µs, store 173 µs, rest 1.0 µs | 0.0 µs | – |
| 256 × 128 | cpu | 0.4 µs | 1.1 µs | 664 µs | kernels 532 µs, store 128 µs, rest 1.2 µs | 0.0 µs | – |
| 256 × 128 | gpu | 0.3 µs | 0.9 µs | 39 µs | upload 3.1 µs, record 1.7 µs, submit 32 µs, rest 1.8 µs | 82 µs | 22 µs |
| 1024 × 128 | cpu-1 | 0.4 µs | 1.1 µs | 2.54 ms | kernels 1.96 ms, store 590 µs, rest 1.4 µs | 0.0 µs | – |
| 1024 × 128 | cpu | 0.7 µs | 2.1 µs | 2.18 ms | kernels 1.64 ms, store 545 µs, rest 2.0 µs | 0.0 µs | – |
| 1024 × 128 | gpu | 0.4 µs | 1.1 µs | 46 µs | upload 3.5 µs, record 2.0 µs, submit 38 µs, rest 2.2 µs | 146 µs | 77 µs |

## sweep circle

Sweep of a circle along the curves, the radius changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 2.0 µs | 1.5 µs | 109 µs | 72.6× slower | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 16 × 32 | 5.3 µs | 12 µs | 111 µs | 9.2× slower | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 32 | 71 µs | 114 µs | 125 µs | 1.1× slower | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 4096 × 32 | 5.60 ms | 4.63 ms | 232 µs | 19.9× faster | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 1 × 128 | 2.3 µs | 2.3 µs | 109 µs | 47.5× slower | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 16 × 128 | 20 µs | 63 µs | 111 µs | 1.8× slower | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 128 | 864 µs | 865 µs | 144 µs | 6.0× faster | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 1024 × 128 | 4.26 ms | 3.88 ms | 230 µs | 16.9× faster | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device |
|---|---|---:|---:|---:|---|---:|---:|
| 1 × 32 | cpu-1 | 0.1 µs | 0.7 µs | 1.2 µs | kernels 0.6 µs, store 0.3 µs, rest 0.4 µs | 0.0 µs | – |
| 1 × 32 | cpu | 0.1 µs | 0.6 µs | 0.8 µs | kernels 0.4 µs, store 0.2 µs, rest 0.3 µs | 0.0 µs | – |
| 1 × 32 | gpu | 0.3 µs | 1.1 µs | 44 µs | upload 3.8 µs, record 2.7 µs, submit 35 µs, download 0.1 µs, rest 2.9 µs | 63 µs | 2.9 µs |
| 16 × 32 | cpu-1 | 0.1 µs | 0.6 µs | 4.7 µs | kernels 3.5 µs, store 0.9 µs, rest 0.3 µs | 0.0 µs | – |
| 16 × 32 | cpu | 0.2 µs | 0.9 µs | 11 µs | kernels 8.9 µs, store 1.2 µs, rest 0.5 µs | 0.0 µs | – |
| 16 × 32 | gpu | 0.3 µs | 1.1 µs | 45 µs | upload 3.9 µs, record 2.8 µs, submit 36 µs, download 0.1 µs, rest 3.2 µs | 64 µs | 4.7 µs |
| 256 × 32 | cpu-1 | 0.2 µs | 0.7 µs | 70 µs | kernels 57 µs, store 12 µs, rest 0.5 µs | 0.0 µs | – |
| 256 × 32 | cpu | 0.2 µs | 1.0 µs | 113 µs | kernels 99 µs, store 13 µs, rest 0.7 µs | 0.0 µs | – |
| 256 × 32 | gpu | 0.3 µs | 1.1 µs | 48 µs | upload 4.1 µs, record 2.8 µs, submit 36 µs, download 0.1 µs, rest 4.2 µs | 75 µs | 11 µs |
| 4096 × 32 | cpu-1 | 0.8 µs | 2.4 µs | 5.59 ms | kernels 4.96 ms, store 624 µs, rest 2.0 µs | 0.0 µs | – |
| 4096 × 32 | cpu | 0.9 µs | 2.7 µs | 4.62 ms | kernels 4.01 ms, store 623 µs, rest 2.2 µs | 0.0 µs | – |
| 4096 × 32 | gpu | 0.4 µs | 1.3 µs | 64 µs | upload 5.9 µs, record 3.2 µs, submit 42 µs, download 0.1 µs, rest 12 µs | 164 µs | 93 µs |
| 1 × 128 | cpu-1 | 0.1 µs | 0.6 µs | 1.6 µs | kernels 0.9 µs, store 0.4 µs, rest 0.3 µs | 0.0 µs | – |
| 1 × 128 | cpu | 0.1 µs | 0.6 µs | 1.6 µs | kernels 1.0 µs, store 0.4 µs, rest 0.3 µs | 0.0 µs | – |
| 1 × 128 | gpu | 0.3 µs | 1.2 µs | 46 µs | upload 3.9 µs, record 2.7 µs, submit 36 µs, download 0.1 µs, rest 3.2 µs | 62 µs | 3.4 µs |
| 16 × 128 | cpu-1 | 0.2 µs | 0.8 µs | 19 µs | kernels 15 µs, store 3.8 µs, rest 0.5 µs | 0.0 µs | – |
| 16 × 128 | cpu | 0.3 µs | 1.1 µs | 61 µs | kernels 56 µs, store 4.7 µs, rest 0.7 µs | 0.0 µs | – |
| 16 × 128 | gpu | 0.3 µs | 1.1 µs | 46 µs | upload 4.0 µs, record 2.7 µs, submit 36 µs, download 0.1 µs, rest 3.1 µs | 63 µs | 5.2 µs |
| 256 × 128 | cpu-1 | 0.2 µs | 1.1 µs | 863 µs | kernels 658 µs, store 193 µs, rest 0.7 µs | 0.0 µs | – |
| 256 × 128 | cpu | 0.3 µs | 1.2 µs | 863 µs | kernels 691 µs, store 195 µs, rest 0.9 µs | 0.0 µs | – |
| 256 × 128 | gpu | 0.3 µs | 1.1 µs | 48 µs | upload 4.1 µs, record 2.8 µs, submit 37 µs, download 0.1 µs, rest 4.0 µs | 96 µs | 30 µs |
| 1024 × 128 | cpu-1 | 0.4 µs | 1.7 µs | 4.26 ms | kernels 3.58 ms, store 683 µs, rest 1.4 µs | 0.0 µs | – |
| 1024 × 128 | cpu | 1.1 µs | 2.7 µs | 3.88 ms | kernels 3.20 ms, store 666 µs, rest 2.3 µs | 0.0 µs | – |
| 1024 × 128 | gpu | 0.4 µs | 1.3 µs | 54 µs | upload 5.0 µs, record 3.0 µs, submit 40 µs, download 0.1 µs, rest 5.7 µs | 170 µs | 100 µs |

## sweep square

Sweep of a square curve along the curves, the square changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 10 µs | 13 µs | 176 µs | 13.7× slower | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 16 × 32 | 64 µs | 78 µs | 176 µs | 2.3× slower | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 256 × 32 | 2.09 ms | 1.86 ms | 216 µs | 8.6× faster | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 4096 × 32 | 63.80 ms | 48.69 ms | 774 µs | 62.9× faster | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 1 × 128 | 21 µs | 20 µs | 176 µs | 8.6× slower | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 16 × 128 | 577 µs | 534 µs | 189 µs | 2.8× faster | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 256 × 128 | 9.23 ms | 8.04 ms | 349 µs | 23.0× faster | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 1024 × 128 | 50.73 ms | 39.60 ms | 706 µs | 56.1× faster | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device |
|---|---|---:|---:|---:|---|---:|---:|
| 1 × 32 | cpu-1 | 0.1 µs | 0.8 µs | 9.4 µs | kernels 8.5 µs, store 0.6 µs, rest 0.4 µs | 0.0 µs | – |
| 1 × 32 | cpu | 0.2 µs | 0.9 µs | 12 µs | kernels 10 µs, store 0.7 µs, rest 0.4 µs | 0.0 µs | – |
| 1 × 32 | gpu | 0.4 µs | 1.2 µs | 57 µs | upload 4.3 µs, record 4.1 µs, submit 42 µs, download 0.2 µs, rest 5.1 µs | 118 µs | 48 µs |
| 16 × 32 | cpu-1 | 0.2 µs | 0.8 µs | 62 µs | kernels 56 µs, store 6.2 µs, rest 0.5 µs | 0.0 µs | – |
| 16 × 32 | cpu | 0.3 µs | 1.0 µs | 77 µs | kernels 69 µs, store 7.3 µs, rest 0.6 µs | 0.0 µs | – |
| 16 × 32 | gpu | 0.4 µs | 1.3 µs | 57 µs | upload 5.4 µs, record 3.9 µs, submit 42 µs, download 0.1 µs, rest 5.5 µs | 117 µs | 51 µs |
| 256 × 32 | cpu-1 | 0.4 µs | 1.3 µs | 2.09 ms | kernels 1.77 ms, store 318 µs, rest 1.0 µs | 0.0 µs | – |
| 256 × 32 | cpu | 0.5 µs | 1.5 µs | 1.86 ms | kernels 1.55 ms, store 304 µs, rest 1.1 µs | 0.0 µs | – |
| 256 × 32 | gpu | 0.4 µs | 1.2 µs | 59 µs | upload 5.7 µs, record 4.0 µs, submit 43 µs, download 0.1 µs, rest 6.3 µs | 156 µs | 87 µs |
| 4096 × 32 | cpu-1 | 2.3 µs | 3.7 µs | 63.79 ms | kernels 59.49 ms, store 4.20 ms, rest 3.8 µs | 0.0 µs | – |
| 4096 × 32 | cpu | 2.5 µs | 4.0 µs | 48.68 ms | kernels 44.55 ms, store 4.06 ms, rest 3.8 µs | 0.1 µs | – |
| 4096 × 32 | gpu | 0.5 µs | 1.4 µs | 73 µs | upload 6.8 µs, record 4.1 µs, submit 46 µs, download 0.2 µs, rest 16 µs | 690 µs | 591 µs |
| 1 × 128 | cpu-1 | 0.2 µs | 0.7 µs | 20 µs | kernels 18 µs, store 1.7 µs, rest 0.4 µs | 0.0 µs | – |
| 1 × 128 | cpu | 0.2 µs | 0.7 µs | 20 µs | kernels 17 µs, store 1.8 µs, rest 0.4 µs | 0.0 µs | – |
| 1 × 128 | gpu | 0.4 µs | 1.2 µs | 55 µs | upload 5.3 µs, record 3.9 µs, submit 41 µs, download 0.2 µs, rest 5.0 µs | 119 µs | 48 µs |
| 16 × 128 | cpu-1 | 0.3 µs | 1.1 µs | 576 µs | kernels 490 µs, store 66 µs, rest 0.8 µs | 0.0 µs | – |
| 16 × 128 | cpu | 0.5 µs | 1.5 µs | 531 µs | kernels 448 µs, store 64 µs, rest 1.1 µs | 0.0 µs | – |
| 16 × 128 | gpu | 0.4 µs | 1.2 µs | 57 µs | upload 5.5 µs, record 4.1 µs, submit 42 µs, download 0.2 µs, rest 5.3 µs | 128 µs | 59 µs |
| 256 × 128 | cpu-1 | 1.4 µs | 2.7 µs | 9.23 ms | kernels 8.05 ms, store 1.19 ms, rest 2.6 µs | 0.0 µs | – |
| 256 × 128 | cpu | 1.8 µs | 3.0 µs | 8.03 ms | kernels 6.84 ms, store 1.21 ms, rest 2.7 µs | 0.0 µs | – |
| 256 × 128 | gpu | 0.4 µs | 1.3 µs | 59 µs | upload 5.7 µs, record 3.9 µs, submit 43 µs, download 0.1 µs, rest 6.5 µs | 286 µs | 213 µs |
| 1024 × 128 | cpu-1 | 2.2 µs | 3.8 µs | 50.73 ms | kernels 46.44 ms, store 4.50 ms, rest 4.8 µs | 0.0 µs | – |
| 1024 × 128 | cpu | 2.5 µs | 3.7 µs | 39.59 ms | kernels 34.99 ms, store 4.55 ms, rest 3.8 µs | 0.0 µs | – |
| 1024 × 128 | gpu | 0.4 µs | 1.3 µs | 62 µs | upload 5.9 µs, record 4.0 µs, submit 44 µs, download 0.2 µs, rest 8.3 µs | 642 µs | 499 µs |

## sample

Sample Curve, 64 frames per curve at places which change.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 35 µs | 35 µs | 111 µs | 3.2× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 32 | 552 µs | 110 µs | 110 µs | 1.0× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 32 | 9.01 ms | 1.22 ms | 115 µs | 10.6× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 4096 × 32 | 145.48 ms | 19.05 ms | 198 µs | 96.0× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 1 × 128 | 43 µs | 35 µs | 109 µs | 3.1× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 128 | 600 µs | 148 µs | 108 µs | 1.4× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 128 | 8.84 ms | 1.20 ms | 117 µs | 10.3× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 1024 × 128 | 36.02 ms | 4.64 ms | 160 µs | 29.0× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device |
|---|---|---:|---:|---:|---|---:|---:|
| 1 × 32 | cpu-1 | 0.1 µs | 0.6 µs | 35 µs | kernels 34 µs, store 0.2 µs, rest 0.3 µs | 0.0 µs | – |
| 1 × 32 | cpu | 0.2 µs | 0.6 µs | 35 µs | kernels 34 µs, store 0.2 µs, rest 0.3 µs | 0.0 µs | – |
| 1 × 32 | gpu | 0.4 µs | 1.1 µs | 40 µs | upload 3.9 µs, record 1.6 µs, submit 32 µs, download 0.1 µs, rest 2.2 µs | 70 µs | 11 µs |
| 16 × 32 | cpu-1 | 0.2 µs | 0.6 µs | 552 µs | kernels 550 µs, store 1.0 µs, rest 0.3 µs | 0.0 µs | – |
| 16 × 32 | cpu | 0.4 µs | 1.1 µs | 108 µs | kernels 106 µs, store 1.5 µs, rest 0.6 µs | 0.0 µs | – |
| 16 × 32 | gpu | 0.4 µs | 1.1 µs | 40 µs | upload 3.0 µs, record 1.7 µs, submit 32 µs, download 0.1 µs, rest 2.4 µs | 69 µs | 11 µs |
| 256 × 32 | cpu-1 | 0.4 µs | 1.3 µs | 9.00 ms | kernels 8.98 ms, store 17 µs, rest 0.9 µs | 0.0 µs | – |
| 256 × 32 | cpu | 0.5 µs | 1.6 µs | 1.21 ms | kernels 1.19 ms, store 20 µs, rest 1.0 µs | 0.0 µs | – |
| 256 × 32 | gpu | 0.4 µs | 1.1 µs | 40 µs | upload 3.1 µs, record 1.6 µs, submit 32 µs, download 0.1 µs, rest 3.4 µs | 72 µs | 12 µs |
| 4096 × 32 | cpu-1 | 1.9 µs | 3.1 µs | 145.48 ms | kernels 144.68 ms, store 794 µs, rest 2.6 µs | 0.0 µs | – |
| 4096 × 32 | cpu | 1.5 µs | 2.9 µs | 19.04 ms | kernels 18.27 ms, store 759 µs, rest 2.3 µs | 0.0 µs | – |
| 4096 × 32 | gpu | 0.4 µs | 1.2 µs | 54 µs | upload 4.1 µs, record 1.7 µs, submit 34 µs, download 0.1 µs, rest 14 µs | 142 µs | 78 µs |
| 1 × 128 | cpu-1 | 0.2 µs | 0.6 µs | 42 µs | kernels 42 µs, store 0.2 µs, rest 0.3 µs | 0.0 µs | – |
| 1 × 128 | cpu | 0.2 µs | 0.5 µs | 34 µs | kernels 34 µs, store 0.2 µs, rest 0.3 µs | 0.0 µs | – |
| 1 × 128 | gpu | 0.4 µs | 1.0 µs | 36 µs | upload 2.8 µs, record 1.6 µs, submit 30 µs, download 0.1 µs, rest 2.0 µs | 71 µs | 11 µs |
| 16 × 128 | cpu-1 | 0.2 µs | 0.6 µs | 599 µs | kernels 598 µs, store 1.0 µs, rest 0.3 µs | 0.0 µs | – |
| 16 × 128 | cpu | 0.3 µs | 1.2 µs | 146 µs | kernels 144 µs, store 1.6 µs, rest 0.6 µs | 0.0 µs | – |
| 16 × 128 | gpu | 0.4 µs | 1.0 µs | 37 µs | upload 2.8 µs, record 1.6 µs, submit 31 µs, download 0.1 µs, rest 2.3 µs | 69 µs | 12 µs |
| 256 × 128 | cpu-1 | 0.4 µs | 1.2 µs | 8.84 ms | kernels 8.82 ms, store 18 µs, rest 0.8 µs | 0.0 µs | – |
| 256 × 128 | cpu | 0.5 µs | 1.3 µs | 1.19 ms | kernels 1.17 ms, store 19 µs, rest 0.9 µs | 0.0 µs | – |
| 256 × 128 | gpu | 0.4 µs | 1.2 µs | 41 µs | upload 3.2 µs, record 1.7 µs, submit 33 µs, download 0.1 µs, rest 3.4 µs | 72 µs | 15 µs |
| 1024 × 128 | cpu-1 | 0.9 µs | 2.3 µs | 36.02 ms | kernels 35.84 ms, store 224 µs, rest 1.8 µs | 0.0 µs | – |
| 1024 × 128 | cpu | 0.6 µs | 1.8 µs | 4.64 ms | kernels 4.42 ms, store 216 µs, rest 1.1 µs | 0.0 µs | – |
| 1024 × 128 | gpu | 0.4 µs | 1.1 µs | 43 µs | upload 3.3 µs, record 1.7 µs, submit 32 µs, download 0.1 µs, rest 5.2 µs | 114 µs | 47 µs |

## chain

NURBS, Translate, a Sweep of a circle and Sample Curve, the control points move.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 75 µs | 123 µs | 241 µs | 2.0× slower | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 16 × 32 | 1.16 ms | 223 µs | 252 µs | 1.1× slower | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 256 × 32 | 18.81 ms | 2.69 ms | 311 µs | 8.7× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 4096 × 32 | 315.36 ms | 47.27 ms | 1.48 ms | 31.9× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 1 × 128 | 198 µs | 302 µs | 284 µs | 1.1× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 16 × 128 | 3.18 ms | 730 µs | 305 µs | 2.4× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 256 × 128 | 52.62 ms | 7.84 ms | 579 µs | 13.6× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 1024 × 128 | 212.88 ms | 33.24 ms | 1.58 ms | 21.1× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device |
|---|---|---:|---:|---:|---|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 1.2 µs | 74 µs | kernels 73 µs, store 0.6 µs, rest 0.6 µs | 0.0 µs | – |
| 1 × 32 | cpu | 0.2 µs | 1.3 µs | 121 µs | kernels 120 µs, store 0.9 µs, rest 0.8 µs | 0.0 µs | – |
| 1 × 32 | gpu | 0.4 µs | 1.9 µs | 72 µs | upload 6.9 µs, record 6.5 µs, submit 51 µs, download 0.3 µs, rest 7.4 µs | 167 µs | 87 µs |
| 16 × 32 | cpu-1 | 0.5 µs | 1.4 µs | 1.16 ms | kernels 1.15 ms, store 3.5 µs, rest 0.7 µs | 0.0 µs | – |
| 16 × 32 | cpu | 1.0 µs | 2.8 µs | 220 µs | kernels 210 µs, store 6.8 µs, rest 1.5 µs | 0.0 µs | – |
| 16 × 32 | gpu | 0.7 µs | 2.0 µs | 73 µs | upload 7.1 µs, record 6.2 µs, submit 51 µs, download 0.2 µs, rest 8.4 µs | 175 µs | 98 µs |
| 256 × 32 | cpu-1 | 6.8 µs | 4.9 µs | 18.79 ms | kernels 18.71 ms, store 61 µs, rest 1.6 µs | 0.0 µs | – |
| 256 × 32 | cpu | 7.4 µs | 6.3 µs | 2.68 ms | kernels 2.59 ms, store 77 µs, rest 2.4 µs | 0.0 µs | – |
| 256 × 32 | gpu | 6.3 µs | 4.1 µs | 94 µs | upload 19 µs, record 6.3 µs, submit 53 µs, download 0.2 µs, rest 15 µs | 205 µs | 120 µs |
| 4096 × 32 | cpu-1 | 334 µs | 145 µs | 314.90 ms | kernels 312.33 ms, store 2.55 ms, rest 8.4 µs | 0.0 µs | – |
| 4096 × 32 | cpu | 337 µs | 144 µs | 46.78 ms | kernels 44.37 ms, store 2.41 ms, rest 7.4 µs | 0.0 µs | – |
| 4096 × 32 | gpu | 294 µs | 86 µs | 305 µs | upload 139 µs, record 6.9 µs, submit 61 µs, download 0.3 µs, rest 96 µs | 801 µs | 593 µs |
| 1 × 128 | cpu-1 | 0.2 µs | 1.2 µs | 197 µs | kernels 195 µs, store 0.9 µs, rest 0.6 µs | 0.0 µs | – |
| 1 × 128 | cpu | 0.3 µs | 1.6 µs | 300 µs | kernels 297 µs, store 1.9 µs, rest 1.0 µs | 0.0 µs | – |
| 1 × 128 | gpu | 0.4 µs | 1.8 µs | 71 µs | upload 6.8 µs, record 6.4 µs, submit 51 µs, download 0.2 µs, rest 7.1 µs | 210 µs | 139 µs |
| 16 × 128 | cpu-1 | 2.0 µs | 2.1 µs | 3.18 ms | kernels 3.16 ms, store 12 µs, rest 0.9 µs | 0.0 µs | – |
| 16 × 128 | cpu | 2.3 µs | 3.1 µs | 725 µs | kernels 703 µs, store 18 µs, rest 2.1 µs | 0.0 µs | – |
| 16 × 128 | gpu | 1.9 µs | 2.5 µs | 77 µs | upload 8.6 µs, record 6.5 µs, submit 53 µs, download 0.2 µs, rest 9.1 µs | 222 µs | 152 µs |
| 256 × 128 | cpu-1 | 27 µs | 24 µs | 52.55 ms | kernels 52.07 ms, store 486 µs, rest 3.9 µs | 0.0 µs | – |
| 256 × 128 | cpu | 27 µs | 23 µs | 7.79 ms | kernels 7.29 ms, store 492 µs, rest 3.1 µs | 0.0 µs | – |
| 256 × 128 | gpu | 25 µs | 12 µs | 134 µs | upload 43 µs, record 6.8 µs, submit 60 µs, download 0.3 µs, rest 21 µs | 410 µs | 251 µs |
| 1024 × 128 | cpu-1 | 316 µs | 86 µs | 212.47 ms | kernels 210.43 ms, store 2.02 ms, rest 7.2 µs | 0.0 µs | – |
| 1024 × 128 | cpu | 318 µs | 87 µs | 32.84 ms | kernels 30.97 ms, store 1.92 ms, rest 6.2 µs | 0.0 µs | – |
| 1024 × 128 | gpu | 337 µs | 86 µs | 266 µs | upload 138 µs, record 7.3 µs, submit 62 µs, download 0.3 µs, rest 56 µs | 875 µs | 690 µs |

## mesh

Mesh of a swept circle, the tolerance changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 2.30 ms | 338 µs | 351 µs | 1.0× slower | 0 on gpu, 1 on cpu, 0 passes, 0 downloads |
| 16 × 32 | 38.01 ms | 5.82 ms | 6.02 ms | 1.0× slower | 0 on gpu, 1 on cpu, 0 passes, 0 downloads |
| 64 × 32 | 152.34 ms | 25.41 ms | 26.00 ms | 1.0× slower | 0 on gpu, 1 on cpu, 0 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device |
|---|---|---:|---:|---:|---|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 1.0 µs | 2.30 ms | kernels 2.30 ms, store 4.3 µs, rest 0.4 µs | 0.0 µs | – |
| 1 × 32 | cpu | 0.3 µs | 1.2 µs | 336 µs | kernels 331 µs, store 5.2 µs, rest 0.6 µs | 0.0 µs | – |
| 1 × 32 | gpu | 0.3 µs | 1.3 µs | 344 µs | download 0.1 µs, cpu 343 µs, rest 1.3 µs | 5.4 µs | 0.0 µs |
| 16 × 32 | cpu-1 | 1.3 µs | 3.2 µs | 38.01 ms | kernels 37.80 ms, store 228 µs, rest 2.0 µs | 0.0 µs | – |
| 16 × 32 | cpu | 0.5 µs | 1.7 µs | 5.82 ms | kernels 5.63 ms, store 190 µs, rest 1.1 µs | 0.0 µs | – |
| 16 × 32 | gpu | 0.4 µs | 1.9 µs | 6.01 ms | download 0.1 µs, cpu 6.01 ms, rest 2.1 µs | 8.6 µs | 0.0 µs |
| 64 × 32 | cpu-1 | 2.1 µs | 3.7 µs | 152.33 ms | kernels 151.50 ms, store 769 µs, rest 2.7 µs | 0.0 µs | – |
| 64 × 32 | cpu | 0.7 µs | 2.2 µs | 25.40 ms | kernels 24.71 ms, store 723 µs, rest 1.5 µs | 0.0 µs | – |
| 64 × 32 | gpu | 0.7 µs | 2.4 µs | 25.99 ms | submit 0.1 µs, download 0.1 µs, cpu 25.99 ms, rest 2.7 µs | 11 µs | 0.0 µs |
