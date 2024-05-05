/*
 * These symbols are used by jemalloc on android but the really old android
 * we're building on doesn't have them defined, so just make sure the symbols
 * are available.
 */
__attribute__((weak)) int
pthread_atfork(void (*prepare)(void) __attribute__((unused)),
               void (*parent)(void) __attribute__((unused)),
               void (*child)(void) __attribute__((unused))) {
  return 0;
}
