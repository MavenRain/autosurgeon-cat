//! Derive logic for `Hydrate` and `HydrateRoot`.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Type};

pub(crate) fn expand(input: &DeriveInput) -> TokenStream {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => {
                expand_named_struct(name, &impl_generics, &ty_generics, where_clause, fields)
            }
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                expand_newtype(name, &impl_generics, &ty_generics, where_clause)
            }
            Fields::Unnamed(_) | Fields::Unit => syn::Error::new_spanned(
                name,
                "Hydrate can only be derived for named structs, newtypes, or enums",
            )
            .to_compile_error(),
        },
        Data::Enum(data) => {
            expand_enum(name, &impl_generics, &ty_generics, where_clause, data)
        }
        Data::Union(_) => {
            syn::Error::new_spanned(name, "Hydrate cannot be derived for unions")
                .to_compile_error()
        }
    }
}

fn field_hydrate(ident: &syn::Ident, ty: &Type, node_var: &str) -> TokenStream {
    let key = ident.to_string();
    let node = syn::Ident::new(node_var, proc_macro2::Span::call_site());
    if is_option_type(ty) {
        quote! {
            let #ident = ::autosurgeon_cat::hydrate_optional_field(doc, #node, #key)?;
        }
    } else {
        quote! {
            let #ident = ::autosurgeon_cat::hydrate_field(doc, #node, #key)?;
        }
    }
}

fn expand_named_struct(
    name: &syn::Ident,
    impl_generics: &syn::ImplGenerics<'_>,
    ty_generics: &syn::TypeGenerics<'_>,
    where_clause: Option<&syn::WhereClause>,
    fields: &syn::FieldsNamed,
) -> TokenStream {
    let field_hydrates: Vec<TokenStream> = fields
        .named
        .iter()
        .map(|f| {
            let ident = f.ident.as_ref().expect("named field");
            field_hydrate(ident, &f.ty, "__node_id")
        })
        .collect();

    let field_names: Vec<&syn::Ident> = fields
        .named
        .iter()
        .filter_map(|f| f.ident.as_ref())
        .collect();

    quote! {
        impl #impl_generics ::autosurgeon_cat::Hydrate for #name #ty_generics #where_clause {
            fn hydrate(
                doc: &::autosurgeon_cat::Document,
                value: &::autosurgeon_cat::Value,
            ) -> ::core::result::Result<Self, ::autosurgeon_cat::Error> {
                let __node_id = ::autosurgeon_cat::expect_map(value)?;
                #(#field_hydrates)*
                ::core::result::Result::Ok(Self { #(#field_names),* })
            }
        }

        impl #impl_generics ::autosurgeon_cat::HydrateRoot for #name #ty_generics #where_clause {
            fn hydrate_root(
                doc: &::autosurgeon_cat::Document,
            ) -> ::core::result::Result<Self, ::autosurgeon_cat::Error> {
                let __node_id = doc.root();
                #(#field_hydrates)*
                ::core::result::Result::Ok(Self { #(#field_names),* })
            }
        }
    }
}

fn expand_newtype(
    name: &syn::Ident,
    impl_generics: &syn::ImplGenerics<'_>,
    ty_generics: &syn::TypeGenerics<'_>,
    where_clause: Option<&syn::WhereClause>,
) -> TokenStream {
    quote! {
        impl #impl_generics ::autosurgeon_cat::Hydrate for #name #ty_generics #where_clause {
            fn hydrate(
                doc: &::autosurgeon_cat::Document,
                value: &::autosurgeon_cat::Value,
            ) -> ::core::result::Result<Self, ::autosurgeon_cat::Error> {
                ::autosurgeon_cat::Hydrate::hydrate(doc, value).map(Self)
            }
        }
    }
}

fn enum_unit_arms(data: &syn::DataEnum) -> Vec<TokenStream> {
    data.variants
        .iter()
        .filter(|v| matches!(v.fields, Fields::Unit))
        .map(|v| {
            let variant = &v.ident;
            let variant_str = variant.to_string();
            quote! { #variant_str => ::core::result::Result::Ok(Self::#variant), }
        })
        .collect()
}

fn enum_map_arm(v: &syn::Variant) -> TokenStream {
    let variant = &v.ident;
    let variant_str = variant.to_string();
    match &v.fields {
        Fields::Named(fields) => {
            let field_hydrates: Vec<TokenStream> = fields
                .named
                .iter()
                .map(|f| {
                    let ident = f.ident.as_ref().expect("named field");
                    field_hydrate(ident, &f.ty, "__inner_node")
                })
                .collect();
            let field_names: Vec<&syn::Ident> = fields
                .named
                .iter()
                .filter_map(|f| f.ident.as_ref())
                .collect();
            quote! {
                #variant_str => {
                    let __inner_value = ::autosurgeon_cat::resolve_key(
                        doc, *__node_id, #variant_str,
                    )?;
                    let __inner_node = ::autosurgeon_cat::expect_map(__inner_value)?;
                    #(#field_hydrates)*
                    ::core::result::Result::Ok(Self::#variant { #(#field_names),* })
                }
            }
        }
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
            quote! {
                #variant_str => {
                    let __inner_value = ::autosurgeon_cat::resolve_key(
                        doc, *__node_id, #variant_str,
                    )?;
                    ::autosurgeon_cat::Hydrate::hydrate(doc, __inner_value)
                        .map(Self::#variant)
                }
            }
        }
        Fields::Unnamed(_) => syn::Error::new_spanned(
            variant,
            "Hydrate derive: tuple variants must have exactly one field",
        )
        .to_compile_error(),
        Fields::Unit => unreachable!(),
    }
}

fn expand_enum(
    name: &syn::Ident,
    impl_generics: &syn::ImplGenerics<'_>,
    ty_generics: &syn::TypeGenerics<'_>,
    where_clause: Option<&syn::WhereClause>,
    data: &syn::DataEnum,
) -> TokenStream {
    let unit_arms = enum_unit_arms(data);
    let map_arms: Vec<TokenStream> = data
        .variants
        .iter()
        .filter(|v| !matches!(v.fields, Fields::Unit))
        .map(enum_map_arm)
        .collect();

    let str_branch = if unit_arms.is_empty() {
        TokenStream::new()
    } else {
        quote! {
            ::autosurgeon_cat::Value::Str(__s) => {
                match __s.as_str() {
                    #(#unit_arms)*
                    __other => ::core::result::Result::Err(
                        ::autosurgeon_cat::Error::UnknownVariant {
                            variant: __other.to_string(),
                        },
                    ),
                }
            }
        }
    };

    let map_branch = if map_arms.is_empty() {
        TokenStream::new()
    } else {
        quote! {
            ::autosurgeon_cat::Value::Map(__node_id) => {
                let __keys = doc.keys(*__node_id)?;
                let __variant_key = __keys
                    .into_iter()
                    .next()
                    .ok_or_else(|| ::autosurgeon_cat::Error::TypeMismatch {
                        expected: "non-empty map (enum variant)",
                        found: "empty map",
                    })?;
                match __variant_key {
                    #(#map_arms)*
                    __other => ::core::result::Result::Err(
                        ::autosurgeon_cat::Error::UnknownVariant {
                            variant: __other.to_string(),
                        },
                    ),
                }
            }
        }
    };

    quote! {
        impl #impl_generics ::autosurgeon_cat::Hydrate for #name #ty_generics #where_clause {
            fn hydrate(
                doc: &::autosurgeon_cat::Document,
                value: &::autosurgeon_cat::Value,
            ) -> ::core::result::Result<Self, ::autosurgeon_cat::Error> {
                match value {
                    #str_branch
                    #map_branch
                    __other => ::core::result::Result::Err(
                        ::autosurgeon_cat::Error::TypeMismatch {
                            expected: "Str or Map (enum)",
                            found: ::autosurgeon_cat::value_type_name(__other),
                        },
                    ),
                }
            }
        }
    }
}

fn is_option_type(ty: &Type) -> bool {
    match ty {
        Type::Path(type_path) => type_path
            .path
            .segments
            .last()
            .is_some_and(|seg| seg.ident == "Option"),
        _ => false,
    }
}
