#include <unistd.h>

int main() {

  // fork syscall - forbidden
  syscall(0x39);
  
  // exit syscall - allowed
  syscall(0x3c, 0);
}

