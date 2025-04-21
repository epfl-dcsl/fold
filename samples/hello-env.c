#include <stdio.h>
#include <stdlib.h>

int main() {
  const char* name = getenv("NAME");
  if (name == NULL){
    puts("Missing name :/");
    return 1;
  }

  printf("Hello %s !\n", name);
  return 0;
}

