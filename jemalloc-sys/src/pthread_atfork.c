#include <stdint.h>

// These symbols are used by jemalloc on android but the really old android
// we're building on doesn't have them defined, so just make sure the symbols
// are available.
__attribute__((weak)) int
pthread_atfork(uint8_t *prefork __attribute__((unused)),
               uint8_t *postfork_parent __attribute__((unused)),
               uint8_t *postfork_child __attribute__((unused))) {
  return 0;
}
