use core::panic;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::Item;

#[proc_macro_attribute]
pub fn hook(_attr: TokenStream, mut item: TokenStream) -> TokenStream {
    let Ok(Item::Fn(fun)) = syn::parse::<Item>(item.clone()) else {
        panic!("Macro can only be used on function");
    };

    let ident = fun.sig.ident;
    let trampoline_ident = format_ident!("__{}_trampoline", ident);

    let trampoline: TokenStream = quote! {
        fn #trampoline_ident() {
            unsafe {
                ::core::arch::asm!(
                    // Save the resolved address of the symbol in the stack. The actual value written must be changed by the linker.
                    "mov rax,{}",
                    "mov [rsp],rax",
                    // Stores all the registers potentially containing arguments on the stack. All other temporary registers are not
                    // used across the call by the trampoline and thus do not need to be saved.
                    "push rcx",
                    "push rdx",
                    "push rsi",
                    "push rdi",
                    "push r8",
                    "push r9",
                    // Call the hook
                    "call {}",
                    // Pops the arguments back into the corresponding registers.
                    "pop r9",
                    "pop r8",
                    "pop rdi",
                    "pop rsi",
                    "pop rdx",
                    "pop rcx",
                    // Recovers the actual symbol to jump to, and jump there will passing the return address of the current function
                    // frame to the callee.
                    "pop rbx",
                    "mov rax,[rsp]",
                    "jmp rbx",
                    const 0xdeadbeef,
                    sym #ident
                );
            }
        }
    }
    .into();

    item.extend(trampoline);
    item
}
