#![feature(proc_macro_quote)]
extern crate proc_macro;
use proc_macro::TokenStream;
use syn::{parse_macro_input, ItemFn, Visibility, FnArg, Pat, PatIdent};
use quote::{quote, format_ident};
use syn::punctuated::Punctuated;
use itertools::Itertools;
use syn::token::Comma;


#[proc_macro_attribute]
pub fn make_gate(args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemFn);
    let mut initializer = input.clone();
    let generics = input.sig.generics;
    let output = input.sig.output;
    let fn_name_ident = input.sig.ident;
    let init_fn_name_ident = format_ident!("_init_{}", fn_name_ident);
    initializer.sig.ident = init_fn_name_ident.clone();
    initializer.vis = Visibility::Inherited;

    let mut inputs_forward: Punctuated<PatIdent, Comma> = Punctuated::new();
    let fn_inputs = input.sig.inputs;
    fn_inputs.iter().map(|arg| {
        match arg {
            FnArg::Receiver(_) => unreachable!("Reciever type is not supported in make_gate"),
            FnArg::Typed(pt) => match (*pt.pat).clone() {
                Pat::Ident(id) => {inputs_forward.push(id)},
                _ => {unreachable!("Unsuported pattern type")}
            }
        };
    }).last();

    let generic_params = inputs_forward.iter().map(|pat_ident| format!("{} = {{}}", pat_ident.ident.to_string())).join(", ");
    let qual_token = format!("{}::<{}>", fn_name_ident.to_string(), generic_params);

    quote! {
        #initializer

        pub fn #fn_name_ident #generics(#fn_inputs) -> (Box<dyn Fn(&FrozenMap<String, Box<Gatebb<'c, F>>>) #output>) {
            Box::new(move |fm: &FrozenMap<String, Box<Gatebb<'c, F>>>|  {
                let qual_name = [module_path!().to_string(), format!(#qual_token, #inputs_forward)].join("::");
                if fm.get(&qual_name).is_none() {
                    fm.insert(qual_name.clone(), Box::new(#init_fn_name_ident(#inputs_forward)));
                }
                return fm.get(&qual_name).unwrap().clone()
            })
        }
    }.into()
}