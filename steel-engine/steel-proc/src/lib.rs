//! The [steel game engine](https://github.com/SSSxCCC/steel) proc-macro library.

mod edit;

use syn;

#[proc_macro_derive(Edit, attributes(edit))]
pub fn edit_macro_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse(input).unwrap();
    crate::edit::impl_edit_macro_derive(&ast).into()
}
