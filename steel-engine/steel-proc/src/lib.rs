mod edit;

use proc_macro::TokenStream;
use syn;

#[proc_macro_derive(Edit)]
pub fn edit_macro_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    crate::edit::impl_edit_macro_derive(&ast)
}
