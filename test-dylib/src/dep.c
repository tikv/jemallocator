#define _GNU_SOURCE
#include <stdlib.h>
#include <stdio.h>
#include <dlfcn.h>

const char* lookup_malloc_address(void) {
    Dl_info info;
    if (!dladdr((void *)malloc, &info)) {
        printf("failed finding `malloc`\n");
        abort();
    }
    return info.dli_fname;
}
