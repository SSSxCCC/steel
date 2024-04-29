use proc_macro::TokenStream;
use quote::quote;
use syn;

pub fn impl_edit_macro_derive(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let gen = quote! {
        impl Edit for #name {
            fn name() -> &'static str { stringify!(#name) }
        }
    };
    gen.into()
}
