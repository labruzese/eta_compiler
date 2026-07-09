use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use std::collections::HashSet;
use syn::{Data, DeriveInput, Fields, Type, parse_macro_input};

/// Strip any number of leading `&`/`&mut` layers off a type,
/// e.g. `&'i VarTy` -> `VarTy`, `&&mut VarTy` -> `VarTy`.
fn strip_refs(mut ty: &Type) -> Type {
    while let Type::Reference(r) = ty {
        ty = &r.elem;
    }
    ty.clone()
}

#[proc_macro_derive(EtaType)]
pub fn derive_eta_type(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let data_enum = match &input.data {
        Data::Enum(data_enum) => data_enum,
        _ => {
            return syn::Error::new_spanned(
                &input,
                "#[derive(EtaType)] only supports enums (need per-variant bounds)",
            )
            .to_compile_error()
            .into();
        }
    };

    let mut seen = HashSet::new();
    let mut bound_tys: Vec<Type> = Vec::new();

    for variant in &data_enum.variants {
        for field in variant.fields.iter() {
            // Look through references so `&'i VarTy` bounds on `VarTy`.
            let ty = strip_refs(&field.ty);
            let key = quote!(#ty).to_string();
            if seen.insert(key) {
                bound_tys.push(ty);
            }
        }
    }

    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let predicates: Vec<TokenStream2> = bound_tys
        .iter()
        .map(|ty| quote! { #ty: crate::types::EtaType })
        .collect();

    let where_tokens = match (where_clause, predicates.is_empty()) {
        (Some(wc), true) => quote! { #wc },
        (Some(wc), false) => quote! { #wc, #(#predicates),* },
        (None, true) => quote! {},
        (None, false) => quote! { where #(#predicates),* },
    };

    let expanded = quote! {
        impl #impl_generics crate::types::EtaType for #name #ty_generics #where_tokens {}
    };

    expanded.into()
}

#[proc_macro_derive(TypecheckTy)]
pub fn derive_typecheck_ty(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let kind_name = &input.ident;                       
    let ty_name = format_ident!("{}Type", kind_name);    

    let Data::Enum(data) = &input.data else {
        return syn::Error::new_spanned(&input, "TypecheckTy only supports enums")
            .to_compile_error().into();
    };

    let mut variants = Vec::new();
    let mut arms = Vec::new();

    for v in &data.variants {
        let vname = &v.ident;
        match &v.fields {
            Fields::Unnamed(f) if f.unnamed.len() == 1 => {
                let payload = &f.unnamed[0].ty;
                variants.push(quote! { #vname(<#payload as crate::Typecheck>::Ty) });
                arms.push(quote! {
                    #kind_name::#vname(inner) => {
                        let t = inner.typecheck(env)?.clone();
                        Ok(env.types.insert(self.node_id(), #ty_name::#vname(t)))
                    }
                });
            }
            Fields::Unit => {
                arms.push(quote! {
                    #kind_name::#vname => Err(unsafe {
                        etac_errors::ErrorGuaranteed::claim_already_emitted()
                    })
                });
            }
            _ => {
                return syn::Error::new_spanned(v, "expected a single unnamed field or unit variant")
                    .to_compile_error().into();
            }
        }
    }

    quote! {
        #[derive(Debug, Clone, EtaType)]
        pub enum #ty_name { #(#variants),* }

        impl crate::Typecheck for #kind_name {
            type Ty = #ty_name;
            fn typecheck<'e>(&self, env: &'e mut crate::context::Env) -> crate::Result<&'e Self::Ty> {
                match self { #(#arms),* }
            }
        }
    }.into()
}
