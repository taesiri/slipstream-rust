#ifdef _WINDOWS
#define WIN32_LEAN_AND_MEAN
#include <Winsock2.h>
#include <Windows.h>

#include "wincompat.h"

int wintimeofday(struct timeval* tv, struct timezone* tz) {
    if (tv) {
        FILETIME ft;
        ULARGE_INTEGER uli;
        GetSystemTimeAsFileTime(&ft);
        uli.LowPart = ft.dwLowDateTime;
        uli.HighPart = ft.dwHighDateTime;
        const unsigned long long epoch = 116444736000000000ULL;
        unsigned long long time = uli.QuadPart - epoch;
        tv->tv_sec = (long)(time / 10000000ULL);
        tv->tv_usec = (long)((time % 10000000ULL) / 10);
    }
    if (tz) {
        tz->tz_minuteswest = 0;
        tz->tz_dsttime = 0;
    }
    return 0;
}
#endif
