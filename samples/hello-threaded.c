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

  printf("[%d] TLS stores id at %p and value at %p\n", gettid(), &id, &value);

  id = gettid();
  printf("[%d] These values contain %d and %d\n", gettid(), id, value);

  printf("[%d] Count is stored at %p\n", gettid(), &count);

  count++;
  printf("[%d] Count is %d\n", gettid(), count);

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