#include <stdio.h>

int main(int argc, char** argv) {
  if (argc != 2){
    puts("Missing name :/");
    return 1;
  }

  printf("Hello %s !", argv[1]);
  return 0;
}

