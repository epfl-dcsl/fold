use darling::ast::NestedMeta;
use darling::{Error, FromMeta};
use proc_macro::TokenStream;
use proc_macro_error::abort;
use quote::quote;
use syn::{FnArg, ItemFn, Path, Type};

#[derive(Default, FromMeta)]
#[darling(default)]
struct ChainAttributes {
    log: Option<Path>,
}

impl ChainAttributes {
    fn from_args(args: TokenStream) -> Result<Self, TokenStream> {
        let attr_args = NestedMeta::parse_meta_list(args.into())
            .map_err(|e| TokenStream::from(Error::from(e).write_errors()))?;

        ChainAttributes::from_list(&attr_args).map_err(|e| TokenStream::from(e.write_errors()))
    }
}

#[proc_macro_error::proc_macro_error]
#[proc_macro_attribute]
/// Creates an executable entrypoint running a `fold::Fold`.
///
/// The macro must be applied to a function that returns a `fold::Fold` and takes either:
/// - no parameters: the functions creates its own chain of modules from scratch
/// - one `fold::Fold` as argument: the generated entry point will feed the default System V module chain to the
/// function.
///
/// The returned `fold::Fold` will be ran on the target ELF file.
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
pub fn chain(args: TokenStream, mut item: TokenStream) -> TokenStream {
    let fun = item.clone();
    let fun = syn::parse_macro_input!(fun as ItemFn);

    let args = match ChainAttributes::from_args(args) {
        Ok(v) => v,
        Err(e) => return e,
    };

    let ident = fun.sig.ident;
    let log_level = args.log.unwrap_or(Path::from_string("Trace").unwrap());

    let chain = match fun.sig.inputs.len() {
        0 => quote! {#ident()},
        1 => quote! {#ident(fold::Fold::default_chain(env, env!("CARGO_BIN_NAME")))},
        _ => abort!(
            fun.sig.inputs,
            "function annotated with #[chain] must have 0 or 1 argument"
        ),
    };

    if let Some(FnArg::Typed(input)) = fun.sig.inputs.first() {
        match input.ty.as_ref() {
            Type::Path(path) if path.path.get_ident().is_some_and(|i| i == "Fold") => {}
            _ => abort!(
                fun.sig.inputs,
                "function annotated with #[chain] take a `fold::Fold` as only argument, if any"
            ),
        }
    }

    let entry: TokenStream = quote! {
        fold::entry!(entry);

        fn entry(env: fold::Env) -> ! {
            fold::logging::init(fold::log::LevelFilter::#log_level);

            #chain.run();

            fold::exit(fold::Exit::Success)
        }
    }
    .into();

    item.extend(entry);
    item
}
