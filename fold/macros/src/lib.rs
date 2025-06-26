use core::panic;

use proc_macro::TokenStream;
use quote::quote;
use syn::Item;

#[proc_macro_attribute]
pub fn chain(_attr: TokenStream, mut item: TokenStream) -> TokenStream {
    let Ok(Item::Fn(fun)) = syn::parse::<Item>(item.clone()) else {
        panic!("Macro can only be used on function");
    };

    let ident = fun.sig.ident;

    let entry: TokenStream = quote! {
        fold::entry!(entry);

        fn entry(env: fold::Env) -> ! {
            fold::logging::init(log::LevelFilter::Trace);

            #ident(fold::default_chain(env!("CARGO_BIN_NAME"), env)).run();

            fold::exit(fold::Exit::Success)
        }
    }
    .into();

    item.extend(entry);
    item
}
