use proc_macro::TokenStream;
use quote::quote;
use syn;

pub fn impl_edit_macro_derive(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let name_fn = quote! {
        fn name() -> &'static str { stringify!(#name) }
    };

    let fields = match &ast.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(fields) => {
                (&fields.named).iter().collect::<Vec<_>>()
            },
            syn::Fields::Unnamed(_) => todo!(),
            syn::Fields::Unit => Vec::new(),
        },
        syn::Data::Enum(_) => panic!("Not yet supported Edit derive macro in Enum"),
        syn::Data::Union(_) => panic!("Not yet supported Edit derive macro in Union"),
    };
    let (value_types, field_idents): (Vec<_>, Vec<_>) = fields.into_iter().filter_map(|field| {
        let field_ident = field.ident.as_ref().unwrap();
        let value_type = match &field.ty {
            syn::Type::Path(type_path) => {
                let type_last_ident = &type_path.path.segments.last().expect(format!("No type path segment for field {field_ident:?}").as_str()).ident;
                match type_last_ident.to_string().as_str() {
                    "bool" => quote! { Value::Bool },
                    "i32" => quote! { Value::Int32 },
                    "f32" => quote! { Value::Float32 },
                    "String" => quote! { Value::String },
                    "Vec2" => quote! { Value::Vec2 },
                    "Vec3" => quote! { Value::Vec3 },
                    "Vec4" => quote! { Value::Vec4 },
                    _ => return None,
                }
            },
            _ => return None,
        };
        Some((value_type, field_ident))
    }).unzip();

    let insert_values = std::iter::zip(&value_types, &field_idents)
        .map(|(value_type, field_ident)| quote! { #value_type (self.#field_ident.clone()) })
        .collect::<Vec<_>>();
    let get_data_fn = quote! {
        fn get_data(&self) -> Data {
            Data::new() #( .insert(stringify!(#field_idents), #insert_values) )*
        }
    };

    let match_values = value_types.iter()
        .map(|value_type| quote! { #value_type (v) })
        .collect::<Vec<_>>();
    let set_data_fn = quote! {
        fn set_data(&mut self, data: &Data) {
            #( if let Some(#match_values) = data.get(stringify!(#field_idents)) { self.#field_idents = v.clone() } )*
        }
    };

    let impl_edit_for = quote! {
        impl Edit for #name {
            #name_fn
            #get_data_fn
            #set_data_fn
        }
    };
    impl_edit_for.into()
}
