# Notes about implementing TLS

Each object may come with a segment annotated with `PT_TLS`, indicating the presence of a TLS module. These modules are assigned IDs at load-time, with the exception of the executable's module whose ID is fixed to 1.

The linker needs to expose a `__tls_get_addr` function to allow the different code section to access the variables in the TLS modules.

TLS modules may be allocated either statically at load-time or dynamically when `__tls_get_addr` is called. Whether modules are static or dynamic can be checked by looking at which relocations use the module, as well as the dynamic table present in the same object. Any module may be statically loaded, while some cannot be dynamically loaded; this can be determined by looking at the `DT_FLAGS` entry of the Dynamic Table, specifically for the `DF_STATIC_TLS` flag and by checking which modules are accessed through the `R_X86_64_TPOFF{32,64}` relocations; both cases indicating that the module must be statically loaded.

The memory layout of the TLS blocks is shown in the figure below, where each `tlsoffset` marks the start of a static block, and `dtv` is a vector storing the address of the start of each TLS Module.

![TLS Modules](../v0.1.0/tls-layout.png)

For

<!-- AFAIU, rules for choosing between static/dynamic module allocations are:

- The executable is static
- SO which exposes variables without accessing them directly are static. This can be observed by the absence of any `__tls_get_addr` relocations.
- Other SO are dynamic.

TODO: initial-exec optimization -->

## Current status:

The stack is growing into the TLS, overwriting stack canary and messing everything up.

## Refs

https://maskray.me/blog/2021-11-07-init-ctors-init-array
