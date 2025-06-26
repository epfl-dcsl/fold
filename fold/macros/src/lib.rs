use core::panic;

use proc_macro::TokenStream;
use quote::quote;
use syn::Item;

#[proc_macro_attribute]
/// Creates an executable entrypoint running a `fold::Fold`. The macro must be applied to a function taking and
/// returning a `Fold`.
///
/// The macro generates an entrypoint that feeds System V's default module chain to the function and uses its return
/// `Fold` to process the executable's object file.
///
/// # Example
///
/// ```
/// #[chain]
/// // Creates a linker using only default System V modules.
/// fn chain(chain: Fold) -> Fold {
///     chain
/// }
/// ```
pub fn chain(_attr: TokenStream, mut item: TokenStream) -> TokenStream {
    let Ok(Item::Fn(fun)) = syn::parse::<Item>(item.clone()) else {
        panic!("Macro can only be used on function");
    };

    let ident = fun.sig.ident;

    let entry: TokenStream = quote! {
        fold::entry!(entry);

        fn entry(env: fold::Env) -> ! {
            fold::logging::init(log::LevelFilter::Trace);

            #ident(fold::default_chain(env, env!("CARGO_BIN_NAME"))).run();

            fold::exit(fold::Exit::Success)
        }
    }
    .into();

    item.extend(entry);
    item
}
