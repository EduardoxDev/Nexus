#ifndef BUFFER_H
#define BUFFER_H

#include <stddef.h>

typedef struct Buffer {
    char *data;
    size_t len;
} Buffer;

Buffer *buffer_new(size_t cap);
void buffer_free(Buffer *b);
size_t buffer_len(const Buffer *b);

#endif
