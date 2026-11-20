# Benchmarks

windows, AMD64 Family 25 Model 97 Stepping 2, AuthenticAMD with 16 threads, AMD Radeon RX 7800 XT on vulkan, driver AMD proprietary driver 26.11.2 (LLPC). Commit 8f4c27c.

The time from an edit until its result is computed, the median of many edits. Below each table, where it goes: setting the new value (which hashes it) and planning are the same for every backend, `flush` is the executor, then waiting for the device. The device column is the time the GPU spends on compute passes, by its timestamps. Ending the frame afterwards frees what is not used anymore, it is not in the total.

## nurbs

NURBS Curve, the control points move.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 39 µs | 39 µs | 187 µs | 4.8× slower | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 16 × 32 | 601 µs | 135 µs | 223 µs | 1.6× slower | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 32 | 9.65 ms | 1.21 ms | 247 µs | 4.9× faster | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 4096 × 32 | 165.41 ms | 20.68 ms | 905 µs | 22.8× faster | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 1 × 128 | 166 µs | 168 µs | 233 µs | 1.4× slower | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 16 × 128 | 2.65 ms | 319 µs | 259 µs | 1.2× faster | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 128 | 43.96 ms | 5.17 ms | 499 µs | 10.4× faster | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 1024 × 128 | 178.20 ms | 20.44 ms | 1.03 ms | 19.8× faster | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 0.2 µs | 38 µs | kernels 38 µs, rest 0.3 µs | 0.0 µs | – | 0.1 µs |
| 1 × 32 | cpu | 0.2 µs | 0.3 µs | 38 µs | kernels 38 µs, rest 0.3 µs | 0.0 µs | – | 0.1 µs |
| 1 × 32 | gpu | 0.5 µs | 0.6 µs | 48 µs | upload 4.7 µs, record 2.9 µs, submit 37 µs, download 0.1 µs, rest 3.7 µs | 136 µs | 71 µs | 0.7 µs |
| 16 × 32 | cpu-1 | 0.5 µs | 0.4 µs | 600 µs | kernels 600 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 16 × 32 | cpu | 1.2 µs | 0.6 µs | 134 µs | kernels 133 µs, store 0.1 µs, rest 0.6 µs | 0.0 µs | – | 0.4 µs |
| 16 × 32 | gpu | 1.2 µs | 0.8 µs | 65 µs | upload 7.0 µs, record 3.3 µs, submit 48 µs, download 0.1 µs, rest 6.2 µs | 156 µs | 80 µs | 0.9 µs |
| 256 × 32 | cpu-1 | 8.9 µs | 0.8 µs | 9.64 ms | kernels 9.64 ms, store 0.2 µs, rest 1.0 µs | 0.0 µs | – | 0.5 µs |
| 256 × 32 | cpu | 7.6 µs | 1.0 µs | 1.20 ms | kernels 1.20 ms, store 0.1 µs, rest 1.0 µs | 0.0 µs | – | 0.7 µs |
| 256 × 32 | gpu | 6.3 µs | 0.6 µs | 73 µs | upload 18 µs, record 3.0 µs, submit 42 µs, download 0.1 µs, rest 9.8 µs | 166 µs | 91 µs | 0.9 µs |
| 4096 × 32 | cpu-1 | 176 µs | 3.1 µs | 165.25 ms | kernels 165.25 ms, store 0.7 µs, rest 4.8 µs | 0.0 µs | – | 2.2 µs |
| 4096 × 32 | cpu | 164 µs | 2.5 µs | 20.50 ms | kernels 20.50 ms, store 1.1 µs, rest 4.8 µs | 0.0 µs | – | 2.9 µs |
| 4096 × 32 | gpu | 123 µs | 0.8 µs | 259 µs | upload 136 µs, record 3.6 µs, submit 51 µs, download 0.1 µs, rest 67 µs | 506 µs | 347 µs | 0.8 µs |
| 1 × 128 | cpu-1 | 0.3 µs | 0.3 µs | 166 µs | kernels 166 µs, rest 0.3 µs | 0.0 µs | – | 0.2 µs |
| 1 × 128 | cpu | 0.3 µs | 0.3 µs | 168 µs | kernels 168 µs, rest 0.3 µs | 0.0 µs | – | 0.2 µs |
| 1 × 128 | gpu | 0.6 µs | 0.5 µs | 46 µs | upload 4.5 µs, record 2.8 µs, submit 35 µs, download 0.1 µs, rest 3.6 µs | 185 µs | 123 µs | 0.7 µs |
| 16 × 128 | cpu-1 | 1.7 µs | 0.4 µs | 2.65 ms | kernels 2.65 ms, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 16 × 128 | cpu | 2.4 µs | 0.5 µs | 316 µs | kernels 315 µs, store 0.1 µs, rest 0.6 µs | 0.0 µs | – | 0.4 µs |
| 16 × 128 | gpu | 2.0 µs | 0.5 µs | 51 µs | upload 7.0 µs, record 3.0 µs, submit 36 µs, download 0.1 µs, rest 5.1 µs | 205 µs | 132 µs | 0.6 µs |
| 256 × 128 | cpu-1 | 33 µs | 1.4 µs | 43.93 ms | kernels 43.93 ms, store 0.4 µs, rest 1.5 µs | 0.0 µs | – | 0.9 µs |
| 256 × 128 | cpu | 36 µs | 1.3 µs | 5.13 ms | kernels 5.12 ms, store 0.3 µs, rest 1.5 µs | 0.0 µs | – | 0.9 µs |
| 256 × 128 | gpu | 27 µs | 0.8 µs | 96 µs | upload 40 µs, record 3.3 µs, submit 38 µs, download 0.1 µs, rest 14 µs | 376 µs | 185 µs | 0.8 µs |
| 1024 × 128 | cpu-1 | 164 µs | 2.6 µs | 178.03 ms | kernels 178.03 ms, store 0.5 µs, rest 4.0 µs | 0.0 µs | – | 2.0 µs |
| 1024 × 128 | cpu | 156 µs | 1.5 µs | 20.30 ms | kernels 20.30 ms, store 0.8 µs, rest 2.2 µs | 0.0 µs | – | 1.4 µs |
| 1024 × 128 | gpu | 131 µs | 0.9 µs | 244 µs | upload 138 µs, record 3.8 µs, submit 55 µs, download 0.1 µs, rest 47 µs | 642 µs | 471 µs | 0.9 µs |

## translate

Translate Curve of the curves, the offset changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 0.7 µs | 3.9 µs | 95 µs | 24.4× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 32 | 1.4 µs | 9.6 µs | 95 µs | 9.9× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 32 | 12 µs | 16 µs | 96 µs | 5.8× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 4096 × 32 | 518 µs | 149 µs | 177 µs | 1.2× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 1 × 128 | 1.0 µs | 6.0 µs | 97 µs | 16.1× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 128 | 3.7 µs | 14 µs | 88 µs | 6.4× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 128 | 51 µs | 30 µs | 119 µs | 4.0× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 1024 × 128 | 588 µs | 160 µs | 182 µs | 1.1× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 1 × 32 | cpu-1 | 0.1 µs | 0.2 µs | 0.4 µs | kernels 0.1 µs, rest 0.2 µs | 0.0 µs | – | 0.1 µs |
| 1 × 32 | cpu | 0.3 µs | 0.3 µs | 3.3 µs | kernels 2.9 µs, rest 0.3 µs | 0.0 µs | – | 0.2 µs |
| 1 × 32 | gpu | 0.4 µs | 0.5 µs | 36 µs | upload 2.7 µs, record 1.6 µs, submit 30 µs, rest 1.7 µs | 58 µs | 1.8 µs | 0.6 µs |
| 16 × 32 | cpu-1 | 0.1 µs | 0.2 µs | 1.0 µs | kernels 0.7 µs, rest 0.3 µs | 0.0 µs | – | 0.2 µs |
| 16 × 32 | cpu | 0.3 µs | 0.4 µs | 8.8 µs | kernels 8.3 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.3 µs |
| 16 × 32 | gpu | 0.4 µs | 0.5 µs | 36 µs | upload 2.7 µs, record 1.6 µs, submit 30 µs, rest 1.7 µs | 58 µs | 2.5 µs | 0.6 µs |
| 256 × 32 | cpu-1 | 0.2 µs | 0.4 µs | 12 µs | kernels 11 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.3 µs |
| 256 × 32 | cpu | 0.3 µs | 0.4 µs | 16 µs | kernels 15 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.3 µs |
| 256 × 32 | gpu | 0.4 µs | 0.5 µs | 36 µs | upload 2.7 µs, record 1.6 µs, submit 30 µs, rest 1.7 µs | 59 µs | 6.5 µs | 0.6 µs |
| 4096 × 32 | cpu-1 | 0.6 µs | 0.8 µs | 516 µs | kernels 514 µs, store 0.4 µs, rest 1.2 µs | 0.0 µs | – | 1.1 µs |
| 4096 × 32 | cpu | 0.6 µs | 0.8 µs | 146 µs | kernels 145 µs, store 0.2 µs, rest 1.0 µs | 0.0 µs | – | 0.9 µs |
| 4096 × 32 | gpu | 0.4 µs | 0.5 µs | 36 µs | upload 2.7 µs, record 1.6 µs, submit 30 µs, rest 1.7 µs | 140 µs | 72 µs | 0.5 µs |
| 1 × 128 | cpu-1 | 0.1 µs | 0.2 µs | 0.6 µs | kernels 0.3 µs, rest 0.3 µs | 0.0 µs | – | 0.1 µs |
| 1 × 128 | cpu | 0.3 µs | 0.4 µs | 5.3 µs | kernels 4.9 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 1 × 128 | gpu | 0.4 µs | 0.5 µs | 36 µs | upload 2.7 µs, record 1.6 µs, submit 30 µs, rest 1.8 µs | 54 µs | 2.1 µs | 0.6 µs |
| 16 × 128 | cpu-1 | 0.1 µs | 0.3 µs | 3.2 µs | kernels 2.9 µs, rest 0.3 µs | 0.0 µs | – | 0.2 µs |
| 16 × 128 | cpu | 0.3 µs | 0.4 µs | 13 µs | kernels 12 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.3 µs |
| 16 × 128 | gpu | 0.4 µs | 0.5 µs | 35 µs | upload 2.7 µs, record 1.6 µs, submit 29 µs, rest 1.7 µs | 52 µs | 2.6 µs | 0.5 µs |
| 256 × 128 | cpu-1 | 0.2 µs | 0.4 µs | 50 µs | kernels 50 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 256 × 128 | cpu | 0.4 µs | 0.6 µs | 29 µs | kernels 28 µs, store 0.1 µs, rest 0.6 µs | 0.0 µs | – | 0.4 µs |
| 256 × 128 | gpu | 0.4 µs | 0.5 µs | 36 µs | upload 2.7 µs, record 1.6 µs, submit 30 µs, rest 1.7 µs | 80 µs | 22 µs | 0.5 µs |
| 1024 × 128 | cpu-1 | 0.7 µs | 0.8 µs | 587 µs | kernels 585 µs, store 0.3 µs, rest 1.3 µs | 0.0 µs | – | 0.7 µs |
| 1024 × 128 | cpu | 0.7 µs | 0.8 µs | 159 µs | kernels 157 µs, store 0.2 µs, rest 1.1 µs | 0.0 µs | – | 0.6 µs |
| 1024 × 128 | gpu | 0.4 µs | 0.5 µs | 36 µs | upload 2.7 µs, record 1.6 µs, submit 30 µs, rest 1.7 µs | 145 µs | 78 µs | 0.5 µs |

## sweep circle

Sweep of a circle along the curves, the radius changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 1.6 µs | 2.0 µs | 107 µs | 53.5× slower | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 16 × 32 | 4.4 µs | 8.4 µs | 113 µs | 13.4× slower | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 32 | 53 µs | 102 µs | 123 µs | 1.2× slower | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 4096 × 32 | 5.46 ms | 3.97 ms | 229 µs | 17.4× faster | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 1 × 128 | 2.4 µs | 2.1 µs | 108 µs | 51.3× slower | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 16 × 128 | 15 µs | 48 µs | 110 µs | 2.3× slower | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 128 | 626 µs | 666 µs | 145 µs | 4.6× faster | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 1024 × 128 | 4.00 ms | 3.14 ms | 229 µs | 13.7× faster | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 1 × 32 | cpu-1 | 0.1 µs | 0.4 µs | 1.0 µs | kernels 0.6 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 1 × 32 | cpu | 0.2 µs | 0.6 µs | 1.2 µs | kernels 0.7 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – | 0.3 µs |
| 1 × 32 | gpu | 0.4 µs | 0.6 µs | 43 µs | upload 3.6 µs, record 2.6 µs, submit 34 µs, download 0.1 µs, rest 2.8 µs | 63 µs | 3.0 µs | 0.6 µs |
| 16 × 32 | cpu-1 | 0.1 µs | 0.3 µs | 3.9 µs | kernels 3.5 µs, store 0.1 µs, rest 0.3 µs | 0.0 µs | – | 0.2 µs |
| 16 × 32 | cpu | 0.2 µs | 0.5 µs | 7.6 µs | kernels 6.9 µs, store 0.1 µs, rest 0.6 µs | 0.0 µs | – | 0.4 µs |
| 16 × 32 | gpu | 0.5 µs | 0.7 µs | 44 µs | upload 3.8 µs, record 2.7 µs, submit 35 µs, download 0.1 µs, rest 3.2 µs | 64 µs | 4.8 µs | 0.7 µs |
| 256 × 32 | cpu-1 | 0.2 µs | 0.6 µs | 53 µs | kernels 52 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – | 0.3 µs |
| 256 × 32 | cpu | 0.4 µs | 0.8 µs | 100 µs | kernels 99 µs, store 0.1 µs, rest 0.8 µs | 0.0 µs | – | 0.8 µs |
| 256 × 32 | gpu | 0.4 µs | 0.7 µs | 46 µs | upload 3.8 µs, record 2.7 µs, submit 35 µs, download 0.1 µs, rest 4.1 µs | 75 µs | 11 µs | 0.7 µs |
| 4096 × 32 | cpu-1 | 2.1 µs | 4.1 µs | 5.45 ms | kernels 5.44 ms, store 1.1 µs, rest 3.8 µs | 0.0 µs | – | 2.8 µs |
| 4096 × 32 | cpu | 2.7 µs | 4.5 µs | 3.96 ms | kernels 3.96 ms, store 1.0 µs, rest 4.2 µs | 0.0 µs | – | 3.0 µs |
| 4096 × 32 | gpu | 0.5 µs | 0.8 µs | 62 µs | upload 5.3 µs, record 3.1 µs, submit 41 µs, download 0.1 µs, rest 13 µs | 165 µs | 93 µs | 0.8 µs |
| 1 × 128 | cpu-1 | 0.2 µs | 0.4 µs | 1.8 µs | kernels 1.3 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 1 × 128 | cpu | 0.1 µs | 0.4 µs | 1.5 µs | kernels 1.1 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 1 × 128 | gpu | 0.4 µs | 0.6 µs | 43 µs | upload 3.6 µs, record 2.6 µs, submit 34 µs, download 0.1 µs, rest 2.9 µs | 63 µs | 3.5 µs | 0.6 µs |
| 16 × 128 | cpu-1 | 0.2 µs | 0.5 µs | 14 µs | kernels 14 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – | 0.3 µs |
| 16 × 128 | cpu | 0.4 µs | 0.8 µs | 47 µs | kernels 46 µs, store 0.1 µs, rest 0.8 µs | 0.0 µs | – | 0.6 µs |
| 16 × 128 | gpu | 0.4 µs | 0.7 µs | 44 µs | upload 3.7 µs, record 2.7 µs, submit 34 µs, download 0.1 µs, rest 3.1 µs | 65 µs | 5.5 µs | 0.6 µs |
| 256 × 128 | cpu-1 | 0.3 µs | 0.7 µs | 624 µs | kernels 624 µs, store 0.2 µs, rest 0.8 µs | 0.0 µs | – | 0.4 µs |
| 256 × 128 | cpu | 0.6 µs | 1.2 µs | 664 µs | kernels 663 µs, store 0.3 µs, rest 1.1 µs | 0.0 µs | – | 1.0 µs |
| 256 × 128 | gpu | 0.4 µs | 0.7 µs | 47 µs | upload 4.0 µs, record 2.8 µs, submit 36 µs, download 0.1 µs, rest 4.1 µs | 95 µs | 30 µs | 0.7 µs |
| 1024 × 128 | cpu-1 | 1.5 µs | 2.9 µs | 3.99 ms | kernels 3.99 ms, store 0.8 µs, rest 3.2 µs | 0.0 µs | – | 2.0 µs |
| 1024 × 128 | cpu | 1.6 µs | 2.8 µs | 3.14 ms | kernels 3.13 ms, store 0.6 µs, rest 3.0 µs | 0.0 µs | – | 1.7 µs |
| 1024 × 128 | gpu | 0.5 µs | 0.8 µs | 55 µs | upload 4.6 µs, record 3.2 µs, submit 41 µs, download 0.1 µs, rest 6.3 µs | 172 µs | 100 µs | 0.8 µs |

## sweep square

Sweep of a square curve along the curves, the square changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 12 µs | 13 µs | 183 µs | 14.3× slower | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 16 × 32 | 54 µs | 71 µs | 172 µs | 2.4× slower | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 256 × 32 | 1.50 ms | 1.31 ms | 214 µs | 6.1× faster | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 4096 × 32 | 64.69 ms | 48.10 ms | 519 µs | 92.7× faster | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 1 × 128 | 19 µs | 18 µs | 180 µs | 9.7× slower | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 16 × 128 | 413 µs | 417 µs | 207 µs | 2.0× faster | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 256 × 128 | 7.70 ms | 6.49 ms | 345 µs | 18.8× faster | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 1024 × 128 | 47.74 ms | 38.20 ms | 704 µs | 54.3× faster | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 0.5 µs | 12 µs | kernels 11 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – | 0.3 µs |
| 1 × 32 | cpu | 0.2 µs | 0.6 µs | 12 µs | kernels 11 µs, store 0.1 µs, rest 0.7 µs | 0.0 µs | – | 0.4 µs |
| 1 × 32 | gpu | 0.5 µs | 0.8 µs | 60 µs | upload 5.7 µs, record 4.5 µs, submit 44 µs, download 0.2 µs, rest 5.9 µs | 121 µs | 47 µs | 1.0 µs |
| 16 × 32 | cpu-1 | 0.3 µs | 0.5 µs | 54 µs | kernels 53 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – | 0.4 µs |
| 16 × 32 | cpu | 0.4 µs | 0.9 µs | 69 µs | kernels 68 µs, store 0.1 µs, rest 0.8 µs | 0.0 µs | – | 0.6 µs |
| 16 × 32 | gpu | 0.5 µs | 0.7 µs | 54 µs | upload 5.5 µs, record 4.0 µs, submit 39 µs, download 0.1 µs, rest 5.0 µs | 116 µs | 51 µs | 0.9 µs |
| 256 × 32 | cpu-1 | 0.4 µs | 0.8 µs | 1.50 ms | kernels 1.50 ms, store 0.2 µs, rest 0.8 µs | 0.0 µs | – | 0.6 µs |
| 256 × 32 | cpu | 0.7 µs | 1.3 µs | 1.31 ms | kernels 1.30 ms, store 0.3 µs, rest 1.1 µs | 0.0 µs | – | 1.1 µs |
| 256 × 32 | gpu | 0.5 µs | 0.7 µs | 56 µs | upload 5.7 µs, record 4.0 µs, submit 40 µs, download 0.1 µs, rest 6.2 µs | 156 µs | 88 µs | 0.9 µs |
| 4096 × 32 | cpu-1 | 2.9 µs | 3.6 µs | 64.68 ms | kernels 64.67 ms, store 2.1 µs, rest 3.8 µs | 0.1 µs | – | 4.0 µs |
| 4096 × 32 | cpu | 2.9 µs | 3.7 µs | 48.09 ms | kernels 48.08 ms, store 2.4 µs, rest 4.3 µs | 0.0 µs | – | 3.7 µs |
| 4096 × 32 | gpu | 0.5 µs | 0.8 µs | 67 µs | upload 6.6 µs, record 4.0 µs, submit 41 µs, download 0.1 µs, rest 16 µs | 448 µs | 386 µs | 1.0 µs |
| 1 × 128 | cpu-1 | 0.2 µs | 0.4 µs | 18 µs | kernels 18 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.3 µs |
| 1 × 128 | cpu | 0.2 µs | 0.4 µs | 18 µs | kernels 17 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.3 µs |
| 1 × 128 | gpu | 0.5 µs | 0.8 µs | 60 µs | upload 5.6 µs, record 4.4 µs, submit 44 µs, download 0.1 µs, rest 5.7 µs | 117 µs | 48 µs | 1.0 µs |
| 16 × 128 | cpu-1 | 0.4 µs | 0.6 µs | 412 µs | kernels 412 µs, store 0.2 µs, rest 0.7 µs | 0.0 µs | – | 0.4 µs |
| 16 × 128 | cpu | 0.6 µs | 1.1 µs | 415 µs | kernels 414 µs, store 0.3 µs, rest 1.0 µs | 0.0 µs | – | 1.0 µs |
| 16 × 128 | gpu | 0.7 µs | 1.1 µs | 73 µs | upload 7.7 µs, record 5.4 µs, submit 52 µs, download 0.2 µs, rest 8.0 µs | 131 µs | 59 µs | 1.4 µs |
| 256 × 128 | cpu-1 | 2.8 µs | 4.5 µs | 7.69 ms | kernels 7.68 ms, store 1.0 µs, rest 4.1 µs | 0.0 µs | – | 3.1 µs |
| 256 × 128 | cpu | 2.7 µs | 3.8 µs | 6.48 ms | kernels 6.48 ms, store 0.9 µs, rest 3.9 µs | 0.0 µs | – | 2.4 µs |
| 256 × 128 | gpu | 0.5 µs | 0.8 µs | 56 µs | upload 5.8 µs, record 4.1 µs, submit 40 µs, download 0.1 µs, rest 6.3 µs | 287 µs | 212 µs | 1.0 µs |
| 1024 × 128 | cpu-1 | 3.8 µs | 4.9 µs | 47.73 ms | kernels 47.72 ms, store 2.0 µs, rest 4.6 µs | 0.1 µs | – | 3.8 µs |
| 1024 × 128 | cpu | 2.4 µs | 3.4 µs | 38.20 ms | kernels 38.19 ms, store 1.9 µs, rest 3.6 µs | 0.1 µs | – | 3.5 µs |
| 1024 × 128 | gpu | 0.5 µs | 0.8 µs | 59 µs | upload 5.9 µs, record 4.0 µs, submit 41 µs, download 0.1 µs, rest 8.1 µs | 642 µs | 492 µs | 1.0 µs |

## sample

Sample Curve, 64 frames per curve at places which change.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 36 µs | 35 µs | 110 µs | 3.1× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 32 | 567 µs | 95 µs | 111 µs | 1.2× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 32 | 9.40 ms | 1.19 ms | 113 µs | 10.6× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 4096 × 32 | 154.43 ms | 18.54 ms | 219 µs | 84.5× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 1 × 128 | 40 µs | 35 µs | 110 µs | 3.1× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 128 | 619 µs | 137 µs | 114 µs | 1.2× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 128 | 9.43 ms | 1.17 ms | 116 µs | 10.1× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 1024 × 128 | 39.75 ms | 4.65 ms | 160 µs | 29.1× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 0.3 µs | 36 µs | kernels 36 µs, rest 0.3 µs | 0.0 µs | – | 0.2 µs |
| 1 × 32 | cpu | 0.2 µs | 0.3 µs | 35 µs | kernels 35 µs, rest 0.3 µs | 0.0 µs | – | 0.2 µs |
| 1 × 32 | gpu | 0.5 µs | 0.6 µs | 38 µs | upload 3.7 µs, record 1.6 µs, submit 30 µs, download 0.1 µs, rest 2.3 µs | 71 µs | 11 µs | 0.7 µs |
| 16 × 32 | cpu-1 | 0.2 µs | 0.3 µs | 566 µs | kernels 566 µs, store 0.1 µs, rest 0.3 µs | 0.0 µs | – | 0.2 µs |
| 16 × 32 | cpu | 0.4 µs | 0.5 µs | 94 µs | kernels 94 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – | 0.4 µs |
| 16 × 32 | gpu | 0.5 µs | 0.6 µs | 38 µs | upload 3.9 µs, record 1.6 µs, submit 30 µs, download 0.1 µs, rest 2.4 µs | 72 µs | 11 µs | 0.7 µs |
| 256 × 32 | cpu-1 | 0.9 µs | 1.4 µs | 9.39 ms | kernels 9.39 ms, store 0.3 µs, rest 1.2 µs | 0.0 µs | – | 0.8 µs |
| 256 × 32 | cpu | 0.7 µs | 0.9 µs | 1.19 ms | kernels 1.19 ms, store 0.2 µs, rest 0.9 µs | 0.0 µs | – | 0.8 µs |
| 256 × 32 | gpu | 0.5 µs | 0.6 µs | 40 µs | upload 3.8 µs, record 1.6 µs, submit 31 µs, download 0.1 µs, rest 3.5 µs | 71 µs | 12 µs | 0.7 µs |
| 4096 × 32 | cpu-1 | 3.8 µs | 3.6 µs | 154.42 ms | kernels 154.41 ms, store 1.2 µs, rest 5.7 µs | 0.0 µs | – | 2.6 µs |
| 4096 × 32 | cpu | 3.2 µs | 3.9 µs | 18.53 ms | kernels 18.53 ms, store 1.1 µs, rest 5.2 µs | 0.0 µs | – | 2.7 µs |
| 4096 × 32 | gpu | 0.6 µs | 0.7 µs | 61 µs | upload 5.3 µs, record 1.9 µs, submit 37 µs, download 0.1 µs, rest 17 µs | 158 µs | 79 µs | 0.8 µs |
| 1 × 128 | cpu-1 | 0.2 µs | 0.3 µs | 39 µs | kernels 39 µs, store 0.1 µs, rest 0.3 µs | 0.0 µs | – | 0.2 µs |
| 1 × 128 | cpu | 0.2 µs | 0.3 µs | 35 µs | kernels 35 µs, store 0.1 µs, rest 0.3 µs | 0.0 µs | – | 0.2 µs |
| 1 × 128 | gpu | 0.5 µs | 0.6 µs | 38 µs | upload 3.8 µs, record 1.6 µs, submit 30 µs, download 0.1 µs, rest 2.3 µs | 70 µs | 11 µs | 0.7 µs |
| 16 × 128 | cpu-1 | 0.2 µs | 0.3 µs | 619 µs | kernels 619 µs, store 0.1 µs, rest 0.3 µs | 0.0 µs | – | 0.2 µs |
| 16 × 128 | cpu | 0.5 µs | 0.6 µs | 136 µs | kernels 135 µs, store 0.1 µs, rest 0.6 µs | 0.0 µs | – | 0.5 µs |
| 16 × 128 | gpu | 0.5 µs | 0.6 µs | 39 µs | upload 4.0 µs, record 1.6 µs, submit 31 µs, download 0.1 µs, rest 2.5 µs | 72 µs | 12 µs | 0.7 µs |
| 256 × 128 | cpu-1 | 0.7 µs | 1.0 µs | 9.43 ms | kernels 9.42 ms, store 0.3 µs, rest 1.1 µs | 0.0 µs | – | 0.8 µs |
| 256 × 128 | cpu | 0.7 µs | 0.9 µs | 1.17 ms | kernels 1.17 ms, store 0.2 µs, rest 1.1 µs | 0.0 µs | – | 0.7 µs |
| 256 × 128 | gpu | 0.5 µs | 0.6 µs | 40 µs | upload 3.8 µs, record 1.5 µs, submit 31 µs, download 0.1 µs, rest 3.5 µs | 75 µs | 15 µs | 0.7 µs |
| 1024 × 128 | cpu-1 | 2.7 µs | 2.8 µs | 39.75 ms | kernels 39.74 ms, store 0.7 µs, rest 4.1 µs | 0.0 µs | – | 1.9 µs |
| 1024 × 128 | cpu | 1.4 µs | 1.7 µs | 4.65 ms | kernels 4.65 ms, store 0.4 µs, rest 2.2 µs | 0.0 µs | – | 1.2 µs |
| 1024 × 128 | gpu | 0.5 µs | 0.6 µs | 43 µs | upload 4.0 µs, record 1.6 µs, submit 32 µs, download 0.1 µs, rest 5.4 µs | 113 µs | 47 µs | 0.7 µs |

## chain

NURBS, Translate, a Sweep of a circle and Sample Curve, the control points move.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 78 µs | 133 µs | 236 µs | 1.8× slower | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 16 × 32 | 1.22 ms | 204 µs | 247 µs | 1.2× slower | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 256 × 32 | 19.99 ms | 2.51 ms | 304 µs | 8.3× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 4096 × 32 | 312.53 ms | 41.11 ms | 1.24 ms | 33.1× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 1 × 128 | 205 µs | 304 µs | 283 µs | 1.1× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 16 × 128 | 3.28 ms | 502 µs | 305 µs | 1.6× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 256 × 128 | 53.24 ms | 7.47 ms | 572 µs | 13.1× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 1024 × 128 | 215.23 ms | 28.32 ms | 1.30 ms | 21.8× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 0.7 µs | 77 µs | kernels 76 µs, store 0.1 µs, rest 0.7 µs | 0.0 µs | – | 0.4 µs |
| 1 × 32 | cpu | 0.3 µs | 0.8 µs | 131 µs | kernels 130 µs, store 0.2 µs, rest 1.0 µs | 0.0 µs | – | 0.5 µs |
| 1 × 32 | gpu | 0.5 µs | 1.1 µs | 67 µs | upload 6.5 µs, record 6.2 µs, submit 47 µs, download 0.2 µs, rest 7.1 µs | 167 µs | 88 µs | 0.9 µs |
| 16 × 32 | cpu-1 | 0.7 µs | 0.9 µs | 1.22 ms | kernels 1.21 ms, store 0.3 µs, rest 1.0 µs | 0.0 µs | – | 0.5 µs |
| 16 × 32 | cpu | 1.3 µs | 1.4 µs | 200 µs | kernels 199 µs, store 0.3 µs, rest 1.6 µs | 0.0 µs | – | 0.8 µs |
| 16 × 32 | gpu | 0.8 µs | 1.0 µs | 69 µs | upload 6.7 µs, record 6.3 µs, submit 48 µs, download 0.2 µs, rest 8.4 µs | 176 µs | 98 µs | 1.0 µs |
| 256 × 32 | cpu-1 | 11 µs | 3.5 µs | 19.98 ms | kernels 19.97 ms, store 0.9 µs, rest 4.1 µs | 0.0 µs | – | 2.0 µs |
| 256 × 32 | cpu | 7.8 µs | 1.8 µs | 2.50 ms | kernels 2.49 ms, store 0.7 µs, rest 2.5 µs | 0.0 µs | – | 1.5 µs |
| 256 × 32 | gpu | 6.3 µs | 1.1 µs | 92 µs | upload 19 µs, record 6.3 µs, submit 51 µs, download 0.2 µs, rest 15 µs | 205 µs | 120 µs | 1.5 µs |
| 4096 × 32 | cpu-1 | 158 µs | 4.8 µs | 312.38 ms | kernels 312.36 ms, store 3.6 µs, rest 13 µs | 0.0 µs | – | 3.8 µs |
| 4096 × 32 | cpu | 162 µs | 4.8 µs | 40.95 ms | kernels 40.93 ms, store 3.8 µs, rest 14 µs | 0.0 µs | – | 4.0 µs |
| 4096 × 32 | gpu | 124 µs | 1.6 µs | 309 µs | upload 140 µs, record 7.1 µs, submit 64 µs, download 0.2 µs, rest 98 µs | 800 µs | 593 µs | 1.1 µs |
| 1 × 128 | cpu-1 | 0.3 µs | 0.7 µs | 204 µs | kernels 203 µs, store 0.2 µs, rest 0.8 µs | 0.0 µs | – | 0.4 µs |
| 1 × 128 | cpu | 0.5 µs | 1.2 µs | 302 µs | kernels 300 µs, store 0.3 µs, rest 1.4 µs | 0.0 µs | – | 0.9 µs |
| 1 × 128 | gpu | 0.6 µs | 1.1 µs | 69 µs | upload 7.3 µs, record 6.5 µs, submit 48 µs, download 0.2 µs, rest 7.4 µs | 211 µs | 140 µs | 1.1 µs |
| 16 × 128 | cpu-1 | 2.0 µs | 1.1 µs | 3.28 ms | kernels 3.27 ms, store 0.3 µs, rest 1.3 µs | 0.0 µs | – | 0.7 µs |
| 16 × 128 | cpu | 2.5 µs | 1.3 µs | 498 µs | kernels 496 µs, store 0.4 µs, rest 2.0 µs | 0.0 µs | – | 1.1 µs |
| 16 × 128 | gpu | 2.0 µs | 1.1 µs | 74 µs | upload 9.0 µs, record 6.5 µs, submit 50 µs, download 0.2 µs, rest 9.3 µs | 226 µs | 151 µs | 1.1 µs |
| 256 × 128 | cpu-1 | 44 µs | 3.8 µs | 53.19 ms | kernels 53.18 ms, store 0.9 µs, rest 4.7 µs | 0.0 µs | – | 2.0 µs |
| 256 × 128 | cpu | 36 µs | 3.2 µs | 7.43 ms | kernels 7.42 ms, store 1.3 µs, rest 5.0 µs | 0.0 µs | – | 1.7 µs |
| 256 × 128 | gpu | 24 µs | 1.3 µs | 124 µs | upload 42 µs, record 6.7 µs, submit 53 µs, download 0.2 µs, rest 21 µs | 416 µs | 252 µs | 1.2 µs |
| 1024 × 128 | cpu-1 | 157 µs | 4.7 µs | 215.08 ms | kernels 215.06 ms, store 3.3 µs, rest 11 µs | 0.0 µs | – | 2.7 µs |
| 1024 × 128 | cpu | 162 µs | 4.3 µs | 28.16 ms | kernels 28.15 ms, store 2.4 µs, rest 8.8 µs | 0.0 µs | – | 2.4 µs |
| 1024 × 128 | gpu | 122 µs | 1.6 µs | 271 µs | upload 138 µs, record 7.1 µs, submit 65 µs, download 0.2 µs, rest 58 µs | 897 µs | 691 µs | 1.2 µs |

## instances

A ring of 10 patches put onto 64 frames along every curve by reference, the control points move.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 78 µs | 134 µs | 228 µs | 1.7× slower | 3 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 16 × 32 | 1.20 ms | 262 µs | 240 µs | 1.1× faster | 3 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 256 × 32 | 19.34 ms | 2.59 ms | 296 µs | 8.7× faster | 3 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 1024 × 32 | 81.10 ms | 10.10 ms | 504 µs | 20.0× faster | 3 on gpu, 0 on cpu, 4 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 0.7 µs | 77 µs | kernels 76 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – | 0.4 µs |
| 1 × 32 | cpu | 0.4 µs | 1.3 µs | 132 µs | kernels 129 µs, store 0.2 µs, rest 0.9 µs | 0.0 µs | – | 0.8 µs |
| 1 × 32 | gpu | 0.5 µs | 1.1 µs | 63 µs | upload 7.2 µs, record 5.1 µs, submit 44 µs, download 0.2 µs, rest 6.7 µs | 163 µs | 84 µs | 1.0 µs |
| 16 × 32 | cpu-1 | 0.6 µs | 0.8 µs | 1.20 ms | kernels 1.20 ms, store 0.2 µs, rest 0.7 µs | 0.0 µs | – | 0.4 µs |
| 16 × 32 | cpu | 0.9 µs | 1.3 µs | 260 µs | kernels 259 µs, store 0.2 µs, rest 1.1 µs | 0.0 µs | – | 0.7 µs |
| 16 × 32 | gpu | 0.9 µs | 1.1 µs | 65 µs | upload 8.0 µs, record 5.0 µs, submit 44 µs, download 0.2 µs, rest 8.2 µs | 173 µs | 93 µs | 1.1 µs |
| 256 × 32 | cpu-1 | 7.0 µs | 1.7 µs | 19.33 ms | kernels 19.33 ms, store 0.5 µs, rest 1.6 µs | 0.0 µs | – | 1.0 µs |
| 256 × 32 | cpu | 7.4 µs | 1.8 µs | 2.58 ms | kernels 2.57 ms, store 0.5 µs, rest 2.0 µs | 0.0 µs | – | 1.3 µs |
| 256 × 32 | gpu | 7.0 µs | 1.3 µs | 96 µs | upload 24 µs, record 5.4 µs, submit 49 µs, download 0.2 µs, rest 19 µs | 192 µs | 108 µs | 1.7 µs |
| 1024 × 32 | cpu-1 | 41 µs | 3.9 µs | 81.06 ms | kernels 81.05 ms, store 1.5 µs, rest 6.0 µs | 0.0 µs | – | 2.5 µs |
| 1024 × 32 | cpu | 39 µs | 2.8 µs | 10.06 ms | kernels 10.05 ms, store 0.9 µs, rest 3.6 µs | 0.0 µs | – | 1.7 µs |
| 1024 × 32 | gpu | 23 µs | 1.3 µs | 151 µs | upload 55 µs, record 5.4 µs, submit 48 µs, download 0.2 µs, rest 43 µs | 328 µs | 165 µs | 1.2 µs |

## instances realized

The rings of `instances`, copied to where each one is.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 149 µs | 138 µs | 256 µs | 1.9× slower | 4 on gpu, 0 on cpu, 6 passes, 0 downloads |
| 16 × 32 | 2.30 ms | 394 µs | 276 µs | 1.4× faster | 4 on gpu, 0 on cpu, 6 passes, 0 downloads |
| 256 × 32 | 37.78 ms | 4.90 ms | 469 µs | 10.4× faster | 4 on gpu, 0 on cpu, 6 passes, 0 downloads |
| 1024 × 32 | 154.45 ms | 19.29 ms | 949 µs | 20.3× faster | 4 on gpu, 0 on cpu, 6 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 0.8 µs | 148 µs | kernels 147 µs, store 0.2 µs, rest 0.7 µs | 0.0 µs | – | 0.5 µs |
| 1 × 32 | cpu | 0.4 µs | 1.2 µs | 136 µs | kernels 134 µs, store 0.2 µs, rest 1.0 µs | 0.0 µs | – | 0.8 µs |
| 1 × 32 | gpu | 0.6 µs | 1.3 µs | 74 µs | upload 7.2 µs, record 7.3 µs, submit 52 µs, download 0.2 µs, rest 8.1 µs | 180 µs | 93 µs | 1.2 µs |
| 16 × 32 | cpu-1 | 0.7 µs | 1.2 µs | 2.30 ms | kernels 2.30 ms, store 0.3 µs, rest 1.2 µs | 0.0 µs | – | 0.7 µs |
| 16 × 32 | cpu | 1.2 µs | 1.7 µs | 391 µs | kernels 390 µs, store 0.3 µs, rest 1.5 µs | 0.0 µs | – | 1.2 µs |
| 16 × 32 | gpu | 0.9 µs | 1.3 µs | 77 µs | upload 7.8 µs, record 7.4 µs, submit 53 µs, download 0.2 µs, rest 9.4 µs | 196 µs | 107 µs | 1.2 µs |
| 256 × 32 | cpu-1 | 11 µs | 2.9 µs | 37.76 ms | kernels 37.76 ms, store 0.8 µs, rest 2.5 µs | 0.0 µs | – | 1.8 µs |
| 256 × 32 | cpu | 9.6 µs | 2.5 µs | 4.89 ms | kernels 4.88 ms, store 0.8 µs, rest 2.8 µs | 0.0 µs | – | 2.3 µs |
| 256 × 32 | gpu | 8.4 µs | 2.2 µs | 148 µs | upload 27 µs, record 11 µs, submit 79 µs, download 0.3 µs, rest 30 µs | 311 µs | 218 µs | 2.5 µs |
| 1024 × 32 | cpu-1 | 47 µs | 5.5 µs | 154.40 ms | kernels 154.37 ms, store 4.7 µs, rest 12 µs | 0.0 µs | – | 5.1 µs |
| 1024 × 32 | cpu | 56 µs | 7.0 µs | 19.23 ms | kernels 19.21 ms, store 2.2 µs, rest 9.6 µs | 0.1 µs | – | 3.5 µs |
| 1024 × 32 | gpu | 28 µs | 1.7 µs | 170 µs | upload 55 µs, record 8.1 µs, submit 59 µs, download 0.2 µs, rest 47 µs | 725 µs | 588 µs | 1.5 µs |

## for each

NURBS and Translate for every curve on its own, the control points of one curve move.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 16 × 8 | 11 µs | 21 µs | 237 µs | 11.2× slower | 2 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 256 × 8 | 47 µs | 54 µs | 273 µs | 5.1× slower | 2 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 4096 × 8 | 694 µs | 708 µs | 964 µs | 1.4× slower | 2 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 16384 × 8 | 3.59 ms | 3.96 ms | 4.02 ms | 1.0× slower | 2 on gpu, 0 on cpu, 9 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 16 × 8 | cpu-1 | 0.3 µs | 0.2 µs | 10 µs | kernels 7.0 µs, store 0.1 µs, rest 3.5 µs | 0.0 µs | – | 0.4 µs |
| 16 × 8 | cpu | 0.5 µs | 0.4 µs | 20 µs | kernels 14 µs, store 0.1 µs, rest 5.8 µs | 0.0 µs | – | 0.6 µs |
| 16 × 8 | gpu | 0.6 µs | 0.6 µs | 79 µs | upload 4.7 µs, record 6.6 µs, submit 56 µs, download 0.1 µs, rest 10 µs | 156 µs | 46 µs | 1.1 µs |
| 256 × 8 | cpu-1 | 1.8 µs | 0.3 µs | 45 µs | kernels 8.3 µs, store 0.1 µs, rest 37 µs | 0.0 µs | – | 1.9 µs |
| 256 × 8 | cpu | 1.8 µs | 0.3 µs | 52 µs | kernels 12 µs, store 0.1 µs, rest 40 µs | 0.0 µs | – | 3.0 µs |
| 256 × 8 | gpu | 1.9 µs | 0.6 µs | 111 µs | upload 6.3 µs, record 6.7 µs, submit 53 µs, download 0.1 µs, rest 45 µs | 156 µs | 47 µs | 2.6 µs |
| 4096 × 8 | cpu-1 | 26 µs | 0.4 µs | 666 µs | kernels 8.6 µs, store 0.1 µs, rest 657 µs | 0.0 µs | – | 29 µs |
| 4096 × 8 | cpu | 25 µs | 0.4 µs | 683 µs | kernels 13 µs, store 0.1 µs, rest 669 µs | 0.0 µs | – | 30 µs |
| 4096 × 8 | gpu | 25 µs | 0.8 µs | 779 µs | upload 39 µs, record 14 µs, submit 69 µs, download 0.2 µs, rest 651 µs | 156 µs | 54 µs | 36 µs |
| 16384 × 8 | cpu-1 | 160 µs | 1.1 µs | 3.44 ms | kernels 11 µs, store 0.2 µs, rest 3.43 ms | 0.0 µs | – | 224 µs |
| 16384 × 8 | cpu | 168 µs | 1.3 µs | 3.77 ms | kernels 21 µs, store 0.3 µs, rest 3.75 ms | 0.0 µs | – | 298 µs |
| 16384 × 8 | gpu | 169 µs | 1.6 µs | 3.67 ms | upload 124 µs, record 40 µs, submit 167 µs, download 0.2 µs, rest 3.26 ms | 193 µs | 76 µs | 288 µs |

## for each, all

NURBS and Translate for every curve on its own, the offset of all of them changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 16 × 8 | 9.0 µs | 69 µs | 170 µs | 2.5× slower | 16 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 8 | 127 µs | 473 µs | 314 µs | 1.5× faster | 256 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 4096 × 8 | 2.43 ms | 7.59 ms | 2.90 ms | 2.6× faster | 4096 on gpu, 0 on cpu, 6 passes, 0 downloads |
| 16384 × 8 | 18.62 ms | 45.30 ms | 17.85 ms | 2.5× faster | 16384 on gpu, 0 on cpu, 19 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 16 × 8 | cpu-1 | 0.1 µs | 0.2 µs | 8.6 µs | kernels 1.5 µs, store 0.5 µs, rest 6.6 µs | 0.0 µs | – | 0.9 µs |
| 16 × 8 | cpu | 0.4 µs | 0.5 µs | 68 µs | kernels 53 µs, store 1.1 µs, rest 14 µs | 0.0 µs | – | 2.0 µs |
| 16 × 8 | gpu | 0.5 µs | 0.6 µs | 68 µs | upload 3.0 µs, record 4.2 µs, submit 45 µs, download 0.4 µs, rest 15 µs | 101 µs | 2.3 µs | 2.8 µs |
| 256 × 8 | cpu-1 | 0.3 µs | 0.3 µs | 127 µs | kernels 27 µs, store 8.3 µs, rest 90 µs | 0.0 µs | – | 22 µs |
| 256 × 8 | cpu | 0.4 µs | 0.5 µs | 472 µs | kernels 314 µs, store 13 µs, rest 127 µs | 0.0 µs | – | 30 µs |
| 256 × 8 | gpu | 0.6 µs | 0.6 µs | 220 µs | upload 4.4 µs, record 5.3 µs, submit 48 µs, download 6.0 µs, rest 154 µs | 93 µs | 3.3 µs | 39 µs |
| 4096 × 8 | cpu-1 | 0.5 µs | 0.5 µs | 2.43 ms | kernels 680 µs, store 161 µs, rest 1.58 ms | 0.0 µs | – | 591 µs |
| 4096 × 8 | cpu | 1.2 µs | 1.0 µs | 7.58 ms | kernels 5.18 ms, store 274 µs, rest 2.12 ms | 0.0 µs | – | 634 µs |
| 4096 × 8 | gpu | 0.7 µs | 0.8 µs | 2.83 ms | upload 7.1 µs, record 37 µs, submit 105 µs, download 96 µs, rest 2.60 ms | 67 µs | 26 µs | 520 µs |
| 16384 × 8 | cpu-1 | 2.4 µs | 1.8 µs | 18.61 ms | kernels 4.24 ms, store 724 µs, rest 13.33 ms | 0.0 µs | – | 3.45 ms |
| 16384 × 8 | cpu | 2.5 µs | 2.0 µs | 45.29 ms | kernels 27.26 ms, store 1.29 ms, rest 15.73 ms | 0.0 µs | – | 4.81 ms |
| 16384 × 8 | gpu | 2.2 µs | 1.9 µs | 17.75 ms | upload 43 µs, record 233 µs, submit 338 µs, download 384 µs, rest 16.77 ms | 194 µs | 102 µs | 4.82 ms |

## mesh

Mesh of a swept circle, the tolerance changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 2.31 ms | 268 µs | 301 µs | 1.1× slower | 0 on gpu, 1 on cpu, 0 passes, 0 downloads |
| 16 × 32 | 37.28 ms | 4.39 ms | 4.42 ms | 1.0× slower | 0 on gpu, 1 on cpu, 0 passes, 0 downloads |
| 64 × 32 | 145.82 ms | 17.01 ms | 17.66 ms | 1.0× slower | 0 on gpu, 1 on cpu, 0 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 0.4 µs | 2.31 ms | kernels 2.31 ms, store 0.1 µs, rest 0.3 µs | 0.0 µs | – | 0.3 µs |
| 1 × 32 | cpu | 0.4 µs | 0.8 µs | 268 µs | kernels 267 µs, store 0.1 µs, rest 0.6 µs | 0.0 µs | – | 0.6 µs |
| 1 × 32 | gpu | 0.4 µs | 0.8 µs | 296 µs | cpu 295 µs, rest 1.2 µs | 3.8 µs | 0.0 µs | 0.7 µs |
| 16 × 32 | cpu-1 | 0.7 µs | 1.4 µs | 37.28 ms | kernels 37.28 ms, store 0.8 µs, rest 1.4 µs | 0.0 µs | – | 1.0 µs |
| 16 × 32 | cpu | 1.0 µs | 1.6 µs | 4.38 ms | kernels 4.38 ms, store 0.3 µs, rest 1.4 µs | 0.0 µs | – | 1.5 µs |
| 16 × 32 | gpu | 0.9 µs | 1.6 µs | 4.41 ms | submit 0.1 µs, download 0.1 µs, cpu 4.41 ms, rest 1.8 µs | 9.4 µs | 0.0 µs | 1.5 µs |
| 64 × 32 | cpu-1 | 1.4 µs | 2.4 µs | 145.82 ms | kernels 145.82 ms, store 1.0 µs, rest 3.3 µs | 0.0 µs | – | 2.3 µs |
| 64 × 32 | cpu | 1.3 µs | 2.1 µs | 17.01 ms | kernels 17.00 ms, store 0.5 µs, rest 1.8 µs | 0.0 µs | – | 1.3 µs |
| 64 × 32 | gpu | 1.4 µs | 2.3 µs | 17.64 ms | submit 0.1 µs, download 0.1 µs, cpu 17.63 ms, rest 3.0 µs | 16 µs | 0.0 µs | 1.7 µs |
