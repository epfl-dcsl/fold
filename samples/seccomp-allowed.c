#include <unistd.h>

int main() {

  // write syscall - allowed
  syscall(0x01, 1, "Hello world!\n", 13);
  
  // exit syscall - allowed
  syscall(0x3c, 0);
}

