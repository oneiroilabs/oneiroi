# Benchmarks

windows, AMD64 Family 25 Model 97 Stepping 2, AuthenticAMD with 16 threads, AMD Radeon RX 7800 XT on vulkan, driver AMD proprietary driver 26.10.1 (LLPC). Commit 19b4de1.

The time from an edit until its result is computed, the median of many edits. Below each table, where it goes: setting the new value (which hashes it) and planning are the same for every backend, `flush` is the executor, then waiting for the device. The device column is the time the GPU spends on compute passes, by its timestamps.

## nurbs

NURBS Curve, the control points move.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 40 µs | 40 µs | 209 µs | 5.3× slower | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 16 × 32 | 636 µs | 161 µs | 230 µs | 1.4× slower | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 32 | 10.39 ms | 1.38 ms | 250 µs | 5.5× faster | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 4096 × 32 | 167.10 ms | 23.72 ms | 1.30 ms | 18.2× faster | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 1 × 128 | 218 µs | 186 µs | 269 µs | 1.4× slower | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 16 × 128 | 2.84 ms | 513 µs | 274 µs | 1.9× faster | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 128 | 46.58 ms | 5.87 ms | 499 µs | 11.8× faster | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 1024 × 128 | 190.50 ms | 28.74 ms | 1.53 ms | 18.8× faster | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device |
|---|---|---:|---:|---:|---|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 0.4 µs | 39 µs | kernels 39 µs, store 0.2 µs, rest 0.3 µs | 0.0 µs | – |
| 1 × 32 | cpu | 0.1 µs | 0.4 µs | 39 µs | kernels 39 µs, store 0.2 µs, rest 0.3 µs | 0.0 µs | – |
| 1 × 32 | gpu | 0.4 µs | 1.3 µs | 68 µs | upload 7.0 µs, record 3.5 µs, submit 52 µs, download 0.1 µs, rest 4.9 µs | 141 µs | 72 µs |
| 16 × 32 | cpu-1 | 0.5 µs | 0.8 µs | 634 µs | kernels 633 µs, store 0.9 µs, rest 0.5 µs | 0.0 µs | – |
| 16 × 32 | cpu | 1.0 µs | 1.4 µs | 159 µs | kernels 157 µs, store 1.3 µs, rest 0.8 µs | 0.0 µs | – |
| 16 × 32 | gpu | 0.8 µs | 1.5 µs | 70 µs | upload 7.6 µs, record 3.4 µs, submit 52 µs, download 0.1 µs, rest 6.8 µs | 158 µs | 80 µs |
| 256 × 32 | cpu-1 | 8.1 µs | 9.9 µs | 10.38 ms | kernels 10.33 ms, store 36 µs, rest 3.0 µs | 0.0 µs | – |
| 256 × 32 | cpu | 7.0 µs | 4.8 µs | 1.37 ms | kernels 1.33 ms, store 24 µs, rest 1.0 µs | 0.0 µs | – |
| 256 × 32 | gpu | 6.3 µs | 3.5 µs | 73 µs | upload 17 µs, record 3.1 µs, submit 43 µs, download 0.1 µs, rest 10 µs | 162 µs | 90 µs |
| 4096 × 32 | cpu-1 | 350 µs | 105 µs | 166.66 ms | kernels 166.17 ms, store 557 µs, rest 5.0 µs | 0.1 µs | – |
| 4096 × 32 | cpu | 368 µs | 150 µs | 23.22 ms | kernels 22.64 ms, store 569 µs, rest 5.5 µs | 0.0 µs | – |
| 4096 × 32 | gpu | 350 µs | 92 µs | 322 µs | upload 141 µs, record 5.2 µs, submit 71 µs, download 0.1 µs, rest 104 µs | 533 µs | 347 µs |
| 1 × 128 | cpu-1 | 0.4 µs | 0.9 µs | 217 µs | kernels 216 µs, store 0.5 µs, rest 0.5 µs | 0.0 µs | – |
| 1 × 128 | cpu | 0.4 µs | 0.7 µs | 184 µs | kernels 183 µs, store 0.4 µs, rest 0.5 µs | 0.0 µs | – |
| 1 × 128 | gpu | 0.7 µs | 1.5 µs | 77 µs | upload 7.9 µs, record 4.4 µs, submit 57 µs, download 0.1 µs, rest 6.2 µs | 190 µs | 123 µs |
| 16 × 128 | cpu-1 | 2.4 µs | 2.6 µs | 2.83 ms | kernels 2.82 ms, store 9.2 µs, rest 1.1 µs | 0.0 µs | – |
| 16 × 128 | cpu | 2.0 µs | 1.9 µs | 509 µs | kernels 504 µs, store 4.3 µs, rest 0.7 µs | 0.0 µs | – |
| 16 × 128 | gpu | 2.2 µs | 1.9 µs | 61 µs | upload 7.1 µs, record 3.5 µs, submit 44 µs, download 0.1 µs, rest 6.2 µs | 204 µs | 126 µs |
| 256 × 128 | cpu-1 | 28 µs | 24 µs | 46.52 ms | kernels 46.33 ms, store 125 µs, rest 3.4 µs | 0.0 µs | – |
| 256 × 128 | cpu | 28 µs | 20 µs | 5.80 ms | kernels 5.65 ms, store 138 µs, rest 1.7 µs | 0.0 µs | – |
| 256 × 128 | gpu | 24 µs | 11 µs | 110 µs | upload 42 µs, record 3.6 µs, submit 48 µs, download 0.1 µs, rest 16 µs | 351 µs | 184 µs |
| 1024 × 128 | cpu-1 | 371 µs | 143 µs | 190.02 ms | kernels 189.42 ms, store 597 µs, rest 5.7 µs | 0.0 µs | – |
| 1024 × 128 | cpu | 396 µs | 152 µs | 28.23 ms | kernels 27.64 ms, store 664 µs, rest 7.0 µs | 0.0 µs | – |
| 1024 × 128 | gpu | 390 µs | 96 µs | 341 µs | upload 147 µs, record 6.7 µs, submit 84 µs, download 0.2 µs, rest 94 µs | 664 µs | 472 µs |

## translate

Translate Curve of the curves, the offset changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 1.0 µs | 4.5 µs | 120 µs | 26.7× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 32 | 5.2 µs | 16 µs | 117 µs | 7.1× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 32 | 44 µs | 108 µs | 125 µs | 1.2× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 4096 × 32 | 2.93 ms | 2.12 ms | 177 µs | 11.9× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 1 × 128 | 1.5 µs | 8.0 µs | 89 µs | 11.1× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 128 | 12 µs | 50 µs | 86 µs | 1.7× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 128 | 679 µs | 691 µs | 119 µs | 5.8× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 1024 × 128 | 2.95 ms | 2.20 ms | 184 µs | 11.9× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device |
|---|---|---:|---:|---:|---|---:|---:|
| 1 × 32 | cpu-1 | 0.1 µs | 0.4 µs | 0.5 µs | kernels 0.2 µs, store 0.1 µs, rest 0.2 µs | 0.0 µs | – |
| 1 × 32 | cpu | 0.2 µs | 0.6 µs | 3.7 µs | kernels 3.2 µs, store 0.2 µs, rest 0.3 µs | 0.0 µs | – |
| 1 × 32 | gpu | 0.4 µs | 1.3 µs | 58 µs | upload 5.3 µs, record 2.4 µs, submit 47 µs, download 0.1 µs, rest 2.7 µs | 59 µs | 1.8 µs |
| 16 × 32 | cpu-1 | 0.2 µs | 0.7 µs | 4.3 µs | kernels 3.1 µs, store 0.9 µs, rest 0.4 µs | 0.0 µs | – |
| 16 × 32 | cpu | 0.2 µs | 0.8 µs | 16 µs | kernels 14 µs, store 1.0 µs, rest 0.4 µs | 0.0 µs | – |
| 16 × 32 | gpu | 0.4 µs | 1.4 µs | 55 µs | upload 4.8 µs, record 2.6 µs, submit 45 µs, rest 2.8 µs | 59 µs | 2.4 µs |
| 256 × 32 | cpu-1 | 0.2 µs | 0.7 µs | 43 µs | kernels 32 µs, store 11 µs, rest 0.4 µs | 0.0 µs | – |
| 256 × 32 | cpu | 0.4 µs | 1.1 µs | 106 µs | kernels 90 µs, store 14 µs, rest 0.9 µs | 0.0 µs | – |
| 256 × 32 | gpu | 0.4 µs | 1.4 µs | 57 µs | upload 4.9 µs, record 2.6 µs, submit 46 µs, rest 2.9 µs | 67 µs | 6.4 µs |
| 4096 × 32 | cpu-1 | 1.0 µs | 2.5 µs | 2.92 ms | kernels 2.35 ms, store 548 µs, rest 3.1 µs | 0.0 µs | – |
| 4096 × 32 | cpu | 0.8 µs | 2.2 µs | 2.12 ms | kernels 1.59 ms, store 522 µs, rest 2.1 µs | 0.0 µs | – |
| 4096 × 32 | gpu | 0.2 µs | 0.9 µs | 37 µs | upload 2.7 µs, record 1.6 µs, submit 31 µs, rest 1.7 µs | 137 µs | 72 µs |
| 1 × 128 | cpu-1 | 0.1 µs | 0.4 µs | 1.0 µs | kernels 0.5 µs, store 0.2 µs, rest 0.2 µs | 0.0 µs | – |
| 1 × 128 | cpu | 0.2 µs | 0.7 µs | 7.1 µs | kernels 6.3 µs, store 0.3 µs, rest 0.4 µs | 0.0 µs | – |
| 1 × 128 | gpu | 0.3 µs | 0.9 µs | 37 µs | upload 2.8 µs, record 1.6 µs, submit 31 µs, rest 1.8 µs | 48 µs | 2.2 µs |
| 16 × 128 | cpu-1 | 0.1 µs | 0.5 µs | 12 µs | kernels 8.7 µs, store 2.8 µs, rest 0.3 µs | 0.0 µs | – |
| 16 × 128 | cpu | 0.3 µs | 1.0 µs | 48 µs | kernels 43 µs, store 3.8 µs, rest 0.6 µs | 0.0 µs | – |
| 16 × 128 | gpu | 0.2 µs | 0.9 µs | 36 µs | upload 2.7 µs, record 1.6 µs, submit 30 µs, rest 1.6 µs | 49 µs | 3.0 µs |
| 256 × 128 | cpu-1 | 0.2 µs | 0.8 µs | 678 µs | kernels 503 µs, store 175 µs, rest 1.0 µs | 0.0 µs | – |
| 256 × 128 | cpu | 0.4 µs | 1.2 µs | 689 µs | kernels 551 µs, store 131 µs, rest 1.3 µs | 0.0 µs | – |
| 256 × 128 | gpu | 0.2 µs | 0.9 µs | 37 µs | upload 2.7 µs, record 1.7 µs, submit 31 µs, rest 1.7 µs | 80 µs | 22 µs |
| 1024 × 128 | cpu-1 | 0.9 µs | 2.0 µs | 2.95 ms | kernels 2.37 ms, store 586 µs, rest 2.7 µs | 0.0 µs | – |
| 1024 × 128 | cpu | 0.9 µs | 1.9 µs | 2.20 ms | kernels 1.65 ms, store 532 µs, rest 2.0 µs | 0.0 µs | – |
| 1024 × 128 | gpu | 0.3 µs | 0.9 µs | 38 µs | upload 2.8 µs, record 1.7 µs, submit 32 µs, rest 1.7 µs | 144 µs | 78 µs |

## sweep circle

Sweep of a circle along the curves, the radius changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 1.7 µs | 1.7 µs | 108 µs | 63.8× slower | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 16 × 32 | 7.5 µs | 14 µs | 107 µs | 7.7× slower | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 32 | 75 µs | 121 µs | 122 µs | 1.0× slower | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 4096 × 32 | 6.03 ms | 5.27 ms | 223 µs | 23.6× faster | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 1 × 128 | 2.4 µs | 2.2 µs | 107 µs | 48.7× slower | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 16 × 128 | 19 µs | 59 µs | 108 µs | 1.8× slower | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 128 | 900 µs | 912 µs | 141 µs | 6.5× faster | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 1024 × 128 | 5.00 ms | 4.16 ms | 219 µs | 19.0× faster | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device |
|---|---|---:|---:|---:|---|---:|---:|
| 1 × 32 | cpu-1 | 0.1 µs | 0.6 µs | 1.0 µs | kernels 0.4 µs, store 0.2 µs, rest 0.3 µs | 0.0 µs | – |
| 1 × 32 | cpu | 0.1 µs | 0.6 µs | 1.0 µs | kernels 0.5 µs, store 0.3 µs, rest 0.3 µs | 0.0 µs | – |
| 1 × 32 | gpu | 0.3 µs | 1.1 µs | 45 µs | upload 3.8 µs, record 2.7 µs, submit 35 µs, download 0.1 µs, rest 2.9 µs | 61 µs | 2.9 µs |
| 16 × 32 | cpu-1 | 0.1 µs | 0.8 µs | 6.4 µs | kernels 4.9 µs, store 1.1 µs, rest 0.4 µs | 0.0 µs | – |
| 16 × 32 | cpu | 0.2 µs | 1.0 µs | 13 µs | kernels 11 µs, store 1.3 µs, rest 0.6 µs | 0.0 µs | – |
| 16 × 32 | gpu | 0.3 µs | 1.1 µs | 44 µs | upload 3.8 µs, record 2.8 µs, submit 34 µs, download 0.1 µs, rest 3.1 µs | 61 µs | 4.6 µs |
| 256 × 32 | cpu-1 | 0.2 µs | 0.8 µs | 74 µs | kernels 61 µs, store 12 µs, rest 0.4 µs | 0.0 µs | – |
| 256 × 32 | cpu | 0.3 µs | 1.1 µs | 119 µs | kernels 104 µs, store 14 µs, rest 0.7 µs | 0.0 µs | – |
| 256 × 32 | gpu | 0.3 µs | 1.1 µs | 47 µs | upload 3.9 µs, record 2.7 µs, submit 36 µs, download 0.1 µs, rest 4.1 µs | 72 µs | 11 µs |
| 4096 × 32 | cpu-1 | 1.0 µs | 3.2 µs | 6.03 ms | kernels 5.45 ms, store 602 µs, rest 2.8 µs | 0.0 µs | – |
| 4096 × 32 | cpu | 1.3 µs | 3.2 µs | 5.27 ms | kernels 4.62 ms, store 645 µs, rest 2.9 µs | 0.0 µs | – |
| 4096 × 32 | gpu | 0.3 µs | 1.2 µs | 58 µs | upload 5.1 µs, record 2.8 µs, submit 38 µs, download 0.1 µs, rest 11 µs | 163 µs | 93 µs |
| 1 × 128 | cpu-1 | 0.1 µs | 0.6 µs | 1.6 µs | kernels 0.9 µs, store 0.4 µs, rest 0.3 µs | 0.0 µs | – |
| 1 × 128 | cpu | 0.1 µs | 0.6 µs | 1.5 µs | kernels 0.8 µs, store 0.4 µs, rest 0.3 µs | 0.0 µs | – |
| 1 × 128 | gpu | 0.3 µs | 1.2 µs | 45 µs | upload 3.8 µs, record 2.8 µs, submit 35 µs, download 0.1 µs, rest 2.9 µs | 60 µs | 3.4 µs |
| 16 × 128 | cpu-1 | 0.2 µs | 0.7 µs | 18 µs | kernels 14 µs, store 3.7 µs, rest 0.4 µs | 0.0 µs | – |
| 16 × 128 | cpu | 0.3 µs | 1.3 µs | 57 µs | kernels 52 µs, store 4.7 µs, rest 0.8 µs | 0.0 µs | – |
| 16 × 128 | gpu | 0.3 µs | 1.1 µs | 44 µs | upload 3.8 µs, record 2.7 µs, submit 35 µs, download 0.1 µs, rest 3.1 µs | 61 µs | 5.2 µs |
| 256 × 128 | cpu-1 | 0.3 µs | 1.2 µs | 898 µs | kernels 710 µs, store 159 µs, rest 0.8 µs | 0.0 µs | – |
| 256 × 128 | cpu | 0.4 µs | 1.7 µs | 909 µs | kernels 730 µs, store 186 µs, rest 1.2 µs | 0.0 µs | – |
| 256 × 128 | gpu | 0.3 µs | 1.1 µs | 46 µs | upload 3.8 µs, record 2.8 µs, submit 36 µs, download 0.1 µs, rest 3.9 µs | 93 µs | 30 µs |
| 1024 × 128 | cpu-1 | 0.8 µs | 3.2 µs | 5.00 ms | kernels 4.34 ms, store 683 µs, rest 2.8 µs | 0.0 µs | – |
| 1024 × 128 | cpu | 1.2 µs | 2.9 µs | 4.15 ms | kernels 3.50 ms, store 670 µs, rest 2.4 µs | 0.0 µs | – |
| 1024 × 128 | gpu | 0.3 µs | 1.1 µs | 48 µs | upload 4.1 µs, record 2.8 µs, submit 36 µs, download 0.1 µs, rest 5.3 µs | 168 µs | 99 µs |

## sweep square

Sweep of a square curve along the curves, the square changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 10 µs | 10 µs | 171 µs | 16.4× slower | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 16 × 32 | 66 µs | 77 µs | 172 µs | 2.2× slower | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 256 × 32 | 1.98 ms | 1.82 ms | 214 µs | 8.5× faster | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 4096 × 32 | 74.05 ms | 54.99 ms | 800 µs | 68.7× faster | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 1 × 128 | 21 µs | 26 µs | 175 µs | 6.6× slower | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 16 × 128 | 500 µs | 513 µs | 183 µs | 2.8× faster | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 256 × 128 | 10.70 ms | 8.15 ms | 349 µs | 23.4× faster | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 1024 × 128 | 55.84 ms | 42.71 ms | 714 µs | 59.9× faster | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device |
|---|---|---:|---:|---:|---|---:|---:|
| 1 × 32 | cpu-1 | 0.1 µs | 0.7 µs | 9.6 µs | kernels 8.6 µs, store 0.6 µs, rest 0.4 µs | 0.0 µs | – |
| 1 × 32 | cpu | 0.1 µs | 0.7 µs | 9.5 µs | kernels 8.6 µs, store 0.7 µs, rest 0.4 µs | 0.0 µs | – |
| 1 × 32 | gpu | 0.3 µs | 1.3 µs | 53 µs | upload 4.1 µs, record 4.1 µs, submit 40 µs, download 0.2 µs, rest 4.8 µs | 116 µs | 48 µs |
| 16 × 32 | cpu-1 | 0.2 µs | 0.8 µs | 65 µs | kernels 58 µs, store 6.3 µs, rest 0.5 µs | 0.0 µs | – |
| 16 × 32 | cpu | 0.3 µs | 1.0 µs | 76 µs | kernels 67 µs, store 7.2 µs, rest 0.6 µs | 0.0 µs | – |
| 16 × 32 | gpu | 0.3 µs | 1.3 µs | 54 µs | upload 5.0 µs, record 3.8 µs, submit 40 µs, download 0.2 µs, rest 5.0 µs | 114 µs | 50 µs |
| 256 × 32 | cpu-1 | 0.3 µs | 1.3 µs | 1.98 ms | kernels 1.67 ms, store 308 µs, rest 0.9 µs | 0.0 µs | – |
| 256 × 32 | cpu | 0.6 µs | 1.6 µs | 1.81 ms | kernels 1.50 ms, store 310 µs, rest 1.3 µs | 0.0 µs | – |
| 256 × 32 | gpu | 0.3 µs | 1.3 µs | 56 µs | upload 5.2 µs, record 3.9 µs, submit 40 µs, download 0.2 µs, rest 6.2 µs | 155 µs | 87 µs |
| 4096 × 32 | cpu-1 | 2.3 µs | 4.3 µs | 74.05 ms | kernels 69.70 ms, store 4.40 ms, rest 5.1 µs | 0.0 µs | – |
| 4096 × 32 | cpu | 2.7 µs | 4.1 µs | 54.98 ms | kernels 50.88 ms, store 4.16 ms, rest 4.9 µs | 0.0 µs | – |
| 4096 × 32 | gpu | 0.4 µs | 1.4 µs | 72 µs | upload 6.7 µs, record 4.1 µs, submit 45 µs, download 0.2 µs, rest 16 µs | 730 µs | 653 µs |
| 1 × 128 | cpu-1 | 0.1 µs | 0.7 µs | 20 µs | kernels 18 µs, store 1.7 µs, rest 0.3 µs | 0.0 µs | – |
| 1 × 128 | cpu | 0.2 µs | 1.1 µs | 25 µs | kernels 22 µs, store 2.1 µs, rest 0.6 µs | 0.0 µs | – |
| 1 × 128 | gpu | 0.3 µs | 1.3 µs | 55 µs | upload 5.1 µs, record 4.0 µs, submit 41 µs, download 0.2 µs, rest 5.0 µs | 117 µs | 48 µs |
| 16 × 128 | cpu-1 | 0.3 µs | 0.9 µs | 499 µs | kernels 431 µs, store 66 µs, rest 0.7 µs | 0.0 µs | – |
| 16 × 128 | cpu | 0.5 µs | 1.7 µs | 511 µs | kernels 424 µs, store 65 µs, rest 1.1 µs | 0.0 µs | – |
| 16 × 128 | gpu | 0.3 µs | 1.2 µs | 54 µs | upload 5.0 µs, record 3.9 µs, submit 40 µs, download 0.1 µs, rest 4.9 µs | 126 µs | 59 µs |
| 256 × 128 | cpu-1 | 1.8 µs | 3.3 µs | 10.70 ms | kernels 9.51 ms, store 1.23 ms, rest 2.9 µs | 0.0 µs | – |
| 256 × 128 | cpu | 1.9 µs | 3.1 µs | 8.15 ms | kernels 6.93 ms, store 1.22 ms, rest 2.9 µs | 0.0 µs | – |
| 256 × 128 | gpu | 0.4 µs | 1.3 µs | 59 µs | upload 5.4 µs, record 4.0 µs, submit 42 µs, download 0.2 µs, rest 6.3 µs | 287 µs | 213 µs |
| 1024 × 128 | cpu-1 | 2.1 µs | 3.7 µs | 55.84 ms | kernels 51.40 ms, store 4.38 ms, rest 4.7 µs | 0.0 µs | – |
| 1024 × 128 | cpu | 2.4 µs | 3.7 µs | 42.71 ms | kernels 38.20 ms, store 4.57 ms, rest 3.9 µs | 0.0 µs | – |
| 1024 × 128 | gpu | 0.4 µs | 1.5 µs | 64 µs | upload 6.1 µs, record 4.1 µs, submit 45 µs, download 0.2 µs, rest 8.4 µs | 644 µs | 570 µs |

## sample

Sample Curve, 64 frames per curve at places which change.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 37 µs | 37 µs | 110 µs | 2.9× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 32 | 587 µs | 152 µs | 108 µs | 1.4× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 32 | 9.58 ms | 1.53 ms | 113 µs | 13.6× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 4096 × 32 | 152.11 ms | 20.95 ms | 198 µs | 105.7× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 1 × 128 | 37 µs | 37 µs | 109 µs | 3.0× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 128 | 579 µs | 152 µs | 108 µs | 1.4× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 128 | 9.41 ms | 1.39 ms | 114 µs | 12.1× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 1024 × 128 | 38.79 ms | 5.59 ms | 160 µs | 35.0× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device |
|---|---|---:|---:|---:|---|---:|---:|
| 1 × 32 | cpu-1 | 0.1 µs | 0.6 µs | 36 µs | kernels 36 µs, store 0.2 µs, rest 0.2 µs | 0.0 µs | – |
| 1 × 32 | cpu | 0.2 µs | 0.6 µs | 37 µs | kernels 36 µs, store 0.2 µs, rest 0.3 µs | 0.0 µs | – |
| 1 × 32 | gpu | 0.4 µs | 1.1 µs | 39 µs | upload 3.7 µs, record 1.7 µs, submit 31 µs, download 0.1 µs, rest 2.2 µs | 69 µs | 11 µs |
| 16 × 32 | cpu-1 | 0.2 µs | 0.6 µs | 586 µs | kernels 584 µs, store 1.1 µs, rest 0.3 µs | 0.0 µs | – |
| 16 × 32 | cpu | 0.4 µs | 1.3 µs | 150 µs | kernels 148 µs, store 1.7 µs, rest 0.7 µs | 0.0 µs | – |
| 16 × 32 | gpu | 0.3 µs | 1.1 µs | 38 µs | upload 2.9 µs, record 1.7 µs, submit 31 µs, download 0.1 µs, rest 2.3 µs | 68 µs | 11 µs |
| 256 × 32 | cpu-1 | 0.6 µs | 1.9 µs | 9.58 ms | kernels 9.54 ms, store 36 µs, rest 1.2 µs | 0.0 µs | – |
| 256 × 32 | cpu | 0.6 µs | 1.7 µs | 1.53 ms | kernels 1.50 ms, store 24 µs, rest 1.1 µs | 0.0 µs | – |
| 256 × 32 | gpu | 0.4 µs | 1.1 µs | 40 µs | upload 3.0 µs, record 1.7 µs, submit 32 µs, download 0.1 µs, rest 3.4 µs | 71 µs | 12 µs |
| 4096 × 32 | cpu-1 | 2.4 µs | 3.6 µs | 152.10 ms | kernels 151.38 ms, store 698 µs, rest 4.9 µs | 0.0 µs | – |
| 4096 × 32 | cpu | 1.9 µs | 3.3 µs | 20.94 ms | kernels 20.26 ms, store 711 µs, rest 4.3 µs | 0.0 µs | – |
| 4096 × 32 | gpu | 0.4 µs | 1.1 µs | 54 µs | upload 4.0 µs, record 1.7 µs, submit 34 µs, download 0.1 µs, rest 14 µs | 142 µs | 78 µs |
| 1 × 128 | cpu-1 | 0.1 µs | 0.6 µs | 36 µs | kernels 35 µs, store 0.2 µs, rest 0.2 µs | 0.0 µs | – |
| 1 × 128 | cpu | 0.1 µs | 0.6 µs | 36 µs | kernels 35 µs, store 0.2 µs, rest 0.3 µs | 0.0 µs | – |
| 1 × 128 | gpu | 0.4 µs | 1.1 µs | 38 µs | upload 2.9 µs, record 1.7 µs, submit 31 µs, download 0.1 µs, rest 2.2 µs | 69 µs | 11 µs |
| 16 × 128 | cpu-1 | 0.2 µs | 0.7 µs | 578 µs | kernels 577 µs, store 1.1 µs, rest 0.4 µs | 0.0 µs | – |
| 16 × 128 | cpu | 0.4 µs | 1.3 µs | 150 µs | kernels 148 µs, store 1.7 µs, rest 0.7 µs | 0.0 µs | – |
| 16 × 128 | gpu | 0.4 µs | 1.1 µs | 38 µs | upload 2.9 µs, record 1.7 µs, submit 31 µs, download 0.1 µs, rest 2.4 µs | 68 µs | 12 µs |
| 256 × 128 | cpu-1 | 0.5 µs | 1.6 µs | 9.41 ms | kernels 9.37 ms, store 34 µs, rest 1.2 µs | 0.0 µs | – |
| 256 × 128 | cpu | 0.6 µs | 1.7 µs | 1.38 ms | kernels 1.35 ms, store 25 µs, rest 1.1 µs | 0.0 µs | – |
| 256 × 128 | gpu | 0.4 µs | 1.1 µs | 40 µs | upload 3.0 µs, record 1.7 µs, submit 32 µs, download 0.1 µs, rest 3.5 µs | 72 µs | 15 µs |
| 1024 × 128 | cpu-1 | 2.3 µs | 4.1 µs | 38.79 ms | kernels 38.58 ms, store 208 µs, rest 3.9 µs | 0.0 µs | – |
| 1024 × 128 | cpu | 1.3 µs | 2.6 µs | 5.59 ms | kernels 5.37 ms, store 212 µs, rest 2.1 µs | 0.0 µs | – |
| 1024 × 128 | gpu | 0.4 µs | 1.2 µs | 44 µs | upload 3.3 µs, record 1.7 µs, submit 33 µs, download 0.1 µs, rest 5.5 µs | 112 µs | 47 µs |

## chain

NURBS, Translate, a Sweep of a circle and Sample Curve, the control points move.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 79 µs | 128 µs | 241 µs | 1.9× slower | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 16 × 32 | 1.23 ms | 351 µs | 253 µs | 1.4× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 256 × 32 | 19.97 ms | 2.87 ms | 313 µs | 9.2× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 4096 × 32 | 324.25 ms | 48.93 ms | 1.51 ms | 32.4× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 1 × 128 | 208 µs | 256 µs | 283 µs | 1.1× slower | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 16 × 128 | 3.33 ms | 772 µs | 311 µs | 2.5× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 256 × 128 | 55.46 ms | 9.29 ms | 582 µs | 16.0× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 1024 × 128 | 218.89 ms | 34.83 ms | 1.63 ms | 21.4× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device |
|---|---|---:|---:|---:|---|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 1.2 µs | 78 µs | kernels 77 µs, store 0.6 µs, rest 0.6 µs | 0.0 µs | – |
| 1 × 32 | cpu | 0.2 µs | 1.5 µs | 125 µs | kernels 123 µs, store 1.0 µs, rest 1.0 µs | 0.0 µs | – |
| 1 × 32 | gpu | 0.4 µs | 1.9 µs | 70 µs | upload 6.8 µs, record 6.6 µs, submit 49 µs, download 0.3 µs, rest 7.3 µs | 168 µs | 87 µs |
| 16 × 32 | cpu-1 | 0.6 µs | 1.5 µs | 1.23 ms | kernels 1.22 ms, store 3.8 µs, rest 0.8 µs | 0.0 µs | – |
| 16 × 32 | cpu | 1.0 µs | 2.7 µs | 348 µs | kernels 339 µs, store 6.7 µs, rest 1.8 µs | 0.0 µs | – |
| 16 × 32 | gpu | 0.7 µs | 2.1 µs | 73 µs | upload 7.1 µs, record 6.4 µs, submit 50 µs, download 0.3 µs, rest 9.0 µs | 175 µs | 98 µs |
| 256 × 32 | cpu-1 | 8.7 µs | 11 µs | 19.95 ms | kernels 19.82 ms, store 124 µs, rest 4.1 µs | 0.0 µs | – |
| 256 × 32 | cpu | 7.3 µs | 6.4 µs | 2.85 ms | kernels 2.74 ms, store 94 µs, rest 2.5 µs | 0.0 µs | – |
| 256 × 32 | gpu | 6.6 µs | 4.3 µs | 95 µs | upload 19 µs, record 6.5 µs, submit 53 µs, download 0.3 µs, rest 15 µs | 206 µs | 120 µs |
| 4096 × 32 | cpu-1 | 334 µs | 151 µs | 323.82 ms | kernels 321.33 ms, store 2.42 ms, rest 12 µs | 0.0 µs | – |
| 4096 × 32 | cpu | 341 µs | 147 µs | 48.45 ms | kernels 46.04 ms, store 2.46 ms, rest 9.8 µs | 0.0 µs | – |
| 4096 × 32 | gpu | 309 µs | 86 µs | 325 µs | upload 140 µs, record 7.5 µs, submit 68 µs, download 0.3 µs, rest 106 µs | 784 µs | 595 µs |
| 1 × 128 | cpu-1 | 0.2 µs | 1.2 µs | 206 µs | kernels 204 µs, store 0.9 µs, rest 0.6 µs | 0.0 µs | – |
| 1 × 128 | cpu | 0.3 µs | 1.5 µs | 254 µs | kernels 252 µs, store 1.6 µs, rest 0.9 µs | 0.0 µs | – |
| 1 × 128 | gpu | 0.4 µs | 1.9 µs | 70 µs | upload 6.6 µs, record 6.4 µs, submit 49 µs, download 0.3 µs, rest 7.3 µs | 210 µs | 139 µs |
| 16 × 128 | cpu-1 | 2.0 µs | 2.2 µs | 3.32 ms | kernels 3.31 ms, store 13 µs, rest 1.0 µs | 0.0 µs | – |
| 16 × 128 | cpu | 2.2 µs | 3.0 µs | 766 µs | kernels 743 µs, store 21 µs, rest 1.8 µs | 0.0 µs | – |
| 16 × 128 | gpu | 2.0 µs | 2.7 µs | 80 µs | upload 8.9 µs, record 6.6 µs, submit 53 µs, download 0.3 µs, rest 10 µs | 224 µs | 152 µs |
| 256 × 128 | cpu-1 | 28 µs | 33 µs | 55.41 ms | kernels 54.88 ms, store 508 µs, rest 8.0 µs | 0.0 µs | – |
| 256 × 128 | cpu | 29 µs | 36 µs | 9.23 ms | kernels 8.68 ms, store 514 µs, rest 5.9 µs | 0.0 µs | – |
| 256 × 128 | gpu | 26 µs | 16 µs | 129 µs | upload 42 µs, record 6.9 µs, submit 56 µs, download 0.3 µs, rest 22 µs | 409 µs | 250 µs |
| 1024 × 128 | cpu-1 | 331 µs | 95 µs | 218.43 ms | kernels 216.41 ms, store 2.02 ms, rest 10 µs | 0.0 µs | – |
| 1024 × 128 | cpu | 342 µs | 96 µs | 34.25 ms | kernels 32.41 ms, store 1.92 ms, rest 9.4 µs | 0.0 µs | – |
| 1024 × 128 | gpu | 323 µs | 143 µs | 288 µs | upload 139 µs, record 7.8 µs, submit 71 µs, download 0.3 µs, rest 68 µs | 876 µs | 694 µs |

## instances

A ring of 10 patches put onto 64 frames along every curve by reference, the control points move.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 80 µs | 137 µs | 228 µs | 1.7× slower | 3 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 16 × 32 | 1.23 ms | 388 µs | 240 µs | 1.6× faster | 3 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 256 × 32 | 19.99 ms | 3.04 ms | 294 µs | 10.4× faster | 3 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 1024 × 32 | 80.68 ms | 12.03 ms | 541 µs | 22.2× faster | 3 on gpu, 0 on cpu, 4 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device |
|---|---|---:|---:|---:|---|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 1.3 µs | 79 µs | kernels 78 µs, store 0.5 µs, rest 0.5 µs | 0.0 µs | – |
| 1 × 32 | cpu | 0.4 µs | 2.3 µs | 133 µs | kernels 131 µs, store 1.0 µs, rest 1.0 µs | 0.0 µs | – |
| 1 × 32 | gpu | 0.4 µs | 1.9 µs | 63 µs | upload 6.0 µs, record 5.0 µs, submit 45 µs, download 0.2 µs, rest 6.4 µs | 162 µs | 84 µs |
| 16 × 32 | cpu-1 | 0.5 µs | 1.5 µs | 1.23 ms | kernels 1.23 ms, store 2.8 µs, rest 0.6 µs | 0.0 µs | – |
| 16 × 32 | cpu | 0.8 µs | 2.6 µs | 384 µs | kernels 377 µs, store 5.0 µs, rest 1.2 µs | 0.0 µs | – |
| 16 × 32 | gpu | 0.7 µs | 2.0 µs | 65 µs | upload 7.1 µs, record 5.1 µs, submit 45 µs, download 0.2 µs, rest 7.9 µs | 170 µs | 92 µs |
| 256 × 32 | cpu-1 | 8.2 µs | 10 µs | 19.98 ms | kernels 19.88 ms, store 96 µs, rest 2.6 µs | 0.0 µs | – |
| 256 × 32 | cpu | 7.0 µs | 6.1 µs | 3.03 ms | kernels 2.95 ms, store 67 µs, rest 1.9 µs | 0.0 µs | – |
| 256 × 32 | gpu | 6.4 µs | 4.6 µs | 93 µs | upload 22 µs, record 5.5 µs, submit 48 µs, download 0.2 µs, rest 18 µs | 188 µs | 107 µs |
| 1024 × 32 | cpu-1 | 28 µs | 28 µs | 80.63 ms | kernels 80.11 ms, store 526 µs, rest 6.7 µs | 0.0 µs | – |
| 1024 × 32 | cpu | 28 µs | 31 µs | 11.96 ms | kernels 11.45 ms, store 508 µs, rest 3.9 µs | 0.0 µs | – |
| 1024 × 32 | gpu | 26 µs | 33 µs | 160 µs | upload 55 µs, record 5.5 µs, submit 53 µs, download 0.3 µs, rest 45 µs | 327 µs | 165 µs |

## instances realized

The rings of `instances`, copied to where each one is.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 161 µs | 159 µs | 416 µs | 2.6× slower | 4 on gpu, 0 on cpu, 5 passes, 1 downloads |
| 16 × 32 | 2.66 ms | 876 µs | 435 µs | 2.0× faster | 4 on gpu, 0 on cpu, 5 passes, 1 downloads |
| 256 × 32 | 47.34 ms | 12.64 ms | 676 µs | 18.7× faster | 4 on gpu, 0 on cpu, 5 passes, 1 downloads |
| 1024 × 32 | 198.45 ms | 51.72 ms | 1.45 ms | 35.7× faster | 4 on gpu, 0 on cpu, 5 passes, 1 downloads |

| size | backend | set | plan | flush | of which | wait | device |
|---|---|---:|---:|---:|---|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 1.6 µs | 160 µs | kernels 157 µs, store 1.7 µs, rest 0.7 µs | 0.0 µs | – |
| 1 × 32 | cpu | 0.4 µs | 2.8 µs | 156 µs | kernels 152 µs, store 2.6 µs, rest 1.2 µs | 0.0 µs | – |
| 1 × 32 | gpu | 0.5 µs | 2.7 µs | 347 µs | upload 10 µs, record 8.1 µs, submit 90 µs, download 228 µs, rest 11 µs | 63 µs | 92 µs |
| 16 × 32 | cpu-1 | 0.7 µs | 2.3 µs | 2.65 ms | kernels 2.62 ms, store 28 µs, rest 1.2 µs | 0.0 µs | – |
| 16 × 32 | cpu | 0.9 µs | 3.3 µs | 871 µs | kernels 842 µs, store 27 µs, rest 2.2 µs | 0.0 µs | – |
| 16 × 32 | gpu | 0.8 µs | 2.5 µs | 357 µs | upload 11 µs, record 7.3 µs, submit 85 µs, download 242 µs, rest 12 µs | 73 µs | 108 µs |
| 256 × 32 | cpu-1 | 9.4 µs | 13 µs | 47.32 ms | kernels 46.37 ms, store 942 µs, rest 6.8 µs | 0.0 µs | – |
| 256 × 32 | cpu | 9.0 µs | 11 µs | 12.62 ms | kernels 11.71 ms, store 950 µs, rest 5.7 µs | 0.0 µs | – |
| 256 × 32 | gpu | 8.2 µs | 5.9 µs | 469 µs | upload 31 µs, record 8.4 µs, submit 99 µs, download 291 µs, rest 37 µs | 191 µs | 224 µs |
| 1024 × 32 | cpu-1 | 34 µs | 33 µs | 198.38 ms | kernels 194.81 ms, store 3.68 ms, rest 11 µs | 0.0 µs | – |
| 1024 × 32 | cpu | 35 µs | 34 µs | 51.65 ms | kernels 47.98 ms, store 3.72 ms, rest 8.3 µs | 0.0 µs | – |
| 1024 × 32 | gpu | 32 µs | 28 µs | 918 µs | upload 77 µs, record 8.7 µs, submit 107 µs, download 582 µs, rest 138 µs | 443 µs | 598 µs |

## mesh

Mesh of a swept circle, the tolerance changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 2.44 ms | 449 µs | 375 µs | 1.2× faster | 0 on gpu, 1 on cpu, 0 passes, 0 downloads |
| 16 × 32 | 39.31 ms | 7.07 ms | 6.98 ms | 1.0× faster | 0 on gpu, 1 on cpu, 0 passes, 0 downloads |
| 64 × 32 | 157.75 ms | 33.18 ms | 33.11 ms | 1.0× faster | 0 on gpu, 1 on cpu, 0 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device |
|---|---|---:|---:|---:|---|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 1.1 µs | 2.44 ms | kernels 2.43 ms, store 5.3 µs, rest 0.5 µs | 0.0 µs | – |
| 1 × 32 | cpu | 0.5 µs | 1.5 µs | 446 µs | kernels 440 µs, store 5.3 µs, rest 0.8 µs | 0.0 µs | – |
| 1 × 32 | gpu | 0.4 µs | 1.5 µs | 364 µs | download 0.1 µs, cpu 362 µs, rest 1.7 µs | 6.3 µs | 0.0 µs |
| 16 × 32 | cpu-1 | 1.8 µs | 4.2 µs | 39.30 ms | kernels 39.01 ms, store 251 µs, rest 4.3 µs | 0.0 µs | – |
| 16 × 32 | cpu | 1.3 µs | 3.4 µs | 7.06 ms | kernels 6.81 ms, store 232 µs, rest 2.8 µs | 0.0 µs | – |
| 16 × 32 | gpu | 0.9 µs | 2.8 µs | 6.96 ms | download 0.1 µs, cpu 6.96 ms, rest 3.8 µs | 13 µs | 0.0 µs |
| 64 × 32 | cpu-1 | 2.1 µs | 4.3 µs | 157.74 ms | kernels 156.96 ms, store 777 µs, rest 5.5 µs | 0.0 µs | – |
| 64 × 32 | cpu | 2.0 µs | 4.1 µs | 33.17 ms | kernels 32.47 ms, store 710 µs, rest 4.1 µs | 0.0 µs | – |
| 64 × 32 | gpu | 1.7 µs | 4.2 µs | 33.08 ms | submit 0.1 µs, download 0.2 µs, cpu 33.07 ms, rest 5.7 µs | 21 µs | 0.0 µs |
