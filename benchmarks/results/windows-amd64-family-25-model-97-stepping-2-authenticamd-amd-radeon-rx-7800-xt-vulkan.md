# Benchmarks

windows, AMD64 Family 25 Model 97 Stepping 2, AuthenticAMD with 16 threads, AMD Radeon RX 7800 XT on vulkan, driver AMD proprietary driver 26.10.1 (LLPC). Commit ed5c4ac.

The time from an edit until its result is computed, the median of many edits. GPU work is split into planning, recording and handing it over, waiting for the device, and the time the device spends on compute passes.

## nurbs

NURBS Curve, the control points move.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu: plan / record / wait / device | gpu work |
|---|---:|---:|---:|---:|---|---|
| 1 × 32 | 38 µs | 38 µs | 194 µs | 5.0× slower | 2 µs / 56 µs / 133 µs / 70 µs | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 16 × 32 | 596 µs | 110 µs | 204 µs | 1.9× slower | 3 µs / 56 µs / 145 µs / 79 µs | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 32 | 9.59 ms | 1.14 ms | 386 µs | 2.9× faster | 37 µs / 185 µs / 163 µs / 91 µs | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 4096 × 32 | 157.76 ms | 20.20 ms | 3.55 ms | 5.7× faster | 817 µs / 2.18 ms / 526 µs / 349 µs | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 1 × 128 | 161 µs | 161 µs | 242 µs | 1.5× slower | 2 µs / 48 µs / 190 µs / 127 µs | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 16 × 128 | 2.56 ms | 326 µs | 288 µs | 1.1× faster | 10 µs / 73 µs / 204 µs / 131 µs | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 128 | 41.79 ms | 5.19 ms | 927 µs | 5.6× faster | 142 µs / 441 µs / 343 µs / 186 µs | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 1024 × 128 | 169.03 ms | 21.26 ms | 3.04 ms | 7.0× faster | 806 µs / 1.58 ms / 647 µs / 473 µs | 1 on gpu, 0 on cpu, 2 passes, 0 downloads |

## translate

Translate Curve of the curves, the offset changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu: plan / record / wait / device | gpu work |
|---|---:|---:|---:|---:|---|---|
| 1 × 32 | 1 µs | 5 µs | 93 µs | 20.2× slower | 1 µs / 35 µs / 57 µs / 2 µs | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 32 | 3 µs | 12 µs | 92 µs | 7.4× slower | 1 µs / 35 µs / 55 µs / 2 µs | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 32 | 43 µs | 87 µs | 94 µs | 1.1× slower | 1 µs / 35 µs / 57 µs / 6 µs | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 4096 × 32 | 2.29 ms | 1.82 ms | 170 µs | 10.7× faster | 1 µs / 36 µs / 133 µs / 72 µs | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 1 × 128 | 2 µs | 7 µs | 88 µs | 12.9× slower | 1 µs / 35 µs / 51 µs / 2 µs | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 128 | 12 µs | 24 µs | 87 µs | 3.6× slower | 1 µs / 34 µs / 52 µs / 3 µs | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 128 | 662 µs | 678 µs | 124 µs | 5.5× faster | 1 µs / 40 µs / 83 µs / 22 µs | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 1024 × 128 | 2.52 ms | 1.93 ms | 182 µs | 10.6× faster | 1 µs / 36 µs / 142 µs / 78 µs | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |

## sweep circle

Sweep of a circle along the curves, the radius changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu: plan / record / wait / device | gpu work |
|---|---:|---:|---:|---:|---|---|
| 1 × 32 | 2 µs | 2 µs | 118 µs | 65.5× slower | 2 µs / 51 µs / 62 µs / 3 µs | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 16 × 32 | 5 µs | 13 µs | 107 µs | 8.4× slower | 2 µs / 42 µs / 62 µs / 5 µs | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 32 | 70 µs | 119 µs | 138 µs | 1.2× slower | 2 µs / 62 µs / 74 µs / 11 µs | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 4096 × 32 | 5.97 ms | 4.72 ms | 217 µs | 21.7× faster | 2 µs / 54 µs / 161 µs / 93 µs | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 1 × 128 | 2 µs | 2 µs | 106 µs | 48.0× slower | 2 µs / 42 µs / 62 µs / 3 µs | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 16 × 128 | 18 µs | 59 µs | 106 µs | 1.8× slower | 1 µs / 42 µs / 63 µs / 5 µs | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 128 | 834 µs | 826 µs | 141 µs | 5.8× faster | 2 µs / 46 µs / 94 µs / 30 µs | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 1024 × 128 | 5.25 ms | 3.85 ms | 218 µs | 17.6× faster | 2 µs / 48 µs / 168 µs / 99 µs | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |

## sweep square

Sweep of a square curve along the curves, the square changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu: plan / record / wait / device | gpu work |
|---|---:|---:|---:|---:|---|---|
| 1 × 32 | 10 µs | 12 µs | 189 µs | 15.3× slower | 2 µs / 68 µs / 119 µs / 47 µs | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 16 × 32 | 61 µs | 79 µs | 184 µs | 2.3× slower | 2 µs / 62 µs / 117 µs / 50 µs | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 256 × 32 | 1.87 ms | 1.72 ms | 212 µs | 8.1× faster | 2 µs / 54 µs / 155 µs / 87 µs | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 4096 × 32 | 62.53 ms | 47.58 ms | 715 µs | 66.5× faster | 2 µs / 66 µs / 634 µs / 546 µs | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 1 × 128 | 20 µs | 20 µs | 171 µs | 8.6× slower | 2 µs / 52 µs / 117 µs / 48 µs | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 16 × 128 | 479 µs | 493 µs | 181 µs | 2.7× faster | 2 µs / 53 µs / 126 µs / 59 µs | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 256 × 128 | 9.18 ms | 7.88 ms | 354 µs | 22.2× faster | 2 µs / 63 µs / 289 µs / 213 µs | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |
| 1024 × 128 | 56.32 ms | 40.93 ms | 702 µs | 58.3× faster | 2 µs / 59 µs / 641 µs / 476 µs | 2 on gpu, 0 on cpu, 3 passes, 0 downloads |

## sample

Sample Curve, 64 frames per curve at places which change.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu: plan / record / wait / device | gpu work |
|---|---:|---:|---:|---:|---|---|
| 1 × 32 | 35 µs | 35 µs | 110 µs | 3.1× slower | 2 µs / 36 µs / 71 µs / 11 µs | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 32 | 542 µs | 66 µs | 108 µs | 1.6× slower | 2 µs / 37 µs / 70 µs / 11 µs | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 32 | 8.86 ms | 1.18 ms | 111 µs | 10.6× faster | 2 µs / 37 µs / 73 µs / 12 µs | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 4096 × 32 | 142.78 ms | 19.04 ms | 210 µs | 90.7× faster | 2 µs / 52 µs / 156 µs / 79 µs | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 1 × 128 | 36 µs | 34 µs | 109 µs | 3.2× slower | 2 µs / 36 µs / 70 µs / 11 µs | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 128 | 540 µs | 131 µs | 108 µs | 1.2× faster | 2 µs / 36 µs / 70 µs / 12 µs | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 128 | 8.72 ms | 1.19 ms | 113 µs | 10.5× faster | 2 µs / 37 µs / 74 µs / 15 µs | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 1024 × 128 | 35.42 ms | 4.59 ms | 148 µs | 31.0× faster | 2 µs / 44 µs / 102 µs / 43 µs | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |

## chain

NURBS, Translate, a Sweep of a circle and Sample Curve, the control points move.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu: plan / record / wait / device | gpu work |
|---|---:|---:|---:|---:|---|---|
| 1 × 32 | 75 µs | 117 µs | 231 µs | 2.0× slower | 2 µs / 66 µs / 162 µs / 85 µs | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 16 × 32 | 1.16 ms | 218 µs | 253 µs | 1.2× slower | 5 µs / 77 µs / 172 µs / 96 µs | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 256 × 32 | 18.55 ms | 2.54 ms | 451 µs | 5.6× faster | 38 µs / 208 µs / 205 µs / 120 µs | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 4096 × 32 | 305.64 ms | 44.49 ms | 3.86 ms | 11.5× faster | 811 µs / 2.25 ms / 789 µs / 595 µs | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 1 × 128 | 197 µs | 292 µs | 280 µs | 1.0× faster | 3 µs / 68 µs / 209 µs / 138 µs | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 16 × 128 | 3.14 ms | 555 µs | 330 µs | 1.7× faster | 12 µs / 94 µs / 222 µs / 151 µs | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 256 × 128 | 51.85 ms | 7.89 ms | 1.02 ms | 7.7× faster | 144 µs / 468 µs / 411 µs / 253 µs | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |
| 1024 × 128 | 211.15 ms | 33.07 ms | 3.36 ms | 9.9× faster | 834 µs / 1.63 ms / 896 µs / 695 µs | 4 on gpu, 0 on cpu, 5 passes, 0 downloads |

## mesh

Mesh of a swept circle, the tolerance changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu: plan / record / wait / device | gpu work |
|---|---:|---:|---:|---:|---|---|
| 1 × 32 | 2.31 ms | 333 µs | 344 µs | 1.0× slower | 2 µs / 337 µs / 5 µs / 0 µs | 0 on gpu, 1 on cpu, 0 passes, 0 downloads |
| 16 × 32 | 39.25 ms | 5.66 ms | 5.96 ms | 1.1× slower | 3 µs / 5.95 ms / 9 µs / 0 µs | 0 on gpu, 1 on cpu, 0 passes, 0 downloads |
| 64 × 32 | 150.50 ms | 24.95 ms | 25.64 ms | 1.0× slower | 4 µs / 25.63 ms / 11 µs / 0 µs | 0 on gpu, 1 on cpu, 0 passes, 0 downloads |
