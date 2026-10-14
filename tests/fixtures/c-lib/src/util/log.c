#include <stdio.h>
#include "log.h"

void log_error(const char *msg) {
    fprintf(stderr, "erro: %s\n", msg);
}
