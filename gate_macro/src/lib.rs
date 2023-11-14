#![feature(proc_macro_quote)]
extern crate proc_macro;
use proc_macro::TokenStream;
use syn::{parse_macro_input, ItemFn, Visibility};
use quote::{quote, format_ident};

#[proc_macro_attribute]
pub fn make_gate(args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemFn);
    let mut initializer = input.clone();
    let fn_name_ident = input.sig.ident;
    let init_fn_name_ident = format_ident!("_init_{}", fn_name_ident);
    initializer.sig.ident = init_fn_name_ident.clone();
    initializer.vis = Visibility::Inherited;

    quote! {
        #initializer

        pub fn #fn_name_ident<'circuit, F: PrimeField + FieldUtils>(fm: &FrozenMap<String, Box<Gatebb<'circuit, F>>>) -> Gatebb<'circuit, F> {
            let qual_name = [module_path!(), "::", stringify!(#fn_name_ident)].join("");
            if fm.get(&qual_name).is_none() {
                fm.insert(qual_name.clone(), Box::new(_init_nonzero_check()));
            }
            return fm.get(&qual_name).unwrap().clone()
        }
    }.into()
}