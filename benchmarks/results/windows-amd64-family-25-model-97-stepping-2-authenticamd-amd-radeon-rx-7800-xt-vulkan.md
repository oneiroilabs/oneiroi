# Benchmarks

windows, AMD64 Family 25 Model 97 Stepping 2, AuthenticAMD with 16 threads, AMD Radeon RX 7800 XT on vulkan, driver AMD proprietary driver 26.11.2 (LLPC). Commit 0b7b978.

The time from an edit until its result is computed, the median of many edits. Below each table, where it goes: setting the new value (which hashes it) and planning are the same for every backend, `flush` is the executor, then waiting for the device. The device column is the time the GPU spends on compute passes, by its timestamps. Ending the frame afterwards frees what is not used anymore, it is not in the total.

## nurbs

NURBS Curve, the control points move.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 32 µs | 32 µs | 183 µs | 5.8× slower | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 16 × 32 | 490 µs | 82 µs | 196 µs | 2.4× slower | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 32 | 7.88 ms | 784 µs | 240 µs | 3.3× faster | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 4096 × 32 | 129.71 ms | 13.71 ms | 908 µs | 15.1× faster | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 1 × 128 | 132 µs | 132 µs | 237 µs | 1.8× slower | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 16 × 128 | 2.10 ms | 229 µs | 259 µs | 1.1× slower | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 128 | 34.53 ms | 3.47 ms | 498 µs | 7.0× faster | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 1024 × 128 | 139.91 ms | 14.90 ms | 1.02 ms | 14.7× faster | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 0.3 µs | 31 µs | kernels 31 µs, rest 0.3 µs | 0.0 µs | – | 0.1 µs |
| 1 × 32 | cpu | 0.2 µs | 0.3 µs | 31 µs | kernels 31 µs, rest 0.3 µs | 0.0 µs | – | 0.1 µs |
| 1 × 32 | gpu | 0.5 µs | 0.7 µs | 46 µs | upload 3.9 µs, record 3.0 µs, submit 36 µs, rest 3.4 µs | 135 µs | 72 µs | 0.7 µs |
| 16 × 32 | cpu-1 | 0.5 µs | 0.4 µs | 489 µs | kernels 488 µs, rest 0.3 µs | 0.0 µs | – | 0.1 µs |
| 16 × 32 | cpu | 1.1 µs | 0.6 µs | 80 µs | kernels 80 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – | 0.3 µs |
| 16 × 32 | gpu | 0.9 µs | 0.7 µs | 48 µs | upload 4.1 µs, record 2.8 µs, submit 36 µs, rest 4.5 µs | 144 µs | 80 µs | 0.7 µs |
| 256 × 32 | cpu-1 | 6.0 µs | 0.5 µs | 7.87 ms | kernels 7.87 ms, store 0.1 µs, rest 0.5 µs | 0.0 µs | – | 0.2 µs |
| 256 × 32 | cpu | 8.0 µs | 0.8 µs | 775 µs | kernels 774 µs, store 0.1 µs, rest 0.8 µs | 0.0 µs | – | 0.5 µs |
| 256 × 32 | gpu | 6.1 µs | 0.7 µs | 68 µs | upload 17 µs, record 2.8 µs, submit 38 µs, rest 9.2 µs | 165 µs | 91 µs | 0.8 µs |
| 4096 × 32 | cpu-1 | 153 µs | 3.1 µs | 129.47 ms | kernels 129.46 ms, store 1.1 µs, rest 4.9 µs | 0.0 µs | – | 2.4 µs |
| 4096 × 32 | cpu | 147 µs | 2.0 µs | 13.56 ms | kernels 13.56 ms, store 0.8 µs, rest 3.3 µs | 0.0 µs | – | 1.9 µs |
| 4096 × 32 | gpu | 121 µs | 1.0 µs | 256 µs | upload 136 µs, record 3.4 µs, submit 50 µs, download 0.1 µs, rest 66 µs | 518 µs | 349 µs | 0.8 µs |
| 1 × 128 | cpu-1 | 0.3 µs | 0.3 µs | 132 µs | kernels 131 µs, rest 0.3 µs | 0.0 µs | – | 0.1 µs |
| 1 × 128 | cpu | 0.3 µs | 0.3 µs | 132 µs | kernels 131 µs, rest 0.3 µs | 0.0 µs | – | 0.1 µs |
| 1 × 128 | gpu | 0.6 µs | 0.7 µs | 46 µs | upload 3.7 µs, record 2.7 µs, submit 36 µs, rest 3.5 µs | 190 µs | 127 µs | 0.7 µs |
| 16 × 128 | cpu-1 | 1.7 µs | 0.5 µs | 2.10 ms | kernels 2.10 ms, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 16 × 128 | cpu | 2.4 µs | 0.6 µs | 226 µs | kernels 225 µs, store 0.1 µs, rest 0.7 µs | 0.0 µs | – | 0.4 µs |
| 16 × 128 | gpu | 2.0 µs | 0.7 µs | 51 µs | upload 6.3 µs, record 2.9 µs, submit 36 µs, rest 5.1 µs | 206 µs | 132 µs | 0.8 µs |
| 256 × 128 | cpu-1 | 29 µs | 1.4 µs | 34.49 ms | kernels 34.49 ms, store 0.2 µs, rest 1.4 µs | 0.0 µs | – | 0.8 µs |
| 256 × 128 | cpu | 25 µs | 1.3 µs | 3.44 ms | kernels 3.44 ms, store 0.2 µs, rest 1.1 µs | 0.0 µs | – | 0.7 µs |
| 256 × 128 | gpu | 23 µs | 0.9 µs | 96 µs | upload 40 µs, record 3.0 µs, submit 39 µs, rest 14 µs | 377 µs | 185 µs | 0.8 µs |
| 1024 × 128 | cpu-1 | 159 µs | 3.2 µs | 139.74 ms | kernels 139.74 ms, store 1.3 µs, rest 5.3 µs | 0.0 µs | – | 3.3 µs |
| 1024 × 128 | cpu | 146 µs | 1.4 µs | 14.76 ms | kernels 14.76 ms, store 0.6 µs, rest 2.0 µs | 0.0 µs | – | 1.2 µs |
| 1024 × 128 | gpu | 122 µs | 1.1 µs | 245 µs | upload 137 µs, record 3.5 µs, submit 50 µs, download 0.1 µs, rest 60 µs | 641 µs | 474 µs | 0.8 µs |

## translate

Translate Curve of the curves, the offset changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 0.7 µs | 4.1 µs | 95 µs | 23.2× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 32 | 1.8 µs | 9.9 µs | 95 µs | 9.6× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 32 | 15 µs | 28 µs | 113 µs | 4.1× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 4096 × 32 | 364 µs | 106 µs | 177 µs | 1.7× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 1 × 128 | 1.1 µs | 6.3 µs | 95 µs | 15.1× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 128 | 3.7 µs | 14 µs | 89 µs | 6.4× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 128 | 52 µs | 29 µs | 121 µs | 4.2× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 1024 × 128 | 427 µs | 157 µs | 183 µs | 1.2× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 1 × 32 | cpu-1 | 0.1 µs | 0.2 µs | 0.4 µs | kernels 0.1 µs, rest 0.2 µs | 0.0 µs | – | 0.1 µs |
| 1 × 32 | cpu | 0.3 µs | 0.5 µs | 3.3 µs | kernels 2.9 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 1 × 32 | gpu | 0.5 µs | 0.6 µs | 36 µs | upload 2.6 µs, record 1.5 µs, submit 30 µs, rest 1.8 µs | 58 µs | 1.8 µs | 0.4 µs |
| 16 × 32 | cpu-1 | 0.2 µs | 0.3 µs | 1.2 µs | kernels 0.9 µs, rest 0.3 µs | 0.0 µs | – | 0.1 µs |
| 16 × 32 | cpu | 0.3 µs | 0.4 µs | 9.2 µs | kernels 8.7 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 16 × 32 | gpu | 0.4 µs | 0.6 µs | 36 µs | upload 2.6 µs, record 1.5 µs, submit 30 µs, rest 1.8 µs | 58 µs | 2.3 µs | 0.5 µs |
| 256 × 32 | cpu-1 | 0.3 µs | 0.6 µs | 14 µs | kernels 13 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – | 0.3 µs |
| 256 × 32 | cpu | 0.4 µs | 0.6 µs | 26 µs | kernels 26 µs, store 0.1 µs, rest 0.6 µs | 0.0 µs | – | 0.3 µs |
| 256 × 32 | gpu | 0.6 µs | 0.7 µs | 46 µs | upload 3.5 µs, record 2.1 µs, submit 38 µs, rest 2.4 µs | 66 µs | 6.4 µs | 0.5 µs |
| 4096 × 32 | cpu-1 | 0.5 µs | 0.8 µs | 362 µs | kernels 362 µs, store 0.2 µs, rest 0.9 µs | 0.0 µs | – | 0.4 µs |
| 4096 × 32 | cpu | 0.5 µs | 0.9 µs | 104 µs | kernels 103 µs, store 0.2 µs, rest 0.7 µs | 0.0 µs | – | 0.4 µs |
| 4096 × 32 | gpu | 0.5 µs | 0.5 µs | 36 µs | upload 2.7 µs, record 1.5 µs, submit 30 µs, rest 1.8 µs | 140 µs | 72 µs | 0.4 µs |
| 1 × 128 | cpu-1 | 0.2 µs | 0.3 µs | 0.7 µs | kernels 0.4 µs, rest 0.3 µs | 0.0 µs | – | 0.1 µs |
| 1 × 128 | cpu | 0.3 µs | 0.5 µs | 5.4 µs | kernels 5.0 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 1 × 128 | gpu | 0.5 µs | 0.6 µs | 36 µs | upload 2.6 µs, record 1.6 µs, submit 30 µs, rest 1.9 µs | 55 µs | 2.0 µs | 0.5 µs |
| 16 × 128 | cpu-1 | 0.2 µs | 0.3 µs | 3.3 µs | kernels 3.0 µs, rest 0.3 µs | 0.0 µs | – | 0.1 µs |
| 16 × 128 | cpu | 0.3 µs | 0.4 µs | 13 µs | kernels 13 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 16 × 128 | gpu | 0.5 µs | 0.6 µs | 36 µs | upload 2.7 µs, record 1.6 µs, submit 30 µs, rest 1.9 µs | 52 µs | 2.6 µs | 0.5 µs |
| 256 × 128 | cpu-1 | 0.2 µs | 0.4 µs | 51 µs | kernels 51 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 256 × 128 | cpu | 0.4 µs | 0.6 µs | 28 µs | kernels 27 µs, store 0.1 µs, rest 0.6 µs | 0.0 µs | – | 0.3 µs |
| 256 × 128 | gpu | 0.5 µs | 0.6 µs | 36 µs | upload 2.7 µs, record 1.6 µs, submit 30 µs, rest 1.8 µs | 83 µs | 22 µs | 0.4 µs |
| 1024 × 128 | cpu-1 | 0.4 µs | 0.8 µs | 426 µs | kernels 425 µs, store 0.2 µs, rest 0.8 µs | 0.0 µs | – | 0.3 µs |
| 1024 × 128 | cpu | 0.5 µs | 1.1 µs | 156 µs | kernels 154 µs, store 0.2 µs, rest 1.0 µs | 0.0 µs | – | 0.4 µs |
| 1024 × 128 | gpu | 0.5 µs | 0.6 µs | 36 µs | upload 2.7 µs, record 1.6 µs, submit 30 µs, rest 1.8 µs | 146 µs | 78 µs | 0.4 µs |

## sweep circle

Sweep of a circle along the curves, the radius changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 2.3 µs | 2.8 µs | 121 µs | 43.2× slower | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 16 × 32 | 16 µs | 12 µs | 125 µs | 10.6× slower | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 32 | 386 µs | 302 µs | 138 µs | 2.2× faster | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 4096 × 32 | 12.33 ms | 6.52 ms | 295 µs | 22.1× faster | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 1 × 128 | 5.3 µs | 5.1 µs | 120 µs | 23.6× slower | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 16 × 128 | 65 µs | 74 µs | 126 µs | 1.7× slower | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 128 | 1.79 ms | 1.33 ms | 164 µs | 8.1× faster | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 1024 × 128 | 13.03 ms | 6.94 ms | 284 µs | 24.5× faster | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 0.4 µs | 1.8 µs | kernels 1.4 µs, store 0.1 µs, rest 0.3 µs | 0.0 µs | – | 0.2 µs |
| 1 × 32 | cpu | 0.2 µs | 0.5 µs | 2.1 µs | kernels 1.6 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 1 × 32 | gpu | 0.5 µs | 0.8 µs | 44 µs | upload 3.6 µs, record 2.6 µs, submit 34 µs, download 0.1 µs, rest 3.4 µs | 76 µs | 11 µs | 0.6 µs |
| 16 × 32 | cpu-1 | 0.2 µs | 0.5 µs | 16 µs | kernels 15 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 16 × 32 | cpu | 0.3 µs | 0.6 µs | 11 µs | kernels 10 µs, store 0.1 µs, rest 0.6 µs | 0.0 µs | – | 0.3 µs |
| 16 × 32 | gpu | 0.5 µs | 0.8 µs | 44 µs | upload 3.7 µs, record 2.6 µs, submit 35 µs, download 0.1 µs, rest 3.6 µs | 80 µs | 15 µs | 0.6 µs |
| 256 × 32 | cpu-1 | 0.3 µs | 0.7 µs | 385 µs | kernels 384 µs, store 0.1 µs, rest 0.7 µs | 0.0 µs | – | 0.3 µs |
| 256 × 32 | cpu | 0.4 µs | 0.9 µs | 301 µs | kernels 300 µs, store 0.2 µs, rest 0.9 µs | 0.0 µs | – | 0.5 µs |
| 256 × 32 | gpu | 0.5 µs | 0.8 µs | 46 µs | upload 3.7 µs, record 2.6 µs, submit 35 µs, rest 4.5 µs | 91 µs | 24 µs | 0.7 µs |
| 4096 × 32 | cpu-1 | 1.4 µs | 3.4 µs | 12.33 ms | kernels 12.33 ms, store 1.0 µs, rest 3.2 µs | 0.0 µs | – | 1.9 µs |
| 4096 × 32 | cpu | 1.9 µs | 3.8 µs | 6.51 ms | kernels 6.50 ms, store 0.6 µs, rest 3.3 µs | 0.0 µs | – | 1.7 µs |
| 4096 × 32 | gpu | 0.5 µs | 0.9 µs | 59 µs | upload 4.7 µs, record 2.7 µs, submit 37 µs, download 0.1 µs, rest 15 µs | 235 µs | 160 µs | 0.7 µs |
| 1 × 128 | cpu-1 | 0.1 µs | 0.4 µs | 4.8 µs | kernels 4.4 µs, store 0.1 µs, rest 0.3 µs | 0.0 µs | – | 0.2 µs |
| 1 × 128 | cpu | 0.2 µs | 0.4 µs | 4.6 µs | kernels 4.2 µs, store 0.1 µs, rest 0.3 µs | 0.0 µs | – | 0.2 µs |
| 1 × 128 | gpu | 0.5 µs | 0.8 µs | 44 µs | upload 3.6 µs, record 2.6 µs, submit 34 µs, rest 3.4 µs | 75 µs | 12 µs | 0.6 µs |
| 16 × 128 | cpu-1 | 0.2 µs | 0.6 µs | 64 µs | kernels 63 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – | 0.2 µs |
| 16 × 128 | cpu | 0.4 µs | 0.8 µs | 73 µs | kernels 72 µs, store 0.1 µs, rest 0.7 µs | 0.0 µs | – | 0.4 µs |
| 16 × 128 | gpu | 0.5 µs | 0.8 µs | 45 µs | upload 3.7 µs, record 2.7 µs, submit 35 µs, download 0.1 µs, rest 3.7 µs | 80 µs | 15 µs | 0.7 µs |
| 256 × 128 | cpu-1 | 0.3 µs | 0.8 µs | 1.79 ms | kernels 1.79 ms, store 0.2 µs, rest 0.7 µs | 0.0 µs | – | 0.3 µs |
| 256 × 128 | cpu | 0.5 µs | 1.5 µs | 1.33 ms | kernels 1.33 ms, store 0.2 µs, rest 1.1 µs | 0.0 µs | – | 0.7 µs |
| 256 × 128 | gpu | 0.5 µs | 0.8 µs | 46 µs | upload 3.7 µs, record 2.7 µs, submit 35 µs, rest 4.6 µs | 116 µs | 47 µs | 0.6 µs |
| 1024 × 128 | cpu-1 | 2.0 µs | 4.8 µs | 13.02 ms | kernels 13.02 ms, store 0.8 µs, rest 3.9 µs | 0.0 µs | – | 1.9 µs |
| 1024 × 128 | cpu | 2.0 µs | 4.6 µs | 6.94 ms | kernels 6.93 ms, store 0.7 µs, rest 3.7 µs | 0.0 µs | – | 1.6 µs |
| 1024 × 128 | gpu | 0.5 µs | 0.8 µs | 50 µs | upload 4.1 µs, record 2.7 µs, submit 36 µs, download 0.1 µs, rest 6.8 µs | 232 µs | 159 µs | 0.7 µs |

## sweep square

Sweep of a square curve along the curves, the square changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 9.2 µs | 11 µs | 166 µs | 15.4× slower | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 16 × 32 | 60 µs | 86 µs | 171 µs | 2.0× slower | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 256 × 32 | 1.91 ms | 1.91 ms | 228 µs | 8.4× faster | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 4096 × 32 | 69.74 ms | 57.16 ms | 926 µs | 61.8× faster | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 1 × 128 | 18 µs | 18 µs | 166 µs | 9.5× slower | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 16 × 128 | 469 µs | 481 µs | 184 µs | 2.6× faster | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 256 × 128 | 8.83 ms | 7.62 ms | 393 µs | 19.4× faster | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 1024 × 128 | 53.77 ms | 39.13 ms | 906 µs | 43.2× faster | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 0.4 µs | 8.6 µs | kernels 8.2 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 1 × 32 | cpu | 0.2 µs | 0.4 µs | 10 µs | kernels 9.6 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 1 × 32 | gpu | 0.5 µs | 0.8 µs | 53 µs | upload 4.9 µs, record 3.8 µs, submit 39 µs, download 0.1 µs, rest 4.5 µs | 112 µs | 45 µs | 0.8 µs |
| 16 × 32 | cpu-1 | 0.3 µs | 0.5 µs | 59 µs | kernels 58 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – | 0.2 µs |
| 16 × 32 | cpu | 0.4 µs | 0.7 µs | 85 µs | kernels 84 µs, store 0.1 µs, rest 0.7 µs | 0.0 µs | – | 0.4 µs |
| 16 × 32 | gpu | 0.5 µs | 0.8 µs | 53 µs | upload 4.7 µs, record 3.9 µs, submit 40 µs, download 0.1 µs, rest 4.7 µs | 116 µs | 49 µs | 0.9 µs |
| 256 × 32 | cpu-1 | 0.6 µs | 1.0 µs | 1.91 ms | kernels 1.91 ms, store 0.2 µs, rest 1.0 µs | 0.0 µs | – | 0.3 µs |
| 256 × 32 | cpu | 0.7 µs | 1.5 µs | 1.91 ms | kernels 1.91 ms, store 0.3 µs, rest 1.1 µs | 0.0 µs | – | 0.8 µs |
| 256 × 32 | gpu | 0.5 µs | 0.8 µs | 54 µs | upload 4.9 µs, record 3.8 µs, submit 40 µs, download 0.1 µs, rest 5.6 µs | 173 µs | 96 µs | 0.8 µs |
| 4096 × 32 | cpu-1 | 2.4 µs | 4.7 µs | 69.72 ms | kernels 69.71 ms, store 1.3 µs, rest 4.4 µs | 0.0 µs | – | 2.7 µs |
| 4096 × 32 | cpu | 2.1 µs | 4.1 µs | 57.15 ms | kernels 57.15 ms, store 0.9 µs, rest 4.0 µs | 0.0 µs | – | 2.4 µs |
| 4096 × 32 | gpu | 0.5 µs | 0.8 µs | 66 µs | upload 5.7 µs, record 3.9 µs, submit 41 µs, download 0.1 µs, rest 16 µs | 858 µs | 608 µs | 0.9 µs |
| 1 × 128 | cpu-1 | 0.2 µs | 0.4 µs | 17 µs | kernels 17 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 1 × 128 | cpu | 0.2 µs | 0.4 µs | 17 µs | kernels 16 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 1 × 128 | gpu | 0.5 µs | 0.8 µs | 52 µs | upload 4.9 µs, record 3.9 µs, submit 39 µs, download 0.1 µs, rest 4.6 µs | 112 µs | 45 µs | 0.8 µs |
| 16 × 128 | cpu-1 | 0.3 µs | 0.6 µs | 468 µs | kernels 467 µs, store 0.2 µs, rest 0.7 µs | 0.0 µs | – | 0.3 µs |
| 16 × 128 | cpu | 0.5 µs | 1.0 µs | 479 µs | kernels 478 µs, store 0.2 µs, rest 0.9 µs | 0.0 µs | – | 0.6 µs |
| 16 × 128 | gpu | 0.5 µs | 0.8 µs | 53 µs | upload 4.8 µs, record 4.0 µs, submit 40 µs, download 0.1 µs, rest 4.7 µs | 129 µs | 58 µs | 0.8 µs |
| 256 × 128 | cpu-1 | 2.9 µs | 5.4 µs | 8.82 ms | kernels 8.81 ms, store 1.1 µs, rest 5.5 µs | 0.0 µs | – | 2.3 µs |
| 256 × 128 | cpu | 1.3 µs | 2.5 µs | 7.61 ms | kernels 7.61 ms, store 0.6 µs, rest 2.7 µs | 0.0 µs | – | 1.2 µs |
| 256 × 128 | gpu | 0.5 µs | 0.8 µs | 54 µs | upload 5.0 µs, record 3.9 µs, submit 40 µs, download 0.1 µs, rest 5.7 µs | 337 µs | 259 µs | 0.8 µs |
| 1024 × 128 | cpu-1 | 4.0 µs | 7.1 µs | 53.76 ms | kernels 53.76 ms, store 2.0 µs, rest 7.7 µs | 0.0 µs | – | 4.2 µs |
| 1024 × 128 | cpu | 2.4 µs | 3.9 µs | 39.12 ms | kernels 39.11 ms, store 1.1 µs, rest 4.3 µs | 0.0 µs | – | 2.4 µs |
| 1024 × 128 | gpu | 0.5 µs | 0.8 µs | 57 µs | upload 5.1 µs, record 3.9 µs, submit 40 µs, download 0.1 µs, rest 7.7 µs | 848 µs | 589 µs | 0.8 µs |

## sample

Sample Curve, 64 frames per curve at places which change.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 12 µs | 15 µs | 110 µs | 7.5× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 32 | 182 µs | 28 µs | 110 µs | 4.0× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 32 | 3.03 ms | 564 µs | 114 µs | 5.0× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 4096 × 32 | 50.01 ms | 8.84 ms | 208 µs | 42.4× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 1 × 128 | 12 µs | 15 µs | 112 µs | 7.4× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 128 | 226 µs | 52 µs | 117 µs | 2.2× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 128 | 3.72 ms | 579 µs | 115 µs | 5.0× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 1024 × 128 | 12.62 ms | 2.04 ms | 160 µs | 12.8× faster | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 0.3 µs | 11 µs | kernels 11 µs, rest 0.3 µs | 0.0 µs | – | 0.1 µs |
| 1 × 32 | cpu | 0.2 µs | 0.3 µs | 14 µs | kernels 14 µs, rest 0.3 µs | 0.0 µs | – | 0.1 µs |
| 1 × 32 | gpu | 0.5 µs | 0.6 µs | 37 µs | upload 3.7 µs, record 1.5 µs, submit 30 µs, rest 2.3 µs | 72 µs | 11 µs | 0.5 µs |
| 16 × 32 | cpu-1 | 0.2 µs | 0.3 µs | 181 µs | kernels 181 µs, store 0.1 µs, rest 0.3 µs | 0.0 µs | – | 0.2 µs |
| 16 × 32 | cpu | 0.3 µs | 0.5 µs | 27 µs | kernels 27 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.3 µs |
| 16 × 32 | gpu | 0.5 µs | 0.6 µs | 38 µs | upload 3.8 µs, record 1.6 µs, submit 30 µs, rest 2.5 µs | 70 µs | 11 µs | 0.5 µs |
| 256 × 32 | cpu-1 | 0.4 µs | 0.6 µs | 3.03 ms | kernels 3.03 ms, store 0.2 µs, rest 0.7 µs | 0.0 µs | – | 0.3 µs |
| 256 × 32 | cpu | 0.5 µs | 0.7 µs | 562 µs | kernels 562 µs, store 0.2 µs, rest 0.8 µs | 0.0 µs | – | 0.4 µs |
| 256 × 32 | gpu | 0.5 µs | 0.6 µs | 39 µs | upload 3.7 µs, record 1.5 µs, submit 30 µs, rest 3.4 µs | 73 µs | 12 µs | 0.5 µs |
| 4096 × 32 | cpu-1 | 4.2 µs | 5.0 µs | 50.00 ms | kernels 49.99 ms, store 1.4 µs, rest 7.4 µs | 0.0 µs | – | 3.6 µs |
| 4096 × 32 | cpu | 3.3 µs | 5.4 µs | 8.83 ms | kernels 8.82 ms, store 1.0 µs, rest 5.3 µs | 0.0 µs | – | 2.7 µs |
| 4096 × 32 | gpu | 0.6 µs | 0.8 µs | 60 µs | upload 5.1 µs, record 1.9 µs, submit 36 µs, rest 16 µs | 147 µs | 78 µs | 0.6 µs |
| 1 × 128 | cpu-1 | 0.2 µs | 0.4 µs | 12 µs | kernels 11 µs, rest 0.3 µs | 0.0 µs | – | 0.1 µs |
| 1 × 128 | cpu | 0.2 µs | 0.4 µs | 15 µs | kernels 14 µs, rest 0.3 µs | 0.0 µs | – | 0.2 µs |
| 1 × 128 | gpu | 0.5 µs | 0.7 µs | 41 µs | upload 3.7 µs, record 1.6 µs, submit 33 µs, download 0.1 µs, rest 2.5 µs | 72 µs | 11 µs | 0.5 µs |
| 16 × 128 | cpu-1 | 0.2 µs | 0.4 µs | 226 µs | kernels 225 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 16 × 128 | cpu | 0.4 µs | 0.6 µs | 51 µs | kernels 50 µs, store 0.1 µs, rest 0.5 µs | 0.0 µs | – | 0.4 µs |
| 16 × 128 | gpu | 0.7 µs | 0.8 µs | 43 µs | upload 4.1 µs, record 1.7 µs, submit 35 µs, download 0.1 µs, rest 2.9 µs | 71 µs | 12 µs | 0.6 µs |
| 256 × 128 | cpu-1 | 0.4 µs | 0.7 µs | 3.72 ms | kernels 3.71 ms, store 0.2 µs, rest 0.8 µs | 0.0 µs | – | 0.3 µs |
| 256 × 128 | cpu | 0.4 µs | 0.7 µs | 578 µs | kernels 577 µs, store 0.2 µs, rest 0.8 µs | 0.0 µs | – | 0.4 µs |
| 256 × 128 | gpu | 0.5 µs | 0.7 µs | 39 µs | upload 3.7 µs, record 1.5 µs, submit 31 µs, rest 3.5 µs | 74 µs | 15 µs | 0.5 µs |
| 1024 × 128 | cpu-1 | 1.1 µs | 2.1 µs | 12.62 ms | kernels 12.62 ms, store 0.4 µs, rest 2.5 µs | 0.0 µs | – | 0.9 µs |
| 1024 × 128 | cpu | 0.6 µs | 1.4 µs | 2.04 ms | kernels 2.04 ms, store 0.3 µs, rest 1.2 µs | 0.0 µs | – | 0.5 µs |
| 1024 × 128 | gpu | 0.5 µs | 0.7 µs | 42 µs | upload 3.9 µs, record 1.5 µs, submit 32 µs, rest 5.4 µs | 115 µs | 46 µs | 0.5 µs |

## chain

NURBS, Translate, a Sweep of a circle and Sample Curve, the control points move.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 44 µs | 83 µs | 244 µs | 2.9× slower | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 16 × 32 | 836 µs | 186 µs | 271 µs | 1.5× slower | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 256 × 32 | 11.33 ms | 1.69 ms | 320 µs | 5.3× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 4096 × 32 | 189.41 ms | 29.67 ms | 1.32 ms | 22.5× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 1 × 128 | 149 µs | 200 µs | 286 µs | 1.4× slower | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 16 × 128 | 2.36 ms | 434 µs | 318 µs | 1.4× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 256 × 128 | 39.22 ms | 5.72 ms | 584 µs | 9.8× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 1024 × 128 | 171.96 ms | 23.26 ms | 1.34 ms | 17.3× faster | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 0.6 µs | 43 µs | kernels 43 µs, store 0.1 µs, rest 0.6 µs | 0.0 µs | – | 0.3 µs |
| 1 × 32 | cpu | 0.3 µs | 1.0 µs | 82 µs | kernels 80 µs, store 0.2 µs, rest 1.0 µs | 0.0 µs | – | 0.7 µs |
| 1 × 32 | gpu | 0.5 µs | 1.2 µs | 67 µs | upload 5.7 µs, record 6.4 µs, submit 48 µs, download 0.1 µs, rest 7.2 µs | 175 µs | 95 µs | 1.0 µs |
| 16 × 32 | cpu-1 | 0.7 µs | 0.9 µs | 835 µs | kernels 834 µs, store 0.3 µs, rest 0.9 µs | 0.0 µs | – | 0.5 µs |
| 16 × 32 | cpu | 1.3 µs | 1.5 µs | 184 µs | kernels 181 µs, store 0.3 µs, rest 1.7 µs | 0.0 µs | – | 0.8 µs |
| 16 × 32 | gpu | 1.0 µs | 1.5 µs | 80 µs | upload 6.6 µs, record 7.2 µs, submit 55 µs, download 0.1 µs, rest 11 µs | 189 µs | 108 µs | 1.2 µs |
| 256 × 32 | cpu-1 | 7.8 µs | 1.9 µs | 11.32 ms | kernels 11.32 ms, store 0.6 µs, rest 2.0 µs | 0.0 µs | – | 1.0 µs |
| 256 × 32 | cpu | 8.0 µs | 2.2 µs | 1.68 ms | kernels 1.67 ms, store 0.7 µs, rest 2.5 µs | 0.0 µs | – | 1.3 µs |
| 256 × 32 | gpu | 6.1 µs | 1.3 µs | 92 µs | upload 19 µs, record 6.3 µs, submit 51 µs, download 0.1 µs, rest 16 µs | 220 µs | 133 µs | 1.3 µs |
| 4096 × 32 | cpu-1 | 164 µs | 6.9 µs | 189.24 ms | kernels 189.22 ms, store 3.6 µs, rest 13 µs | 0.0 µs | – | 4.5 µs |
| 4096 × 32 | cpu | 162 µs | 6.3 µs | 29.51 ms | kernels 29.49 ms, store 3.5 µs, rest 12 µs | 0.0 µs | – | 4.3 µs |
| 4096 × 32 | gpu | 122 µs | 1.9 µs | 314 µs | upload 139 µs, record 7.2 µs, submit 65 µs, download 0.2 µs, rest 103 µs | 856 µs | 665 µs | 1.1 µs |
| 1 × 128 | cpu-1 | 0.3 µs | 0.7 µs | 148 µs | kernels 148 µs, store 0.2 µs, rest 0.7 µs | 0.0 µs | – | 0.3 µs |
| 1 × 128 | cpu | 0.4 µs | 1.0 µs | 198 µs | kernels 196 µs, store 0.2 µs, rest 1.1 µs | 0.0 µs | – | 0.6 µs |
| 1 × 128 | gpu | 0.6 µs | 1.2 µs | 67 µs | upload 5.6 µs, record 6.4 µs, submit 48 µs, download 0.1 µs, rest 7.4 µs | 217 µs | 147 µs | 1.1 µs |
| 16 × 128 | cpu-1 | 1.7 µs | 0.9 µs | 2.36 ms | kernels 2.36 ms, store 0.3 µs, rest 1.0 µs | 0.0 µs | – | 0.5 µs |
| 16 × 128 | cpu | 2.4 µs | 1.5 µs | 430 µs | kernels 428 µs, store 0.4 µs, rest 2.0 µs | 0.0 µs | – | 1.2 µs |
| 16 × 128 | gpu | 2.2 µs | 1.3 µs | 75 µs | upload 9.2 µs, record 6.6 µs, submit 50 µs, download 0.2 µs, rest 9.6 µs | 238 µs | 162 µs | 1.3 µs |
| 256 × 128 | cpu-1 | 33 µs | 3.6 µs | 39.18 ms | kernels 39.18 ms, store 1.1 µs, rest 4.7 µs | 0.0 µs | – | 1.9 µs |
| 256 × 128 | cpu | 34 µs | 2.3 µs | 5.69 ms | kernels 5.68 ms, store 1.1 µs, rest 3.0 µs | 0.0 µs | – | 1.3 µs |
| 256 × 128 | gpu | 27 µs | 1.5 µs | 122 µs | upload 42 µs, record 6.6 µs, submit 52 µs, download 0.1 µs, rest 21 µs | 434 µs | 269 µs | 1.1 µs |
| 1024 × 128 | cpu-1 | 167 µs | 6.1 µs | 171.81 ms | kernels 171.79 ms, store 3.7 µs, rest 12 µs | 0.1 µs | – | 2.8 µs |
| 1024 × 128 | cpu | 155 µs | 5.3 µs | 23.10 ms | kernels 23.09 ms, store 2.3 µs, rest 8.9 µs | 0.0 µs | – | 2.5 µs |
| 1024 × 128 | gpu | 121 µs | 1.9 µs | 267 µs | upload 138 µs, record 7.2 µs, submit 64 µs, download 0.2 µs, rest 58 µs | 936 µs | 750 µs | 1.2 µs |

## instances

A ring of 10 patches put onto 64 frames along every curve by reference, the control points move.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 52 µs | 88 µs | 228 µs | 2.6× slower | 3 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 16 × 32 | 687 µs | 176 µs | 242 µs | 1.4× slower | 3 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 256 × 32 | 11.10 ms | 1.60 ms | 289 µs | 5.5× faster | 3 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 1024 × 32 | 45.51 ms | 6.47 ms | 503 µs | 12.9× faster | 3 on gpu, 0 on cpu, 4 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 1 × 32 | cpu-1 | 0.3 µs | 0.9 µs | 50 µs | kernels 49 µs, store 0.1 µs, rest 0.7 µs | 0.0 µs | – | 0.5 µs |
| 1 × 32 | cpu | 0.4 µs | 1.1 µs | 87 µs | kernels 86 µs, store 0.2 µs, rest 0.8 µs | 0.0 µs | – | 0.7 µs |
| 1 × 32 | gpu | 0.5 µs | 1.2 µs | 63 µs | upload 7.3 µs, record 5.0 µs, submit 44 µs, download 0.1 µs, rest 6.5 µs | 164 µs | 84 µs | 1.1 µs |
| 16 × 32 | cpu-1 | 0.6 µs | 0.7 µs | 686 µs | kernels 685 µs, store 0.2 µs, rest 0.6 µs | 0.0 µs | – | 0.4 µs |
| 16 × 32 | cpu | 0.9 µs | 1.0 µs | 173 µs | kernels 172 µs, store 0.3 µs, rest 1.1 µs | 0.0 µs | – | 0.5 µs |
| 16 × 32 | gpu | 0.9 µs | 1.3 µs | 66 µs | upload 8.1 µs, record 5.0 µs, submit 45 µs, download 0.1 µs, rest 8.8 µs | 173 µs | 92 µs | 1.2 µs |
| 256 × 32 | cpu-1 | 6.6 µs | 1.7 µs | 11.09 ms | kernels 11.09 ms, store 0.4 µs, rest 1.4 µs | 0.0 µs | – | 0.9 µs |
| 256 × 32 | cpu | 7.1 µs | 2.0 µs | 1.59 ms | kernels 1.58 ms, store 0.4 µs, rest 1.7 µs | 0.0 µs | – | 1.2 µs |
| 256 × 32 | gpu | 6.1 µs | 1.4 µs | 92 µs | upload 21 µs, record 5.1 µs, submit 47 µs, download 0.1 µs, rest 17 µs | 190 µs | 107 µs | 1.3 µs |
| 1024 × 32 | cpu-1 | 35 µs | 3.3 µs | 45.48 ms | kernels 45.47 ms, store 0.9 µs, rest 3.2 µs | 0.0 µs | – | 1.5 µs |
| 1024 × 32 | cpu | 36 µs | 2.8 µs | 6.44 ms | kernels 6.43 ms, store 0.9 µs, rest 2.8 µs | 0.0 µs | – | 1.8 µs |
| 1024 × 32 | gpu | 23 µs | 1.5 µs | 148 µs | upload 54 µs, record 5.3 µs, submit 48 µs, download 0.1 µs, rest 41 µs | 330 µs | 165 µs | 1.1 µs |

## instances realized

The rings of `instances`, copied to where each one is.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 107 µs | 106 µs | 259 µs | 2.4× slower | 4 on gpu, 0 on cpu, 6 passes, 0 downloads |
| 16 × 32 | 1.68 ms | 299 µs | 282 µs | 1.1× faster | 4 on gpu, 0 on cpu, 6 passes, 0 downloads |
| 256 × 32 | 27.58 ms | 3.67 ms | 550 µs | 6.7× faster | 4 on gpu, 0 on cpu, 6 passes, 0 downloads |
| 1024 × 32 | 111.37 ms | 14.17 ms | 1.33 ms | 10.7× faster | 4 on gpu, 0 on cpu, 6 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 0.7 µs | 106 µs | kernels 105 µs, store 0.2 µs, rest 0.7 µs | 0.0 µs | – | 0.5 µs |
| 1 × 32 | cpu | 0.4 µs | 1.2 µs | 104 µs | kernels 102 µs, store 0.2 µs, rest 1.1 µs | 0.0 µs | – | 0.9 µs |
| 1 × 32 | gpu | 0.6 µs | 1.3 µs | 74 µs | upload 7.5 µs, record 7.2 µs, submit 52 µs, download 0.2 µs, rest 8.1 µs | 182 µs | 94 µs | 1.4 µs |
| 16 × 32 | cpu-1 | 0.7 µs | 1.1 µs | 1.67 ms | kernels 1.67 ms, store 0.3 µs, rest 1.0 µs | 0.0 µs | – | 0.6 µs |
| 16 × 32 | cpu | 1.2 µs | 1.6 µs | 296 µs | kernels 294 µs, store 0.3 µs, rest 1.4 µs | 0.0 µs | – | 1.1 µs |
| 16 × 32 | gpu | 0.9 µs | 1.3 µs | 78 µs | upload 8.3 µs, record 7.2 µs, submit 53 µs, download 0.2 µs, rest 9.5 µs | 201 µs | 112 µs | 1.3 µs |
| 256 × 32 | cpu-1 | 11 µs | 2.7 µs | 27.56 ms | kernels 27.56 ms, store 1.0 µs, rest 2.8 µs | 0.0 µs | – | 2.2 µs |
| 256 × 32 | cpu | 10 µs | 2.9 µs | 3.66 ms | kernels 3.65 ms, store 0.9 µs, rest 3.1 µs | 0.0 µs | – | 2.5 µs |
| 256 × 32 | gpu | 11 µs | 2.1 µs | 147 µs | upload 27 µs, record 11 µs, submit 78 µs, download 0.2 µs, rest 30 µs | 389 µs | 296 µs | 2.4 µs |
| 1024 × 32 | cpu-1 | 49 µs | 6.8 µs | 111.32 ms | kernels 111.31 ms, store 2.5 µs, rest 7.1 µs | 0.0 µs | – | 4.8 µs |
| 1024 × 32 | cpu | 53 µs | 5.7 µs | 14.12 ms | kernels 14.11 ms, store 1.7 µs, rest 6.0 µs | 0.0 µs | – | 3.3 µs |
| 1024 × 32 | gpu | 46 µs | 2.1 µs | 189 µs | upload 57 µs, record 8.4 µs, submit 72 µs, download 0.2 µs, rest 51 µs | 1.09 ms | 912 µs | 2.5 µs |

## for each

NURBS and Translate for every curve on its own, the control points of one curve move.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 16 × 8 | 9.8 µs | 22 µs | 232 µs | 10.4× slower | 2 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 256 × 8 | 42 µs | 51 µs | 268 µs | 5.3× slower | 2 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 4096 × 8 | 660 µs | 671 µs | 952 µs | 1.4× slower | 2 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 16384 × 8 | 3.43 ms | 3.42 ms | 3.40 ms | 1.0× faster | 2 on gpu, 0 on cpu, 9 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 16 × 8 | cpu-1 | 0.3 µs | 0.3 µs | 9.2 µs | kernels 5.7 µs, store 0.1 µs, rest 3.4 µs | 0.0 µs | – | 0.4 µs |
| 16 × 8 | cpu | 0.5 µs | 0.5 µs | 21 µs | kernels 15 µs, store 0.1 µs, rest 5.7 µs | 0.0 µs | – | 0.7 µs |
| 16 × 8 | gpu | 0.6 µs | 0.8 µs | 74 µs | upload 3.6 µs, record 6.6 µs, submit 54 µs, download 0.1 µs, rest 10 µs | 154 µs | 47 µs | 1.2 µs |
| 256 × 8 | cpu-1 | 1.7 µs | 0.4 µs | 40 µs | kernels 5.9 µs, store 0.1 µs, rest 34 µs | 0.0 µs | – | 1.9 µs |
| 256 × 8 | cpu | 1.8 µs | 0.4 µs | 49 µs | kernels 10 µs, store 0.1 µs, rest 38 µs | 0.0 µs | – | 3.1 µs |
| 256 × 8 | gpu | 1.9 µs | 0.7 µs | 108 µs | upload 6.2 µs, record 6.6 µs, submit 53 µs, download 0.1 µs, rest 42 µs | 155 µs | 46 µs | 2.9 µs |
| 4096 × 8 | cpu-1 | 29 µs | 0.6 µs | 632 µs | kernels 6.9 µs, store 0.1 µs, rest 625 µs | 0.0 µs | – | 31 µs |
| 4096 × 8 | cpu | 24 µs | 0.6 µs | 646 µs | kernels 12 µs, store 0.1 µs, rest 634 µs | 0.0 µs | – | 32 µs |
| 4096 × 8 | gpu | 24 µs | 1.0 µs | 733 µs | upload 37 µs, record 13 µs, submit 66 µs, download 0.1 µs, rest 616 µs | 193 µs | 54 µs | 39 µs |
| 16384 × 8 | cpu-1 | 151 µs | 1.4 µs | 3.26 ms | kernels 8.4 µs, store 0.2 µs, rest 3.25 ms | 0.0 µs | – | 237 µs |
| 16384 × 8 | cpu | 146 µs | 1.2 µs | 3.25 ms | kernels 14 µs, store 0.2 µs, rest 3.24 ms | 0.0 µs | – | 196 µs |
| 16384 × 8 | gpu | 130 µs | 1.3 µs | 3.09 ms | upload 124 µs, record 35 µs, submit 145 µs, download 0.1 µs, rest 2.79 ms | 156 µs | 77 µs | 198 µs |

## for each, all

NURBS and Translate for every curve on its own, the offset of all of them changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 16 × 8 | 8.8 µs | 61 µs | 176 µs | 2.9× slower | 16 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 8 | 127 µs | 488 µs | 254 µs | 1.9× faster | 256 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 4096 × 8 | 2.35 ms | 7.95 ms | 1.81 ms | 4.4× faster | 4096 on gpu, 0 on cpu, 6 passes, 0 downloads |
| 16384 × 8 | 14.10 ms | 38.27 ms | 9.04 ms | 4.2× faster | 16384 on gpu, 0 on cpu, 19 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 16 × 8 | cpu-1 | 0.2 µs | 0.3 µs | 8.3 µs | kernels 1.6 µs, store 0.4 µs, rest 6.4 µs | 0.0 µs | – | 0.9 µs |
| 16 × 8 | cpu | 0.3 µs | 0.5 µs | 60 µs | kernels 48 µs, store 0.9 µs, rest 11 µs | 0.0 µs | – | 1.6 µs |
| 16 × 8 | gpu | 0.4 µs | 0.7 µs | 63 µs | upload 3.4 µs, record 3.7 µs, submit 44 µs, rest 12 µs | 111 µs | 2.5 µs | 2.2 µs |
| 256 × 8 | cpu-1 | 0.2 µs | 0.4 µs | 127 µs | kernels 27 µs, store 8.8 µs, rest 90 µs | 0.0 µs | – | 22 µs |
| 256 × 8 | cpu | 0.3 µs | 0.5 µs | 487 µs | kernels 358 µs, store 13 µs, rest 112 µs | 0.0 µs | – | 28 µs |
| 256 × 8 | gpu | 0.5 µs | 0.7 µs | 152 µs | upload 3.8 µs, record 4.9 µs, submit 47 µs, rest 97 µs | 101 µs | 3.5 µs | 32 µs |
| 4096 × 8 | cpu-1 | 0.4 µs | 0.6 µs | 2.34 ms | kernels 609 µs, store 166 µs, rest 1.56 ms | 0.0 µs | – | 605 µs |
| 4096 × 8 | cpu | 0.8 µs | 1.2 µs | 7.95 ms | kernels 5.67 ms, store 259 µs, rest 2.05 ms | 0.0 µs | – | 649 µs |
| 4096 × 8 | gpu | 0.8 µs | 1.0 µs | 1.73 ms | upload 12 µs, record 36 µs, submit 103 µs, rest 1.58 ms | 67 µs | 28 µs | 363 µs |
| 16384 × 8 | cpu-1 | 1.4 µs | 1.9 µs | 14.10 ms | kernels 3.77 ms, store 732 µs, rest 9.59 ms | 0.0 µs | – | 3.46 ms |
| 16384 × 8 | cpu | 2.2 µs | 3.0 µs | 38.27 ms | kernels 25.47 ms, store 1.09 ms, rest 11.62 ms | 0.0 µs | – | 4.79 ms |
| 16384 × 8 | gpu | 1.6 µs | 2.1 µs | 8.86 ms | upload 36 µs, record 239 µs, submit 292 µs, download 0.1 µs, rest 8.27 ms | 219 µs | 104 µs | 2.81 ms |

## mesh

Mesh of a swept circle, the tolerance changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 1 × 32 | 1.95 ms | 197 µs | 223 µs | 1.1× slower | 0 on gpu, 1 on cpu, 0 passes, 0 downloads |
| 16 × 32 | 31.04 ms | 3.84 ms | 3.71 ms | 1.0× faster | 0 on gpu, 1 on cpu, 0 passes, 0 downloads |
| 64 × 32 | 128.42 ms | 16.61 ms | 18.08 ms | 1.1× slower | 0 on gpu, 1 on cpu, 0 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 1 × 32 | cpu-1 | 0.2 µs | 0.4 µs | 1.95 ms | kernels 1.95 ms, store 0.1 µs, rest 0.3 µs | 0.0 µs | – | 0.3 µs |
| 1 × 32 | cpu | 0.3 µs | 0.6 µs | 196 µs | kernels 195 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.4 µs |
| 1 × 32 | gpu | 0.4 µs | 0.8 µs | 218 µs | cpu 217 µs, rest 1.2 µs | 3.7 µs | 0.0 µs | 0.6 µs |
| 16 × 32 | cpu-1 | 0.6 µs | 1.5 µs | 31.04 ms | kernels 31.04 ms, store 0.3 µs, rest 1.1 µs | 0.0 µs | – | 1.0 µs |
| 16 × 32 | cpu | 0.9 µs | 1.6 µs | 3.84 ms | kernels 3.84 ms, store 0.3 µs, rest 1.5 µs | 0.0 µs | – | 1.4 µs |
| 16 × 32 | gpu | 0.8 µs | 1.7 µs | 3.70 ms | download 0.1 µs, cpu 3.70 ms, rest 2.0 µs | 9.4 µs | 0.0 µs | 1.4 µs |
| 64 × 32 | cpu-1 | 2.1 µs | 4.2 µs | 128.42 ms | kernels 128.41 ms, store 1.3 µs, rest 4.2 µs | 0.0 µs | – | 3.3 µs |
| 64 × 32 | cpu | 3.0 µs | 5.4 µs | 16.60 ms | kernels 16.59 ms, store 1.1 µs, rest 4.3 µs | 0.0 µs | – | 3.3 µs |
| 64 × 32 | gpu | 2.3 µs | 4.4 µs | 18.05 ms | submit 0.2 µs, download 0.3 µs, cpu 18.04 ms, rest 6.2 µs | 22 µs | 0.0 µs | 3.3 µs |

## fence

The picket fence of the editor, with as many pickets, gets a little longer.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 10 × 1 | 24 µs | 100 µs | 687 µs | 6.9× slower | 18 on gpu, 7 on cpu, 8 passes, 4 downloads |
| 100 × 1 | 57 µs | 238 µs | 662 µs | 2.8× slower | 18 on gpu, 7 on cpu, 8 passes, 4 downloads |
| 1000 × 1 | 380 µs | 797 µs | 1.12 ms | 1.4× slower | 18 on gpu, 7 on cpu, 8 passes, 4 downloads |
| 10000 × 1 | 3.73 ms | 5.03 ms | 4.43 ms | 1.1× faster | 18 on gpu, 7 on cpu, 8 passes, 4 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 10 × 1 | cpu-1 | 0.2 µs | 3.7 µs | 20 µs | kernels 16 µs, store 0.9 µs, rest 3.0 µs | 0.0 µs | – | 2.2 µs |
| 10 × 1 | cpu | 0.3 µs | 6.6 µs | 92 µs | kernels 86 µs, store 1.2 µs, rest 4.8 µs | 0.0 µs | – | 5.1 µs |
| 10 × 1 | gpu | 0.6 µs | 5.6 µs | 594 µs | upload 21 µs, record 11 µs, submit 122 µs, download 400 µs, cpu 14 µs, rest 21 µs | 80 µs | 17 µs | 5.3 µs |
| 100 × 1 | cpu-1 | 0.2 µs | 3.5 µs | 53 µs | kernels 49 µs, store 0.9 µs, rest 3.0 µs | 0.0 µs | – | 2.3 µs |
| 100 × 1 | cpu | 0.4 µs | 7.5 µs | 230 µs | kernels 223 µs, store 1.2 µs, rest 5.1 µs | 0.0 µs | – | 5.8 µs |
| 100 × 1 | gpu | 0.5 µs | 5.3 µs | 584 µs | upload 19 µs, record 11 µs, submit 116 µs, download 367 µs, cpu 44 µs, rest 20 µs | 73 µs | 17 µs | 4.7 µs |
| 1000 × 1 | cpu-1 | 0.2 µs | 4.2 µs | 376 µs | kernels 371 µs, store 1.0 µs, rest 3.3 µs | 0.0 µs | – | 2.8 µs |
| 1000 × 1 | cpu | 0.4 µs | 5.5 µs | 790 µs | kernels 784 µs, store 1.1 µs, rest 4.3 µs | 0.0 µs | – | 5.6 µs |
| 1000 × 1 | gpu | 0.6 µs | 5.8 µs | 944 µs | upload 36 µs, record 12 µs, submit 124 µs, download 387 µs, cpu 356 µs, rest 24 µs | 169 µs | 18 µs | 6.0 µs |
| 10000 × 1 | cpu-1 | 0.3 µs | 4.8 µs | 3.72 ms | kernels 3.72 ms, store 1.5 µs, rest 4.3 µs | 0.0 µs | – | 3.6 µs |
| 10000 × 1 | cpu | 0.7 µs | 7.7 µs | 5.02 ms | kernels 5.01 ms, store 1.8 µs, rest 6.4 µs | 0.0 µs | – | 7.3 µs |
| 10000 × 1 | gpu | 0.7 µs | 6.4 µs | 4.22 ms | upload 134 µs, record 12 µs, submit 147 µs, download 425 µs, cpu 3.46 ms, rest 39 µs | 196 µs | 24 µs | 6.5 µs |

## mostly constant

As many curves of constant points, each moved by a constant and an output of its own, and one curve moved by the only parameter, which changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 16 × 8 | 0.7 µs | 2.7 µs | 94 µs | 34.7× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 8 | 0.9 µs | 3.7 µs | 94 µs | 25.5× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 4096 × 8 | 1.1 µs | 3.0 µs | 97 µs | 32.2× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16384 × 8 | 1.5 µs | 4.3 µs | 97 µs | 22.5× slower | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 16 × 8 | cpu-1 | 0.1 µs | 0.2 µs | 0.3 µs | kernels 0.1 µs, rest 0.2 µs | 0.0 µs | – | 0.1 µs |
| 16 × 8 | cpu | 0.3 µs | 0.4 µs | 2.0 µs | kernels 1.5 µs, store 0.1 µs, rest 0.4 µs | 0.0 µs | – | 0.2 µs |
| 16 × 8 | gpu | 0.4 µs | 0.6 µs | 36 µs | upload 2.6 µs, record 1.6 µs, submit 30 µs, rest 1.8 µs | 57 µs | 1.6 µs | 0.4 µs |
| 256 × 8 | cpu-1 | 0.2 µs | 0.2 µs | 0.4 µs | kernels 0.2 µs, rest 0.2 µs | 0.0 µs | – | 0.2 µs |
| 256 × 8 | cpu | 0.4 µs | 0.8 µs | 2.4 µs | kernels 1.9 µs, rest 0.4 µs | 0.0 µs | – | 0.3 µs |
| 256 × 8 | gpu | 0.5 µs | 0.6 µs | 36 µs | upload 2.6 µs, record 1.6 µs, submit 30 µs, rest 1.8 µs | 57 µs | 1.5 µs | 0.4 µs |
| 4096 × 8 | cpu-1 | 0.2 µs | 0.3 µs | 0.6 µs | kernels 0.2 µs, rest 0.3 µs | 0.0 µs | – | 0.2 µs |
| 4096 × 8 | cpu | 0.3 µs | 0.5 µs | 2.1 µs | kernels 1.6 µs, rest 0.5 µs | 0.0 µs | – | 0.3 µs |
| 4096 × 8 | gpu | 0.6 µs | 0.7 µs | 38 µs | upload 2.8 µs, record 1.6 µs, submit 31 µs, rest 2.2 µs | 57 µs | 1.7 µs | 0.8 µs |
| 16384 × 8 | cpu-1 | 0.2 µs | 0.3 µs | 0.9 µs | kernels 0.2 µs, rest 0.7 µs | 0.0 µs | – | 0.2 µs |
| 16384 × 8 | cpu | 0.4 µs | 0.5 µs | 3.4 µs | kernels 2.5 µs, store 0.1 µs, rest 0.9 µs | 0.0 µs | – | 0.3 µs |
| 16384 × 8 | gpu | 0.5 µs | 0.8 µs | 39 µs | upload 2.8 µs, record 1.6 µs, submit 32 µs, rest 2.6 µs | 57 µs | 1.8 µs | 0.5 µs |

## scene, a few move

Instances of different graphs, all evaluated together, of which 4 move with every edit. A size is the instances × the graphs. Every graph makes a curve out of the points of its instance and moves it, then sweeps, samples or moves it again in its own way.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 16 × 4 | 31 µs | 81 µs | 200 µs | 2.5× slower | 12 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 256 × 16 | 33 µs | 80 µs | 213 µs | 2.7× slower | 12 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 4096 × 16 | 52 µs | 101 µs | 233 µs | 2.3× slower | 12 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 4096 × 64 | 59 µs | 110 µs | 246 µs | 2.2× slower | 12 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 16384 × 64 | 101 µs | 156 µs | 312 µs | 2.0× slower | 12 on gpu, 0 on cpu, 4 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 16 × 4 | cpu-1 | 0.6 µs | 2.0 µs | 28 µs | kernels 26 µs, store 0.4 µs, rest 1.6 µs | 0.0 µs | – | 2.2 µs |
| 16 × 4 | cpu | 0.9 µs | 3.0 µs | 76 µs | kernels 73 µs, store 0.6 µs, rest 2.4 µs | 0.0 µs | – | 3.0 µs |
| 16 × 4 | gpu | 1.2 µs | 3.1 µs | 69 µs | upload 7.3 µs, record 4.9 µs, submit 46 µs, download 0.1 µs, rest 10 µs | 125 µs | 47 µs | 4.0 µs |
| 256 × 16 | cpu-1 | 0.8 µs | 3.2 µs | 29 µs | kernels 27 µs, store 0.4 µs, rest 1.7 µs | 0.0 µs | – | 3.7 µs |
| 256 × 16 | cpu | 1.1 µs | 4.8 µs | 74 µs | kernels 72 µs, store 0.6 µs, rest 2.4 µs | 0.0 µs | – | 5.3 µs |
| 256 × 16 | gpu | 1.5 µs | 4.8 µs | 75 µs | upload 11 µs, record 5.0 µs, submit 48 µs, download 0.1 µs, rest 11 µs | 131 µs | 47 µs | 6.2 µs |
| 4096 × 16 | cpu-1 | 1.3 µs | 19 µs | 31 µs | kernels 29 µs, store 0.4 µs, rest 2.1 µs | 0.0 µs | – | 16 µs |
| 4096 × 16 | cpu | 1.8 µs | 20 µs | 78 µs | kernels 75 µs, store 0.6 µs, rest 2.9 µs | 0.0 µs | – | 21 µs |
| 4096 × 16 | gpu | 2.6 µs | 21 µs | 79 µs | upload 9.1 µs, record 5.3 µs, submit 50 µs, download 0.1 µs, rest 13 µs | 128 µs | 47 µs | 19 µs |
| 4096 × 64 | cpu-1 | 2.3 µs | 22 µs | 35 µs | kernels 32 µs, store 0.5 µs, rest 2.5 µs | 0.0 µs | – | 18 µs |
| 4096 × 64 | cpu | 1.9 µs | 22 µs | 84 µs | kernels 80 µs, store 0.6 µs, rest 2.9 µs | 0.0 µs | – | 24 µs |
| 4096 × 64 | gpu | 2.8 µs | 22 µs | 85 µs | upload 11 µs, record 5.2 µs, submit 54 µs, download 0.2 µs, rest 14 µs | 132 µs | 47 µs | 20 µs |
| 16384 × 64 | cpu-1 | 1.8 µs | 65 µs | 33 µs | kernels 30 µs, store 0.5 µs, rest 2.7 µs | 0.0 µs | – | 52 µs |
| 16384 × 64 | cpu | 2.0 µs | 68 µs | 82 µs | kernels 79 µs, store 0.7 µs, rest 3.5 µs | 0.0 µs | – | 64 µs |
| 16384 × 64 | gpu | 3.4 µs | 71 µs | 104 µs | upload 12 µs, record 5.5 µs, submit 68 µs, download 0.2 µs, rest 16 µs | 130 µs | 47 µs | 59 µs |

## scene, all move

The instances of `scene, a few move`, all of which move with every edit.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 16 × 4 | 29 µs | 142 µs | 223 µs | 1.6× slower | 36 on gpu, 0 on cpu, 6 passes, 0 downloads |
| 256 × 16 | 585 µs | 2.63 ms | 908 µs | 2.9× faster | 576 on gpu, 0 on cpu, 6 passes, 0 downloads |
| 4096 × 16 | 12.41 ms | 49.93 ms | 4.46 ms | 11.2× faster | 9216 on gpu, 0 on cpu, 9 passes, 0 downloads |
| 4096 × 64 | 17.20 ms | 65.38 ms | 4.54 ms | 14.4× faster | 9216 on gpu, 0 on cpu, 9 passes, 0 downloads |
| 16384 × 64 | 82.46 ms | 283.90 ms | 30.19 ms | 9.4× faster | 36864 on gpu, 0 on cpu, 33 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 16 × 4 | cpu-1 | 2.2 µs | 5.0 µs | 22 µs | kernels 16 µs, store 1.1 µs, rest 4.3 µs | 0.0 µs | – | 3.0 µs |
| 16 × 4 | cpu | 4.2 µs | 9.0 µs | 128 µs | kernels 116 µs, store 2.3 µs, rest 9.3 µs | 0.0 µs | – | 6.2 µs |
| 16 × 4 | gpu | 3.5 µs | 5.9 µs | 86 µs | upload 14 µs, record 7.4 µs, submit 52 µs, download 0.2 µs, rest 13 µs | 124 µs | 33 µs | 4.6 µs |
| 256 × 16 | cpu-1 | 43 µs | 80 µs | 459 µs | kernels 366 µs, store 20 µs, rest 69 µs | 0.0 µs | – | 118 µs |
| 256 × 16 | cpu | 50 µs | 89 µs | 2.50 ms | kernels 2.34 ms, store 35 µs, rest 126 µs | 0.0 µs | – | 133 µs |
| 256 × 16 | gpu | 39 µs | 80 µs | 341 µs | upload 189 µs, record 9.7 µs, submit 58 µs, download 0.6 µs, rest 87 µs | 381 µs | 34 µs | 97 µs |
| 4096 × 16 | cpu-1 | 1.17 ms | 1.79 ms | 9.61 ms | kernels 7.75 ms, store 420 µs, rest 1.42 ms | 0.0 µs | – | 2.97 ms |
| 4096 × 16 | cpu | 1.16 ms | 1.93 ms | 47.03 ms | kernels 44.00 ms, store 718 µs, rest 2.34 ms | 0.0 µs | – | 3.37 ms |
| 4096 × 16 | gpu | 878 µs | 1.46 ms | 1.92 ms | upload 74 µs, record 130 µs, submit 106 µs, download 6.5 µs, rest 1.61 ms | 226 µs | 92 µs | 1.53 ms |
| 4096 × 64 | cpu-1 | 1.14 ms | 1.90 ms | 14.19 ms | kernels 12.34 ms, store 428 µs, rest 1.52 ms | 0.0 µs | – | 2.71 ms |
| 4096 × 64 | cpu | 1.25 ms | 1.99 ms | 62.28 ms | kernels 58.77 ms, store 721 µs, rest 2.37 ms | 0.0 µs | – | 3.53 ms |
| 4096 × 64 | gpu | 915 µs | 1.46 ms | 1.96 ms | upload 69 µs, record 123 µs, submit 105 µs, download 6.8 µs, rest 1.65 ms | 189 µs | 106 µs | 1.53 ms |
| 16384 × 64 | cpu-1 | 7.22 ms | 12.49 ms | 63.70 ms | kernels 52.70 ms, store 1.84 ms, rest 9.95 ms | 0.0 µs | – | 15.37 ms |
| 16384 × 64 | cpu | 7.20 ms | 12.02 ms | 264.81 ms | kernels 248.60 ms, store 3.04 ms, rest 13.75 ms | 0.0 µs | – | 15.60 ms |
| 16384 × 64 | gpu | 5.95 ms | 11.06 ms | 13.48 ms | upload 1.05 ms, record 385 µs, submit 825 µs, download 26 µs, rest 11.20 ms | 28 µs | 411 µs | 11.04 ms |

## scene, linked

Rails of the curves of `scene, a few move`, each followed by a tube along it, of which 4 rails move with every edit. A size is the rails × the graphs of tubes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu work |
|---|---:|---:|---:|---:|---|
| 16 × 4 | 30 µs | 72 µs | 197 µs | 2.7× slower | 12 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 256 × 16 | 35 µs | 77 µs | 210 µs | 2.7× slower | 12 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 4096 × 16 | 98 µs | 150 µs | 274 µs | 1.8× slower | 12 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 4096 × 64 | 97 µs | 148 µs | 274 µs | 1.9× slower | 12 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 16384 × 64 | 286 µs | 341 µs | 611 µs | 1.8× slower | 12 on gpu, 0 on cpu, 4 passes, 0 downloads |

| size | backend | set | plan | flush | of which | wait | device | end frame |
|---|---|---:|---:|---:|---|---:|---:|---:|
| 16 × 4 | cpu-1 | 0.7 µs | 3.2 µs | 26 µs | kernels 24 µs, store 0.4 µs, rest 1.6 µs | 0.0 µs | – | 2.0 µs |
| 16 × 4 | cpu | 0.9 µs | 5.0 µs | 66 µs | kernels 63 µs, store 0.5 µs, rest 2.3 µs | 0.0 µs | – | 2.9 µs |
| 16 × 4 | gpu | 1.3 µs | 4.9 µs | 70 µs | upload 8.2 µs, record 5.1 µs, submit 46 µs, download 0.1 µs, rest 10 µs | 120 µs | 45 µs | 4.2 µs |
| 256 × 16 | cpu-1 | 0.7 µs | 7.0 µs | 27 µs | kernels 25 µs, store 0.4 µs, rest 1.6 µs | 0.0 µs | – | 3.9 µs |
| 256 × 16 | cpu | 1.0 µs | 8.9 µs | 66 µs | kernels 63 µs, store 0.5 µs, rest 2.2 µs | 0.0 µs | – | 5.9 µs |
| 256 × 16 | gpu | 1.5 µs | 9.7 µs | 72 µs | upload 9.8 µs, record 5.2 µs, submit 46 µs, download 0.1 µs, rest 11 µs | 121 µs | 44 µs | 7.2 µs |
| 4096 × 16 | cpu-1 | 1.3 µs | 67 µs | 29 µs | kernels 27 µs, store 0.4 µs, rest 2.1 µs | 0.0 µs | – | 29 µs |
| 4096 × 16 | cpu | 1.5 µs | 72 µs | 75 µs | kernels 72 µs, store 0.6 µs, rest 3.0 µs | 0.0 µs | – | 38 µs |
| 4096 × 16 | gpu | 2.3 µs | 71 µs | 75 µs | upload 7.9 µs, record 5.3 µs, submit 49 µs, download 0.1 µs, rest 12 µs | 124 µs | 45 µs | 32 µs |
| 4096 × 64 | cpu-1 | 1.3 µs | 66 µs | 29 µs | kernels 26 µs, store 0.4 µs, rest 2.2 µs | 0.0 µs | – | 30 µs |
| 4096 × 64 | cpu | 1.7 µs | 72 µs | 72 µs | kernels 69 µs, store 0.6 µs, rest 3.0 µs | 0.0 µs | – | 38 µs |
| 4096 × 64 | gpu | 2.3 µs | 70 µs | 76 µs | upload 8.6 µs, record 5.5 µs, submit 50 µs, download 0.1 µs, rest 12 µs | 124 µs | 45 µs | 33 µs |
| 16384 × 64 | cpu-1 | 2.0 µs | 252 µs | 31 µs | kernels 28 µs, store 0.4 µs, rest 2.9 µs | 0.0 µs | – | 106 µs |
| 16384 × 64 | cpu | 2.1 µs | 257 µs | 79 µs | kernels 75 µs, store 0.6 µs, rest 3.7 µs | 0.0 µs | – | 114 µs |
| 16384 × 64 | gpu | 3.7 µs | 303 µs | 106 µs | upload 11 µs, record 6.7 µs, submit 68 µs, download 0.2 µs, rest 18 µs | 198 µs | 46 µs | 131 µs |
