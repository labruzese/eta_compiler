use proc_macro::TokenStream;
use proc_macro2::TokenStream as Ts2;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DataEnum, DataStruct, DeriveInput, Error, Field, Fields, Result};

#[proc_macro_derive(Ast, attributes(ast))]
pub fn derive_ast(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand(&input).unwrap_or_else(Error::into_compile_error).into()
}

// ---- attribute models ----

#[derive(Default)]
struct ContainerOpts {
    transparent: bool,
}

#[derive(Default)]
struct VariantOpts {
    transparent: bool,
}

#[derive(Default)]
struct FieldOpts {
    skip: bool,
}

static CONTAINER_OPTIONS: &str = "ast(transparent)";
fn parse_container(attrs: &[syn::Attribute]) -> Result<ContainerOpts> {
    let mut o = ContainerOpts::default();
    for attr in attrs {
        if !attr.path().is_ident("ast") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("transparent") {
                o.transparent = true;
            } else {
                return Err(meta.error(format!("unknown `ast` container option. Options are: {CONTAINER_OPTIONS}")));
            }
            Ok(())
        })?;
    }
    Ok(o)
}

static VARIANT_OPTIONS: &str = "ast(transparent)";
fn parse_variant(attrs: &[syn::Attribute]) -> Result<VariantOpts> {
    let mut o = VariantOpts::default();
    for attr in attrs {
        if !attr.path().is_ident("ast") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("transparent") {
                o.transparent = true;
            } else {
                return Err(meta.error(format!("unknown `ast` variant option. Options are: {VARIANT_OPTIONS}")));
            }
            Ok(())
        })?;
    }
    Ok(o)
}

static FIELD_OPTIONS: &str = "ast(skip)";
fn parse_field(attrs: &[syn::Attribute]) -> Result<FieldOpts> {
    let mut o = FieldOpts::default();
    for attr in attrs {
        if !attr.path().is_ident("ast") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("skip") {
                o.skip = true;
            } else {
                return Err(meta.error(format!("unknown `ast` field option. Options are: {FIELD_OPTIONS}")));
            }
            Ok(())
        })?;
    }
    Ok(o)
}

// ---- field rendering ----

enum Mode {
    Skip,
    List,
    Optional,
    Recurse,
}

fn last_segment_is(ty: &syn::Type, name: &str) -> bool {
    match ty {
        // `macro_rules!` `$ty:ty` fragments arrive wrapped in none-delimited
        // groups; look through them.
        syn::Type::Group(g) => last_segment_is(&g.elem, name),
        syn::Type::Paren(p) => last_segment_is(&p.elem, name),
        syn::Type::Path(p) => p.path.segments.last().is_some_and(|seg| seg.ident == name),
        _ => false,
    }
}

fn field_mode(f: &Field) -> Result<Mode> {
    let o = parse_field(&f.attrs)?;
    Ok(if o.skip {
        Mode::Skip
    } else if last_segment_is(&f.ty, "Vec") {
        Mode::List
    } else if last_segment_is(&f.ty, "Option") {
        Mode::Optional
    } else {
        Mode::Recurse
    })
}

/// Emit the statements that push `acc` (an expression of type `&T`) onto
/// `__parts` according to `mode`.
fn emit_part(mode: &Mode, visit_name: &syn::Ident, dispatcher: &Ts2) -> Ts2 {
    let visit_name = format_ident!("visit_{}", visit_name.to_string().to_lowercase());    
    match mode {
        Mode::Skip => quote! {},
        Mode::Recurse => quote! {
            (&#dispatcher).__visit_dispatch(<visitor as Visitor>::#visit_name, visitor);
        },
        Mode::List => quote! {
            (&#dispatcher).iter().for_each(|__x| __x.__visit_dispatch(<visitor as Visitor>::#visit_name), visitor);
        },
        Mode::Optional => quote! {
            if let Some(__x) = (&#dispatcher) {
                __x.__visit_dispatch(<visitor as Visitor>::#visit_name, visitor);
            }
        },
    }
}

// ---- expansion ----

fn expand(input: &DeriveInput) -> Result<Ts2> {
    let copts = parse_container(&input.attrs)?;
    let name = &input.ident;
    let (impl_g, ty_g, where_c) = input.generics.split_for_impl();

    // (repr body, optional `to_doc` override)
    let body = match &input.data {
        Data::Struct(s) => expand_struct(input, &copts, s)?,
        Data::Enum(e) => expand_enum(input, &copts, e)?,
        Data::Union(_) => return Err(Error::new_spanned(name, "Ast cannot be derived for unions")),
    };

    Ok(quote! {
        #[automatically_derived]
        impl #impl_g crate::ast_interface::Ast for #name #ty_g #where_c {
            fn recurse<V: Visitor>(&self, visitor: &mut V) {
                #body
            }
        }
    })
}

fn expand_struct(input: &DeriveInput, copts: &ContainerOpts, s: &DataStruct) -> Result<Ts2> {
    let Fields::Named(fields) = &s.fields else {
        return Err(Error::new_spanned(
            &input.ident,
            "Ast on structs requires named fields",
        ));
    };

    if copts.transparent {
        return Ok(quote!{});
    }

    let parts = fields.named
        .iter()
        .map(|f| {
            let ident = f.ident.as_ref().unwrap();
            Ok(emit_part(&field_mode(f)?, &input.ident, &quote!(self.#ident)))
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(
        quote! {
            #(#parts)*
        },
    )
}

fn expand_enum(input: &DeriveInput, copts: &ContainerOpts, e: &DataEnum) -> Result<Ts2> {
    let mut arms: Vec<Ts2> = Vec::new();
    for v in &e.variants {
        let vopts = parse_variant(&v.attrs)?;
        let vname = &v.ident;

        if copts.transparent || vopts.transparent {
            arms.push(quote! {
                Self::#vname(_) => ()
            });
            continue;
        }

        match &v.fields {
            Fields::Unit => {
                arms.push(quote! {
                    Self::#vname => (),
                });
            }
            Fields::Unnamed(fs) => {
                let binds: Vec<_> = (0..fs.unnamed.len()).map(|i| format_ident!("__f{i}")).collect();
                let modes = fs.unnamed.iter().map(field_mode).collect::<Result<Vec<_>>>()?;
                let parts: Vec<_> = modes
                    .iter()
                    .zip(&binds)
                    .map(|(m, b)| emit_part(m, &input.ident,  &quote!(#b)))
                    .collect();
                arms.push(quote! {
                    Self::#vname(#(#binds),*) => {
                        #(#parts)*
                    }
                });
            }
            Fields::Named(fs) => {
                let names: Vec<_> = fs.named.iter().map(|f| f.ident.clone().unwrap()).collect();
                let modes = fs.named.iter().map(field_mode).collect::<Result<Vec<_>>>()?;
                let parts: Vec<_> = modes
                    .iter()
                    .zip(&names)
                    .map(|(m, b)| emit_part(m, &input.ident, &quote!(#b)))
                    .collect();
                arms.push(quote! {
                    Self::#vname { #(#names),* } => {
                        #(#parts)*
                    }
                });
            }
        }
    }

    Ok(quote! {
        #[allow(unused_variables)]
        match self {
            #(#arms),*
        }
    })
}
