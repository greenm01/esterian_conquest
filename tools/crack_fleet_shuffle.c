/*
 * crack_fleet_shuffle.c — Exhaustive 2^32 seed search for ECMAINT fleet shuffle.
 *
 * Tests shuffle algorithms with TP7/full32 Random extraction against known
 * fleet visit orders from 4 scenarios.
 *
 * Borland Pascal LCG: seed = seed * 0x08088405 + 1
 * TP7 Random(Range): advance seed, then result = ((seed >> 16) * Range) >> 16
 *
 * Compile: cc -O3 -march=native -o crack_fleet_shuffle tools/crack_fleet_shuffle.c -lpthread
 * Run:     ./crack_fleet_shuffle [alg_id]
 *
 * Algorithm IDs (run specific one, or omit to list them):
 *   0  FY-reverse + TP7        4  simple-swap + TP7
 *   1  FY-forward + TP7        5  simple-swap-rev + TP7
 *   2  FY-reverse + full32     6  sort-by-key + TP7
 *   3  FY-forward + full32     7  sort-by-raw-seed
 *   8  simple-swap + full32    9  simple-swap-rev + full32
 *  10  sort-by-key + full32
 *  11  FY-reverse + TP7 (1-based, Random(i) not Random(i+1))
 *  12  FY-forward + TP7 (swap a[i] with a[Random(N)], not a[i+Random(N-i)])
 */

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>

#define N 16
#define LCG_MULT 0x08088405U

static inline uint32_t tp7_random(uint32_t *seed, uint32_t range) {
    *seed = *seed * LCG_MULT + 1;
    return ((uint32_t)((*seed >> 16) & 0xFFFF) * range) >> 16;
}

static inline uint32_t full32_random(uint32_t *seed, uint32_t range) {
    *seed = *seed * LCG_MULT + 1;
    return (uint32_t)(((uint64_t)*seed * (uint64_t)range) >> 32);
}

/* Known visit orders */
static const int bombard[N]      = {11,15,0,10,4,3,2,1,14,5,13,8,7,6,9,12};
static const int econ[N]         = {11,1,4,14,12,8,3,15,0,5,7,6,9,13,2,10};
static const int fleet_order[N]  = {6,3,7,1,2,9,13,12,4,5,0,15,10,14,11,8};
static const int planet_build[N] = {15,12,9,4,0,3,7,8,11,14,2,1,13,6,10,5};

typedef struct { const char *name; const int *target; } scenario_t;
static const scenario_t scenarios[] = {
    {"bombard", bombard}, {"econ", econ},
    {"fleet-order", fleet_order}, {"planet-build", planet_build},
};
#define N_SCENARIOS 4

typedef uint32_t (*random_fn)(uint32_t *seed, uint32_t range);

/* ---- Algorithm implementations ---- */

static int try_fy_reverse(uint32_t seed0, const int *target, random_fn rfn) {
    int a[N]; uint32_t s = seed0;
    for (int i = 0; i < N; i++) a[i] = i;
    for (int i = N-1; i >= 1; i--) {
        uint32_t j = rfn(&s, i+1);
        int tmp = a[i]; a[i] = a[j]; a[j] = tmp;
        if (a[i] != target[i]) return 0;
    }
    return a[0] == target[0];
}

static int try_fy_forward(uint32_t seed0, const int *target, random_fn rfn) {
    int a[N]; uint32_t s = seed0;
    for (int i = 0; i < N; i++) a[i] = i;
    for (int i = 0; i < N-1; i++) {
        uint32_t j = i + rfn(&s, N-i);
        int tmp = a[i]; a[i] = a[j]; a[j] = tmp;
        if (a[i] != target[i]) return 0;
    }
    return a[N-1] == target[N-1];
}

/* FY-reverse but Random(i) instead of Random(i+1) — off-by-one variant */
static int try_fy_reverse_obo(uint32_t seed0, const int *target, random_fn rfn) {
    int a[N]; uint32_t s = seed0;
    for (int i = 0; i < N; i++) a[i] = i;
    for (int i = N-1; i >= 1; i--) {
        uint32_t j = rfn(&s, i); /* Random(i) not Random(i+1) */
        int tmp = a[i]; a[i] = a[j]; a[j] = tmp;
        if (a[i] != target[i]) return 0;
    }
    return a[0] == target[0];
}

/* Forward loop but j = Random(N) instead of i + Random(N-i) */
static int try_forward_rn(uint32_t seed0, const int *target, random_fn rfn) {
    int a[N]; uint32_t s = seed0;
    for (int i = 0; i < N; i++) a[i] = i;
    for (int i = 0; i < N-1; i++) {
        uint32_t j = rfn(&s, N);
        int tmp = a[i]; a[i] = a[j]; a[j] = tmp;
        if (a[i] != target[i]) return 0;
    }
    return a[N-1] == target[N-1];
}

static int try_simple_swap(uint32_t seed0, const int *target, random_fn rfn) {
    int a[N]; uint32_t s = seed0;
    for (int i = 0; i < N; i++) a[i] = i;
    for (int i = 0; i < N; i++) {
        uint32_t j = rfn(&s, N);
        int tmp = a[i]; a[i] = a[j]; a[j] = tmp;
    }
    return memcmp(a, target, N * sizeof(int)) == 0;
}

static int try_simple_swap_rev(uint32_t seed0, const int *target, random_fn rfn) {
    int a[N]; uint32_t s = seed0;
    for (int i = 0; i < N; i++) a[i] = i;
    for (int i = N-1; i >= 0; i--) {
        uint32_t j = rfn(&s, N);
        int tmp = a[i]; a[i] = a[j]; a[j] = tmp;
    }
    return memcmp(a, target, N * sizeof(int)) == 0;
}

static int try_sort_by_key(uint32_t seed0, const int *target, random_fn rfn) {
    uint32_t keys[N]; int idx[N]; uint32_t s = seed0;
    for (int i = 0; i < N; i++) { keys[i] = rfn(&s, 65536); idx[i] = i; }
    for (int i = 1; i < N; i++) {
        uint32_t kk = keys[i]; int ii = idx[i]; int j = i - 1;
        while (j >= 0 && keys[j] > kk) { keys[j+1] = keys[j]; idx[j+1] = idx[j]; j--; }
        keys[j+1] = kk; idx[j+1] = ii;
    }
    return memcmp(idx, target, N * sizeof(int)) == 0;
}

static int try_sort_by_raw_seed(uint32_t seed0, const int *target, random_fn rfn) {
    (void)rfn;
    uint32_t keys[N]; int idx[N]; uint32_t s = seed0;
    for (int i = 0; i < N; i++) { s = s * LCG_MULT + 1; keys[i] = s; idx[i] = i; }
    for (int i = 1; i < N; i++) {
        uint32_t kk = keys[i]; int ii = idx[i]; int j = i - 1;
        while (j >= 0 && keys[j] > kk) { keys[j+1] = keys[j]; idx[j+1] = idx[j]; j--; }
        keys[j+1] = kk; idx[j+1] = ii;
    }
    return memcmp(idx, target, N * sizeof(int)) == 0;
}

/* ---- Search driver ---- */

typedef struct {
    int id;
    const char *label;
    int (*try_fn)(uint32_t, const int*, random_fn);
    random_fn rfn;
} search_job_t;

static const search_job_t jobs[] = {
    { 0, "FY-reverse+TP7",          try_fy_reverse,      tp7_random},
    { 1, "FY-forward+TP7",          try_fy_forward,      tp7_random},
    { 2, "FY-reverse+full32",       try_fy_reverse,      full32_random},
    { 3, "FY-forward+full32",       try_fy_forward,      full32_random},
    { 4, "simple-swap+TP7",         try_simple_swap,      tp7_random},
    { 5, "simple-swap-rev+TP7",     try_simple_swap_rev,  tp7_random},
    { 6, "sort-by-key+TP7",         try_sort_by_key,      tp7_random},
    { 7, "sort-by-raw-seed",        try_sort_by_raw_seed, NULL},
    { 8, "simple-swap+full32",      try_simple_swap,      full32_random},
    { 9, "simple-swap-rev+full32",  try_simple_swap_rev,  full32_random},
    {10, "sort-by-key+full32",      try_sort_by_key,      full32_random},
    {11, "FY-reverse-obo+TP7",      try_fy_reverse_obo,   tp7_random},
    {12, "forward-RN+TP7",          try_forward_rn,        tp7_random},
};
#define N_JOBS (sizeof(jobs)/sizeof(jobs[0]))

static void run_job(const search_job_t *job) {
    printf("[%2d] %s: searching 2^32 seeds x %d scenarios...\n", job->id, job->label, N_SCENARIOS);
    fflush(stdout);

    for (int sc = 0; sc < N_SCENARIOS; sc++) {
        const scenario_t *s = &scenarios[sc];
        int found = 0;
        for (uint64_t seed = 0; seed < 0x100000000ULL; seed++) {
            if (job->try_fn((uint32_t)seed, s->target, job->rfn)) {
                printf("[%2d] MATCH: %-12s seed=0x%08X  %s\n",
                       job->id, s->name, (uint32_t)seed, job->label);
                fflush(stdout);
                found++;
                if (found >= 3) break;
            }
        }
        if (!found)
            printf("[%2d] no match: %-12s %s\n", job->id, s->name, job->label);
        fflush(stdout);
    }
    printf("[%2d] DONE: %s\n", job->id, job->label);
    fflush(stdout);
}

int main(int argc, char **argv) {
    if (argc < 2) {
        printf("Usage: %s <job_id>\n\nJobs:\n", argv[0]);
        for (size_t i = 0; i < N_JOBS; i++)
            printf("  %2d  %s\n", jobs[i].id, jobs[i].label);
        return 0;
    }

    int id = atoi(argv[1]);
    for (size_t i = 0; i < N_JOBS; i++) {
        if (jobs[i].id == id) {
            run_job(&jobs[i]);
            return 0;
        }
    }
    fprintf(stderr, "Unknown job id: %d\n", id);
    return 1;
}
