file target/x86_64-unknown-linux-none/debug/fold
add-symbol-file sqlite-build/sqlite3 -o 0x7fffeef36000
break fold::sysv::start::jmp
run sqlite-build/sqlite3
break main
disable 1
continue