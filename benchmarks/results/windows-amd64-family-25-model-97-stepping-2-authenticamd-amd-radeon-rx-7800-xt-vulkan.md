# Benchmarks

windows, AMD64 Family 25 Model 97 Stepping 2, AuthenticAMD with 16 threads, AMD Radeon RX 7800 XT on vulkan, driver AMD proprietary driver 26.11.2 (LLPC). Commit 0ba48e2.

The time from an edit until its result is computed, the median of many edits. Below each table, where it goes: setting the new value (which hashes it) and planning are the same for every backend, `flush` is the executor, then waiting for the device. The device column is the time the GPU spends on compute passes, by its timestamps.

## nurbs

NURBS Curve, the control points move.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 38 µs | 39 µs | 189 µs | 4.8× slower | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 16 × 32 | 622 µs | 101 µs | 206 µs | 2.0× slower | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 32 | 9.58 ms | 1.18 ms | 238 µs | 4.9× faster | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 4096 × 32 | 156.89 ms | 18.82 ms | 921 µs | 20.4× faster | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 1 × 128 | 166 µs | 167 µs | 241 µs | 1.4× slower | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 16 × 128 | 2.66 ms | 317 µs | 259 µs | 1.2× faster | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 128 | 43.63 ms | 4.86 ms | 497 µs | 9.8× faster | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 1024 × 128 | 181.04 ms | 20.22 ms | 1.02 ms | 19.8× faster | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device |
|---|---|---:|---:|---:|---|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 0.3 µs | 38 µs | kernels 38 µs, rest 0.3 µs | 0.0 µs | – |
| 1 × 32 | cpu | 0.2 µs | 0.3 µs | 38 µs | kernels 38 µs, rest 0.3 µs | 0.0 µs | – |
| 1 × 32 | gpu | 0.6 µs | 0.7 µs | 50 µs | upload 4.7 µs, record 2.8 µs, submit 38 µs, download 0.1 µs, rest 3.8 µs | 138 µs | 71 µs |
| 16 × 32 | cpu-1 | 0.5 µs | 0.4 µs | 621 µs | kernels 621 µs, store 0.1 µs, rest 0.3 µs | 0.0 µs | – |
| 16 × 32 | cpu | 1.2 µs | 0.6 µs | 99 µs | kernels 98 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – |
| 16 × 32 | gpu | 0.9 µs | 0.7 µs | 51 µs | upload 5.1 µs, record 2.8 µs, submit 38 µs, download 0.1 µs, rest 4.9 µs | 153 µs | 80 µs |
| 256 × 32 | cpu-1 | 5.6 µs | 0.6 µs | 9.57 ms | kernels 9.57 ms, store 0.1 µs, rest 0.5 µs | 0.0 µs | – |
| 256 × 32 | cpu | 8.1 µs | 0.9 µs | 1.17 ms | kernels 1.17 ms, store 0.2 µs, rest 0.9 µs | 0.0 µs | – |
| 256 × 32 | gpu | 6.1 µs | 0.8 µs | 67 µs | upload 17 µs, record 2.8 µs, submit 38 µs, download 0.1 µs, rest 9.3 µs | 164 µs | 91 µs |
| 4096 × 32 | cpu-1 | 237 µs | 3.3 µs | 156.73 ms | kernels 156.72 ms, store 0.9 µs, rest 3.7 µs | 0.0 µs | – |
| 4096 × 32 | cpu | 146 µs | 1.5 µs | 18.67 ms | kernels 18.66 ms, store 0.8 µs, rest 2.8 µs | 0.0 µs | – |
| 4096 × 32 | gpu | 119 µs | 0.9 µs | 255 µs | upload 136 µs, record 3.4 µs, submit 48 µs, download 0.1 µs, rest 66 µs | 541 µs | 349 µs |
| 1 × 128 | cpu-1 | 0.3 µs | 0.4 µs | 166 µs | kernels 166 µs, rest 0.3 µs | 0.0 µs | – |
| 1 × 128 | cpu | 0.3 µs | 0.4 µs | 166 µs | kernels 166 µs, store 0.1 µs, rest 0.3 µs | 0.0 µs | – |
| 1 × 128 | gpu | 0.6 µs | 0.7 µs | 47 µs | upload 4.6 µs, record 2.8 µs, submit 36 µs, download 0.1 µs, rest 3.7 µs | 192 µs | 127 µs |
| 16 × 128 | cpu-1 | 1.7 µs | 0.5 µs | 2.66 ms | kernels 2.66 ms, store 0.1 µs, rest 0.4 µs | 0.0 µs | – |
| 16 × 128 | cpu | 2.9 µs | 0.6 µs | 313 µs | kernels 312 µs, store 0.1 µs, rest 0.7 µs | 0.0 µs | – |
| 16 × 128 | gpu | 2.0 µs | 0.7 µs | 51 µs | upload 6.4 µs, record 2.9 µs, submit 36 µs, download 0.1 µs, rest 5.2 µs | 205 µs | 131 µs |
| 256 × 128 | cpu-1 | 34 µs | 1.3 µs | 43.59 ms | kernels 43.59 ms, store 0.3 µs, rest 1.8 µs | 0.0 µs | – |
| 256 × 128 | cpu | 27 µs | 1.2 µs | 4.83 ms | kernels 4.83 ms, store 0.2 µs, rest 1.0 µs | 0.0 µs | – |
| 256 × 128 | gpu | 23 µs | 0.8 µs | 95 µs | upload 40 µs, record 2.9 µs, submit 38 µs, download 0.1 µs, rest 14 µs | 377 µs | 185 µs |
| 1024 × 128 | cpu-1 | 164 µs | 2.8 µs | 180.88 ms | kernels 180.87 ms, store 0.9 µs, rest 4.5 µs | 0.0 µs | – |
| 1024 × 128 | cpu | 142 µs | 1.7 µs | 20.09 ms | kernels 20.09 ms, store 0.4 µs, rest 3.3 µs | 0.0 µs | – |
| 1024 × 128 | gpu | 120 µs | 1.0 µs | 231 µs | upload 137 µs, record 3.4 µs, submit 48 µs, download 0.1 µs, rest 42 µs | 663 µs | 472 µs |

## translate

Translate Curve of the curves, the offset changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 1.3 µs | 4.5 µs | 93 µs | 20.7× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 32 | 1.9 µs | 9.8 µs | 94 µs | 9.6× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 32 | 13 µs | 20 µs | 119 µs | 6.0× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 4096 × 32 | 341 µs | 111 µs | 177 µs | 1.6× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 1 × 128 | 1.3 µs | 6.5 µs | 88 µs | 13.6× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 128 | 3.8 µs | 14 µs | 89 µs | 6.3× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 128 | 51 µs | 29 µs | 120 µs | 4.1× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 1024 × 128 | 401 µs | 153 µs | 183 µs | 1.2× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device |
|---|---|---:|---:|---:|---|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 0.5 µs | 0.6 µs | kernels 0.2 µs, rest 0.3 µs | 0.0 µs | – |
| 1 × 32 | cpu | 0.3 µs | 0.7 µs | 3.5 µs | kernels 2.9 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – |
| 1 × 32 | gpu | 0.5 µs | 0.7 µs | 36 µs | upload 2.5 µs, record 1.5 µs, submit 30 µs, rest 1.7 µs | 55 µs | 1.8 µs |
| 16 × 32 | cpu-1 | 0.2 µs | 0.5 µs | 1.2 µs | kernels 0.9 µs, store 0.1 µs, rest 0.3 µs | 0.0 µs | – |
| 16 × 32 | cpu | 0.3 µs | 0.7 µs | 8.7 µs | kernels 8.1 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – |
| 16 × 32 | gpu | 0.5 µs | 0.7 µs | 36 µs | upload 2.5 µs, record 1.6 µs, submit 30 µs, rest 1.7 µs | 56 µs | 2.4 µs |
| 256 × 32 | cpu-1 | 0.2 µs | 0.5 µs | 12 µs | kernels 12 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – |
| 256 × 32 | cpu | 0.3 µs | 0.6 µs | 19 µs | kernels 18 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – |
| 256 × 32 | gpu | 0.7 µs | 1.0 µs | 50 µs | upload 4.0 µs, record 2.4 µs, submit 41 µs, rest 2.7 µs | 65 µs | 6.4 µs |
| 4096 × 32 | cpu-1 | 0.5 µs | 0.9 µs | 340 µs | kernels 339 µs, store 0.2 µs, rest 1.0 µs | 0.0 µs | – |
| 4096 × 32 | cpu | 0.5 µs | 1.2 µs | 109 µs | kernels 108 µs, store 0.1 µs, rest 1.0 µs | 0.0 µs | – |
| 4096 × 32 | gpu | 0.5 µs | 0.7 µs | 36 µs | upload 2.5 µs, record 1.6 µs, submit 30 µs, rest 1.8 µs | 140 µs | 72 µs |
| 1 × 128 | cpu-1 | 0.2 µs | 0.4 µs | 0.7 µs | kernels 0.3 µs, store 0.1 µs, rest 0.3 µs | 0.0 µs | – |
| 1 × 128 | cpu | 0.3 µs | 0.7 µs | 5.5 µs | kernels 5.0 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – |
| 1 × 128 | gpu | 0.5 µs | 0.7 µs | 36 µs | upload 2.5 µs, record 1.6 µs, submit 30 µs, rest 1.8 µs | 52 µs | 2.2 µs |
| 16 × 128 | cpu-1 | 0.2 µs | 0.4 µs | 3.3 µs | kernels 3.0 µs, rest 0.3 µs | 0.0 µs | – |
| 16 × 128 | cpu | 0.3 µs | 0.6 µs | 13 µs | kernels 13 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – |
| 16 × 128 | gpu | 0.5 µs | 0.7 µs | 35 µs | upload 2.5 µs, record 1.6 µs, submit 29 µs, rest 1.8 µs | 52 µs | 2.9 µs |
| 256 × 128 | cpu-1 | 0.2 µs | 0.5 µs | 50 µs | kernels 50 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – |
| 256 × 128 | cpu | 0.4 µs | 0.8 µs | 28 µs | kernels 27 µs, store 0.1 µs, rest 0.6 µs | 0.0 µs | – |
| 256 × 128 | gpu | 0.5 µs | 0.7 µs | 36 µs | upload 2.5 µs, record 1.6 µs, submit 30 µs, rest 1.8 µs | 83 µs | 22 µs |
| 1024 × 128 | cpu-1 | 0.4 µs | 0.8 µs | 400 µs | kernels 398 µs, store 0.3 µs, rest 0.8 µs | 0.0 µs | – |
| 1024 × 128 | cpu | 0.6 µs | 1.1 µs | 151 µs | kernels 149 µs, store 0.2 µs, rest 0.9 µs | 0.0 µs | – |
| 1024 × 128 | gpu | 0.5 µs | 0.7 µs | 36 µs | upload 2.6 µs, record 1.6 µs, submit 30 µs, rest 1.8 µs | 146 µs | 78 µs |

## sweep circle

Sweep of a circle along the curves, the radius changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 1.7 µs | 2.1 µs | 107 µs | 51.1× slower | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 16 × 32 | 4.5 µs | 11 µs | 110 µs | 10.2× slower | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 32 | 54 µs | 127 µs | 123 µs | 1.0× faster | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 4096 × 32 | 5.14 ms | 3.80 ms | 220 µs | 17.3× faster | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 1 × 128 | 2.2 µs | 2.6 µs | 107 µs | 41.1× slower | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 16 × 128 | 16 µs | 44 µs | 110 µs | 2.5× slower | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 128 | 636 µs | 661 µs | 142 µs | 4.7× faster | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 1024 × 128 | 3.46 ms | 3.31 ms | 223 µs | 14.8× faster | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device |
|---|---|---:|---:|---:|---|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 0.6 µs | 1.0 µs | kernels 0.5 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – |
| 1 × 32 | cpu | 0.2 µs | 0.7 µs | 1.2 µs | kernels 0.7 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – |
| 1 × 32 | gpu | 0.5 µs | 0.9 µs | 43 µs | upload 3.4 µs, record 2.5 µs, submit 34 µs, download 0.1 µs, rest 3.1 µs | 63 µs | 2.8 µs |
| 16 × 32 | cpu-1 | 0.2 µs | 0.5 µs | 3.8 µs | kernels 3.3 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – |
| 16 × 32 | cpu | 0.3 µs | 0.9 µs | 9.6 µs | kernels 8.8 µs, store 0.2 µs, rest 0.7 µs | 0.0 µs | – |
| 16 × 32 | gpu | 0.5 µs | 0.9 µs | 44 µs | upload 3.5 µs, record 2.6 µs, submit 34 µs, download 0.1 µs, rest 3.3 µs | 64 µs | 4.8 µs |
| 256 × 32 | cpu-1 | 0.2 µs | 0.7 µs | 53 µs | kernels 52 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – |
| 256 × 32 | cpu | 0.3 µs | 0.9 µs | 125 µs | kernels 124 µs, store 0.2 µs, rest 0.7 µs | 0.0 µs | – |
| 256 × 32 | gpu | 0.6 µs | 0.9 µs | 46 µs | upload 3.7 µs, record 2.6 µs, submit 35 µs, download 0.1 µs, rest 4.4 µs | 75 µs | 11 µs |
| 4096 × 32 | cpu-1 | 2.2 µs | 4.5 µs | 5.13 ms | kernels 5.13 ms, store 0.7 µs, rest 4.3 µs | 0.0 µs | – |
| 4096 × 32 | cpu | 2.3 µs | 4.6 µs | 3.80 ms | kernels 3.79 ms, store 0.7 µs, rest 3.9 µs | 0.0 µs | – |
| 4096 × 32 | gpu | 0.5 µs | 1.0 µs | 55 µs | upload 4.7 µs, record 2.7 µs, submit 37 µs, download 0.1 µs, rest 11 µs | 163 µs | 93 µs |
| 1 × 128 | cpu-1 | 0.2 µs | 0.6 µs | 1.5 µs | kernels 1.0 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – |
| 1 × 128 | cpu | 0.2 µs | 0.7 µs | 1.7 µs | kernels 1.2 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – |
| 1 × 128 | gpu | 0.5 µs | 0.9 µs | 42 µs | upload 3.3 µs, record 2.6 µs, submit 34 µs, download 0.1 µs, rest 3.0 µs | 63 µs | 3.5 µs |
| 16 × 128 | cpu-1 | 0.3 µs | 0.7 µs | 15 µs | kernels 15 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – |
| 16 × 128 | cpu | 0.4 µs | 1.0 µs | 43 µs | kernels 42 µs, store 0.2 µs, rest 0.8 µs | 0.0 µs | – |
| 16 × 128 | gpu | 0.5 µs | 0.9 µs | 43 µs | upload 3.5 µs, record 2.6 µs, submit 34 µs, download 0.1 µs, rest 3.3 µs | 65 µs | 5.5 µs |
| 256 × 128 | cpu-1 | 0.3 µs | 0.8 µs | 635 µs | kernels 634 µs, store 0.2 µs, rest 0.7 µs | 0.0 µs | – |
| 256 × 128 | cpu | 0.4 µs | 1.1 µs | 659 µs | kernels 658 µs, store 0.3 µs, rest 0.9 µs | 0.0 µs | – |
| 256 × 128 | gpu | 0.5 µs | 0.9 µs | 46 µs | upload 3.6 µs, record 2.7 µs, submit 35 µs, download 0.1 µs, rest 4.2 µs | 94 µs | 30 µs |
| 1024 × 128 | cpu-1 | 1.6 µs | 4.2 µs | 3.46 ms | kernels 3.46 ms, store 0.7 µs, rest 3.1 µs | 0.0 µs | – |
| 1024 × 128 | cpu | 2.4 µs | 5.7 µs | 3.30 ms | kernels 3.29 ms, store 1.0 µs, rest 4.6 µs | 0.0 µs | – |
| 1024 × 128 | gpu | 0.6 µs | 1.0 µs | 51 µs | upload 4.2 µs, record 2.8 µs, submit 39 µs, download 0.1 µs, rest 5.7 µs | 169 µs | 99 µs |

## sweep square

Sweep of a square curve along the curves, the square changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 10 µs | 12 µs | 173 µs | 14.4× slower | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 16 × 32 | 57 µs | 68 µs | 172 µs | 2.5× slower | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 256 × 32 | 1.66 ms | 1.41 ms | 219 µs | 6.4× faster | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 4096 × 32 | 70.76 ms | 46.04 ms | 535 µs | 86.1× faster | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 1 × 128 | 19 µs | 20 µs | 183 µs | 9.3× slower | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 16 × 128 | 415 µs | 404 µs | 191 µs | 2.1× faster | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 256 × 128 | 7.61 ms | 6.33 ms | 345 µs | 18.4× faster | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 1024 × 128 | 48.35 ms | 33.89 ms | 703 µs | 48.2× faster | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device |
|---|---|---:|---:|---:|---|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 0.7 µs | 9.4 µs | kernels 8.8 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – |
| 1 × 32 | cpu | 0.2 µs | 0.8 µs | 11 µs | kernels 10 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – |
| 1 × 32 | gpu | 0.5 µs | 1.1 µs | 53 µs | upload 5.3 µs, record 3.8 µs, submit 39 µs, download 0.1 µs, rest 5.0 µs | 118 µs | 48 µs |
| 16 × 32 | cpu-1 | 0.3 µs | 0.8 µs | 56 µs | kernels 55 µs, store 0.2 µs, rest 0.6 µs | 0.0 µs | – |
| 16 × 32 | cpu | 0.4 µs | 1.2 µs | 66 µs | kernels 65 µs, store 0.2 µs, rest 0.8 µs | 0.0 µs | – |
| 16 × 32 | gpu | 0.5 µs | 1.1 µs | 53 µs | upload 5.3 µs, record 3.7 µs, submit 39 µs, download 0.1 µs, rest 5.1 µs | 118 µs | 50 µs |
| 256 × 32 | cpu-1 | 0.4 µs | 1.1 µs | 1.66 ms | kernels 1.65 ms, store 0.3 µs, rest 0.9 µs | 0.0 µs | – |
| 256 × 32 | cpu | 0.7 µs | 1.6 µs | 1.41 ms | kernels 1.41 ms, store 0.4 µs, rest 1.3 µs | 0.0 µs | – |
| 256 × 32 | gpu | 0.6 µs | 1.1 µs | 57 µs | upload 5.9 µs, record 3.9 µs, submit 41 µs, download 0.1 µs, rest 6.5 µs | 157 µs | 87 µs |
| 4096 × 32 | cpu-1 | 3.7 µs | 6.4 µs | 70.76 ms | kernels 70.75 ms, store 2.2 µs, rest 5.3 µs | 0.0 µs | – |
| 4096 × 32 | cpu | 3.6 µs | 6.2 µs | 46.03 ms | kernels 46.03 ms, store 1.8 µs, rest 5.2 µs | 0.1 µs | – |
| 4096 × 32 | gpu | 0.6 µs | 1.3 µs | 77 µs | upload 7.3 µs, record 4.4 µs, submit 46 µs, download 0.2 µs, rest 19 µs | 454 µs | 391 µs |
| 1 × 128 | cpu-1 | 0.2 µs | 0.6 µs | 18 µs | kernels 18 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – |
| 1 × 128 | cpu | 0.2 µs | 0.7 µs | 19 µs | kernels 18 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – |
| 1 × 128 | gpu | 0.6 µs | 1.3 µs | 60 µs | upload 5.4 µs, record 4.5 µs, submit 44 µs, download 0.1 µs, rest 5.8 µs | 121 µs | 48 µs |
| 16 × 128 | cpu-1 | 0.4 µs | 0.9 µs | 414 µs | kernels 413 µs, store 0.2 µs, rest 0.7 µs | 0.0 µs | – |
| 16 × 128 | cpu | 0.5 µs | 1.3 µs | 402 µs | kernels 400 µs, store 0.3 µs, rest 1.0 µs | 0.0 µs | – |
| 16 × 128 | gpu | 0.5 µs | 1.1 µs | 54 µs | upload 5.4 µs, record 3.9 µs, submit 40 µs, download 0.1 µs, rest 5.3 µs | 129 µs | 59 µs |
| 256 × 128 | cpu-1 | 2.9 µs | 5.4 µs | 7.61 ms | kernels 7.60 ms, store 1.0 µs, rest 4.8 µs | 0.0 µs | – |
| 256 × 128 | cpu | 2.9 µs | 5.4 µs | 6.32 ms | kernels 6.32 ms, store 1.1 µs, rest 4.8 µs | 0.0 µs | – |
| 256 × 128 | gpu | 0.5 µs | 1.1 µs | 56 µs | upload 5.5 µs, record 3.9 µs, submit 40 µs, download 0.1 µs, rest 6.3 µs | 286 µs | 212 µs |
| 1024 × 128 | cpu-1 | 3.4 µs | 7.4 µs | 48.35 ms | kernels 48.33 ms, store 1.6 µs, rest 7.1 µs | 0.0 µs | – |
| 1024 × 128 | cpu | 2.4 µs | 4.2 µs | 33.88 ms | kernels 33.88 ms, store 0.9 µs, rest 3.8 µs | 0.0 µs | – |
| 1024 × 128 | gpu | 0.6 µs | 1.1 µs | 59 µs | upload 5.7 µs, record 3.9 µs, submit 41 µs, download 0.1 µs, rest 8.1 µs | 643 µs | 476 µs |

## sample

Sample Curve, 64 frames per curve at places which change.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 36 µs | 36 µs | 110 µs | 3.1× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 32 | 569 µs | 64 µs | 109 µs | 1.7× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 32 | 9.23 ms | 1.19 ms | 133 µs | 9.0× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 4096 × 32 | 149.56 ms | 18.33 ms | 197 µs | 93.1× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 1 × 128 | 37 µs | 35 µs | 110 µs | 3.1× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 128 | 569 µs | 82 µs | 110 µs | 1.3× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 128 | 8.93 ms | 1.16 ms | 115 µs | 10.1× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 1024 × 128 | 39.38 ms | 4.59 ms | 158 µs | 29.0× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device |
|---|---|---:|---:|---:|---|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 0.5 µs | 35 µs | kernels 34 µs, store 0.1 µs, rest 0.3 µs | 0.0 µs | – |
| 1 × 32 | cpu | 0.2 µs | 0.5 µs | 35 µs | kernels 35 µs, store 0.1 µs, rest 0.3 µs | 0.0 µs | – |
| 1 × 32 | gpu | 0.5 µs | 0.9 µs | 37 µs | upload 3.5 µs, record 1.5 µs, submit 30 µs, download 0.1 µs, rest 2.2 µs | 71 µs | 11 µs |
| 16 × 32 | cpu-1 | 0.2 µs | 0.5 µs | 568 µs | kernels 567 µs, store 0.1 µs, rest 0.3 µs | 0.0 µs | – |
| 16 × 32 | cpu | 0.4 µs | 0.8 µs | 63 µs | kernels 62 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – |
| 16 × 32 | gpu | 0.5 µs | 0.9 µs | 37 µs | upload 3.5 µs, record 1.5 µs, submit 30 µs, download 0.1 µs, rest 2.4 µs | 70 µs | 11 µs |
| 256 × 32 | cpu-1 | 0.5 µs | 1.0 µs | 9.23 ms | kernels 9.23 ms, store 0.2 µs, rest 0.8 µs | 0.0 µs | – |
| 256 × 32 | cpu | 0.6 µs | 1.2 µs | 1.19 ms | kernels 1.19 ms, store 0.2 µs, rest 0.9 µs | 0.0 µs | – |
| 256 × 32 | gpu | 0.8 µs | 1.4 µs | 56 µs | upload 5.3 µs, record 2.4 µs, submit 42 µs, download 0.1 µs, rest 5.4 µs | 73 µs | 12 µs |
| 4096 × 32 | cpu-1 | 4.6 µs | 5.0 µs | 149.55 ms | kernels 149.55 ms, store 0.9 µs, rest 6.5 µs | 0.1 µs | – |
| 4096 × 32 | cpu | 3.5 µs | 5.2 µs | 18.33 ms | kernels 18.32 ms, store 1.0 µs, rest 5.5 µs | 0.0 µs | – |
| 4096 × 32 | gpu | 0.6 µs | 1.1 µs | 52 µs | upload 4.6 µs, record 1.6 µs, submit 32 µs, download 0.1 µs, rest 14 µs | 143 µs | 78 µs |
| 1 × 128 | cpu-1 | 0.2 µs | 0.5 µs | 36 µs | kernels 36 µs, store 0.1 µs, rest 0.3 µs | 0.0 µs | – |
| 1 × 128 | cpu | 0.2 µs | 0.5 µs | 34 µs | kernels 34 µs, store 0.1 µs, rest 0.3 µs | 0.0 µs | – |
| 1 × 128 | gpu | 0.5 µs | 0.9 µs | 38 µs | upload 3.5 µs, record 1.5 µs, submit 30 µs, download 0.1 µs, rest 2.2 µs | 72 µs | 11 µs |
| 16 × 128 | cpu-1 | 0.2 µs | 0.6 µs | 568 µs | kernels 568 µs, store 0.1 µs, rest 0.3 µs | 0.0 µs | – |
| 16 × 128 | cpu | 0.4 µs | 0.8 µs | 81 µs | kernels 80 µs, store 0.1 µs, rest 0.6 µs | 0.0 µs | – |
| 16 × 128 | gpu | 0.5 µs | 0.9 µs | 38 µs | upload 3.6 µs, record 1.5 µs, submit 30 µs, download 0.1 µs, rest 2.4 µs | 71 µs | 12 µs |
| 256 × 128 | cpu-1 | 0.7 µs | 1.4 µs | 8.92 ms | kernels 8.92 ms, store 0.2 µs, rest 0.9 µs | 0.0 µs | – |
| 256 × 128 | cpu | 0.6 µs | 1.3 µs | 1.16 ms | kernels 1.16 ms, store 0.2 µs, rest 0.9 µs | 0.0 µs | – |
| 256 × 128 | gpu | 0.6 µs | 0.9 µs | 39 µs | upload 3.5 µs, record 1.5 µs, submit 30 µs, download 0.1 µs, rest 3.4 µs | 74 µs | 15 µs |
| 1024 × 128 | cpu-1 | 2.8 µs | 3.8 µs | 39.37 ms | kernels 39.36 ms, store 0.8 µs, rest 5.0 µs | 0.0 µs | – |
| 1024 × 128 | cpu | 1.4 µs | 2.7 µs | 4.58 ms | kernels 4.58 ms, store 0.4 µs, rest 2.4 µs | 0.0 µs | – |
| 1024 × 128 | gpu | 0.6 µs | 0.9 µs | 42 µs | upload 3.8 µs, record 1.5 µs, submit 32 µs, download 0.1 µs, rest 5.5 µs | 112 µs | 46 µs |

## chain

NURBS, Translate, a Sweep of a circle and Sample Curve, the control points move.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 78 µs | 128 µs | 237 µs | 1.8× slower | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 16 × 32 | 1.17 ms | 208 µs | 248 µs | 1.2× slower | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 256 × 32 | 20.11 ms | 2.55 ms | 314 µs | 8.1× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 4096 × 32 | 303.47 ms | 41.46 ms | 1.24 ms | 33.5× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 1 × 128 | 199 µs | 304 µs | 295 µs | 1.0× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 16 × 128 | 3.14 ms | 500 µs | 301 µs | 1.7× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 256 × 128 | 51.32 ms | 6.73 ms | 566 µs | 11.9× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 1024 × 128 | 206.90 ms | 28.25 ms | 1.30 ms | 21.8× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device |
|---|---|---:|---:|---:|---|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 1.1 µs | 77 µs | kernels 76 µs, store 0.2 µs, rest 0.8 µs | 0.0 µs | – |
| 1 × 32 | cpu | 0.3 µs | 1.4 µs | 127 µs | kernels 124 µs, store 0.3 µs, rest 1.2 µs | 0.0 µs | – |
| 1 × 32 | gpu | 0.6 µs | 1.6 µs | 67 µs | upload 6.4 µs, record 6.2 µs, submit 48 µs, download 0.2 µs, rest 7.3 µs | 167 µs | 87 µs |
| 16 × 32 | cpu-1 | 0.6 µs | 1.3 µs | 1.17 ms | kernels 1.17 ms, store 0.3 µs, rest 1.0 µs | 0.0 µs | – |
| 16 × 32 | cpu | 1.3 µs | 2.0 µs | 204 µs | kernels 202 µs, store 0.4 µs, rest 1.7 µs | 0.0 µs | – |
| 16 × 32 | gpu | 0.9 µs | 1.6 µs | 69 µs | upload 6.5 µs, record 6.1 µs, submit 48 µs, download 0.2 µs, rest 8.5 µs | 176 µs | 98 µs |
| 256 × 32 | cpu-1 | 10 µs | 2.9 µs | 20.09 ms | kernels 20.09 ms, store 0.8 µs, rest 2.9 µs | 0.0 µs | – |
| 256 × 32 | cpu | 9.2 µs | 2.6 µs | 2.54 ms | kernels 2.53 ms, store 0.8 µs, rest 3.1 µs | 0.0 µs | – |
| 256 × 32 | gpu | 7.9 µs | 1.9 µs | 101 µs | upload 20 µs, record 6.5 µs, submit 56 µs, download 0.3 µs, rest 18 µs | 198 µs | 118 µs |
| 4096 × 32 | cpu-1 | 155 µs | 6.1 µs | 303.31 ms | kernels 303.29 ms, store 3.9 µs, rest 14 µs | 0.0 µs | – |
| 4096 × 32 | cpu | 156 µs | 6.3 µs | 41.30 ms | kernels 41.28 ms, store 4.2 µs, rest 18 µs | 0.0 µs | – |
| 4096 × 32 | gpu | 126 µs | 2.3 µs | 318 µs | upload 140 µs, record 7.3 µs, submit 68 µs, download 0.3 µs, rest 103 µs | 780 µs | 593 µs |
| 1 × 128 | cpu-1 | 0.3 µs | 1.1 µs | 198 µs | kernels 197 µs, store 0.2 µs, rest 0.8 µs | 0.0 µs | – |
| 1 × 128 | cpu | 0.5 µs | 1.7 µs | 302 µs | kernels 300 µs, store 0.4 µs, rest 1.6 µs | 0.0 µs | – |
| 1 × 128 | gpu | 0.8 µs | 2.0 µs | 79 µs | upload 7.3 µs, record 7.4 µs, submit 55 µs, download 0.3 µs, rest 8.9 µs | 213 µs | 138 µs |
| 16 × 128 | cpu-1 | 1.6 µs | 1.3 µs | 3.14 ms | kernels 3.14 ms, store 0.3 µs, rest 1.1 µs | 0.0 µs | – |
| 16 × 128 | cpu | 2.4 µs | 1.9 µs | 495 µs | kernels 492 µs, store 0.5 µs, rest 2.3 µs | 0.0 µs | – |
| 16 × 128 | gpu | 2.0 µs | 1.7 µs | 75 µs | upload 8.4 µs, record 6.3 µs, submit 51 µs, download 0.2 µs, rest 9.4 µs | 222 µs | 152 µs |
| 256 × 128 | cpu-1 | 38 µs | 3.1 µs | 51.27 ms | kernels 51.27 ms, store 1.7 µs, rest 2.9 µs | 0.1 µs | – |
| 256 × 128 | cpu | 34 µs | 3.1 µs | 6.69 ms | kernels 6.69 ms, store 1.1 µs, rest 3.5 µs | 0.0 µs | – |
| 256 × 128 | gpu | 23 µs | 1.9 µs | 125 µs | upload 42 µs, record 6.4 µs, submit 54 µs, download 0.2 µs, rest 21 µs | 415 µs | 251 µs |
| 1024 × 128 | cpu-1 | 155 µs | 5.7 µs | 206.74 ms | kernels 206.73 ms, store 1.8 µs, rest 10 µs | 0.0 µs | – |
| 1024 × 128 | cpu | 158 µs | 5.3 µs | 28.08 ms | kernels 28.07 ms, store 2.1 µs, rest 10 µs | 0.0 µs | – |
| 1024 × 128 | gpu | 123 µs | 2.2 µs | 268 µs | upload 138 µs, record 7.1 µs, submit 63 µs, download 0.3 µs, rest 58 µs | 894 µs | 689 µs |

## instances

A ring of 10 patches put onto 64 frames along every curve by reference, the control points move.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 75 µs | 120 µs | 229 µs | 1.9× slower | 3 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 16 × 32 | 1.17 ms | 365 µs | 252 µs | 1.4× faster | 3 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 256 × 32 | 18.62 ms | 2.55 ms | 293 µs | 8.7× faster | 3 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 1024 × 32 | 75.62 ms | 9.76 ms | 508 µs | 19.2× faster | 3 on gpu, 0 on cpu, 4 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device |
|---|---|---:|---:|---:|---|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 1.2 µs | 74 µs | kernels 73 µs, store 0.2 µs, rest 0.6 µs | 0.0 µs | – |
| 1 × 32 | cpu | 0.4 µs | 1.8 µs | 117 µs | kernels 116 µs, store 0.2 µs, rest 0.9 µs | 0.0 µs | – |
| 1 × 32 | gpu | 0.6 µs | 1.9 µs | 64 µs | upload 6.7 µs, record 5.1 µs, submit 45 µs, download 0.2 µs, rest 7.0 µs | 162 µs | 84 µs |
| 16 × 32 | cpu-1 | 0.5 µs | 1.3 µs | 1.16 ms | kernels 1.16 ms, store 0.2 µs, rest 0.7 µs | 0.0 µs | – |
| 16 × 32 | cpu | 0.9 µs | 1.9 µs | 363 µs | kernels 362 µs, store 0.3 µs, rest 1.3 µs | 0.0 µs | – |
| 16 × 32 | gpu | 1.1 µs | 2.2 µs | 75 µs | upload 8.4 µs, record 5.6 µs, submit 50 µs, download 0.2 µs, rest 10 µs | 175 µs | 93 µs |
| 256 × 32 | cpu-1 | 6.6 µs | 1.7 µs | 18.61 ms | kernels 18.61 ms, store 0.4 µs, rest 1.4 µs | 0.0 µs | – |
| 256 × 32 | cpu | 7.3 µs | 2.5 µs | 2.54 ms | kernels 2.53 ms, store 0.5 µs, rest 2.1 µs | 0.0 µs | – |
| 256 × 32 | gpu | 6.1 µs | 2.0 µs | 93 µs | upload 23 µs, record 5.1 µs, submit 48 µs, download 0.2 µs, rest 18 µs | 191 µs | 108 µs |
| 1024 × 32 | cpu-1 | 39 µs | 3.6 µs | 75.57 ms | kernels 75.56 ms, store 0.8 µs, rest 2.8 µs | 0.0 µs | – |
| 1024 × 32 | cpu | 33 µs | 3.1 µs | 9.72 ms | kernels 9.72 ms, store 0.8 µs, rest 3.7 µs | 0.0 µs | – |
| 1024 × 32 | gpu | 27 µs | 2.1 µs | 152 µs | upload 55 µs, record 5.3 µs, submit 49 µs, download 0.2 µs, rest 42 µs | 328 µs | 166 µs |

## instances realized

The rings of `instances`, copied to where each one is.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 146 µs | 139 µs | 257 µs | 1.8× slower | 4 on gpu, 0 on cpu, 6 passes, 0 downloads |
| 16 × 32 | 2.30 ms | 424 µs | 281 µs | 1.5× faster | 4 on gpu, 0 on cpu, 6 passes, 0 downloads |
| 256 × 32 | 36.79 ms | 4.52 ms | 426 µs | 10.6× faster | 4 on gpu, 0 on cpu, 6 passes, 0 downloads |
| 1024 × 32 | 146.80 ms | 17.76 ms | 944 µs | 18.8× faster | 4 on gpu, 0 on cpu, 6 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device |
|---|---|---:|---:|---:|---|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 1.4 µs | 145 µs | kernels 144 µs, store 0.2 µs, rest 0.8 µs | 0.0 µs | – |
| 1 × 32 | cpu | 0.4 µs | 2.2 µs | 136 µs | kernels 135 µs, store 0.3 µs, rest 1.3 µs | 0.0 µs | – |
| 1 × 32 | gpu | 0.6 µs | 2.1 µs | 77 µs | upload 6.8 µs, record 7.2 µs, submit 54 µs, download 0.2 µs, rest 8.4 µs | 177 µs | 93 µs |
| 16 × 32 | cpu-1 | 0.6 µs | 1.7 µs | 2.29 ms | kernels 2.29 ms, store 0.3 µs, rest 1.0 µs | 0.0 µs | – |
| 16 × 32 | cpu | 1.3 µs | 2.8 µs | 418 µs | kernels 415 µs, store 0.4 µs, rest 1.9 µs | 0.0 µs | – |
| 16 × 32 | gpu | 0.9 µs | 2.1 µs | 81 µs | upload 7.9 µs, record 7.5 µs, submit 56 µs, download 0.2 µs, rest 9.9 µs | 196 µs | 107 µs |
| 256 × 32 | cpu-1 | 9.9 µs | 3.1 µs | 36.78 ms | kernels 36.77 ms, store 0.7 µs, rest 2.4 µs | 0.0 µs | – |
| 256 × 32 | cpu | 12 µs | 3.2 µs | 4.51 ms | kernels 4.50 ms, store 0.8 µs, rest 3.0 µs | 0.0 µs | – |
| 256 × 32 | gpu | 6.8 µs | 2.2 µs | 107 µs | upload 23 µs, record 7.4 µs, submit 57 µs, download 0.2 µs, rest 19 µs | 309 µs | 218 µs |
| 1024 × 32 | cpu-1 | 47 µs | 6.8 µs | 146.75 ms | kernels 146.74 ms, store 2.0 µs, rest 7.2 µs | 0.0 µs | – |
| 1024 × 32 | cpu | 50 µs | 5.5 µs | 17.71 ms | kernels 17.70 ms, store 1.7 µs, rest 5.1 µs | 0.0 µs | – |
| 1024 × 32 | gpu | 25 µs | 2.4 µs | 170 µs | upload 55 µs, record 7.9 µs, submit 61 µs, download 0.3 µs, rest 46 µs | 752 µs | 588 µs |

## mesh

Mesh of a swept circle, the tolerance changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 2.29 ms | 267 µs | 304 µs | 1.1× slower | 0 on gpu, 1 on cpu, 0 passes, 0 downloads |
| 16 × 32 | 36.40 ms | 4.23 ms | 4.37 ms | 1.0× slower | 0 on gpu, 1 on cpu, 0 passes, 0 downloads |
| 64 × 32 | 145.35 ms | 17.62 ms | 18.48 ms | 1.0× slower | 0 on gpu, 1 on cpu, 0 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device |
|---|---|---:|---:|---:|---|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 0.8 µs | 2.29 ms | kernels 2.29 ms, store 0.1 µs, rest 0.3 µs | 0.0 µs | – |
| 1 × 32 | cpu | 0.4 µs | 1.2 µs | 265 µs | kernels 264 µs, store 0.2 µs, rest 0.6 µs | 0.0 µs | – |
| 1 × 32 | gpu | 0.5 µs | 1.3 µs | 298 µs | download 0.1 µs, cpu 296 µs, rest 1.4 µs | 4.5 µs | 0.0 µs |
| 16 × 32 | cpu-1 | 0.3 µs | 1.1 µs | 36.40 ms | kernels 36.40 ms, store 0.2 µs, rest 0.5 µs | 0.0 µs | – |
| 16 × 32 | cpu | 0.8 µs | 1.8 µs | 4.23 ms | kernels 4.23 ms, store 0.4 µs, rest 1.3 µs | 0.0 µs | – |
| 16 × 32 | gpu | 0.8 µs | 1.9 µs | 4.36 ms | download 0.2 µs, cpu 4.36 ms, rest 1.8 µs | 9.3 µs | 0.0 µs |
| 64 × 32 | cpu-1 | 2.0 µs | 3.4 µs | 145.34 ms | kernels 145.34 ms, store 1.0 µs, rest 2.6 µs | 0.0 µs | – |
| 64 × 32 | cpu | 1.4 µs | 2.5 µs | 17.62 ms | kernels 17.61 ms, store 0.4 µs, rest 2.1 µs | 0.0 µs | – |
| 64 × 32 | gpu | 1.7 µs | 2.8 µs | 18.46 ms | submit 0.1 µs, download 0.2 µs, cpu 18.46 ms, rest 3.4 µs | 16 µs | 0.0 µs |
