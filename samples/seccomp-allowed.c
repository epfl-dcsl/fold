#include <unistd.h>

int main() {

  // write syscall - allowed
  syscall(0x01, 1, "hi there\n", 10);
  
  // exit syscall - allowed
  syscall(0x3c, 0);
}

