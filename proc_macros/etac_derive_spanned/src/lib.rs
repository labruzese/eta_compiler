use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(Spanned)]
pub fn derive_spanned(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    // Only works on structs with a named `span` field — error otherwise
    let has_span_field = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(f) => f.named.iter().any(|f| {
                f.ident.as_ref().map_or(false, |id| id == "span")
            }),
            _ => false,
        },
        _ => false,
    };

    if !has_span_field {
        return syn::Error::new_spanned(
            &input.ident,
            "Spanned can only be derived on structs with a named `span: Span` field",
        )
        .to_compile_error()
        .into();
    }

    quote! {
        impl #impl_generics Spanned for #name #ty_generics #where_clause {
            fn span(&self) -> Span { self.span }
        }
    }
    .into()
}
