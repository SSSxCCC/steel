use proc_macro2::TokenStream;
use quote::quote;
use std::{iter::zip, str::FromStr};
use syn;

pub fn impl_edit_macro_derive(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let name_fn = quote! {
        fn name() -> &'static str { stringify!(#name) }
    };

    let fields = match &ast.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(fields) => (&fields.named).iter().collect::<Vec<_>>(),
            syn::Fields::Unnamed(_) => todo!(),
            syn::Fields::Unit => Vec::new(),
        },
        syn::Data::Enum(_) => panic!("Not yet supported Edit derive macro in Enum"),
        syn::Data::Union(_) => panic!("Not yet supported Edit derive macro in Union"),
    };
    let (field_idents, (value_types, (value_names, value_limits))): (Vec<_>, (Vec<_>, (Vec<_>, Vec<_>))) = fields.into_iter().filter_map(|field| {
        let field_ident = field.ident.as_ref().unwrap();

        let value_type = match &field.ty {
            syn::Type::Path(type_path) => {
                let type_last_segment = type_path.path.segments.last()
                    .expect(format!("No type path segment for field {field_ident:?}").as_str());
                let type_last_ident = &type_last_segment.ident;
                match type_last_ident.to_string().as_str() {
                    "bool" => quote! { Value::Bool },
                    "i32" => quote! { Value::Int32 },
                    "u32" => quote! { Value::UInt32 },
                    "f32" => quote! { Value::Float32 },
                    "String" => quote! { Value::String },
                    "EntityId" => quote! { Value::Entity },
                    "Vec2" => quote! { Value::Vec2 },
                    "Vec3" => quote! { Value::Vec3 },
                    "Vec4" => quote! { Value::Vec4 },
                    "IVec2" => quote! { Value::IVec2 },
                    "IVec3" => quote! { Value::IVec3 },
                    "IVec4" => quote! { Value::IVec4 },
                    "UVec2" => quote! { Value::UVec2 },
                    "UVec3" => quote! { Value::UVec3 },
                    "UVec4" => quote! { Value::UVec4 },
                    "AssetId" => quote! { Value::Asset },
                    "Vec" => {
                        let generic_arg = match &type_last_segment.arguments {
                            syn::PathArguments::AngleBracketed(generic_arguments) => generic_arguments.args.first().unwrap(),
                            _ => return None,
                        };
                        match generic_arg {
                            syn::GenericArgument::Type(syn::Type::Path(generic_path)) => {
                                let generic_type_last_ident = &generic_path.path.segments.last()
                                    .expect(format!("No type path segment for field {field_ident:?}").as_str()).ident;
                                match generic_type_last_ident.to_string().as_str() {
                                    "EntityId" => quote! { Value::VecEntity },
                                    _ => return None,
                                }
                            }
                            _ => return None,
                        }
                    }
                    _ => return None,
                }
            }
            _ => return None,
        };

        let mut value_name = field_ident.to_string();
        let mut value_limit = None;
        field.attrs.iter().for_each(|attr| {
            if attr.path().is_ident("edit") {
                if let syn::Meta::List(meta) = &attr.meta {
                    if let Err(err) = meta.parse_nested_meta(|meta| {
                        if meta.path.is_ident("name") {
                            if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(lit), .. }) = meta.value()?.parse()? {
                                value_name = lit.value();
                            } else {
                                return Err(meta.error("name must be a string literal"));
                            }
                        } else if meta.path.is_ident("limit") {
                            if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(lit), .. }) = meta.value()?.parse()? {
                                value_limit = Some(TokenStream::from_str(&lit.value())?);
                            } else {
                                return Err(meta.error("limit must be a string literal"));
                            }
                        } else {
                            return Err(meta.error("unsupported edit property"));
                        }
                        Ok(())
                    }) {
                        panic!("source={:?}, error={}", err.span().source_text(), err.to_string());
                    }
                } else {
                    panic!("edit attribute content should be key value pair list, example: #[edit(limit = \"Limit::ReadOnly\", name = \"foo\")]");
                }
            }
        });

        Some((field_ident, (value_type, (value_name, value_limit))))
    }).unzip();

    let insert_values = zip(&value_types, &field_idents)
        .map(|(value_type, field_ident)| quote! { #value_type (self.#field_ident.clone()) })
        .collect::<Vec<_>>();
    let insert_tokens = zip(&value_names, zip(insert_values, &value_limits))
        .map(|(value_name, (insert_value, value_limit))| {
            if let Some(limit) = value_limit {
                quote! { .insert_with_limit(#value_name, #insert_value, #limit) }
            } else {
                quote! { .insert(#value_name, #insert_value) }
            }
        })
        .collect::<Vec<_>>();
    let get_data_fn = quote! {
        fn get_data(&self) -> Data {
            Data::new() #( #insert_tokens )*
        }
    };

    let set_datas = zip(value_limits, zip(value_types, zip(value_names, field_idents)))
        .filter(|(value_limit, _)| !value_limit.as_ref().is_some_and(|limit| limit.to_string().contains("ReadOnly")))
        .map(|(_, (value_type, (value_name, field_ident)))| quote! { if let Some(#value_type (v)) = data.get(#value_name) { self.#field_ident = v.clone() } })
        .collect::<Vec<_>>();
    let set_data_fn = quote! {
        fn set_data(&mut self, data: &Data) {
            #( #set_datas )*
        }
    };

    quote! {
        impl Edit for #name {
            #name_fn
            #get_data_fn
            #set_data_fn
        }
    }
}
