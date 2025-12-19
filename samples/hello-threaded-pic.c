#define _GNU_SOURCE

#include <stdio.h>
#include <threads.h>
#include <unistd.h>

#include "count-pic.h"

#define THREAD_COUNT 5

thread_local volatile int id;
thread_local volatile const int value = 50;

int procedure(void *name) {
  // printf("Hello from *%p with value *%p\n", &id, &value);
  id = gettid();
  printf("Hello from %s (%d) with value %d (count = %d)\n", (const char *)name,
         id, value, incr());
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

  sleep(1);
  return 0;
}