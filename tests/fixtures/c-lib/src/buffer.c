#include <stdlib.h>
#include "buffer.h"
#include "util/log.h"

#define DEFAULT_CAP 16

static size_t grow(size_t cap) {
    return cap < DEFAULT_CAP ? DEFAULT_CAP : cap * 2;
}

Buffer *buffer_new(size_t cap) {
    Buffer *b = malloc(sizeof(Buffer));
    if (b == NULL) {
        log_error("sem memoria");
        return NULL;
    }
    b->data = malloc(grow(cap));
    if (b->data == NULL) {
        free(b);
        log_error("sem memoria");
        return NULL;
    }
    b->len = 0;
    return b;
}

void buffer_free(Buffer *b) {
    free(b->data);
    free(b);
}

size_t buffer_len(const Buffer *b) { return b->len; }
