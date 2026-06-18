use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(NodeId)]
pub fn derive_nodeid(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    // Only works on structs with a named `node_id` field — error otherwise
    let has_nodeid_field = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(f) => f.named.iter().any(|f| {
                f.ident.as_ref().map_or(false, |id| id == "node_id")
            }),
            _ => false,
        },
        _ => false,
    };

    if !has_nodeid_field {
        return syn::Error::new_spanned(
            &input.ident,
            "NodeId can only be derived on structs with a named `node_id: u64` field",
        )
        .to_compile_error()
        .into();
    }

    quote! {
        impl #impl_generics NodeId for #name #ty_generics #where_clause {
            fn node_id(&self) -> u64 { self.node_id }
        }
    }
    .into()
}
