//! Derive logic for `Reconcile` and `ReconcileRoot`.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields};

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
                "Reconcile can only be derived for named structs, newtypes, or enums",
            )
            .to_compile_error(),
        },
        Data::Enum(data) => {
            expand_enum(name, &impl_generics, &ty_generics, where_clause, data)
        }
        Data::Union(_) => {
            syn::Error::new_spanned(name, "Reconcile cannot be derived for unions")
                .to_compile_error()
        }
    }
}

fn field_reconcile(ident: &syn::Ident, target_var: &str) -> TokenStream {
    let key = ident.to_string();
    let target = syn::Ident::new(target_var, proc_macro2::Span::call_site());
    quote! {
        let __session = ::autosurgeon_cat::Reconcile::reconcile(
            &self.#ident, &__session, #target, #key,
        )?;
    }
}

fn expand_named_struct(
    name: &syn::Ident,
    impl_generics: &syn::ImplGenerics<'_>,
    ty_generics: &syn::TypeGenerics<'_>,
    where_clause: Option<&syn::WhereClause>,
    fields: &syn::FieldsNamed,
) -> TokenStream {
    let reconcile_fields: Vec<TokenStream> = fields
        .named
        .iter()
        .map(|f| field_reconcile(f.ident.as_ref().expect("named field"), "__map_id"))
        .collect();

    let to_value_fields: Vec<TokenStream> = fields
        .named
        .iter()
        .map(|f| field_reconcile(f.ident.as_ref().expect("named field"), "__map_id"))
        .collect();

    let root_fields: Vec<TokenStream> = fields
        .named
        .iter()
        .map(|f| field_reconcile(f.ident.as_ref().expect("named field"), "__root"))
        .collect();

    quote! {
        impl #impl_generics ::autosurgeon_cat::Reconcile for #name #ty_generics #where_clause {
            fn reconcile(
                &self,
                session: &::autosurgeon_cat::Session,
                node: ::autosurgeon_cat::NodeId,
                key: &str,
            ) -> ::core::result::Result<::autosurgeon_cat::Session, ::autosurgeon_cat::Error> {
                let (__session, __map_id) = ::autosurgeon_cat::ensure_map(session, node, key)?;
                #(#reconcile_fields)*
                ::core::result::Result::Ok(__session)
            }

            fn to_value(
                &self,
                session: &::autosurgeon_cat::Session,
            ) -> ::core::result::Result<
                (::autosurgeon_cat::Session, ::autosurgeon_cat::Value),
                ::autosurgeon_cat::Error,
            > {
                let (__session, __map_id) = session.create_map()?;
                #(#to_value_fields)*
                ::core::result::Result::Ok((__session, ::autosurgeon_cat::Value::Map(__map_id)))
            }
        }

        impl #impl_generics ::autosurgeon_cat::ReconcileRoot for #name #ty_generics #where_clause {
            fn reconcile_root(
                &self,
                session: &::autosurgeon_cat::Session,
            ) -> ::core::result::Result<::autosurgeon_cat::Session, ::autosurgeon_cat::Error> {
                let __root = session.document().root();
                let __session = session.clone();
                #(#root_fields)*
                ::core::result::Result::Ok(__session)
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
        impl #impl_generics ::autosurgeon_cat::Reconcile for #name #ty_generics #where_clause {
            fn reconcile(
                &self,
                session: &::autosurgeon_cat::Session,
                node: ::autosurgeon_cat::NodeId,
                key: &str,
            ) -> ::core::result::Result<::autosurgeon_cat::Session, ::autosurgeon_cat::Error> {
                ::autosurgeon_cat::Reconcile::reconcile(&self.0, session, node, key)
            }

            fn to_value(
                &self,
                session: &::autosurgeon_cat::Session,
            ) -> ::core::result::Result<
                (::autosurgeon_cat::Session, ::autosurgeon_cat::Value),
                ::autosurgeon_cat::Error,
            > {
                ::autosurgeon_cat::Reconcile::to_value(&self.0, session)
            }
        }
    }
}

fn reconcile_arm(v: &syn::Variant) -> TokenStream {
    let variant = &v.ident;
    let variant_str = variant.to_string();
    match &v.fields {
        Fields::Unit => quote! {
            Self::#variant => {
                session.set_key(
                    node, key,
                    &::autosurgeon_cat::Value::Str(#variant_str.to_string()),
                ).map_err(::autosurgeon_cat::Error::from)
            }
        },
        Fields::Named(fields) => {
            let field_idents: Vec<&syn::Ident> = fields
                .named
                .iter()
                .filter_map(|f| f.ident.as_ref())
                .collect();
            let field_reconciles: Vec<TokenStream> = fields
                .named
                .iter()
                .map(|f| {
                    let ident = f.ident.as_ref().expect("named field");
                    let key_str = ident.to_string();
                    quote! {
                        let __session = ::autosurgeon_cat::Reconcile::reconcile(
                            #ident, &__session, __inner_map, #key_str,
                        )?;
                    }
                })
                .collect();
            quote! {
                Self::#variant { #(#field_idents),* } => {
                    let (__session, __wrapper_map) =
                        ::autosurgeon_cat::ensure_map(session, node, key)?;
                    let (__session, __inner_map) =
                        ::autosurgeon_cat::ensure_map(&__session, __wrapper_map, #variant_str)?;
                    #(#field_reconciles)*
                    ::core::result::Result::Ok(__session)
                }
            }
        }
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => quote! {
            Self::#variant(__inner) => {
                let (__session, __wrapper_map) =
                    ::autosurgeon_cat::ensure_map(session, node, key)?;
                ::autosurgeon_cat::Reconcile::reconcile(
                    __inner, &__session, __wrapper_map, #variant_str,
                )
            }
        },
        Fields::Unnamed(_) => syn::Error::new_spanned(
            variant,
            "Reconcile derive: tuple variants must have exactly one field",
        )
        .to_compile_error(),
    }
}

fn to_value_arm(v: &syn::Variant) -> TokenStream {
    let variant = &v.ident;
    let variant_str = variant.to_string();
    match &v.fields {
        Fields::Unit => quote! {
            Self::#variant => ::core::result::Result::Ok((
                session.clone(),
                ::autosurgeon_cat::Value::Str(#variant_str.to_string()),
            )),
        },
        Fields::Named(fields) => {
            let field_idents: Vec<&syn::Ident> = fields
                .named
                .iter()
                .filter_map(|f| f.ident.as_ref())
                .collect();
            let field_writes: Vec<TokenStream> = fields
                .named
                .iter()
                .map(|f| {
                    let ident = f.ident.as_ref().expect("named field");
                    let key_str = ident.to_string();
                    quote! {
                        let __session = ::autosurgeon_cat::Reconcile::reconcile(
                            #ident, &__session, __inner_map, #key_str,
                        )?;
                    }
                })
                .collect();
            quote! {
                Self::#variant { #(#field_idents),* } => {
                    let (__session, __wrapper_map) = session.create_map()?;
                    let (__session, __inner_map) = __session.create_map()?;
                    let __session = __session.set_key(
                        __wrapper_map, #variant_str,
                        &::autosurgeon_cat::Value::Map(__inner_map),
                    )?;
                    #(#field_writes)*
                    ::core::result::Result::Ok((
                        __session,
                        ::autosurgeon_cat::Value::Map(__wrapper_map),
                    ))
                }
            }
        }
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => quote! {
            Self::#variant(__inner) => {
                let (__session, __wrapper_map) = session.create_map()?;
                let (__session, __inner_val) =
                    ::autosurgeon_cat::Reconcile::to_value(__inner, &__session)?;
                let __session = __session.set_key(
                    __wrapper_map, #variant_str, &__inner_val,
                )?;
                ::core::result::Result::Ok((
                    __session,
                    ::autosurgeon_cat::Value::Map(__wrapper_map),
                ))
            }
        },
        Fields::Unnamed(_) => syn::Error::new_spanned(
            variant,
            "Reconcile derive: tuple variants must have exactly one field",
        )
        .to_compile_error(),
    }
}

fn expand_enum(
    name: &syn::Ident,
    impl_generics: &syn::ImplGenerics<'_>,
    ty_generics: &syn::TypeGenerics<'_>,
    where_clause: Option<&syn::WhereClause>,
    data: &syn::DataEnum,
) -> TokenStream {
    let reconcile_arms: Vec<TokenStream> =
        data.variants.iter().map(reconcile_arm).collect();
    let to_value_arms: Vec<TokenStream> =
        data.variants.iter().map(to_value_arm).collect();

    quote! {
        impl #impl_generics ::autosurgeon_cat::Reconcile for #name #ty_generics #where_clause {
            fn reconcile(
                &self,
                session: &::autosurgeon_cat::Session,
                node: ::autosurgeon_cat::NodeId,
                key: &str,
            ) -> ::core::result::Result<::autosurgeon_cat::Session, ::autosurgeon_cat::Error> {
                match self {
                    #(#reconcile_arms)*
                }
            }

            fn to_value(
                &self,
                session: &::autosurgeon_cat::Session,
            ) -> ::core::result::Result<
                (::autosurgeon_cat::Session, ::autosurgeon_cat::Value),
                ::autosurgeon_cat::Error,
            > {
                match self {
                    #(#to_value_arms)*
                }
            }
        }
    }
}
