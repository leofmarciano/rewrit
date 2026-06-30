//! Proc macros for the Rewrit Rust SDK.

#![forbid(unsafe_code)]

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, LitStr};

#[proc_macro_attribute]
pub fn case(args: TokenStream, input: TokenStream) -> TokenStream {
    let case_id = parse_macro_input!(args as LitStr);
    let mut function = parse_macro_input!(input as ItemFn);
    let body = function.block;

    function.block = Box::new(syn::parse_quote!({
        rewrit::cargo_test_case(#case_id)
            .expect("failed to emit Rewrit case discovery");
        #body
    }));

    TokenStream::from(quote!(#function))
}
