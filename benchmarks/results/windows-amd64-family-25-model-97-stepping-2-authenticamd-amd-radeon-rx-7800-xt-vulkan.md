# Benchmarks

windows, AMD64 Family 25 Model 97 Stepping 2, AuthenticAMD with 16 threads, AMD Radeon RX 7800 XT on vulkan, driver AMD proprietary driver 26.11.2 (LLPC). Commit 6ee7efe.

The time from an edit until its result is computed, the median of many edits. Below each table, where it goes: setting the new value (which hashes it) and planning are the same for every backend, `flush` is the executor, then waiting for the device. The device column is the time the GPU spends on compute passes, by its timestamps. Ending the frame afterwards frees what is not used anymore, it is not in the total.

## nurbs

NURBS Curve, the control points move.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 30 µs | 31 µs | 185 µs | 6.0× slower | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 16 × 32 | 489 µs | 82 µs | 204 µs | 2.5× slower | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 32 | 7.84 ms | 750 µs | 238 µs | 3.1× faster | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 4096 × 32 | 128.78 ms | 13.91 ms | 905 µs | 15.4× faster | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 1 × 128 | 132 µs | 132 µs | 246 µs | 1.9× slower | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 16 × 128 | 2.12 ms | 255 µs | 259 µs | 1.0× slower | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 128 | 38.52 ms | 3.45 ms | 488 µs | 7.1× faster | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 1024 × 128 | 139.45 ms | 14.57 ms | 1.03 ms | 14.1× faster | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 0.2 µs | 30 µs | kernels 30 µs, rest 0.3 µs | 0.0 µs | – | 0.1 µs |
| 1 × 32 | cpu | 0.2 µs | 0.2 µs | 30 µs | kernels 30 µs, rest 0.3 µs | 0.0 µs | – | 0.1 µs |
| 1 × 32 | gpu | 0.5 µs | 0.6 µs | 47 µs | upload 4.5 µs, record 2.8 µs, submit 36 µs, download 0.1 µs, rest 3.7 µs | 136 µs | 72 µs | 0.6 µs |
| 16 × 32 | cpu-1 | 0.5 µs | 0.3 µs | 488 µs | kernels 488 µs, store 0.1 µs, rest 0.3 µs | 0.0 µs | – | 0.1 µs |
| 16 × 32 | cpu | 1.1 µs | 0.5 µs | 80 µs | kernels 79 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – | 0.3 µs |
| 16 × 32 | gpu | 0.9 µs | 0.8 µs | 50 µs | upload 4.9 µs, record 2.9 µs, submit 37 µs, download 0.1 µs, rest 4.9 µs | 152 µs | 80 µs | 0.7 µs |
| 256 × 32 | cpu-1 | 6.5 µs | 0.6 µs | 7.83 ms | kernels 7.83 ms, store 0.1 µs, rest 0.6 µs | 0.0 µs | – | 0.2 µs |
| 256 × 32 | cpu | 7.8 µs | 0.6 µs | 742 µs | kernels 741 µs, store 0.1 µs, rest 0.7 µs | 0.0 µs | – | 0.4 µs |
| 256 × 32 | gpu | 6.3 µs | 0.7 µs | 68 µs | upload 17 µs, record 2.8 µs, submit 38 µs, download 0.1 µs, rest 9.5 µs | 163 µs | 91 µs | 0.8 µs |
| 4096 × 32 | cpu-1 | 156 µs | 3.0 µs | 128.63 ms | kernels 128.63 ms, store 0.6 µs, rest 3.7 µs | 0.1 µs | – | 1.3 µs |
| 4096 × 32 | cpu | 144 µs | 1.8 µs | 13.76 ms | kernels 13.76 ms, store 0.4 µs, rest 2.6 µs | 0.0 µs | – | 1.6 µs |
| 4096 × 32 | gpu | 119 µs | 0.9 µs | 255 µs | upload 137 µs, record 3.4 µs, submit 48 µs, download 0.1 µs, rest 67 µs | 517 µs | 349 µs | 0.9 µs |
| 1 × 128 | cpu-1 | 0.3 µs | 0.3 µs | 131 µs | kernels 131 µs, rest 0.3 µs | 0.0 µs | – | 0.1 µs |
| 1 × 128 | cpu | 0.3 µs | 0.3 µs | 131 µs | kernels 131 µs, rest 0.3 µs | 0.0 µs | – | 0.1 µs |
| 1 × 128 | gpu | 0.7 µs | 0.7 µs | 55 µs | upload 5.2 µs, record 3.2 µs, submit 41 µs, download 0.1 µs, rest 4.6 µs | 190 µs | 123 µs | 0.8 µs |
| 16 × 128 | cpu-1 | 1.7 µs | 0.4 µs | 2.11 ms | kernels 2.11 ms, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 16 × 128 | cpu | 2.4 µs | 0.5 µs | 252 µs | kernels 251 µs, store 0.1 µs, rest 0.7 µs | 0.0 µs | – | 0.4 µs |
| 16 × 128 | gpu | 2.0 µs | 0.7 µs | 53 µs | upload 7.4 µs, record 3.0 µs, submit 37 µs, download 0.1 µs, rest 5.5 µs | 202 µs | 127 µs | 0.7 µs |
| 256 × 128 | cpu-1 | 38 µs | 1.5 µs | 38.49 ms | kernels 38.49 ms, store 0.2 µs, rest 1.8 µs | 0.0 µs | – | 0.8 µs |
| 256 × 128 | cpu | 26 µs | 1.2 µs | 3.42 ms | kernels 3.42 ms, store 0.2 µs, rest 1.1 µs | 0.0 µs | – | 0.7 µs |
| 256 × 128 | gpu | 23 µs | 0.8 µs | 98 µs | upload 40 µs, record 3.2 µs, submit 40 µs, download 0.1 µs, rest 15 µs | 363 µs | 185 µs | 0.9 µs |
| 1024 × 128 | cpu-1 | 156 µs | 2.5 µs | 139.30 ms | kernels 139.29 ms, store 0.4 µs, rest 3.2 µs | 0.1 µs | – | 1.9 µs |
| 1024 × 128 | cpu | 131 µs | 1.2 µs | 14.44 ms | kernels 14.44 ms, store 0.4 µs, rest 1.7 µs | 0.0 µs | – | 0.9 µs |
| 1024 × 128 | gpu | 129 µs | 1.0 µs | 242 µs | upload 138 µs, record 3.6 µs, submit 52 µs, download 0.1 µs, rest 45 µs | 642 µs | 473 µs | 0.9 µs |

## translate

Translate Curve of the curves, the offset changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 0.7 µs | 3.8 µs | 92 µs | 24.2× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 32 | 1.4 µs | 9.3 µs | 93 µs | 10.0× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 32 | 13 µs | 18 µs | 95 µs | 5.2× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 4096 × 32 | 371 µs | 130 µs | 177 µs | 1.4× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 1 × 128 | 0.9 µs | 6.3 µs | 87 µs | 13.8× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 128 | 3.7 µs | 14 µs | 92 µs | 6.5× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 128 | 50 µs | 31 µs | 120 µs | 3.9× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 1024 × 128 | 428 µs | 162 µs | 182 µs | 1.1× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 1 × 32 | cpu-1 | 0.1 µs | 0.2 µs | 0.4 µs | kernels 0.1 µs, rest 0.2 µs | 0.0 µs | – | 0.1 µs |
| 1 × 32 | cpu | 0.2 µs | 0.3 µs | 3.2 µs | kernels 2.8 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.3 µs |
| 1 × 32 | gpu | 0.4 µs | 0.5 µs | 36 µs | upload 2.6 µs, record 1.5 µs, submit 30 µs, rest 1.7 µs | 56 µs | 1.8 µs | 0.5 µs |
| 16 × 32 | cpu-1 | 0.1 µs | 0.2 µs | 1.0 µs | kernels 0.7 µs, rest 0.3 µs | 0.0 µs | – | 0.1 µs |
| 16 × 32 | cpu | 0.3 µs | 0.4 µs | 8.5 µs | kernels 8.0 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.3 µs |
| 16 × 32 | gpu | 0.4 µs | 0.5 µs | 36 µs | upload 2.7 µs, record 1.6 µs, submit 30 µs, rest 1.8 µs | 57 µs | 2.5 µs | 0.6 µs |
| 256 × 32 | cpu-1 | 0.2 µs | 0.3 µs | 12 µs | kernels 12 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 256 × 32 | cpu | 0.3 µs | 0.6 µs | 17 µs | kernels 17 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – | 0.3 µs |
| 256 × 32 | gpu | 0.4 µs | 0.5 µs | 36 µs | upload 2.7 µs, record 1.5 µs, submit 30 µs, rest 1.8 µs | 57 µs | 6.4 µs | 0.6 µs |
| 4096 × 32 | cpu-1 | 0.4 µs | 0.5 µs | 370 µs | kernels 369 µs, store 0.2 µs, rest 0.9 µs | 0.0 µs | – | 0.5 µs |
| 4096 × 32 | cpu | 0.5 µs | 0.7 µs | 128 µs | kernels 127 µs, store 0.2 µs, rest 1.1 µs | 0.0 µs | – | 0.7 µs |
| 4096 × 32 | gpu | 0.5 µs | 0.6 µs | 37 µs | upload 2.9 µs, record 1.6 µs, submit 31 µs, rest 1.9 µs | 138 µs | 72 µs | 0.6 µs |
| 1 × 128 | cpu-1 | 0.1 µs | 0.2 µs | 0.6 µs | kernels 0.3 µs, rest 0.3 µs | 0.0 µs | – | 0.1 µs |
| 1 × 128 | cpu | 0.3 µs | 0.6 µs | 5.4 µs | kernels 4.9 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – | 0.3 µs |
| 1 × 128 | gpu | 0.4 µs | 0.5 µs | 35 µs | upload 2.6 µs, record 1.5 µs, submit 29 µs, rest 1.8 µs | 51 µs | 2.0 µs | 0.5 µs |
| 16 × 128 | cpu-1 | 0.1 µs | 0.2 µs | 3.3 µs | kernels 3.0 µs, store 0.1 µs, rest 0.2 µs | 0.0 µs | – | 0.1 µs |
| 16 × 128 | cpu | 0.2 µs | 0.3 µs | 13 µs | kernels 13 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.3 µs |
| 16 × 128 | gpu | 0.4 µs | 0.5 µs | 36 µs | upload 2.8 µs, record 1.6 µs, submit 30 µs, rest 1.9 µs | 52 µs | 2.6 µs | 0.6 µs |
| 256 × 128 | cpu-1 | 0.2 µs | 0.3 µs | 49 µs | kernels 49 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 256 × 128 | cpu | 0.4 µs | 0.5 µs | 30 µs | kernels 29 µs, store 0.1 µs, rest 0.6 µs | 0.0 µs | – | 0.4 µs |
| 256 × 128 | gpu | 0.4 µs | 0.5 µs | 36 µs | upload 2.7 µs, record 1.6 µs, submit 30 µs, rest 1.8 µs | 82 µs | 22 µs | 0.6 µs |
| 1024 × 128 | cpu-1 | 0.3 µs | 0.7 µs | 427 µs | kernels 426 µs, store 0.2 µs, rest 0.8 µs | 0.0 µs | – | 0.4 µs |
| 1024 × 128 | cpu | 0.5 µs | 0.7 µs | 160 µs | kernels 159 µs, store 0.2 µs, rest 1.0 µs | 0.0 µs | – | 0.7 µs |
| 1024 × 128 | gpu | 0.4 µs | 0.5 µs | 37 µs | upload 2.8 µs, record 1.6 µs, submit 31 µs, rest 1.9 µs | 144 µs | 78 µs | 0.6 µs |

## sweep circle

Sweep of a circle along the curves, the radius changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 2.2 µs | 2.8 µs | 120 µs | 42.8× slower | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 16 × 32 | 16 µs | 14 µs | 124 µs | 9.0× slower | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 32 | 360 µs | 270 µs | 137 µs | 2.0× faster | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 4096 × 32 | 12.09 ms | 6.56 ms | 298 µs | 22.0× faster | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 1 × 128 | 6.3 µs | 6.1 µs | 118 µs | 19.4× slower | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 16 × 128 | 63 µs | 67 µs | 124 µs | 1.8× slower | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 128 | 1.64 ms | 1.11 ms | 174 µs | 6.4× faster | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 1024 × 128 | 12.41 ms | 8.22 ms | 285 µs | 28.8× faster | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 1 × 32 | cpu-1 | 0.1 µs | 0.3 µs | 1.8 µs | kernels 1.3 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 1 × 32 | cpu | 0.2 µs | 0.4 µs | 2.1 µs | kernels 1.5 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – | 0.3 µs |
| 1 × 32 | gpu | 0.4 µs | 0.7 µs | 45 µs | upload 3.7 µs, record 2.7 µs, submit 35 µs, download 0.1 µs, rest 3.3 µs | 74 µs | 11 µs | 0.7 µs |
| 16 × 32 | cpu-1 | 0.1 µs | 0.5 µs | 15 µs | kernels 14 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 16 × 32 | cpu | 0.3 µs | 0.6 µs | 13 µs | kernels 12 µs, store 0.1 µs, rest 0.7 µs | 0.0 µs | – | 0.5 µs |
| 16 × 32 | gpu | 0.4 µs | 0.8 µs | 45 µs | upload 3.8 µs, record 2.7 µs, submit 35 µs, download 0.1 µs, rest 3.5 µs | 77 µs | 14 µs | 0.7 µs |
| 256 × 32 | cpu-1 | 0.3 µs | 0.5 µs | 359 µs | kernels 358 µs, store 0.2 µs, rest 0.6 µs | 0.0 µs | – | 0.4 µs |
| 256 × 32 | cpu | 0.3 µs | 0.7 µs | 269 µs | kernels 268 µs, store 0.2 µs, rest 0.8 µs | 0.0 µs | – | 0.6 µs |
| 256 × 32 | gpu | 0.4 µs | 0.8 µs | 47 µs | upload 3.9 µs, record 2.7 µs, submit 35 µs, download 0.1 µs, rest 4.5 µs | 88 µs | 24 µs | 0.8 µs |
| 4096 × 32 | cpu-1 | 1.4 µs | 2.9 µs | 12.09 ms | kernels 12.09 ms, store 0.6 µs, rest 2.8 µs | 0.0 µs | – | 2.1 µs |
| 4096 × 32 | cpu | 1.9 µs | 3.0 µs | 6.55 ms | kernels 6.55 ms, store 0.7 µs, rest 3.1 µs | 0.0 µs | – | 2.5 µs |
| 4096 × 32 | gpu | 0.5 µs | 0.9 µs | 61 µs | upload 5.0 µs, record 2.8 µs, submit 38 µs, download 0.2 µs, rest 15 µs | 234 µs | 161 µs | 0.8 µs |
| 1 × 128 | cpu-1 | 0.2 µs | 0.4 µs | 5.7 µs | kernels 5.2 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.3 µs |
| 1 × 128 | cpu | 0.2 µs | 0.4 µs | 5.5 µs | kernels 5.0 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 1 × 128 | gpu | 0.4 µs | 0.7 µs | 44 µs | upload 3.7 µs, record 2.6 µs, submit 34 µs, download 0.1 µs, rest 3.3 µs | 72 µs | 12 µs | 0.7 µs |
| 16 × 128 | cpu-1 | 0.2 µs | 0.6 µs | 62 µs | kernels 62 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – | 0.3 µs |
| 16 × 128 | cpu | 0.3 µs | 0.6 µs | 66 µs | kernels 65 µs, store 0.1 µs, rest 0.7 µs | 0.0 µs | – | 0.5 µs |
| 16 × 128 | gpu | 0.4 µs | 0.8 µs | 46 µs | upload 3.8 µs, record 2.7 µs, submit 36 µs, download 0.1 µs, rest 3.8 µs | 76 µs | 15 µs | 0.7 µs |
| 256 × 128 | cpu-1 | 0.3 µs | 0.7 µs | 1.64 ms | kernels 1.64 ms, store 0.2 µs, rest 0.7 µs | 0.0 µs | – | 0.5 µs |
| 256 × 128 | cpu | 0.6 µs | 1.1 µs | 1.11 ms | kernels 1.11 ms, store 0.2 µs, rest 1.1 µs | 0.0 µs | – | 1.1 µs |
| 256 × 128 | gpu | 0.5 µs | 0.9 µs | 55 µs | upload 4.5 µs, record 3.2 µs, submit 42 µs, download 0.2 µs, rest 5.5 µs | 117 µs | 47 µs | 0.9 µs |
| 1024 × 128 | cpu-1 | 1.4 µs | 2.9 µs | 12.40 ms | kernels 12.40 ms, store 0.6 µs, rest 2.6 µs | 0.0 µs | – | 1.4 µs |
| 1024 × 128 | cpu | 1.9 µs | 3.2 µs | 8.22 ms | kernels 8.21 ms, store 0.8 µs, rest 3.8 µs | 0.0 µs | – | 2.4 µs |
| 1024 × 128 | gpu | 0.4 µs | 0.8 µs | 51 µs | upload 4.4 µs, record 2.8 µs, submit 37 µs, download 0.1 µs, rest 6.7 µs | 232 µs | 159 µs | 0.8 µs |

## sweep square

Sweep of a square curve along the curves, the square changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 9.1 µs | 11 µs | 165 µs | 15.6× slower | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 16 × 32 | 60 µs | 86 µs | 173 µs | 2.0× slower | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 256 × 32 | 2.08 ms | 1.83 ms | 233 µs | 7.8× faster | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 4096 × 32 | 72.26 ms | 56.63 ms | 614 µs | 92.2× faster | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 1 × 128 | 18 µs | 18 µs | 174 µs | 9.7× slower | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 16 × 128 | 465 µs | 473 µs | 193 µs | 2.4× faster | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 256 × 128 | 8.74 ms | 7.55 ms | 396 µs | 19.0× faster | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 1024 × 128 | 54.46 ms | 39.02 ms | 889 µs | 43.9× faster | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 0.3 µs | 8.5 µs | kernels 8.0 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 1 × 32 | cpu | 0.2 µs | 0.4 µs | 10 µs | kernels 9.4 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – | 0.3 µs |
| 1 × 32 | gpu | 0.5 µs | 0.8 µs | 52 µs | upload 4.8 µs, record 3.9 µs, submit 39 µs, download 0.1 µs, rest 4.7 µs | 111 µs | 46 µs | 0.9 µs |
| 16 × 32 | cpu-1 | 0.3 µs | 0.4 µs | 60 µs | kernels 59 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – | 0.3 µs |
| 16 × 32 | cpu | 0.4 µs | 0.6 µs | 84 µs | kernels 83 µs, store 0.1 µs, rest 0.7 µs | 0.0 µs | – | 0.5 µs |
| 16 × 32 | gpu | 0.5 µs | 0.8 µs | 54 µs | upload 4.9 µs, record 4.0 µs, submit 40 µs, download 0.2 µs, rest 5.0 µs | 115 µs | 49 µs | 1.0 µs |
| 256 × 32 | cpu-1 | 0.4 µs | 0.8 µs | 2.08 ms | kernels 2.08 ms, store 0.2 µs, rest 0.9 µs | 0.0 µs | – | 0.6 µs |
| 256 × 32 | cpu | 0.7 µs | 1.2 µs | 1.82 ms | kernels 1.82 ms, store 0.3 µs, rest 1.3 µs | 0.0 µs | – | 1.3 µs |
| 256 × 32 | gpu | 0.5 µs | 0.8 µs | 56 µs | upload 5.2 µs, record 4.0 µs, submit 41 µs, download 0.1 µs, rest 6.2 µs | 174 µs | 97 µs | 1.0 µs |
| 4096 × 32 | cpu-1 | 3.8 µs | 6.1 µs | 72.25 ms | kernels 72.24 ms, store 1.6 µs, rest 7.8 µs | 0.1 µs | – | 5.7 µs |
| 4096 × 32 | cpu | 2.2 µs | 3.5 µs | 56.62 ms | kernels 56.61 ms, store 1.4 µs, rest 3.7 µs | 0.0 µs | – | 3.6 µs |
| 4096 × 32 | gpu | 0.6 µs | 1.1 µs | 82 µs | upload 6.9 µs, record 4.8 µs, submit 50 µs, download 0.2 µs, rest 20 µs | 528 µs | 443 µs | 1.3 µs |
| 1 × 128 | cpu-1 | 0.2 µs | 0.3 µs | 18 µs | kernels 17 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 1 × 128 | cpu | 0.2 µs | 0.3 µs | 18 µs | kernels 17 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 1 × 128 | gpu | 0.5 µs | 0.9 µs | 60 µs | upload 5.0 µs, record 4.4 µs, submit 44 µs, download 0.2 µs, rest 5.6 µs | 112 µs | 45 µs | 1.0 µs |
| 16 × 128 | cpu-1 | 0.3 µs | 0.6 µs | 464 µs | kernels 463 µs, store 0.2 µs, rest 0.7 µs | 0.0 µs | – | 0.4 µs |
| 16 × 128 | cpu | 0.5 µs | 1.0 µs | 472 µs | kernels 470 µs, store 0.2 µs, rest 0.9 µs | 0.0 µs | – | 0.9 µs |
| 16 × 128 | gpu | 0.5 µs | 0.9 µs | 61 µs | upload 5.2 µs, record 4.5 µs, submit 45 µs, download 0.2 µs, rest 5.8 µs | 130 µs | 59 µs | 1.1 µs |
| 256 × 128 | cpu-1 | 2.6 µs | 4.6 µs | 8.74 ms | kernels 8.73 ms, store 1.0 µs, rest 5.0 µs | 0.1 µs | – | 3.2 µs |
| 256 × 128 | cpu | 2.3 µs | 3.0 µs | 7.54 ms | kernels 7.54 ms, store 0.7 µs, rest 3.5 µs | 0.0 µs | – | 2.6 µs |
| 256 × 128 | gpu | 0.5 µs | 0.8 µs | 57 µs | upload 5.2 µs, record 4.0 µs, submit 42 µs, download 0.1 µs, rest 6.1 µs | 337 µs | 258 µs | 1.0 µs |
| 1024 × 128 | cpu-1 | 2.3 µs | 3.8 µs | 54.45 ms | kernels 54.45 ms, store 1.5 µs, rest 5.5 µs | 0.0 µs | – | 3.8 µs |
| 1024 × 128 | cpu | 2.2 µs | 3.2 µs | 39.01 ms | kernels 39.01 ms, store 1.0 µs, rest 3.9 µs | 0.0 µs | – | 3.1 µs |
| 1024 × 128 | gpu | 0.5 µs | 0.9 µs | 63 µs | upload 5.7 µs, record 4.2 µs, submit 44 µs, download 0.2 µs, rest 8.5 µs | 823 µs | 594 µs | 1.1 µs |

## sample

Sample Curve, 64 frames per curve at places which change.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 12 µs | 15 µs | 111 µs | 7.6× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 32 | 220 µs | 51 µs | 113 µs | 2.2× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 32 | 3.18 ms | 549 µs | 113 µs | 4.9× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 4096 × 32 | 49.62 ms | 8.52 ms | 198 µs | 42.9× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 1 × 128 | 12 µs | 12 µs | 110 µs | 9.2× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 128 | 185 µs | 31 µs | 110 µs | 3.6× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 128 | 3.14 ms | 560 µs | 116 µs | 4.8× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 1024 × 128 | 12.54 ms | 2.08 ms | 159 µs | 13.1× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 0.2 µs | 11 µs | kernels 11 µs, rest 0.2 µs | 0.0 µs | – | 0.2 µs |
| 1 × 32 | cpu | 0.2 µs | 0.3 µs | 14 µs | kernels 14 µs, rest 0.3 µs | 0.0 µs | – | 0.2 µs |
| 1 × 32 | gpu | 0.5 µs | 0.7 µs | 40 µs | upload 3.7 µs, record 1.6 µs, submit 32 µs, download 0.1 µs, rest 2.6 µs | 69 µs | 11 µs | 0.7 µs |
| 16 × 32 | cpu-1 | 0.2 µs | 0.3 µs | 219 µs | kernels 219 µs, store 0.1 µs, rest 0.3 µs | 0.0 µs | – | 0.2 µs |
| 16 × 32 | cpu | 0.4 µs | 0.5 µs | 49 µs | kernels 48 µs, store 0.1 µs, rest 0.6 µs | 0.0 µs | – | 0.4 µs |
| 16 × 32 | gpu | 0.5 µs | 0.6 µs | 39 µs | upload 3.9 µs, record 1.5 µs, submit 31 µs, download 0.1 µs, rest 2.7 µs | 72 µs | 11 µs | 0.6 µs |
| 256 × 32 | cpu-1 | 0.4 µs | 0.6 µs | 3.18 ms | kernels 3.18 ms, store 0.1 µs, rest 0.8 µs | 0.0 µs | – | 0.3 µs |
| 256 × 32 | cpu | 0.5 µs | 0.6 µs | 548 µs | kernels 547 µs, store 0.2 µs, rest 0.8 µs | 0.0 µs | – | 0.5 µs |
| 256 × 32 | gpu | 0.5 µs | 0.6 µs | 40 µs | upload 3.8 µs, record 1.5 µs, submit 31 µs, download 0.1 µs, rest 3.6 µs | 71 µs | 12 µs | 0.7 µs |
| 4096 × 32 | cpu-1 | 2.9 µs | 3.8 µs | 49.61 ms | kernels 49.60 ms, store 1.1 µs, rest 5.8 µs | 0.1 µs | – | 2.8 µs |
| 4096 × 32 | cpu | 1.9 µs | 2.7 µs | 8.51 ms | kernels 8.51 ms, store 0.7 µs, rest 3.7 µs | 0.0 µs | – | 2.1 µs |
| 4096 × 32 | gpu | 0.5 µs | 0.6 µs | 53 µs | upload 4.8 µs, record 1.6 µs, submit 33 µs, download 0.1 µs, rest 14 µs | 144 µs | 78 µs | 0.7 µs |
| 1 × 128 | cpu-1 | 0.2 µs | 0.3 µs | 12 µs | kernels 11 µs, rest 0.3 µs | 0.0 µs | – | 0.2 µs |
| 1 × 128 | cpu | 0.2 µs | 0.3 µs | 12 µs | kernels 11 µs, rest 0.3 µs | 0.0 µs | – | 0.2 µs |
| 1 × 128 | gpu | 0.5 µs | 0.6 µs | 39 µs | upload 3.8 µs, record 1.6 µs, submit 31 µs, download 0.1 µs, rest 2.4 µs | 70 µs | 11 µs | 0.6 µs |
| 16 × 128 | cpu-1 | 0.2 µs | 0.3 µs | 184 µs | kernels 184 µs, store 0.1 µs, rest 0.3 µs | 0.0 µs | – | 0.2 µs |
| 16 × 128 | cpu | 0.3 µs | 0.4 µs | 30 µs | kernels 29 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.3 µs |
| 16 × 128 | gpu | 0.5 µs | 0.6 µs | 39 µs | upload 3.9 µs, record 1.6 µs, submit 31 µs, download 0.1 µs, rest 2.6 µs | 69 µs | 12 µs | 0.6 µs |
| 256 × 128 | cpu-1 | 0.3 µs | 0.5 µs | 3.14 ms | kernels 3.14 ms, store 0.2 µs, rest 0.7 µs | 0.0 µs | – | 0.5 µs |
| 256 × 128 | cpu | 0.5 µs | 0.7 µs | 559 µs | kernels 558 µs, store 0.2 µs, rest 0.9 µs | 0.0 µs | – | 0.8 µs |
| 256 × 128 | gpu | 0.5 µs | 0.6 µs | 40 µs | upload 3.8 µs, record 1.5 µs, submit 31 µs, download 0.1 µs, rest 3.6 µs | 74 µs | 15 µs | 0.7 µs |
| 1024 × 128 | cpu-1 | 1.2 µs | 1.5 µs | 12.54 ms | kernels 12.53 ms, store 0.3 µs, rest 2.4 µs | 0.0 µs | – | 1.1 µs |
| 1024 × 128 | cpu | 0.6 µs | 1.1 µs | 2.08 ms | kernels 2.08 ms, store 0.2 µs, rest 1.3 µs | 0.0 µs | – | 0.8 µs |
| 1024 × 128 | gpu | 0.5 µs | 0.6 µs | 43 µs | upload 4.0 µs, record 1.5 µs, submit 32 µs, download 0.1 µs, rest 5.5 µs | 112 µs | 47 µs | 0.7 µs |

## chain

NURBS, Translate, a Sweep of a circle and Sample Curve, the control points move.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 45 µs | 81 µs | 247 µs | 3.0× slower | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 16 × 32 | 690 µs | 134 µs | 262 µs | 2.0× slower | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 256 × 32 | 11.38 ms | 1.71 ms | 332 µs | 5.2× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 4096 × 32 | 187.73 ms | 28.75 ms | 1.32 ms | 21.8× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 1 × 128 | 179 µs | 217 µs | 290 µs | 1.3× slower | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 16 × 128 | 2.38 ms | 416 µs | 316 µs | 1.3× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 256 × 128 | 39.43 ms | 5.80 ms | 579 µs | 10.0× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 1024 × 128 | 160.10 ms | 23.60 ms | 1.35 ms | 17.5× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 0.6 µs | 44 µs | kernels 44 µs, store 0.1 µs, rest 0.7 µs | 0.0 µs | – | 0.3 µs |
| 1 × 32 | cpu | 0.3 µs | 0.9 µs | 80 µs | kernels 78 µs, store 0.3 µs, rest 1.1 µs | 0.0 µs | – | 0.6 µs |
| 1 × 32 | gpu | 0.5 µs | 1.2 µs | 70 µs | upload 6.9 µs, record 6.5 µs, submit 48 µs, download 0.2 µs, rest 7.9 µs | 175 µs | 95 µs | 1.0 µs |
| 16 × 32 | cpu-1 | 0.5 µs | 0.7 µs | 689 µs | kernels 688 µs, store 0.2 µs, rest 0.8 µs | 0.0 µs | – | 0.4 µs |
| 16 × 32 | cpu | 1.2 µs | 1.2 µs | 130 µs | kernels 128 µs, store 0.3 µs, rest 1.5 µs | 0.0 µs | – | 0.7 µs |
| 16 × 32 | gpu | 0.9 µs | 1.2 µs | 73 µs | upload 7.1 µs, record 6.3 µs, submit 49 µs, download 0.2 µs, rest 9.9 µs | 187 µs | 108 µs | 1.0 µs |
| 256 × 32 | cpu-1 | 7.4 µs | 1.4 µs | 11.37 ms | kernels 11.37 ms, store 0.6 µs, rest 2.0 µs | 0.0 µs | – | 0.9 µs |
| 256 × 32 | cpu | 7.1 µs | 1.8 µs | 1.70 ms | kernels 1.70 ms, store 0.7 µs, rest 2.5 µs | 0.0 µs | – | 1.3 µs |
| 256 × 32 | gpu | 6.3 µs | 1.3 µs | 96 µs | upload 19 µs, record 6.3 µs, submit 53 µs, download 0.3 µs, rest 17 µs | 227 µs | 133 µs | 1.3 µs |
| 4096 × 32 | cpu-1 | 162 µs | 5.4 µs | 187.54 ms | kernels 187.53 ms, store 2.4 µs, rest 10 µs | 0.0 µs | – | 3.4 µs |
| 4096 × 32 | cpu | 158 µs | 5.3 µs | 28.59 ms | kernels 28.57 ms, store 2.9 µs, rest 11 µs | 0.0 µs | – | 4.1 µs |
| 4096 × 32 | gpu | 129 µs | 1.8 µs | 318 µs | upload 140 µs, record 7.1 µs, submit 65 µs, download 0.3 µs, rest 102 µs | 849 µs | 662 µs | 1.4 µs |
| 1 × 128 | cpu-1 | 0.3 µs | 0.7 µs | 178 µs | kernels 177 µs, store 0.2 µs, rest 0.8 µs | 0.0 µs | – | 0.4 µs |
| 1 × 128 | cpu | 0.4 µs | 1.1 µs | 216 µs | kernels 214 µs, store 0.3 µs, rest 1.4 µs | 0.0 µs | – | 0.7 µs |
| 1 × 128 | gpu | 0.6 µs | 1.2 µs | 70 µs | upload 6.8 µs, record 6.5 µs, submit 49 µs, download 0.2 µs, rest 8.1 µs | 217 µs | 147 µs | 1.0 µs |
| 16 × 128 | cpu-1 | 1.7 µs | 0.8 µs | 2.38 ms | kernels 2.38 ms, store 0.3 µs, rest 1.1 µs | 0.0 µs | – | 0.4 µs |
| 16 × 128 | cpu | 2.4 µs | 1.4 µs | 412 µs | kernels 410 µs, store 0.4 µs, rest 2.2 µs | 0.0 µs | – | 1.2 µs |
| 16 × 128 | gpu | 2.0 µs | 1.2 µs | 77 µs | upload 8.6 µs, record 6.5 µs, submit 51 µs, download 0.2 µs, rest 10 µs | 234 µs | 160 µs | 1.3 µs |
| 256 × 128 | cpu-1 | 39 µs | 3.3 µs | 39.39 ms | kernels 39.38 ms, store 1.0 µs, rest 3.7 µs | 0.0 µs | – | 1.7 µs |
| 256 × 128 | cpu | 34 µs | 2.4 µs | 5.76 ms | kernels 5.75 ms, store 1.2 µs, rest 4.1 µs | 0.0 µs | – | 1.6 µs |
| 256 × 128 | gpu | 23 µs | 1.4 µs | 125 µs | upload 42 µs, record 6.5 µs, submit 54 µs, download 0.3 µs, rest 22 µs | 428 µs | 269 µs | 1.2 µs |
| 1024 × 128 | cpu-1 | 155 µs | 5.1 µs | 159.95 ms | kernels 159.94 ms, store 2.2 µs, rest 9.8 µs | 0.0 µs | – | 2.7 µs |
| 1024 × 128 | cpu | 156 µs | 5.0 µs | 23.43 ms | kernels 23.42 ms, store 2.1 µs, rest 9.1 µs | 0.0 µs | – | 2.4 µs |
| 1024 × 128 | gpu | 120 µs | 1.9 µs | 269 µs | upload 139 µs, record 7.1 µs, submit 64 µs, download 0.3 µs, rest 60 µs | 937 µs | 749 µs | 1.3 µs |

## instances

A ring of 10 patches put onto 64 frames along every curve by reference, the control points move.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 45 µs | 86 µs | 230 µs | 2.7× slower | 3 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 16 × 32 | 695 µs | 219 µs | 251 µs | 1.1× slower | 3 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 256 × 32 | 11.21 ms | 1.59 ms | 295 µs | 5.4× faster | 3 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 1024 × 32 | 45.69 ms | 6.75 ms | 505 µs | 13.4× faster | 3 on gpu, 0 on cpu, 4 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 0.6 µs | 44 µs | kernels 44 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – | 0.3 µs |
| 1 × 32 | cpu | 0.4 µs | 0.9 µs | 85 µs | kernels 83 µs, store 0.2 µs, rest 0.8 µs | 0.0 µs | – | 0.7 µs |
| 1 × 32 | gpu | 0.5 µs | 1.1 µs | 63 µs | upload 7.4 µs, record 5.0 µs, submit 44 µs, download 0.2 µs, rest 7.0 µs | 164 µs | 84 µs | 1.1 µs |
| 16 × 32 | cpu-1 | 0.6 µs | 0.7 µs | 694 µs | kernels 693 µs, store 0.2 µs, rest 0.6 µs | 0.0 µs | – | 0.4 µs |
| 16 × 32 | cpu | 0.9 µs | 1.0 µs | 216 µs | kernels 214 µs, store 0.3 µs, rest 1.3 µs | 0.0 µs | – | 0.8 µs |
| 16 × 32 | gpu | 1.0 µs | 1.3 µs | 75 µs | upload 9.1 µs, record 5.7 µs, submit 50 µs, download 0.2 µs, rest 9.8 µs | 174 µs | 92 µs | 1.3 µs |
| 256 × 32 | cpu-1 | 7.5 µs | 1.2 µs | 11.20 ms | kernels 11.20 ms, store 0.4 µs, rest 1.4 µs | 0.0 µs | – | 0.6 µs |
| 256 × 32 | cpu | 7.2 µs | 1.5 µs | 1.58 ms | kernels 1.58 ms, store 0.5 µs, rest 1.9 µs | 0.0 µs | – | 1.2 µs |
| 256 × 32 | gpu | 6.4 µs | 1.2 µs | 96 µs | upload 24 µs, record 5.2 µs, submit 48 µs, download 0.2 µs, rest 18 µs | 190 µs | 107 µs | 1.5 µs |
| 1024 × 32 | cpu-1 | 34 µs | 2.6 µs | 45.65 ms | kernels 45.64 ms, store 0.9 µs, rest 2.8 µs | 0.1 µs | – | 1.4 µs |
| 1024 × 32 | cpu | 34 µs | 2.1 µs | 6.72 ms | kernels 6.71 ms, store 1.0 µs, rest 3.2 µs | 0.0 µs | – | 1.9 µs |
| 1024 × 32 | gpu | 25 µs | 1.3 µs | 152 µs | upload 55 µs, record 5.3 µs, submit 49 µs, download 0.3 µs, rest 42 µs | 328 µs | 166 µs | 1.3 µs |

## instances realized

The rings of `instances`, copied to where each one is.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 106 µs | 103 µs | 270 µs | 2.6× slower | 4 on gpu, 0 on cpu, 6 passes, 0 downloads |
| 16 × 32 | 1.70 ms | 298 µs | 282 µs | 1.1× faster | 4 on gpu, 0 on cpu, 6 passes, 0 downloads |
| 256 × 32 | 27.64 ms | 3.60 ms | 503 µs | 7.2× faster | 4 on gpu, 0 on cpu, 6 passes, 0 downloads |
| 1024 × 32 | 112.31 ms | 13.94 ms | 1.28 ms | 10.9× faster | 4 on gpu, 0 on cpu, 6 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 0.6 µs | 105 µs | kernels 104 µs, store 0.2 µs, rest 0.7 µs | 0.0 µs | – | 0.4 µs |
| 1 × 32 | cpu | 0.4 µs | 1.2 µs | 100 µs | kernels 99 µs, store 0.2 µs, rest 1.2 µs | 0.0 µs | – | 0.8 µs |
| 1 × 32 | gpu | 0.6 µs | 1.4 µs | 86 µs | upload 7.7 µs, record 8.3 µs, submit 59 µs, download 0.2 µs, rest 10 µs | 182 µs | 93 µs | 1.4 µs |
| 16 × 32 | cpu-1 | 0.6 µs | 0.9 µs | 1.69 ms | kernels 1.69 ms, store 0.3 µs, rest 0.9 µs | 0.0 µs | – | 0.5 µs |
| 16 × 32 | cpu | 1.2 µs | 1.5 µs | 295 µs | kernels 293 µs, store 0.3 µs, rest 1.6 µs | 0.0 µs | – | 1.1 µs |
| 16 × 32 | gpu | 0.9 µs | 1.3 µs | 79 µs | upload 8.0 µs, record 7.2 µs, submit 53 µs, download 0.2 µs, rest 10 µs | 200 µs | 112 µs | 1.3 µs |
| 256 × 32 | cpu-1 | 10 µs | 2.5 µs | 27.63 ms | kernels 27.63 ms, store 1.0 µs, rest 2.7 µs | 0.0 µs | – | 1.8 µs |
| 256 × 32 | cpu | 9.5 µs | 2.2 µs | 3.59 ms | kernels 3.58 ms, store 0.8 µs, rest 3.1 µs | 0.0 µs | – | 2.7 µs |
| 256 × 32 | gpu | 6.5 µs | 1.3 µs | 109 µs | upload 23 µs, record 7.4 µs, submit 57 µs, download 0.3 µs, rest 20 µs | 386 µs | 297 µs | 1.6 µs |
| 1024 × 32 | cpu-1 | 48 µs | 6.0 µs | 112.26 ms | kernels 112.25 ms, store 2.9 µs, rest 6.4 µs | 0.0 µs | – | 4.1 µs |
| 1024 × 32 | cpu | 46 µs | 4.3 µs | 13.89 ms | kernels 13.88 ms, store 1.5 µs, rest 5.3 µs | 0.0 µs | – | 3.4 µs |
| 1024 × 32 | gpu | 24 µs | 1.6 µs | 170 µs | upload 56 µs, record 7.9 µs, submit 61 µs, download 0.3 µs, rest 44 µs | 1.08 ms | 912 µs | 1.8 µs |

## for each

NURBS and Translate for every curve on its own, the control points of one curve move.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 16 × 8 | 9.7 µs | 16 µs | 226 µs | 14.6× slower | 2 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 256 × 8 | 43 µs | 46 µs | 271 µs | 6.0× slower | 2 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 4096 × 8 | 635 µs | 632 µs | 1.02 ms | 1.6× slower | 2 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 16384 × 8 | 3.28 ms | 3.31 ms | 3.72 ms | 1.1× slower | 2 on gpu, 0 on cpu, 9 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 16 × 8 | cpu-1 | 0.3 µs | 0.3 µs | 9.1 µs | kernels 5.7 µs, store 0.1 µs, rest 3.4 µs | 0.0 µs | – | 0.4 µs |
| 16 × 8 | cpu | 0.3 µs | 0.3 µs | 15 µs | kernels 10 µs, store 0.1 µs, rest 4.2 µs | 0.0 µs | – | 0.5 µs |
| 16 × 8 | gpu | 0.6 µs | 0.7 µs | 74 µs | upload 4.5 µs, record 6.3 µs, submit 53 µs, download 0.1 µs, rest 10 µs | 149 µs | 46 µs | 1.1 µs |
| 256 × 8 | cpu-1 | 1.7 µs | 0.3 µs | 41 µs | kernels 6.0 µs, store 0.1 µs, rest 35 µs | 0.0 µs | – | 1.7 µs |
| 256 × 8 | cpu | 1.7 µs | 0.3 µs | 44 µs | kernels 8.9 µs, store 0.1 µs, rest 35 µs | 0.0 µs | – | 1.8 µs |
| 256 × 8 | gpu | 1.9 µs | 0.7 µs | 116 µs | upload 6.4 µs, record 6.5 µs, submit 53 µs, download 0.1 µs, rest 49 µs | 151 µs | 47 µs | 2.7 µs |
| 4096 × 8 | cpu-1 | 22 µs | 0.4 µs | 612 µs | kernels 6.7 µs, store 0.1 µs, rest 606 µs | 0.0 µs | – | 28 µs |
| 4096 × 8 | cpu | 22 µs | 0.4 µs | 609 µs | kernels 10 µs, store 0.1 µs, rest 598 µs | 0.0 µs | – | 28 µs |
| 4096 × 8 | gpu | 23 µs | 0.9 µs | 835 µs | upload 44 µs, record 14 µs, submit 68 µs, download 0.1 µs, rest 710 µs | 160 µs | 54 µs | 36 µs |
| 16384 × 8 | cpu-1 | 152 µs | 1.0 µs | 3.12 ms | kernels 8.6 µs, store 0.2 µs, rest 3.11 ms | 0.0 µs | – | 161 µs |
| 16384 × 8 | cpu | 150 µs | 1.0 µs | 3.15 ms | kernels 12 µs, store 0.2 µs, rest 3.14 ms | 0.0 µs | – | 194 µs |
| 16384 × 8 | gpu | 129 µs | 1.2 µs | 3.42 ms | upload 125 µs, record 32 µs, submit 137 µs, download 0.1 µs, rest 3.12 ms | 151 µs | 78 µs | 178 µs |

## for each, all

NURBS and Translate for every curve on its own, the offset of all of them changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 16 × 8 | 8.9 µs | 67 µs | 176 µs | 2.6× slower | 16 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 8 | 123 µs | 494 µs | 320 µs | 1.5× faster | 256 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 4096 × 8 | 2.30 ms | 7.92 ms | 2.96 ms | 2.7× faster | 4096 on gpu, 0 on cpu, 6 passes, 0 downloads |
| 16384 × 8 | 13.91 ms | 41.62 ms | 14.92 ms | 2.8× faster | 16384 on gpu, 0 on cpu, 19 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 16 × 8 | cpu-1 | 0.2 µs | 0.3 µs | 8.5 µs | kernels 1.5 µs, store 0.5 µs, rest 6.5 µs | 0.0 µs | – | 0.9 µs |
| 16 × 8 | cpu | 0.3 µs | 0.4 µs | 66 µs | kernels 52 µs, store 0.9 µs, rest 12 µs | 0.0 µs | – | 1.7 µs |
| 16 × 8 | gpu | 0.4 µs | 0.6 µs | 68 µs | upload 2.8 µs, record 4.0 µs, submit 45 µs, download 0.4 µs, rest 16 µs | 107 µs | 2.5 µs | 2.6 µs |
| 256 × 8 | cpu-1 | 0.2 µs | 0.3 µs | 122 µs | kernels 26 µs, store 8.3 µs, rest 88 µs | 0.0 µs | – | 20 µs |
| 256 × 8 | cpu | 0.3 µs | 0.4 µs | 494 µs | kernels 359 µs, store 13 µs, rest 122 µs | 0.0 µs | – | 29 µs |
| 256 × 8 | gpu | 0.5 µs | 0.7 µs | 221 µs | upload 4.2 µs, record 5.0 µs, submit 47 µs, download 5.9 µs, rest 158 µs | 97 µs | 3.5 µs | 39 µs |
| 4096 × 8 | cpu-1 | 0.3 µs | 0.5 µs | 2.30 ms | kernels 568 µs, store 159 µs, rest 1.54 ms | 0.0 µs | – | 464 µs |
| 4096 × 8 | cpu | 0.8 µs | 1.0 µs | 7.92 ms | kernels 5.61 ms, store 246 µs, rest 2.05 ms | 0.0 µs | – | 634 µs |
| 4096 × 8 | gpu | 0.8 µs | 0.8 µs | 2.89 ms | upload 7.6 µs, record 36 µs, submit 106 µs, download 94 µs, rest 2.64 ms | 69 µs | 27 µs | 731 µs |
| 16384 × 8 | cpu-1 | 1.2 µs | 1.5 µs | 13.91 ms | kernels 3.89 ms, store 712 µs, rest 9.29 ms | 0.0 µs | – | 3.27 ms |
| 16384 × 8 | cpu | 2.1 µs | 2.5 µs | 41.61 ms | kernels 25.79 ms, store 1.17 ms, rest 14.55 ms | 0.0 µs | – | 4.66 ms |
| 16384 × 8 | gpu | 1.7 µs | 1.7 µs | 14.65 ms | upload 36 µs, record 267 µs, submit 300 µs, download 378 µs, rest 13.70 ms | 277 µs | 175 µs | 3.61 ms |

## mesh

Mesh of a swept circle, the tolerance changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 1.98 ms | 198 µs | 229 µs | 1.2× slower | 0 on gpu, 1 on cpu, 0 passes, 0 downloads |
| 16 × 32 | 30.97 ms | 3.36 ms | 3.55 ms | 1.1× slower | 0 on gpu, 1 on cpu, 0 passes, 0 downloads |
| 64 × 32 | 125.17 ms | 13.40 ms | 13.93 ms | 1.0× slower | 0 on gpu, 1 on cpu, 0 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 0.4 µs | 1.98 ms | kernels 1.98 ms, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.4 µs |
| 1 × 32 | cpu | 0.3 µs | 0.5 µs | 197 µs | kernels 197 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.5 µs |
| 1 × 32 | gpu | 0.4 µs | 0.8 µs | 223 µs | cpu 222 µs, rest 1.3 µs | 4.0 µs | 0.0 µs | 0.8 µs |
| 16 × 32 | cpu-1 | 0.2 µs | 0.8 µs | 30.97 ms | kernels 30.97 ms, store 0.1 µs, rest 0.6 µs | 0.0 µs | – | 0.4 µs |
| 16 × 32 | cpu | 0.7 µs | 1.7 µs | 3.36 ms | kernels 3.36 ms, store 0.2 µs, rest 1.3 µs | 0.0 µs | – | 1.3 µs |
| 16 × 32 | gpu | 0.8 µs | 1.4 µs | 3.54 ms | download 0.1 µs, cpu 3.54 ms, rest 1.8 µs | 9.2 µs | 0.0 µs | 1.5 µs |
| 64 × 32 | cpu-1 | 0.9 µs | 1.9 µs | 125.17 ms | kernels 125.16 ms, store 0.7 µs, rest 1.6 µs | 0.0 µs | – | 1.3 µs |
| 64 × 32 | cpu | 1.2 µs | 1.8 µs | 13.40 ms | kernels 13.39 ms, store 0.4 µs, rest 2.0 µs | 0.0 µs | – | 1.5 µs |
| 64 × 32 | gpu | 1.4 µs | 2.4 µs | 13.92 ms | download 0.1 µs, cpu 13.91 ms, rest 3.0 µs | 13 µs | 0.0 µs | 1.9 µs |

## fence

The picket fence of the editor, with as many pickets, gets a little longer.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 10 × 1 | 24 µs | 105 µs | 727 µs | 6.9× slower | 18 on gpu, 7 on cpu, 8 passes, 4 downloads |
| 100 × 1 | 57 µs | 241 µs | 765 µs | 3.2× slower | 18 on gpu, 7 on cpu, 8 passes, 4 downloads |
| 1000 × 1 | 380 µs | 821 µs | 1.19 ms | 1.4× slower | 18 on gpu, 7 on cpu, 8 passes, 4 downloads |
| 10000 × 1 | 3.75 ms | 5.06 ms | 4.53 ms | 1.1× faster | 18 on gpu, 7 on cpu, 8 passes, 4 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 10 × 1 | cpu-1 | 0.2 µs | 3.1 µs | 20 µs | kernels 16 µs, store 0.9 µs, rest 3.5 µs | 0.0 µs | – | 2.2 µs |
| 10 × 1 | cpu | 0.3 µs | 5.9 µs | 98 µs | kernels 91 µs, store 1.1 µs, rest 5.4 µs | 0.0 µs | – | 4.5 µs |
| 10 × 1 | gpu | 0.7 µs | 4.9 µs | 632 µs | upload 19 µs, record 12 µs, submit 124 µs, download 439 µs, cpu 14 µs, rest 22 µs | 88 µs | 18 µs | 4.7 µs |
| 100 × 1 | cpu-1 | 0.1 µs | 3.1 µs | 54 µs | kernels 49 µs, store 0.8 µs, rest 3.6 µs | 0.0 µs | – | 2.3 µs |
| 100 × 1 | cpu | 0.4 µs | 6.5 µs | 234 µs | kernels 227 µs, store 1.1 µs, rest 5.6 µs | 0.0 µs | – | 5.5 µs |
| 100 × 1 | gpu | 0.6 µs | 4.9 µs | 666 µs | upload 21 µs, record 12 µs, submit 125 µs, download 438 µs, cpu 45 µs, rest 22 µs | 93 µs | 18 µs | 4.5 µs |
| 1000 × 1 | cpu-1 | 0.2 µs | 3.8 µs | 376 µs | kernels 371 µs, store 1.0 µs, rest 3.7 µs | 0.0 µs | – | 2.8 µs |
| 1000 × 1 | cpu | 0.5 µs | 5.6 µs | 814 µs | kernels 808 µs, store 1.1 µs, rest 5.7 µs | 0.0 µs | – | 5.9 µs |
| 1000 × 1 | gpu | 0.7 µs | 5.2 µs | 994 µs | upload 35 µs, record 12 µs, submit 128 µs, download 432 µs, cpu 355 µs, rest 25 µs | 185 µs | 19 µs | 5.5 µs |
| 10000 × 1 | cpu-1 | 0.3 µs | 4.5 µs | 3.74 ms | kernels 3.74 ms, store 1.5 µs, rest 5.1 µs | 0.0 µs | – | 3.6 µs |
| 10000 × 1 | cpu | 0.7 µs | 7.0 µs | 5.06 ms | kernels 5.05 ms, store 1.7 µs, rest 8.0 µs | 0.0 µs | – | 7.0 µs |
| 10000 × 1 | gpu | 0.6 µs | 5.4 µs | 4.25 ms | upload 136 µs, record 13 µs, submit 145 µs, download 448 µs, cpu 3.46 ms, rest 38 µs | 266 µs | 45 µs | 6.0 µs |

## mostly constant

As many curves of constant points, each moved by a constant and an output of its own, and one curve moved by the only parameter, which changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 16 × 8 | 0.7 µs | 2.2 µs | 98 µs | 44.5× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 8 | 0.8 µs | 2.6 µs | 99 µs | 38.0× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 4096 × 8 | 1.1 µs | 3.9 µs | 100 µs | 25.7× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16384 × 8 | 1.4 µs | 6.3 µs | 102 µs | 16.2× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 16 × 8 | cpu-1 | 0.1 µs | 0.2 µs | 0.3 µs | kernels 0.1 µs, rest 0.2 µs | 0.0 µs | – | 0.1 µs |
| 16 × 8 | cpu | 0.2 µs | 0.3 µs | 1.7 µs | kernels 1.3 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 16 × 8 | gpu | 0.4 µs | 0.6 µs | 38 µs | upload 2.8 µs, record 1.7 µs, submit 32 µs, rest 1.9 µs | 59 µs | 1.6 µs | 0.4 µs |
| 256 × 8 | cpu-1 | 0.2 µs | 0.2 µs | 0.4 µs | kernels 0.2 µs, rest 0.2 µs | 0.0 µs | – | 0.2 µs |
| 256 × 8 | cpu | 0.4 µs | 0.3 µs | 2.0 µs | kernels 1.6 µs, rest 0.4 µs | 0.0 µs | – | 0.3 µs |
| 256 × 8 | gpu | 0.4 µs | 0.6 µs | 38 µs | upload 2.8 µs, record 1.7 µs, submit 32 µs, rest 1.9 µs | 60 µs | 1.5 µs | 0.4 µs |
| 4096 × 8 | cpu-1 | 0.2 µs | 0.3 µs | 0.6 µs | kernels 0.2 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 4096 × 8 | cpu | 0.4 µs | 0.4 µs | 3.0 µs | kernels 2.4 µs, store 0.1 µs, rest 0.6 µs | 0.0 µs | – | 0.3 µs |
| 4096 × 8 | gpu | 0.5 µs | 0.6 µs | 39 µs | upload 2.8 µs, record 1.7 µs, submit 32 µs, rest 2.1 µs | 60 µs | 1.6 µs | 0.4 µs |
| 16384 × 8 | cpu-1 | 0.2 µs | 0.3 µs | 0.9 µs | kernels 0.2 µs, rest 0.7 µs | 0.0 µs | – | 0.2 µs |
| 16384 × 8 | cpu | 0.4 µs | 0.4 µs | 5.6 µs | kernels 4.5 µs, store 0.1 µs, rest 1.0 µs | 0.0 µs | – | 0.4 µs |
| 16384 × 8 | gpu | 0.5 µs | 0.6 µs | 42 µs | upload 3.0 µs, record 1.8 µs, submit 34 µs, rest 2.7 µs | 60 µs | 1.8 µs | 0.6 µs |
