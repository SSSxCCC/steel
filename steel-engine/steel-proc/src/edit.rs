use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
use syn::{self, Index, Type};

const BASIC_TYPES: &[&str] = &[
    "bool", "i32", "i64", "u32", "u64", "f32", "f64", "String", "Vec2", "Vec3", "Vec4", "IVec2",
    "IVec3", "IVec4", "UVec2", "UVec3", "UVec4", "EntityId", "AssetId", "Data",
];

pub fn impl_edit_macro_derive(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;

    // generate name() implementation
    let name_fn = quote! {
        fn name() -> &'static str {
            stringify!(#name)
        }
    };

    // extract struct fields
    let fields = match &ast.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(fields) => fields.named.iter().collect(),
            syn::Fields::Unnamed(fields) => fields.unnamed.iter().collect(),
            syn::Fields::Unit => Vec::new(),
        },
        _ => panic!("Edit derive only supports structs"),
    };

    // process each field
    let (get_data_lines, set_data_lines): (Vec<TokenStream>, Vec<TokenStream>) = fields
        .iter()
        .enumerate()
        .filter_map(|(i, field)| {
            // determine field accessor
            let field_accessor = match &field.ident {
                Some(ident) => FieldAccessor::Ident(ident.clone()),
                None => FieldAccessor::Index(Index::from(i)),
            };

            // parse field attributes
            let mut value_name = field_accessor.to_string();
            let mut value_limit = None;

            // process #[edit(...)] attributes
            let mut ignore = false;
            for attr in &field.attrs {
                if attr.path().is_ident("edit") {
                    attr.parse_nested_meta(|meta| {
                        if meta.path.is_ident("name") {
                            if let syn::Expr::Lit(syn::ExprLit {
                                lit: syn::Lit::Str(lit),
                                ..
                            }) = meta.value()?.parse()?
                            {
                                value_name = lit.value();
                            }
                        } else if meta.path.is_ident("limit") {
                            if let syn::Expr::Lit(syn::ExprLit {
                                lit: syn::Lit::Str(lit),
                                ..
                            }) = meta.value()?.parse()?
                            {
                                value_limit = Some(syn::parse_str::<syn::Expr>(&lit.value())?);
                            }
                        } else if meta.path.is_ident("ignore") {
                            ignore = true
                        }
                        Ok(())
                    })
                    .unwrap();
                    if ignore {
                        return None;
                    }
                }
            }

            // handle Vec types and nested structures
            match &field.ty {
                Type::Path(type_path) => {
                    let last_segment = type_path.path.segments.last().unwrap();
                    let type_name = last_segment.ident.to_string();

                    // check for Vec<T> types
                    if type_name == "Vec" {
                        if let syn::PathArguments::AngleBracketed(args) = &last_segment.arguments {
                            if let Some(syn::GenericArgument::Type(element_type)) =
                                args.args.first()
                            {
                                return handle_vec_type(
                                    &field_accessor,
                                    &value_name,
                                    &value_limit,
                                    element_type,
                                );
                            }
                        }
                    }

                    // handle basic types
                    if BASIC_TYPES.contains(&type_name.as_str()) {
                        return handle_basic_type(
                            &field_accessor,
                            &value_name,
                            &value_limit,
                            &type_name,
                        );
                    }
                }
                _ => {}
            }

            // handle nested structures
            Some(handle_nested_data(
                &field_accessor,
                &value_name,
                &value_limit,
            ))
        })
        .unzip();

    // assemble final implementation
    let expanded = quote! {
        impl Edit for #name {
            #name_fn

            fn get_data(&self, data: &mut Data) {
                #(#get_data_lines)*
            }

            fn set_data(&mut self, data: &Data) {
                #(#set_data_lines)*
            }
        }
    };

    expanded.into()
}

fn handle_vec_type(
    field_accessor: &FieldAccessor,
    value_name: &str,
    value_limit: &Option<syn::Expr>,
    element_type: &Type,
) -> Option<(TokenStream, TokenStream)> {
    // get inner type name
    let element_type_name = if let Type::Path(type_path) = element_type {
        type_path.path.segments.last()?.ident.to_string()
    } else {
        return None;
    };

    // generate Value type and conversion
    let (value_type, is_basic) = match element_type_name.as_str() {
        "bool" => (quote! { Value::VecBool }, true),
        "i32" => (quote! { Value::VecInt32 }, true),
        "i64" => (quote! { Value::VecInt64 }, true),
        "u32" => (quote! { Value::VecUInt32 }, true),
        "u64" => (quote! { Value::VecUInt64 }, true),
        "f32" => (quote! { Value::VecFloat32 }, true),
        "f64" => (quote! { Value::VecFloat64 }, true),
        "String" => (quote! { Value::VecString }, true),
        "EntityId" => (quote! { Value::VecEntity }, true),
        "AssetId" => (quote! { Value::VecAsset }, true),
        _ => (quote! { Value::VecData }, false),
    };

    if is_basic {
        // handle basic vector types
        let get_line = if let Some(limit) = value_limit {
            quote! {
                data.insert_with_limit(#value_name, #value_type(self.#field_accessor.clone()), #limit);
            }
        } else {
            quote! {
                data.insert(#value_name, #value_type(self.#field_accessor.clone()));
            }
        };

        let set_line = if !is_read_only(value_limit) {
            quote! {
                if let Some(#value_type(v)) = data.get(#value_name) {
                    self.#field_accessor = v.clone();
                }
            }
        } else {
            quote! {}
        };

        Some((get_line, set_line))
    } else {
        // handle nested vector types (Vec<MyStruct>)
        let get_line = quote! {
            let nested_vec_data = self.#field_accessor.iter()
                .map(|item| item.to_data())
                .collect();
        };
        let get_line = if let Some(limit) = value_limit {
            quote! {
                #get_line
                data.insert_with_limit(#value_name, Value::VecData(nested_vec_data), #limit);
            }
        } else {
            quote! {
                #get_line
                data.insert(#value_name, Value::VecData(nested_vec_data));
            }
        };

        let set_line = if !is_read_only(value_limit) {
            quote! {
                if let Some(#value_type(v)) = data.get(#value_name) {
                    self.#field_accessor = v.iter()
                        .map(|nested_data| Edit::from_data(nested_data))
                        .collect();
                }
            }
        } else {
            quote! {}
        };

        Some((get_line, set_line))
    }
}

fn handle_basic_type(
    field_accessor: &FieldAccessor,
    value_name: &str,
    value_limit: &Option<syn::Expr>,
    type_name: &str,
) -> Option<(TokenStream, TokenStream)> {
    let value_type = match type_name {
        "bool" => quote! { Value::Bool },
        "i32" => quote! { Value::Int32 },
        "i64" => quote! { Value::Int64 },
        "u32" => quote! { Value::UInt32 },
        "u64" => quote! { Value::UInt64 },
        "f32" => quote! { Value::Float32 },
        "f64" => quote! { Value::Float64 },
        "String" => quote! { Value::String },
        "Vec2" => quote! { Value::Vec2 },
        "Vec3" => quote! { Value::Vec3 },
        "Vec4" => quote! { Value::Vec4 },
        "IVec2" => quote! { Value::IVec2 },
        "IVec3" => quote! { Value::IVec3 },
        "IVec4" => quote! { Value::IVec4 },
        "UVec2" => quote! { Value::UVec2 },
        "UVec3" => quote! { Value::UVec3 },
        "UVec4" => quote! { Value::UVec4 },
        "EntityId" => quote! { Value::Entity },
        "AssetId" => quote! { Value::Asset },
        "Data" => quote! { Value::Data },
        _ => return None,
    };

    let get_line = if let Some(limit) = value_limit {
        quote! {
            data.insert_with_limit(#value_name, #value_type(self.#field_accessor.clone()), #limit);
        }
    } else {
        quote! {
            data.insert(#value_name, #value_type(self.#field_accessor.clone()));
        }
    };

    let set_line = if !is_read_only(value_limit) {
        quote! {
            if let Some(#value_type(v)) = data.get(#value_name) {
                self.#field_accessor = v.clone();
            }
        }
    } else {
        quote! {}
    };

    Some((get_line, set_line))
}

fn handle_nested_data(
    field_accessor: &FieldAccessor,
    value_name: &str,
    value_limit: &Option<syn::Expr>,
) -> (TokenStream, TokenStream) {
    let get_line = quote! {
        let nested_data = self.#field_accessor.to_data();
    };
    let get_line = if let Some(limit) = value_limit {
        quote! {
            #get_line
            data.insert_with_limit(#value_name, Value::Data(nested_data), #limit);
        }
    } else {
        quote! {
            #get_line
            data.insert(#value_name, Value::Data(nested_data));
        }
    };

    let set_line = if !is_read_only(value_limit) {
        quote! {
            if let Some(Value::Data(nested_data)) = data.get(#value_name) {
                self.#field_accessor.set_data(nested_data);
            }
        }
    } else {
        quote! {}
    };

    (get_line, set_line)
}

/// Check if limit is ReadOnly.
fn is_read_only(limit: &Option<syn::Expr>) -> bool {
    limit.as_ref().map_or(false, |l| {
        l.to_token_stream().to_string().contains("ReadOnly")
    })
}

/// Field accessor helper.
#[derive(Debug)]
enum FieldAccessor {
    Ident(Ident),
    Index(Index),
}

impl ToString for FieldAccessor {
    fn to_string(&self) -> String {
        match self {
            Self::Ident(ident) => ident.to_string(),
            Self::Index(index) => format!("unnamed-{}", index.index),
        }
    }
}

impl ToTokens for FieldAccessor {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Ident(ident) => ident.to_tokens(tokens),
            Self::Index(index) => index.to_tokens(tokens),
        }
    }
}
