#define _GNU_SOURCE

#include <stdio.h>
#include <threads.h>
#include <unistd.h>

#include "count.h"

#define THREAD_COUNT 5

thread_local volatile int id;
thread_local volatile const int value = 50;

int procedure(void *name) {
  printf("[%d] Hello from %s\n", gettid(), (const char *)name);

  printf("[%d] TLS contain id=%d and value=%d\n", gettid(), id, value);
  id = gettid();
  printf("[%d] TLS contain id=%d and value=%d\n", gettid(), id, value);

  count++;
  printf("[%d] Local count is %d (after increment)\n", gettid(), count);

  return 0;
}

int main() {
  thrd_t threads[THREAD_COUNT];

  procedure("parent");

  for (int i = 0; i < THREAD_COUNT; i++) {

    int ret = thrd_create(&threads[i], &procedure, "child");

    if (ret != 0) {
      perror("Could not create thread");
      return 1;
    }
  }

  for (int i = 0; i < THREAD_COUNT; ++i) {
    thrd_join(threads[i], NULL);
  }

  return 0;
}