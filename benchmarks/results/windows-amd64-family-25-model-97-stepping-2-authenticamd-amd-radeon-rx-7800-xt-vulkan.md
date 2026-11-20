# Benchmarks

windows, AMD64 Family 25 Model 97 Stepping 2, AuthenticAMD with 16 threads, AMD Radeon RX 7800 XT on vulkan, driver AMD proprietary driver 26.11.2 (LLPC). Commit 6446ed0.

The time from an edit until its result is computed, the median of many edits. Below each table, where it goes: setting the new value (which hashes it) and planning are the same for every backend, `flush` is the executor, then waiting for the device. The device column is the time the GPU spends on compute passes, by its timestamps. Ending the frame afterwards frees what is not used anymore, it is not in the total.

## nurbs

NURBS Curve, the control points move.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 38 µs | 38 µs | 186 µs | 4.9× slower | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 16 × 32 | 594 µs | 102 µs | 209 µs | 2.1× slower | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 32 | 9.54 ms | 1.19 ms | 266 µs | 4.5× faster | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 4096 × 32 | 155.67 ms | 18.74 ms | 927 µs | 20.2× faster | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 1 × 128 | 160 µs | 160 µs | 243 µs | 1.5× slower | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 16 × 128 | 2.58 ms | 312 µs | 264 µs | 1.2× faster | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 128 | 41.96 ms | 4.91 ms | 489 µs | 10.1× faster | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 1024 × 128 | 168.57 ms | 19.94 ms | 1.01 ms | 19.6× faster | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 0.2 µs | 38 µs | kernels 38 µs, rest 0.3 µs | 0.0 µs | – | 0.1 µs |
| 1 × 32 | cpu | 0.2 µs | 0.2 µs | 38 µs | kernels 38 µs, rest 0.3 µs | 0.0 µs | – | 0.1 µs |
| 1 × 32 | gpu | 0.5 µs | 0.6 µs | 49 µs | upload 4.8 µs, record 3.0 µs, submit 38 µs, rest 3.8 µs | 136 µs | 72 µs | 0.7 µs |
| 16 × 32 | cpu-1 | 0.5 µs | 0.3 µs | 593 µs | kernels 593 µs, rest 0.3 µs | 0.0 µs | – | 0.1 µs |
| 16 × 32 | cpu | 1.1 µs | 0.5 µs | 99 µs | kernels 99 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – | 0.3 µs |
| 16 × 32 | gpu | 0.9 µs | 0.6 µs | 54 µs | upload 5.3 µs, record 3.1 µs, submit 40 µs, download 0.1 µs, rest 5.2 µs | 154 µs | 80 µs | 0.7 µs |
| 256 × 32 | cpu-1 | 5.9 µs | 0.5 µs | 9.53 ms | kernels 9.53 ms, store 0.1 µs, rest 0.7 µs | 0.0 µs | – | 0.3 µs |
| 256 × 32 | cpu | 7.6 µs | 1.0 µs | 1.18 ms | kernels 1.18 ms, store 0.1 µs, rest 1.0 µs | 0.0 µs | – | 0.6 µs |
| 256 × 32 | gpu | 6.7 µs | 0.8 µs | 87 µs | upload 21 µs, record 3.4 µs, submit 51 µs, download 0.1 µs, rest 11 µs | 172 µs | 91 µs | 1.2 µs |
| 4096 × 32 | cpu-1 | 157 µs | 2.1 µs | 155.44 ms | kernels 155.44 ms, store 0.7 µs, rest 2.9 µs | 0.0 µs | – | 1.1 µs |
| 4096 × 32 | cpu | 147 µs | 1.4 µs | 18.60 ms | kernels 18.60 ms, store 0.5 µs, rest 2.6 µs | 0.0 µs | – | 1.3 µs |
| 4096 × 32 | gpu | 126 µs | 0.9 µs | 268 µs | upload 138 µs, record 3.6 µs, submit 52 µs, download 0.1 µs, rest 73 µs | 517 µs | 350 µs | 0.9 µs |
| 1 × 128 | cpu-1 | 0.3 µs | 0.2 µs | 160 µs | kernels 160 µs, rest 0.3 µs | 0.0 µs | – | 0.1 µs |
| 1 × 128 | cpu | 0.3 µs | 0.2 µs | 160 µs | kernels 159 µs, rest 0.3 µs | 0.0 µs | – | 0.1 µs |
| 1 × 128 | gpu | 0.6 µs | 0.6 µs | 50 µs | upload 4.9 µs, record 3.1 µs, submit 38 µs, download 0.1 µs, rest 4.0 µs | 191 µs | 127 µs | 0.7 µs |
| 16 × 128 | cpu-1 | 1.6 µs | 0.4 µs | 2.58 ms | kernels 2.58 ms, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 16 × 128 | cpu | 2.6 µs | 0.5 µs | 309 µs | kernels 308 µs, store 0.1 µs, rest 0.7 µs | 0.0 µs | – | 0.4 µs |
| 16 × 128 | gpu | 2.0 µs | 0.6 µs | 56 µs | upload 7.6 µs, record 3.2 µs, submit 40 µs, download 0.1 µs, rest 5.4 µs | 205 µs | 132 µs | 0.7 µs |
| 256 × 128 | cpu-1 | 29 µs | 0.8 µs | 41.92 ms | kernels 41.92 ms, store 0.3 µs, rest 1.5 µs | 0.0 µs | – | 0.5 µs |
| 256 × 128 | cpu | 26 µs | 1.1 µs | 4.88 ms | kernels 4.88 ms, store 0.2 µs, rest 1.3 µs | 0.0 µs | – | 0.6 µs |
| 256 × 128 | gpu | 23 µs | 0.9 µs | 99 µs | upload 40 µs, record 3.2 µs, submit 40 µs, download 0.1 µs, rest 15 µs | 364 µs | 185 µs | 0.9 µs |
| 1024 × 128 | cpu-1 | 158 µs | 2.1 µs | 168.41 ms | kernels 168.41 ms, store 0.6 µs, rest 2.6 µs | 0.1 µs | – | 1.5 µs |
| 1024 × 128 | cpu | 135 µs | 1.4 µs | 19.80 ms | kernels 19.80 ms, store 0.4 µs, rest 2.2 µs | 0.0 µs | – | 1.3 µs |
| 1024 × 128 | gpu | 124 µs | 0.8 µs | 237 µs | upload 138 µs, record 3.6 µs, submit 51 µs, download 0.1 µs, rest 42 µs | 641 µs | 472 µs | 0.9 µs |

## translate

Translate Curve of the curves, the offset changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 0.7 µs | 3.9 µs | 95 µs | 24.3× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 32 | 1.7 µs | 10 µs | 95 µs | 9.1× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 32 | 12 µs | 19 µs | 100 µs | 5.3× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 4096 × 32 | 339 µs | 100 µs | 177 µs | 1.8× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 1 × 128 | 0.9 µs | 6.4 µs | 89 µs | 13.9× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 128 | 3.6 µs | 13 µs | 89 µs | 6.7× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 128 | 50 µs | 29 µs | 121 µs | 4.2× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 1024 × 128 | 415 µs | 149 µs | 183 µs | 1.2× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 1 × 32 | cpu-1 | 0.1 µs | 0.2 µs | 0.3 µs | kernels 0.1 µs, rest 0.2 µs | 0.0 µs | – | 0.1 µs |
| 1 × 32 | cpu | 0.3 µs | 0.3 µs | 3.3 µs | kernels 2.9 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 1 × 32 | gpu | 0.5 µs | 0.6 µs | 36 µs | upload 2.6 µs, record 1.6 µs, submit 30 µs, rest 1.8 µs | 58 µs | 1.8 µs | 0.5 µs |
| 16 × 32 | cpu-1 | 0.2 µs | 0.3 µs | 1.2 µs | kernels 0.9 µs, rest 0.3 µs | 0.0 µs | – | 0.3 µs |
| 16 × 32 | cpu | 0.3 µs | 0.5 µs | 9.7 µs | kernels 9.2 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.4 µs |
| 16 × 32 | gpu | 0.4 µs | 0.6 µs | 37 µs | upload 2.7 µs, record 1.6 µs, submit 30 µs, rest 1.8 µs | 58 µs | 2.5 µs | 0.6 µs |
| 256 × 32 | cpu-1 | 0.2 µs | 0.4 µs | 12 µs | kernels 11 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.4 µs |
| 256 × 32 | cpu | 0.3 µs | 0.4 µs | 18 µs | kernels 18 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – | 0.3 µs |
| 256 × 32 | gpu | 0.4 µs | 0.6 µs | 37 µs | upload 2.8 µs, record 1.6 µs, submit 31 µs, rest 1.9 µs | 61 µs | 6.4 µs | 0.6 µs |
| 4096 × 32 | cpu-1 | 0.4 µs | 0.6 µs | 338 µs | kernels 337 µs, store 0.2 µs, rest 0.8 µs | 0.0 µs | – | 0.7 µs |
| 4096 × 32 | cpu | 0.5 µs | 0.7 µs | 99 µs | kernels 98 µs, store 0.2 µs, rest 0.9 µs | 0.0 µs | – | 0.5 µs |
| 4096 × 32 | gpu | 0.5 µs | 0.6 µs | 38 µs | upload 3.0 µs, record 1.6 µs, submit 32 µs, rest 1.9 µs | 138 µs | 72 µs | 0.6 µs |
| 1 × 128 | cpu-1 | 0.1 µs | 0.2 µs | 0.6 µs | kernels 0.3 µs, rest 0.3 µs | 0.0 µs | – | 0.1 µs |
| 1 × 128 | cpu | 0.3 µs | 0.5 µs | 5.6 µs | kernels 5.0 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – | 0.3 µs |
| 1 × 128 | gpu | 0.5 µs | 0.6 µs | 36 µs | upload 2.8 µs, record 1.6 µs, submit 30 µs, rest 1.8 µs | 52 µs | 2.1 µs | 0.6 µs |
| 16 × 128 | cpu-1 | 0.1 µs | 0.3 µs | 3.2 µs | kernels 2.9 µs, rest 0.3 µs | 0.0 µs | – | 0.2 µs |
| 16 × 128 | cpu | 0.3 µs | 0.4 µs | 13 µs | kernels 12 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.3 µs |
| 16 × 128 | gpu | 0.5 µs | 0.6 µs | 36 µs | upload 2.8 µs, record 1.6 µs, submit 30 µs, rest 1.8 µs | 52 µs | 2.7 µs | 0.6 µs |
| 256 × 128 | cpu-1 | 0.2 µs | 0.3 µs | 49 µs | kernels 49 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.3 µs |
| 256 × 128 | cpu | 0.4 µs | 0.5 µs | 28 µs | kernels 27 µs, store 0.1 µs, rest 0.6 µs | 0.0 µs | – | 0.4 µs |
| 256 × 128 | gpu | 0.4 µs | 0.6 µs | 38 µs | upload 2.9 µs, record 1.6 µs, submit 31 µs, rest 1.9 µs | 82 µs | 22 µs | 0.6 µs |
| 1024 × 128 | cpu-1 | 0.4 µs | 0.5 µs | 414 µs | kernels 412 µs, store 0.2 µs, rest 0.7 µs | 0.0 µs | – | 0.6 µs |
| 1024 × 128 | cpu | 0.5 µs | 0.7 µs | 148 µs | kernels 147 µs, store 0.2 µs, rest 0.9 µs | 0.0 µs | – | 0.5 µs |
| 1024 × 128 | gpu | 0.5 µs | 0.6 µs | 38 µs | upload 3.0 µs, record 1.7 µs, submit 32 µs, rest 1.9 µs | 144 µs | 78 µs | 0.6 µs |

## sweep circle

Sweep of a circle along the curves, the radius changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 6.9 µs | 8.2 µs | 121 µs | 14.7× slower | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 16 × 32 | 86 µs | 18 µs | 125 µs | 6.8× slower | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 32 | 2.01 ms | 406 µs | 140 µs | 2.9× faster | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 4096 × 32 | 29.80 ms | 7.93 ms | 293 µs | 27.0× faster | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 1 × 128 | 26 µs | 34 µs | 120 µs | 3.5× slower | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 16 × 128 | 377 µs | 107 µs | 126 µs | 1.2× slower | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 128 | 8.80 ms | 1.70 ms | 165 µs | 10.3× faster | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 1024 × 128 | 32.07 ms | 8.12 ms | 284 µs | 28.6× faster | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 1 × 32 | cpu-1 | 0.1 µs | 0.4 µs | 6.4 µs | kernels 5.9 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 1 × 32 | cpu | 0.2 µs | 0.4 µs | 7.6 µs | kernels 7.1 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.3 µs |
| 1 × 32 | gpu | 0.5 µs | 0.8 µs | 45 µs | upload 3.8 µs, record 2.6 µs, submit 35 µs, download 0.1 µs, rest 3.3 µs | 75 µs | 11 µs | 0.8 µs |
| 16 × 32 | cpu-1 | 0.1 µs | 0.4 µs | 86 µs | kernels 86 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 16 × 32 | cpu | 0.3 µs | 0.6 µs | 18 µs | kernels 17 µs, store 0.1 µs, rest 0.6 µs | 0.0 µs | – | 0.4 µs |
| 16 × 32 | gpu | 0.4 µs | 0.8 µs | 45 µs | upload 3.8 µs, record 2.6 µs, submit 35 µs, download 0.1 µs, rest 3.5 µs | 79 µs | 14 µs | 0.7 µs |
| 256 × 32 | cpu-1 | 0.3 µs | 0.6 µs | 2.01 ms | kernels 2.01 ms, store 0.2 µs, rest 0.7 µs | 0.0 µs | – | 0.4 µs |
| 256 × 32 | cpu | 0.4 µs | 0.8 µs | 405 µs | kernels 404 µs, store 0.2 µs, rest 0.7 µs | 0.0 µs | – | 0.6 µs |
| 256 × 32 | gpu | 0.5 µs | 0.8 µs | 48 µs | upload 4.1 µs, record 2.7 µs, submit 36 µs, download 0.1 µs, rest 4.5 µs | 91 µs | 24 µs | 0.8 µs |
| 4096 × 32 | cpu-1 | 2.0 µs | 3.2 µs | 29.79 ms | kernels 29.79 ms, store 0.7 µs, rest 2.5 µs | 0.0 µs | – | 1.7 µs |
| 4096 × 32 | cpu | 1.7 µs | 2.5 µs | 7.93 ms | kernels 7.92 ms, store 0.4 µs, rest 2.5 µs | 0.0 µs | – | 1.8 µs |
| 4096 × 32 | gpu | 0.5 µs | 0.8 µs | 57 µs | upload 5.1 µs, record 2.8 µs, submit 38 µs, download 0.1 µs, rest 11 µs | 234 µs | 161 µs | 0.9 µs |
| 1 × 128 | cpu-1 | 0.2 µs | 0.4 µs | 25 µs | kernels 24 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 1 × 128 | cpu | 0.2 µs | 0.4 µs | 34 µs | kernels 33 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 1 × 128 | gpu | 0.5 µs | 0.8 µs | 45 µs | upload 3.8 µs, record 2.6 µs, submit 35 µs, download 0.1 µs, rest 3.3 µs | 74 µs | 12 µs | 0.7 µs |
| 16 × 128 | cpu-1 | 0.3 µs | 0.5 µs | 376 µs | kernels 375 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – | 0.3 µs |
| 16 × 128 | cpu | 0.4 µs | 0.9 µs | 106 µs | kernels 104 µs, store 0.1 µs, rest 1.0 µs | 0.0 µs | – | 0.6 µs |
| 16 × 128 | gpu | 0.5 µs | 0.8 µs | 46 µs | upload 3.9 µs, record 2.6 µs, submit 36 µs, download 0.1 µs, rest 3.6 µs | 79 µs | 15 µs | 0.8 µs |
| 256 × 128 | cpu-1 | 0.4 µs | 0.7 µs | 8.80 ms | kernels 8.80 ms, store 0.2 µs, rest 0.7 µs | 0.0 µs | – | 0.4 µs |
| 256 × 128 | cpu | 0.6 µs | 1.1 µs | 1.69 ms | kernels 1.69 ms, store 0.2 µs, rest 1.0 µs | 0.0 µs | – | 0.9 µs |
| 256 × 128 | gpu | 0.4 µs | 0.8 µs | 48 µs | upload 4.0 µs, record 2.7 µs, submit 36 µs, download 0.1 µs, rest 4.4 µs | 117 µs | 47 µs | 0.8 µs |
| 1024 × 128 | cpu-1 | 2.6 µs | 4.7 µs | 32.06 ms | kernels 32.05 ms, store 0.7 µs, rest 3.4 µs | 0.0 µs | – | 3.0 µs |
| 1024 × 128 | cpu | 2.3 µs | 2.9 µs | 8.11 ms | kernels 8.11 ms, store 0.6 µs, rest 2.6 µs | 0.0 µs | – | 2.2 µs |
| 1024 × 128 | gpu | 0.5 µs | 0.8 µs | 50 µs | upload 4.4 µs, record 2.8 µs, submit 37 µs, download 0.1 µs, rest 5.9 µs | 232 µs | 159 µs | 0.8 µs |

## sweep square

Sweep of a square curve along the curves, the square changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 14 µs | 15 µs | 182 µs | 12.1× slower | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 16 × 32 | 140 µs | 96 µs | 188 µs | 2.0× slower | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 256 × 32 | 2.95 ms | 1.60 ms | 246 µs | 6.5× faster | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 4096 × 32 | 81.85 ms | 53.37 ms | 912 µs | 58.5× faster | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 1 × 128 | 65 µs | 55 µs | 180 µs | 3.3× slower | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 16 × 128 | 771 µs | 481 µs | 201 µs | 2.4× faster | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 256 × 128 | 13.78 ms | 7.50 ms | 410 µs | 18.3× faster | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 1024 × 128 | 73.35 ms | 39.32 ms | 888 µs | 44.3× faster | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 0.4 µs | 14 µs | kernels 13 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.3 µs |
| 1 × 32 | cpu | 0.2 µs | 0.4 µs | 14 µs | kernels 14 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.3 µs |
| 1 × 32 | gpu | 0.5 µs | 0.9 µs | 56 µs | upload 5.7 µs, record 4.0 µs, submit 41 µs, download 0.1 µs, rest 5.4 µs | 125 µs | 58 µs | 1.0 µs |
| 16 × 32 | cpu-1 | 0.3 µs | 0.5 µs | 140 µs | kernels 139 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – | 0.4 µs |
| 16 × 32 | cpu | 0.4 µs | 0.7 µs | 95 µs | kernels 94 µs, store 0.1 µs, rest 0.7 µs | 0.0 µs | – | 0.6 µs |
| 16 × 32 | gpu | 0.5 µs | 0.9 µs | 56 µs | upload 5.7 µs, record 4.0 µs, submit 41 µs, download 0.1 µs, rest 5.5 µs | 130 µs | 61 µs | 1.0 µs |
| 256 × 32 | cpu-1 | 0.4 µs | 0.8 µs | 2.95 ms | kernels 2.95 ms, store 0.2 µs, rest 0.8 µs | 0.0 µs | – | 0.5 µs |
| 256 × 32 | cpu | 0.5 µs | 1.0 µs | 1.60 ms | kernels 1.60 ms, store 0.2 µs, rest 1.0 µs | 0.0 µs | – | 0.9 µs |
| 256 × 32 | gpu | 0.5 µs | 0.9 µs | 59 µs | upload 6.1 µs, record 4.2 µs, submit 42 µs, download 0.1 µs, rest 6.8 µs | 185 µs | 109 µs | 1.0 µs |
| 4096 × 32 | cpu-1 | 3.0 µs | 3.7 µs | 81.84 ms | kernels 81.84 ms, store 0.9 µs, rest 3.7 µs | 0.0 µs | – | 3.8 µs |
| 4096 × 32 | cpu | 2.4 µs | 3.4 µs | 53.37 ms | kernels 53.36 ms, store 0.8 µs, rest 3.2 µs | 0.0 µs | – | 2.9 µs |
| 4096 × 32 | gpu | 0.6 µs | 1.0 µs | 72 µs | upload 7.2 µs, record 4.3 µs, submit 45 µs, download 0.1 µs, rest 15 µs | 837 µs | 594 µs | 1.2 µs |
| 1 × 128 | cpu-1 | 0.2 µs | 0.7 µs | 64 µs | kernels 64 µs, store 0.1 µs, rest 0.6 µs | 0.0 µs | – | 0.4 µs |
| 1 × 128 | cpu | 0.3 µs | 0.8 µs | 54 µs | kernels 53 µs, store 0.1 µs, rest 0.7 µs | 0.0 µs | – | 0.5 µs |
| 1 × 128 | gpu | 0.5 µs | 0.9 µs | 56 µs | upload 5.8 µs, record 4.0 µs, submit 41 µs, download 0.1 µs, rest 5.4 µs | 122 µs | 58 µs | 1.0 µs |
| 16 × 128 | cpu-1 | 0.3 µs | 0.7 µs | 770 µs | kernels 769 µs, store 0.2 µs, rest 0.7 µs | 0.0 µs | – | 0.4 µs |
| 16 × 128 | cpu | 0.5 µs | 1.1 µs | 479 µs | kernels 478 µs, store 0.2 µs, rest 1.0 µs | 0.0 µs | – | 0.8 µs |
| 16 × 128 | gpu | 0.5 µs | 0.9 µs | 58 µs | upload 5.8 µs, record 4.1 µs, submit 42 µs, download 0.1 µs, rest 5.6 µs | 140 µs | 71 µs | 1.1 µs |
| 256 × 128 | cpu-1 | 2.7 µs | 4.2 µs | 13.77 ms | kernels 13.77 ms, store 1.2 µs, rest 4.7 µs | 0.0 µs | – | 3.6 µs |
| 256 × 128 | cpu | 0.9 µs | 1.8 µs | 7.50 ms | kernels 7.50 ms, store 0.5 µs, rest 2.1 µs | 0.0 µs | – | 1.6 µs |
| 256 × 128 | gpu | 0.5 µs | 0.9 µs | 59 µs | upload 6.1 µs, record 4.1 µs, submit 42 µs, download 0.1 µs, rest 6.7 µs | 348 µs | 272 µs | 1.1 µs |
| 1024 × 128 | cpu-1 | 4.3 µs | 6.1 µs | 73.34 ms | kernels 73.33 ms, store 1.8 µs, rest 5.7 µs | 0.1 µs | – | 4.5 µs |
| 1024 × 128 | cpu | 2.3 µs | 3.4 µs | 39.32 ms | kernels 39.31 ms, store 0.8 µs, rest 3.2 µs | 0.0 µs | – | 2.6 µs |
| 1024 × 128 | gpu | 0.5 µs | 1.0 µs | 63 µs | upload 6.6 µs, record 4.2 µs, submit 44 µs, download 0.1 µs, rest 8.2 µs | 823 µs | 574 µs | 1.2 µs |

## sample

Sample Curve, 64 frames per curve at places which change.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 35 µs | 35 µs | 110 µs | 3.1× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 32 | 551 µs | 93 µs | 111 µs | 1.2× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 32 | 8.91 ms | 1.24 ms | 114 µs | 10.8× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 4096 × 32 | 143.73 ms | 17.93 ms | 200 µs | 89.8× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 1 × 128 | 35 µs | 34 µs | 110 µs | 3.2× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 128 | 538 µs | 67 µs | 111 µs | 1.7× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 128 | 8.81 ms | 1.15 ms | 116 µs | 9.9× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 1024 × 128 | 35.36 ms | 4.36 ms | 160 µs | 27.2× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 0.3 µs | 34 µs | kernels 34 µs, rest 0.3 µs | 0.0 µs | – | 0.2 µs |
| 1 × 32 | cpu | 0.2 µs | 0.3 µs | 35 µs | kernels 34 µs, rest 0.3 µs | 0.0 µs | – | 0.2 µs |
| 1 × 32 | gpu | 0.5 µs | 0.6 µs | 39 µs | upload 3.7 µs, record 1.6 µs, submit 31 µs, rest 2.4 µs | 70 µs | 11 µs | 0.7 µs |
| 16 × 32 | cpu-1 | 0.2 µs | 0.3 µs | 550 µs | kernels 550 µs, rest 0.3 µs | 0.0 µs | – | 0.2 µs |
| 16 × 32 | cpu | 0.4 µs | 0.5 µs | 92 µs | kernels 91 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – | 0.4 µs |
| 16 × 32 | gpu | 0.5 µs | 0.6 µs | 39 µs | upload 4.0 µs, record 1.5 µs, submit 31 µs, rest 2.6 µs | 70 µs | 11 µs | 0.7 µs |
| 256 × 32 | cpu-1 | 0.4 µs | 0.5 µs | 8.91 ms | kernels 8.91 ms, store 0.2 µs, rest 0.7 µs | 0.0 µs | – | 0.4 µs |
| 256 × 32 | cpu | 0.6 µs | 0.8 µs | 1.23 ms | kernels 1.23 ms, store 0.2 µs, rest 1.0 µs | 0.0 µs | – | 0.7 µs |
| 256 × 32 | gpu | 0.5 µs | 0.6 µs | 39 µs | upload 3.6 µs, record 1.6 µs, submit 30 µs, rest 3.5 µs | 73 µs | 12 µs | 0.7 µs |
| 4096 × 32 | cpu-1 | 3.6 µs | 3.6 µs | 143.73 ms | kernels 143.72 ms, store 1.2 µs, rest 4.5 µs | 0.0 µs | – | 2.3 µs |
| 4096 × 32 | cpu | 2.9 µs | 2.9 µs | 17.92 ms | kernels 17.92 ms, store 0.8 µs, rest 3.5 µs | 0.0 µs | – | 2.0 µs |
| 4096 × 32 | gpu | 0.5 µs | 0.7 µs | 55 µs | upload 5.1 µs, record 1.7 µs, submit 33 µs, rest 15 µs | 143 µs | 78 µs | 0.7 µs |
| 1 × 128 | cpu-1 | 0.2 µs | 0.3 µs | 34 µs | kernels 34 µs, rest 0.3 µs | 0.0 µs | – | 0.2 µs |
| 1 × 128 | cpu | 0.2 µs | 0.3 µs | 34 µs | kernels 34 µs, rest 0.3 µs | 0.0 µs | – | 0.2 µs |
| 1 × 128 | gpu | 0.5 µs | 0.6 µs | 38 µs | upload 3.8 µs, record 1.5 µs, submit 30 µs, rest 2.4 µs | 71 µs | 11 µs | 0.6 µs |
| 16 × 128 | cpu-1 | 0.2 µs | 0.3 µs | 538 µs | kernels 538 µs, store 0.1 µs, rest 0.3 µs | 0.0 µs | – | 0.2 µs |
| 16 × 128 | cpu | 0.3 µs | 0.5 µs | 66 µs | kernels 66 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – | 0.3 µs |
| 16 × 128 | gpu | 0.5 µs | 0.6 µs | 39 µs | upload 4.0 µs, record 1.6 µs, submit 31 µs, rest 2.6 µs | 70 µs | 12 µs | 0.7 µs |
| 256 × 128 | cpu-1 | 0.4 µs | 0.5 µs | 8.81 ms | kernels 8.81 ms, store 0.1 µs, rest 0.6 µs | 0.0 µs | – | 0.4 µs |
| 256 × 128 | cpu | 0.6 µs | 0.8 µs | 1.15 ms | kernels 1.15 ms, store 0.2 µs, rest 0.9 µs | 0.0 µs | – | 0.7 µs |
| 256 × 128 | gpu | 0.5 µs | 0.6 µs | 40 µs | upload 3.9 µs, record 1.6 µs, submit 31 µs, rest 3.7 µs | 74 µs | 15 µs | 0.7 µs |
| 1024 × 128 | cpu-1 | 1.0 µs | 1.3 µs | 35.35 ms | kernels 35.35 ms, store 0.4 µs, rest 1.6 µs | 0.0 µs | – | 1.1 µs |
| 1024 × 128 | cpu | 0.8 µs | 1.0 µs | 4.35 ms | kernels 4.35 ms, store 0.3 µs, rest 1.3 µs | 0.0 µs | – | 0.8 µs |
| 1024 × 128 | gpu | 0.5 µs | 0.7 µs | 43 µs | upload 4.2 µs, record 1.6 µs, submit 32 µs, rest 5.6 µs | 114 µs | 47 µs | 0.7 µs |

## chain

NURBS, Translate, a Sweep of a circle and Sample Curve, the control points move.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 80 µs | 125 µs | 246 µs | 2.0× slower | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 16 × 32 | 1.24 ms | 240 µs | 264 µs | 1.1× slower | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 256 × 32 | 20.06 ms | 2.69 ms | 324 µs | 8.3× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 4096 × 32 | 324.91 ms | 43.76 ms | 1.32 ms | 33.2× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 1 × 128 | 219 µs | 328 µs | 303 µs | 1.1× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 16 × 128 | 3.50 ms | 560 µs | 315 µs | 1.8× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 256 × 128 | 59.26 ms | 9.68 ms | 637 µs | 15.2× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 1024 × 128 | 242.24 ms | 35.59 ms | 1.38 ms | 25.7× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 0.8 µs | 79 µs | kernels 78 µs, store 0.1 µs, rest 0.7 µs | 0.0 µs | – | 0.4 µs |
| 1 × 32 | cpu | 0.2 µs | 0.8 µs | 124 µs | kernels 122 µs, store 0.2 µs, rest 1.0 µs | 0.0 µs | – | 0.5 µs |
| 1 × 32 | gpu | 0.6 µs | 1.3 µs | 71 µs | upload 6.8 µs, record 6.3 µs, submit 50 µs, download 0.1 µs, rest 7.7 µs | 173 µs | 95 µs | 1.0 µs |
| 16 × 32 | cpu-1 | 0.6 µs | 0.8 µs | 1.24 ms | kernels 1.24 ms, store 0.2 µs, rest 0.8 µs | 0.0 µs | – | 0.4 µs |
| 16 × 32 | cpu | 1.3 µs | 1.7 µs | 237 µs | kernels 234 µs, store 0.3 µs, rest 1.8 µs | 0.0 µs | – | 0.9 µs |
| 16 × 32 | gpu | 0.9 µs | 1.3 µs | 74 µs | upload 7.2 µs, record 6.4 µs, submit 50 µs, download 0.1 µs, rest 9.7 µs | 188 µs | 108 µs | 1.1 µs |
| 256 × 32 | cpu-1 | 7.2 µs | 1.3 µs | 20.05 ms | kernels 20.05 ms, store 0.7 µs, rest 1.8 µs | 0.0 µs | – | 0.7 µs |
| 256 × 32 | cpu | 7.5 µs | 1.8 µs | 2.68 ms | kernels 2.68 ms, store 0.7 µs, rest 2.4 µs | 0.0 µs | – | 1.3 µs |
| 256 × 32 | gpu | 6.4 µs | 1.2 µs | 95 µs | upload 19 µs, record 6.3 µs, submit 52 µs, download 0.2 µs, rest 17 µs | 222 µs | 133 µs | 1.4 µs |
| 4096 × 32 | cpu-1 | 158 µs | 5.7 µs | 324.75 ms | kernels 324.74 ms, store 3.0 µs, rest 12 µs | 0.0 µs | – | 3.0 µs |
| 4096 × 32 | cpu | 156 µs | 5.2 µs | 43.59 ms | kernels 43.58 ms, store 2.3 µs, rest 10 µs | 0.0 µs | – | 3.2 µs |
| 4096 × 32 | gpu | 127 µs | 1.8 µs | 311 µs | upload 141 µs, record 7.0 µs, submit 62 µs, download 0.3 µs, rest 102 µs | 872 µs | 663 µs | 1.3 µs |
| 1 × 128 | cpu-1 | 0.3 µs | 0.7 µs | 218 µs | kernels 217 µs, store 0.2 µs, rest 0.7 µs | 0.0 µs | – | 0.3 µs |
| 1 × 128 | cpu | 0.5 µs | 1.1 µs | 325 µs | kernels 324 µs, store 0.3 µs, rest 1.6 µs | 0.0 µs | – | 0.7 µs |
| 1 × 128 | gpu | 0.8 µs | 1.4 µs | 79 µs | upload 7.6 µs, record 7.3 µs, submit 55 µs, download 0.2 µs, rest 9.4 µs | 220 µs | 148 µs | 1.2 µs |
| 16 × 128 | cpu-1 | 1.7 µs | 0.8 µs | 3.50 ms | kernels 3.50 ms, store 0.3 µs, rest 1.0 µs | 0.0 µs | – | 0.4 µs |
| 16 × 128 | cpu | 2.5 µs | 1.4 µs | 556 µs | kernels 554 µs, store 0.4 µs, rest 2.2 µs | 0.0 µs | – | 1.2 µs |
| 16 × 128 | gpu | 2.1 µs | 1.2 µs | 77 µs | upload 8.6 µs, record 6.6 µs, submit 51 µs, download 0.1 µs, rest 10 µs | 234 µs | 161 µs | 1.3 µs |
| 256 × 128 | cpu-1 | 43 µs | 4.8 µs | 59.21 ms | kernels 59.19 ms, store 2.2 µs, rest 11 µs | 0.0 µs | – | 2.5 µs |
| 256 × 128 | cpu | 55 µs | 4.8 µs | 9.62 ms | kernels 9.61 ms, store 1.7 µs, rest 9.0 µs | 0.0 µs | – | 2.3 µs |
| 256 × 128 | gpu | 39 µs | 2.3 µs | 179 µs | upload 47 µs, record 9.3 µs, submit 82 µs, download 0.3 µs, rest 38 µs | 427 µs | 264 µs | 1.8 µs |
| 1024 × 128 | cpu-1 | 170 µs | 6.9 µs | 242.07 ms | kernels 242.05 ms, store 5.2 µs, rest 16 µs | 0.0 µs | – | 4.1 µs |
| 1024 × 128 | cpu | 172 µs | 5.5 µs | 35.39 ms | kernels 35.37 ms, store 3.7 µs, rest 12 µs | 0.0 µs | – | 2.5 µs |
| 1024 × 128 | gpu | 155 µs | 2.0 µs | 295 µs | upload 140 µs, record 7.6 µs, submit 72 µs, download 0.3 µs, rest 72 µs | 933 µs | 747 µs | 1.5 µs |

## instances

A ring of 10 patches put onto 64 frames along every curve by reference, the control points move.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 76 µs | 148 µs | 255 µs | 1.7× slower | 3 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 16 × 32 | 1.22 ms | 379 µs | 254 µs | 1.5× faster | 3 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 256 × 32 | 18.89 ms | 2.71 ms | 306 µs | 8.9× faster | 3 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 1024 × 32 | 76.28 ms | 10.05 ms | 519 µs | 19.4× faster | 3 on gpu, 0 on cpu, 4 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 0.7 µs | 75 µs | kernels 74 µs, store 0.1 µs, rest 0.6 µs | 0.0 µs | – | 0.4 µs |
| 1 × 32 | cpu | 0.6 µs | 1.6 µs | 146 µs | kernels 144 µs, store 0.2 µs, rest 1.1 µs | 0.0 µs | – | 1.1 µs |
| 1 × 32 | gpu | 0.9 µs | 1.5 µs | 84 µs | upload 9.9 µs, record 5.8 µs, submit 59 µs, download 0.2 µs, rest 9.4 µs | 167 µs | 85 µs | 1.6 µs |
| 16 × 32 | cpu-1 | 0.7 µs | 1.2 µs | 1.22 ms | kernels 1.22 ms, store 0.2 µs, rest 0.9 µs | 0.0 µs | – | 0.6 µs |
| 16 × 32 | cpu | 1.2 µs | 1.7 µs | 376 µs | kernels 374 µs, store 0.3 µs, rest 1.7 µs | 0.0 µs | – | 1.1 µs |
| 16 × 32 | gpu | 1.1 µs | 1.4 µs | 73 µs | upload 9.4 µs, record 5.2 µs, submit 49 µs, download 0.2 µs, rest 9.7 µs | 175 µs | 93 µs | 1.4 µs |
| 256 × 32 | cpu-1 | 9.7 µs | 2.2 µs | 18.88 ms | kernels 18.88 ms, store 0.9 µs, rest 2.5 µs | 0.0 µs | – | 1.3 µs |
| 256 × 32 | cpu | 8.2 µs | 2.1 µs | 2.70 ms | kernels 2.70 ms, store 0.6 µs, rest 2.1 µs | 0.0 µs | – | 1.3 µs |
| 256 × 32 | gpu | 6.7 µs | 1.4 µs | 96 µs | upload 24 µs, record 5.1 µs, submit 49 µs, download 0.2 µs, rest 18 µs | 193 µs | 108 µs | 1.7 µs |
| 1024 × 32 | cpu-1 | 40 µs | 4.3 µs | 76.24 ms | kernels 76.22 ms, store 2.2 µs, rest 6.4 µs | 0.0 µs | – | 2.5 µs |
| 1024 × 32 | cpu | 36 µs | 2.9 µs | 10.01 ms | kernels 10.01 ms, store 1.0 µs, rest 3.5 µs | 0.0 µs | – | 2.0 µs |
| 1024 × 32 | gpu | 24 µs | 1.7 µs | 170 µs | upload 55 µs, record 6.0 µs, submit 59 µs, download 0.2 µs, rest 49 µs | 324 µs | 165 µs | 1.5 µs |

## instances realized

The rings of `instances`, copied to where each one is.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 162 µs | 152 µs | 274 µs | 1.8× slower | 4 on gpu, 0 on cpu, 6 passes, 0 downloads |
| 16 × 32 | 2.60 ms | 660 µs | 285 µs | 2.3× faster | 4 on gpu, 0 on cpu, 6 passes, 0 downloads |
| 256 × 32 | 40.62 ms | 5.05 ms | 507 µs | 10.0× faster | 4 on gpu, 0 on cpu, 6 passes, 0 downloads |
| 1024 × 32 | 170.44 ms | 22.24 ms | 1.35 ms | 16.5× faster | 4 on gpu, 0 on cpu, 6 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 0.9 µs | 161 µs | kernels 160 µs, store 0.2 µs, rest 0.7 µs | 0.0 µs | – | 0.5 µs |
| 1 × 32 | cpu | 0.5 µs | 1.6 µs | 150 µs | kernels 148 µs, store 0.2 µs, rest 1.3 µs | 0.0 µs | – | 1.1 µs |
| 1 × 32 | gpu | 0.6 µs | 1.6 µs | 86 µs | upload 7.9 µs, record 8.0 µs, submit 59 µs, download 0.2 µs, rest 10 µs | 185 µs | 94 µs | 1.5 µs |
| 16 × 32 | cpu-1 | 0.6 µs | 1.1 µs | 2.59 ms | kernels 2.59 ms, store 0.3 µs, rest 1.0 µs | 0.0 µs | – | 0.7 µs |
| 16 × 32 | cpu | 1.6 µs | 2.6 µs | 655 µs | kernels 651 µs, store 0.5 µs, rest 3.0 µs | 0.0 µs | – | 1.7 µs |
| 16 × 32 | gpu | 0.9 µs | 1.4 µs | 82 µs | upload 8.2 µs, record 7.3 µs, submit 56 µs, download 0.1 µs, rest 10 µs | 201 µs | 112 µs | 1.4 µs |
| 256 × 32 | cpu-1 | 14 µs | 4.4 µs | 40.61 ms | kernels 40.60 ms, store 1.6 µs, rest 4.0 µs | 0.0 µs | – | 2.5 µs |
| 256 × 32 | cpu | 12 µs | 3.1 µs | 5.03 ms | kernels 5.03 ms, store 0.9 µs, rest 3.9 µs | 0.0 µs | – | 2.8 µs |
| 256 × 32 | gpu | 6.4 µs | 1.5 µs | 110 µs | upload 24 µs, record 7.5 µs, submit 59 µs, download 0.1 µs, rest 20 µs | 388 µs | 298 µs | 1.8 µs |
| 1024 × 32 | cpu-1 | 44 µs | 6.3 µs | 170.39 ms | kernels 170.38 ms, store 2.6 µs, rest 9.2 µs | 0.0 µs | – | 4.9 µs |
| 1024 × 32 | cpu | 61 µs | 7.9 µs | 22.18 ms | kernels 22.16 ms, store 2.7 µs, rest 11 µs | 0.0 µs | – | 5.0 µs |
| 1024 × 32 | gpu | 35 µs | 2.3 µs | 223 µs | upload 60 µs, record 9.7 µs, submit 83 µs, download 0.3 µs, rest 68 µs | 1.09 ms | 916 µs | 3.0 µs |

## for each

NURBS and Translate for every curve on its own, the control points of one curve move.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 16 × 8 | 12 µs | 22 µs | 246 µs | 11.3× slower | 2 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 256 × 8 | 46 µs | 54 µs | 275 µs | 5.1× slower | 2 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 4096 × 8 | 699 µs | 673 µs | 946 µs | 1.4× slower | 2 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 16384 × 8 | 4.20 ms | 4.27 ms | 5.45 ms | 1.3× slower | 2 on gpu, 0 on cpu, 9 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 16 × 8 | cpu-1 | 0.3 µs | 0.2 µs | 11 µs | kernels 7.3 µs, store 0.1 µs, rest 3.7 µs | 0.0 µs | – | 0.4 µs |
| 16 × 8 | cpu | 0.5 µs | 0.4 µs | 21 µs | kernels 15 µs, store 0.1 µs, rest 5.8 µs | 0.0 µs | – | 0.7 µs |
| 16 × 8 | gpu | 0.7 µs | 0.7 µs | 85 µs | upload 5.5 µs, record 7.1 µs, submit 61 µs, download 0.1 µs, rest 12 µs | 154 µs | 47 µs | 1.4 µs |
| 256 × 8 | cpu-1 | 1.7 µs | 0.3 µs | 44 µs | kernels 7.6 µs, store 0.1 µs, rest 36 µs | 0.0 µs | – | 1.8 µs |
| 256 × 8 | cpu | 1.8 µs | 0.3 µs | 52 µs | kernels 12 µs, store 0.1 µs, rest 40 µs | 0.0 µs | – | 3.0 µs |
| 256 × 8 | gpu | 2.0 µs | 0.7 µs | 116 µs | upload 6.9 µs, record 6.9 µs, submit 56 µs, download 0.1 µs, rest 46 µs | 152 µs | 47 µs | 3.1 µs |
| 4096 × 8 | cpu-1 | 27 µs | 0.5 µs | 671 µs | kernels 8.9 µs, store 0.1 µs, rest 662 µs | 0.0 µs | – | 30 µs |
| 4096 × 8 | cpu | 26 µs | 0.5 µs | 647 µs | kernels 14 µs, store 0.2 µs, rest 633 µs | 0.0 µs | – | 29 µs |
| 4096 × 8 | gpu | 25 µs | 0.7 µs | 760 µs | upload 39 µs, record 16 µs, submit 68 µs, download 0.1 µs, rest 637 µs | 160 µs | 54 µs | 36 µs |
| 16384 × 8 | cpu-1 | 171 µs | 1.5 µs | 4.01 ms | kernels 14 µs, store 0.3 µs, rest 4.00 ms | 0.0 µs | – | 430 µs |
| 16384 × 8 | cpu | 168 µs | 1.3 µs | 4.12 ms | kernels 19 µs, store 0.3 µs, rest 4.10 ms | 0.0 µs | – | 398 µs |
| 16384 × 8 | gpu | 171 µs | 1.8 µs | 5.08 ms | upload 137 µs, record 58 µs, submit 225 µs, download 0.3 µs, rest 4.67 ms | 197 µs | 78 µs | 527 µs |

## for each, all

NURBS and Translate for every curve on its own, the offset of all of them changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 16 × 8 | 9.0 µs | 72 µs | 183 µs | 2.5× slower | 16 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 8 | 140 µs | 476 µs | 350 µs | 1.4× faster | 256 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 4096 × 8 | 2.73 ms | 10.90 ms | 3.25 ms | 3.3× faster | 4096 on gpu, 0 on cpu, 6 passes, 0 downloads |
| 16384 × 8 | 13.16 ms | 37.13 ms | 13.81 ms | 2.7× faster | 16384 on gpu, 0 on cpu, 19 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 16 × 8 | cpu-1 | 0.2 µs | 0.2 µs | 8.6 µs | kernels 1.5 µs, store 0.5 µs, rest 6.6 µs | 0.0 µs | – | 0.9 µs |
| 16 × 8 | cpu | 0.4 µs | 0.5 µs | 70 µs | kernels 54 µs, store 1.1 µs, rest 14 µs | 0.0 µs | – | 2.0 µs |
| 16 × 8 | gpu | 0.5 µs | 0.7 µs | 71 µs | upload 3.1 µs, record 4.3 µs, submit 47 µs, download 0.4 µs, rest 16 µs | 109 µs | 2.5 µs | 3.1 µs |
| 256 × 8 | cpu-1 | 0.3 µs | 0.4 µs | 139 µs | kernels 30 µs, store 9.2 µs, rest 99 µs | 0.0 µs | – | 22 µs |
| 256 × 8 | cpu | 0.5 µs | 0.6 µs | 475 µs | kernels 323 µs, store 15 µs, rest 132 µs | 0.0 µs | – | 32 µs |
| 256 × 8 | gpu | 0.8 µs | 0.8 µs | 241 µs | upload 5.9 µs, record 6.0 µs, submit 60 µs, download 6.2 µs, rest 165 µs | 103 µs | 3.5 µs | 41 µs |
| 4096 × 8 | cpu-1 | 1.0 µs | 1.0 µs | 2.72 ms | kernels 812 µs, store 171 µs, rest 1.75 ms | 0.0 µs | – | 621 µs |
| 4096 × 8 | cpu | 2.5 µs | 2.0 µs | 10.89 ms | kernels 6.97 ms, store 313 µs, rest 3.37 ms | 0.0 µs | – | 1.07 ms |
| 4096 × 8 | gpu | 1.0 µs | 1.0 µs | 3.15 ms | upload 9.7 µs, record 40 µs, submit 124 µs, download 97 µs, rest 2.88 ms | 75 µs | 27 µs | 758 µs |
| 16384 × 8 | cpu-1 | 1.2 µs | 1.1 µs | 13.16 ms | kernels 3.67 ms, store 668 µs, rest 8.75 ms | 0.0 µs | – | 3.33 ms |
| 16384 × 8 | cpu | 2.4 µs | 1.9 µs | 37.12 ms | kernels 24.03 ms, store 1.10 ms, rest 11.93 ms | 0.0 µs | – | 3.45 ms |
| 16384 × 8 | gpu | 1.4 µs | 1.4 µs | 13.59 ms | upload 35 µs, record 265 µs, submit 264 µs, download 374 µs, rest 12.62 ms | 219 µs | 99 µs | 4.03 ms |

## mesh

Mesh of a swept circle, the tolerance changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 3.10 ms | 332 µs | 366 µs | 1.1× slower | 0 on gpu, 1 on cpu, 0 passes, 0 downloads |
| 16 × 32 | 49.19 ms | 7.61 ms | 7.97 ms | 1.0× slower | 0 on gpu, 1 on cpu, 0 passes, 0 downloads |
| 64 × 32 | 196.80 ms | 21.55 ms | 22.06 ms | 1.0× slower | 0 on gpu, 1 on cpu, 0 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 0.5 µs | 3.09 ms | kernels 3.09 ms, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.3 µs |
| 1 × 32 | cpu | 0.3 µs | 0.7 µs | 330 µs | kernels 329 µs, store 0.1 µs, rest 0.6 µs | 0.0 µs | – | 0.7 µs |
| 1 × 32 | gpu | 0.5 µs | 0.9 µs | 360 µs | download 0.1 µs, cpu 358 µs, rest 1.6 µs | 4.2 µs | 0.0 µs | 0.8 µs |
| 16 × 32 | cpu-1 | 0.4 µs | 0.8 µs | 49.18 ms | kernels 49.18 ms, store 0.2 µs, rest 0.6 µs | 0.0 µs | – | 0.6 µs |
| 16 × 32 | cpu | 0.9 µs | 1.5 µs | 7.61 ms | kernels 7.60 ms, store 0.3 µs, rest 1.3 µs | 0.0 µs | – | 1.3 µs |
| 16 × 32 | gpu | 0.8 µs | 1.5 µs | 7.96 ms | download 0.1 µs, cpu 7.96 ms, rest 2.5 µs | 10 µs | 0.0 µs | 1.5 µs |
| 64 × 32 | cpu-1 | 1.8 µs | 2.1 µs | 196.80 ms | kernels 196.80 ms, store 0.6 µs, rest 2.2 µs | 0.0 µs | – | 1.4 µs |
| 64 × 32 | cpu | 1.7 µs | 2.3 µs | 21.55 ms | kernels 21.55 ms, store 0.5 µs, rest 2.0 µs | 0.0 µs | – | 1.7 µs |
| 64 × 32 | gpu | 1.3 µs | 2.2 µs | 22.04 ms | download 0.2 µs, cpu 22.04 ms, rest 3.5 µs | 12 µs | 0.0 µs | 1.9 µs |

## fence

The picket fence of the editor, with as many pickets, gets a little longer.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 10 × 1 | 48 µs | 154 µs | 877 µs | 5.7× slower | 18 on gpu, 9 on cpu, 7 passes, 7 downloads |
| 100 × 1 | 125 µs | 329 µs | 969 µs | 2.9× slower | 18 on gpu, 9 on cpu, 7 passes, 7 downloads |
| 1000 × 1 | 887 µs | 1.56 ms | 1.88 ms | 1.2× slower | 18 on gpu, 9 on cpu, 7 passes, 7 downloads |
| 10000 × 1 | 0.9 µs | 1.8 µs | 4.6 µs | 2.6× slower | 31 on gpu, 14 on cpu, 10 passes, 11 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 10 × 1 | cpu-1 | 0.2 µs | 5.2 µs | 42 µs | kernels 38 µs, store 0.9 µs, rest 3.8 µs | 0.0 µs | – | 3.9 µs |
| 10 × 1 | cpu | 0.4 µs | 8.8 µs | 144 µs | kernels 136 µs, store 1.2 µs, rest 5.6 µs | 0.0 µs | – | 7.1 µs |
| 10 × 1 | gpu | 0.6 µs | 6.4 µs | 774 µs | upload 22 µs, record 11 µs, submit 118 µs, download 565 µs, cpu 33 µs, rest 23 µs | 97 µs | 38 µs | 8.5 µs |
| 100 × 1 | cpu-1 | 0.2 µs | 5.1 µs | 120 µs | kernels 115 µs, store 1.0 µs, rest 4.0 µs | 0.0 µs | – | 2.8 µs |
| 100 × 1 | cpu | 0.5 µs | 9.4 µs | 318 µs | kernels 311 µs, store 1.2 µs, rest 5.9 µs | 0.0 µs | – | 7.9 µs |
| 100 × 1 | gpu | 0.7 µs | 6.4 µs | 845 µs | upload 29 µs, record 10 µs, submit 118 µs, download 555 µs, cpu 108 µs, rest 23 µs | 114 µs | 39 µs | 6.2 µs |
| 1000 × 1 | cpu-1 | 0.3 µs | 5.2 µs | 881 µs | kernels 876 µs, store 1.0 µs, rest 3.9 µs | 0.0 µs | – | 3.2 µs |
| 1000 × 1 | cpu | 0.7 µs | 8.9 µs | 1.55 ms | kernels 1.54 ms, store 1.5 µs, rest 6.8 µs | 0.0 µs | – | 8.0 µs |
| 1000 × 1 | gpu | 0.6 µs | 6.6 µs | 1.67 ms | upload 51 µs, record 11 µs, submit 125 µs, download 580 µs, cpu 850 µs, rest 26 µs | 200 µs | 38 µs | 6.7 µs |
| 10000 × 1 | cpu-1 | 0.2 µs | 0.6 µs | 0.1 µs | rest 0.1 µs | 0.0 µs | – | 2.3 µs |
| 10000 × 1 | cpu | 0.5 µs | 0.8 µs | 0.1 µs | rest 0.1 µs | 0.0 µs | – | 4.1 µs |
| 10000 × 1 | gpu | 0.5 µs | 0.7 µs | 0.2 µs | submit 0.1 µs, rest 0.2 µs | 3.1 µs | 0.0 µs | 3.5 µs |
