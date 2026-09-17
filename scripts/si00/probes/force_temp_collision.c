/* SI00 diagnostic ONLY. Linux/glibc subprocess-local interposition.
 * Preserve monotonic time and real filesystem APIs. Freeze only REALTIME used
 * for legacy temp names; hold the first N CAS temporary opens until all N own
 * open descriptors. A timeout is INVALID instrumentation, never a causal kill.
 */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <errno.h>
#include <fcntl.h>
#include <pthread.h>
#include <stdarg.h>
#include <stdatomic.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <time.h>
#include <unistd.h>

static int (*next_open)(const char *, int, ...);
static int (*next_open64)(const char *, int, ...);
static pthread_mutex_t mu = PTHREAD_MUTEX_INITIALIZER;
static pthread_cond_t cv;
static unsigned target = 4, arrived;
static int timed_out;
static _Atomic unsigned clocks;

__attribute__((constructor)) static void init_probe(void) {
    next_open = dlsym(RTLD_NEXT, "open");
    next_open64 = dlsym(RTLD_NEXT, "open64");
    if (!next_open || !next_open64) _exit(120);
    const char *s = getenv("SI00_COLLISION_PARTICIPANTS");
    if (s && strcmp(s, "1") == 0) target = 1;
    else if (!s || strcmp(s, "4") != 0) _exit(121);
    pthread_condattr_t attr;
    pthread_condattr_init(&attr);
    pthread_condattr_setclock(&attr, CLOCK_MONOTONIC);
    pthread_cond_init(&cv, &attr);
    pthread_condattr_destroy(&attr);
}

int clock_gettime(clockid_t clock, struct timespec *value) {
    if (clock == CLOCK_REALTIME) {
        value->tv_sec = 1700000000;
        value->tv_nsec = 123456789;
        atomic_fetch_add_explicit(&clocks, 1, memory_order_relaxed);
        return 0;
    }
    return syscall(SYS_clock_gettime, clock, value);
}

static void opened(const char *path, int flags, int fd) {
    if (fd < 0 || !(flags & O_CREAT) || !strstr(path, "/objects/") || !strstr(path, "/.tmp-")) return;
    int saved = errno;
    pthread_mutex_lock(&mu);
    if (arrived < target) {
        struct stat st;
        if (fstat(fd, &st)) _exit(122);
        arrived++;
        dprintf(STDERR_FILENO, "SI00_OPEN %u/%u tid=%ld inode=%lu path=%s\n", arrived, target,
                syscall(SYS_gettid), (unsigned long)st.st_ino, path);
        if (arrived == target) pthread_cond_broadcast(&cv);
        struct timespec deadline;
        syscall(SYS_clock_gettime, CLOCK_MONOTONIC, &deadline);
        deadline.tv_sec += 5;
        while (arrived < target && !timed_out) {
            if (pthread_cond_timedwait(&cv, &mu, &deadline) == ETIMEDOUT) {
                timed_out = 1;
                dprintf(STDERR_FILENO, "SI00_BARRIER_TIMEOUT INVALID\n");
                pthread_cond_broadcast(&cv);
            }
        }
    }
    pthread_mutex_unlock(&mu);
    errno = saved;
}

int open(const char *path, int flags, ...) {
    mode_t mode = 0;
    if ((flags & O_CREAT) || (flags & O_TMPFILE) == O_TMPFILE) {
        va_list ap; va_start(ap, flags); mode = va_arg(ap, mode_t); va_end(ap);
    }
    int fd = next_open(path, flags, mode);
    opened(path, flags, fd);
    return fd;
}

int open64(const char *path, int flags, ...) {
    mode_t mode = 0;
    if ((flags & O_CREAT) || (flags & O_TMPFILE) == O_TMPFILE) {
        va_list ap; va_start(ap, flags); mode = va_arg(ap, mode_t); va_end(ap);
    }
    int fd = next_open64(path, flags, mode);
    opened(path, flags, fd);
    return fd;
}

__attribute__((destructor)) static void end_probe(void) {
    dprintf(STDERR_FILENO, "SI00_END arrived=%u target=%u clocks=%u timeout=%d\n",
            arrived, target, atomic_load_explicit(&clocks, memory_order_relaxed), timed_out);
}
