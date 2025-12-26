#include <threads.h>

thread_local volatile int count = 0;

int incr() { return ++count; }
