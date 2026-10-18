//! The [steel game engine](https://github.com/SSSxCCC/steel) proc-macro library.

mod edit;
mod steel_main;

use proc_macro::TokenStream;
use syn;

#[proc_macro_derive(Edit, attributes(edit))]
pub fn edit_macro_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse(input).unwrap();
    crate::edit::impl_edit_macro_derive(&ast).into()
}

/// Generates the required main function boilerplate for Android.
#[proc_macro_attribute]
pub fn steel_main(attr: TokenStream, item: TokenStream) -> TokenStream {
    steel_main::steel_main(attr, item)
}
