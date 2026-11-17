# Benchmarks

windows, AMD64 Family 25 Model 97 Stepping 2, AuthenticAMD with 16 threads, AMD Radeon RX 7800 XT on vulkan, driver AMD proprietary driver 26.10.1 (LLPC). Commit 1b90310.

The time from an edit until its result is computed, the median of many edits. GPU work is split into planning, recording and handing it over, waiting for the device, and the time the device spends on compute passes.

## nurbs

NURBS Curve, the control points move.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu: plan / record / wait / device | gpu work |
|---|---:|---:|---:|---:|---|---|
| 1 × 32 | 39 µs | 39 µs | 1.16 ms | 30.0× slower | 1 µs / 42 µs / 1.11 ms / 823 µs | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 32 | 605 µs | 113 µs | 1.31 ms | 11.6× slower | 4 µs / 48 µs / 1.26 ms / 1.05 ms | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 32 | 9.72 ms | 1.22 ms | 5.09 ms | 4.2× slower | 40 µs / 124 µs / 4.93 ms / 4.83 ms | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 4096 × 32 | 161.59 ms | 20.95 ms | 20.84 ms | 1.0× faster | 912 µs / 19.93 ms / 17 µs / 0 µs | 0 on gpu, 1 on cpu, 0 passes, 0 downloads |
| 1 × 128 | 166 µs | 166 µs | 4.26 ms | 25.7× slower | 2 µs / 66 µs / 4.18 ms / 4.08 ms | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 128 | 2.61 ms | 454 µs | 4.61 ms | 10.2× slower | 11 µs / 73 µs / 4.53 ms / 4.41 ms | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 128 | 42.45 ms | 5.50 ms | 21.66 ms | 3.9× slower | 165 µs / 248 µs / 21.25 ms / 20.93 ms | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 1024 × 128 | 173.08 ms | 22.18 ms | 107.46 ms | 4.8× slower | 884 µs / 783 µs / 105.81 ms / 105.58 ms | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |

## translate

Translate Curve of the curves, the offset changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu: plan / record / wait / device | gpu work |
|---|---:|---:|---:|---:|---|---|
| 1 × 32 | 1 µs | 4 µs | 82 µs | 19.0× slower | 1 µs / 38 µs / 42 µs / 1 µs | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 32 | 4 µs | 13 µs | 87 µs | 6.8× slower | 1 µs / 38 µs / 48 µs / 2 µs | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 32 | 47 µs | 110 µs | 94 µs | 1.2× faster | 1 µs / 38 µs / 53 µs / 6 µs | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 4096 × 32 | 2.42 ms | 1.84 ms | 177 µs | 10.4× faster | 1 µs / 37 µs / 136 µs / 72 µs | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 1 × 128 | 1 µs | 10 µs | 88 µs | 9.1× slower | 1 µs / 36 µs / 51 µs / 2 µs | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 128 | 12 µs | 66 µs | 86 µs | 1.3× slower | 1 µs / 34 µs / 50 µs / 3 µs | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 128 | 627 µs | 708 µs | 124 µs | 5.7× faster | 2 µs / 42 µs / 80 µs / 21 µs | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 1024 × 128 | 2.61 ms | 1.99 ms | 125 µs | 15.9× faster | 1 µs / 36 µs / 87 µs / 46 µs | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |

## sweep circle

Sweep of a circle along the curves, the radius changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu: plan / record / wait / device | gpu work |
|---|---:|---:|---:|---:|---|---|
| 1 × 32 | 2 µs | 2 µs | 101 µs | 53.0× slower | 2 µs / 41 µs / 58 µs / 3 µs | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 16 × 32 | 5 µs | 13 µs | 106 µs | 8.0× slower | 2 µs / 42 µs / 62 µs / 5 µs | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 32 | 72 µs | 119 µs | 117 µs | 1.0× faster | 2 µs / 44 µs / 73 µs / 11 µs | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 4096 × 32 | 4.39 ms | 4.15 ms | 216 µs | 19.2× faster | 2 µs / 53 µs / 161 µs / 93 µs | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 1 × 128 | 3 µs | 3 µs | 106 µs | 37.7× slower | 2 µs / 41 µs / 63 µs / 4 µs | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 16 × 128 | 18 µs | 59 µs | 106 µs | 1.8× slower | 2 µs / 42 µs / 63 µs / 6 µs | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 128 | 849 µs | 890 µs | 137 µs | 6.5× faster | 2 µs / 44 µs / 91 µs / 29 µs | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 1024 × 128 | 4.17 ms | 3.75 ms | 158 µs | 23.7× faster | 2 µs / 46 µs / 110 µs / 63 µs | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |

## sweep square

Sweep of a square curve along the curves, the square changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu: plan / record / wait / device | gpu work |
|---|---:|---:|---:|---:|---|---|
| 1 × 32 | 10 µs | 10 µs | 269 µs | 27.2× slower | 2 µs / 46 µs / 220 µs / 156 µs | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 16 × 32 | 62 µs | 74 µs | 279 µs | 3.8× slower | 2 µs / 45 µs / 232 µs / 166 µs | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 32 | 1.87 ms | 1.77 ms | 319 µs | 5.5× faster | 2 µs / 47 µs / 269 µs / 203 µs | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 4096 × 32 | 69.07 ms | 49.95 ms | 784 µs | 63.7× faster | 2 µs / 59 µs / 721 µs / 577 µs | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 1 × 128 | 20 µs | 20 µs | 287 µs | 14.0× slower | 2 µs / 50 µs / 235 µs / 164 µs | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 16 × 128 | 485 µs | 494 µs | 300 µs | 1.6× faster | 2 µs / 52 µs / 245 µs / 174 µs | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 256 × 128 | 8.84 ms | 7.64 ms | 469 µs | 16.3× faster | 3 µs / 66 µs / 401 µs / 325 µs | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |
| 1024 × 128 | 52.21 ms | 43.68 ms | 600 µs | 72.8× faster | 3 µs / 70 µs / 528 µs / 466 µs | 2 on gpu, 0 on cpu, 2 passes, 0 downloads |

## sample

Sample Curve, 64 frames per curve at places which change.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu: plan / record / wait / device | gpu work |
|---|---:|---:|---:|---:|---|---|
| 1 × 32 | 37 µs | 37 µs | 109 µs | 3.0× slower | 2 µs / 40 µs / 64 µs / 11 µs | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 32 | 571 µs | 151 µs | 106 µs | 1.4× faster | 2 µs / 36 µs / 65 µs / 11 µs | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 32 | 9.09 ms | 1.21 ms | 111 µs | 10.9× faster | 2 µs / 37 µs / 72 µs / 12 µs | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 4096 × 32 | 146.65 ms | 18.86 ms | 194 µs | 97.2× faster | 2 µs / 51 µs / 141 µs / 78 µs | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 1 × 128 | 36 µs | 35 µs | 104 µs | 2.9× slower | 2 µs / 36 µs / 64 µs / 11 µs | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 16 × 128 | 555 µs | 107 µs | 103 µs | 1.0× faster | 2 µs / 36 µs / 64 µs / 12 µs | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 256 × 128 | 8.97 ms | 1.19 ms | 112 µs | 10.7× faster | 2 µs / 37 µs / 73 µs / 15 µs | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |
| 1024 × 128 | 36.36 ms | 4.68 ms | 116 µs | 40.2× faster | 2 µs / 39 µs / 74 µs / 29 µs | 1 on gpu, 0 on cpu, 1 passes, 0 downloads |

## chain

NURBS, Translate, a Sweep of a circle and Sample Curve, the control points move.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu: plan / record / wait / device | gpu work |
|---|---:|---:|---:|---:|---|---|
| 1 × 32 | 75 µs | 123 µs | 1.22 ms | 9.9× slower | 3 µs / 62 µs / 1.15 ms / 829 µs | 4 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 16 × 32 | 1.17 ms | 245 µs | 1.38 ms | 5.6× slower | 5 µs / 68 µs / 1.31 ms / 1.06 ms | 4 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 256 × 32 | 18.85 ms | 2.96 ms | 5.13 ms | 1.7× slower | 42 µs / 148 µs / 4.94 ms / 4.83 ms | 4 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 4096 × 32 | 311.45 ms | 45.53 ms | 21.64 ms | 2.1× faster | 910 µs / 20.23 ms / 486 µs / 248 µs | 3 on gpu, 1 on cpu, 3 passes, 0 downloads |
| 1 × 128 | 204 µs | 284 µs | 3.85 ms | 13.6× slower | 4 µs / 80 µs / 3.76 ms / 3.67 ms | 4 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 16 × 128 | 3.27 ms | 775 µs | 4.61 ms | 5.9× slower | 12 µs / 85 µs / 4.51 ms / 4.41 ms | 4 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 256 × 128 | 53.53 ms | 8.06 ms | 21.63 ms | 2.7× slower | 153 µs / 262 µs / 21.20 ms / 21.00 ms | 4 on gpu, 0 on cpu, 4 passes, 0 downloads |
| 1024 × 128 | 213.87 ms | 33.15 ms | 107.62 ms | 3.2× slower | 900 µs / 836 µs / 105.85 ms / 105.58 ms | 4 on gpu, 0 on cpu, 4 passes, 0 downloads |

## mesh

Mesh of a swept circle, the tolerance changes.

| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu: plan / record / wait / device | gpu work |
|---|---:|---:|---:|---:|---|---|
| 1 × 32 | 2.31 ms | 345 µs | 346 µs | 1.0× slower | 2 µs / 339 µs / 5 µs / 0 µs | 0 on gpu, 1 on cpu, 0 passes, 0 downloads |
| 16 × 32 | 37.56 ms | 5.81 ms | 5.97 ms | 1.0× slower | 3 µs / 5.95 ms / 9 µs / 0 µs | 0 on gpu, 1 on cpu, 0 passes, 0 downloads |
| 64 × 32 | 152.09 ms | 25.31 ms | 27.27 ms | 1.1× slower | 6 µs / 27.25 ms / 16 µs / 0 µs | 0 on gpu, 1 on cpu, 0 passes, 0 downloads |
