//! `#[derive(Sexpr)]` 
//! derives `crate::sexpr::Sexpr` (plus `Display`) for AST types
//!
//! # Struct/Enum Attributes
//! - `#[sexpr(keyword = "kw")]`    prepend a literal token to the list
//! - `#[sexpr(transparent)]`       single-field struct renders as that field
//! - `#[sexpr(node)]`              enum that implements `AstNode` (auto for structs with .node_id)
//! - `#[sexpr(no_display)]`        don't emit the `impl Display`
//!
//! # Variant attributes
//! - `#[sexpr(atom = "tok")]`      render the variant as a bare token, ignoring any payload
//! - `#[sexpr(keyword = "kw")]`    as above, for this variant's list
//! - `#[sexpr(with = "path")]`     newtype variants only: call `path(&field, ctx) -> Repr` for a fully custom rendering
//!
//! # Field attributes
//! - `#[sexpr(atom)]`      render with `Display` instead of recursing
//! - `#[sexpr(splice)]`    `Vec<T>`: splice elements into the enclosing list (default for `Vec` is a nested `(a b c)` group)
//! - `#[sexpr(group)]`     `Vec<T>`: bare element if `len == 1`, else `(a b c)`
//! - `#[sexpr(skip)]`      omit from the rendering
//!
//! Unannotated fields recurse via `Sexpr::to_doc`; 
//! `Option<T>` fields are omitted when `None`;
//! `Vec<T>` fields become a nested parenthesized group.
//! Newtype enum variants with no attributes are transparent (they render as their payload, preserving the payload's own annotations).

use proc_macro::TokenStream;
use proc_macro2::TokenStream as Ts2;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DataEnum, DataStruct, DeriveInput, Error, Field, Fields, LitStr, Path, Result};

#[proc_macro_derive(Sexpr, attributes(sexpr))]
pub fn derive_sexpr(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand(&input).unwrap_or_else(Error::into_compile_error).into()
}

// ---- attribute models ----

#[derive(Default)]
struct ContainerOpts {
    keyword: Option<String>,
    transparent: bool,
    node: bool,
    no_display: bool,
}

#[derive(Default)]
struct VariantOpts {
    keyword: Option<String>,
    atom: Option<String>,
    with: Option<Path>,
    transparent: bool,
}

#[derive(Default)]
struct FieldOpts {
    atom: bool,
    skip: bool,
    splice: bool,
    group: bool,
}

fn parse_container(attrs: &[syn::Attribute]) -> Result<ContainerOpts> {
    let mut o = ContainerOpts::default();
    for attr in attrs {
        if !attr.path().is_ident("sexpr") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("keyword") {
                o.keyword = Some(meta.value()?.parse::<LitStr>()?.value());
            } else if meta.path.is_ident("transparent") {
                o.transparent = true;
            } else if meta.path.is_ident("node") {
                o.node = true;
            } else if meta.path.is_ident("no_display") {
                o.no_display = true;
            } else {
                return Err(meta.error("unknown `sexpr` container option"));
            }
            Ok(())
        })?;
    }
    Ok(o)
}

fn parse_variant(attrs: &[syn::Attribute]) -> Result<VariantOpts> {
    let mut o = VariantOpts::default();
    for attr in attrs {
        if !attr.path().is_ident("sexpr") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("keyword") {
                o.keyword = Some(meta.value()?.parse::<LitStr>()?.value());
            } else if meta.path.is_ident("atom") {
                o.atom = Some(meta.value()?.parse::<LitStr>()?.value());
            } else if meta.path.is_ident("with") {
                o.with = Some(meta.value()?.parse::<LitStr>()?.parse()?);
            } else if meta.path.is_ident("transparent") {
                o.transparent = true;
            } else {
                return Err(meta.error("unknown `sexpr` variant option"));
            }
            Ok(())
        })?;
    }
    Ok(o)
}

fn parse_field(attrs: &[syn::Attribute]) -> Result<FieldOpts> {
    let mut o = FieldOpts::default();
    for attr in attrs {
        if !attr.path().is_ident("sexpr") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("atom") {
                o.atom = true;
            } else if meta.path.is_ident("skip") {
                o.skip = true;
            } else if meta.path.is_ident("splice") {
                o.splice = true;
            } else if meta.path.is_ident("group") {
                o.group = true;
            } else {
                return Err(meta.error("unknown `sexpr` field option"));
            }
            Ok(())
        })?;
    }
    Ok(o)
}

// ---- field rendering ----

enum Mode {
    Skip,
    Atom,
    Splice,
    Group,
    List,     // Vec default: nested parens
    Optional, // Option default: omit if None
    Recurse,  // everything else: Sexpr::to_doc
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
    } else if o.atom {
        Mode::Atom
    } else if o.splice {
        Mode::Splice
    } else if o.group {
        Mode::Group
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
fn emit_part(mode: &Mode, acc: &Ts2) -> Ts2 {
    match mode {
        Mode::Skip => quote! {},
        Mode::Atom => quote! { __parts.push(crate::sexpr::atom(&#acc)); },
        Mode::Recurse => quote! {
            __parts.push(crate::sexpr::Sexpr::to_doc(#acc, __ctx));
        },
        Mode::List => quote! {
            __parts.push(crate::sexpr::parens(
                #acc.iter().map(|__x| crate::sexpr::Sexpr::to_doc(__x, __ctx)),
            ));
        },
        Mode::Splice => quote! {
            __parts.extend(#acc.iter().map(|__x| crate::sexpr::Sexpr::to_doc(__x, __ctx)));
        },
        Mode::Group => quote! {
            __parts.push(if #acc.len() == 1 {
                crate::sexpr::Sexpr::to_doc(&#acc[0], __ctx)
            } else {
                crate::sexpr::parens(
                    #acc.iter().map(|__x| crate::sexpr::Sexpr::to_doc(__x, __ctx)),
                )
            });
        },
        Mode::Optional => quote! {
            if let ::core::option::Option::Some(__x) = #acc {
                __parts.push(crate::sexpr::Sexpr::to_doc(__x, __ctx));
            }
        },
    }
}

fn keyword_part(kw: &Option<String>) -> Ts2 {
    match kw {
        Some(kw) => quote! { __parts.push(::pretty::RcDoc::text(#kw)); },
        None => quote! {},
    }
}

// ---- expansion ----

fn expand(input: &DeriveInput) -> Result<Ts2> {
    let copts = parse_container(&input.attrs)?;
    let name = &input.ident;
    let (impl_g, ty_g, where_c) = input.generics.split_for_impl();

    // (repr body, optional `to_doc` override)
    let (repr_body, to_doc) = match &input.data {
        Data::Struct(s) => expand_struct(input, &copts, s)?,
        Data::Enum(e) => {
            let body = expand_enum(&copts, e)?;
            let to_doc = if copts.node {
                // Requires the type to implement `AstNode` (in scope at the
                // derive site) so `self.node_id()` resolves.
                Some(quote! {
                    fn to_doc(
                        &self,
                        __ctx: &dyn crate::sexpr::SexprCtx,
                    ) -> ::pretty::RcDoc<'static, ()> {
                        crate::sexpr::finish(
                            crate::sexpr::Sexpr::repr(self, __ctx),
                            __ctx.annotate(self.node_id()),
                        )
                    }
                })
            } else {
                None
            };
            (body, to_doc)
        }
        Data::Union(_) => return Err(Error::new_spanned(name, "Sexpr cannot be derived for unions")),
    };

    let display = if copts.no_display {
        quote! {}
    } else {
        quote! {
            impl #impl_g ::core::fmt::Display for #name #ty_g #where_c {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    crate::sexpr::Sexpr::to_doc(self, &crate::sexpr::Plain)
                        .render_fmt(crate::sexpr::WIDTH, f)
                }
            }
        }
    };

    Ok(quote! {
        #[automatically_derived]
        impl #impl_g crate::sexpr::Sexpr for #name #ty_g #where_c {
            fn repr(&self, __ctx: &dyn crate::sexpr::SexprCtx) -> crate::sexpr::Repr {
                #repr_body
            }
            #to_doc
        }
        #display
    })
}

fn expand_struct(input: &DeriveInput, copts: &ContainerOpts, s: &DataStruct) -> Result<(Ts2, Option<Ts2>)> {
    let Fields::Named(fields) = &s.fields else {
        return Err(Error::new_spanned(
            &input.ident,
            "Sexpr on structs requires named fields",
        ));
    };

    // Fields other than `node_id`.
    let mut payload: Vec<&Field> = Vec::new();
    let mut is_node = copts.node;
    for f in &fields.named {
        if f.ident.as_ref().is_some_and(|i| i == "node_id") {
            is_node = true;
        } else {
            payload.push(f);
        }
    }

    let to_doc = is_node.then(|| {
        quote! {
            fn to_doc(
                &self,
                __ctx: &dyn crate::sexpr::SexprCtx,
            ) -> ::pretty::RcDoc<'static, ()> {
                crate::sexpr::finish(
                    crate::sexpr::Sexpr::repr(self, __ctx),
                    __ctx.annotate(self.node_id),
                )
            }
        }
    });

    // Carrier/kind convention: `T { node_id, kind }` delegates its repr to the
    // kind so annotations splice *inside* the kind's parens.
    let is_carrier = is_node
        && copts.keyword.is_none()
        && !copts.transparent
        && payload.len() == 1
        && payload[0].ident.as_ref().is_some_and(|i| i == "kind");
    if is_carrier {
        return Ok((
            quote! { crate::sexpr::Sexpr::repr(&self.kind, __ctx) },
            to_doc,
        ));
    }

    if copts.transparent {
        if payload.len() != 1 {
            return Err(Error::new_spanned(
                &input.ident,
                "`sexpr(transparent)` requires exactly one field (besides `node_id`)",
            ));
        }
        let f = payload[0].ident.as_ref().unwrap();
        return Ok((
            quote! { crate::sexpr::Sexpr::repr(&self.#f, __ctx) },
            to_doc,
        ));
    }

    let kw = keyword_part(&copts.keyword);
    let parts = payload
        .iter()
        .map(|f| {
            let ident = f.ident.as_ref().unwrap();
            let acc = quote!((&self.#ident));
            Ok(emit_part(&field_mode(f)?, &acc))
        })
        .collect::<Result<Vec<_>>>()?;

    Ok((
        quote! {
            let mut __parts: ::std::vec::Vec<::pretty::RcDoc<'static, ()>> =
                ::std::vec::Vec::new();
            #kw
            #(#parts)*
            crate::sexpr::Repr::List(__parts)
        },
        to_doc,
    ))
}

fn expand_enum(copts: &ContainerOpts, e: &DataEnum) -> Result<Ts2> {
    if copts.keyword.is_some() || copts.transparent {
        return Err(Error::new_spanned(
            e.enum_token,
            "`keyword`/`transparent` are variant-level options on enums",
        ));
    }

    let mut arms: Vec<Ts2> = Vec::new();
    for v in &e.variants {
        let vopts = parse_variant(&v.attrs)?;
        let vname = &v.ident;

        // `atom = "tok"`: bare token, payload ignored.
        if let Some(tok) = &vopts.atom {
            arms.push(quote! {
                Self::#vname { .. } => crate::sexpr::Repr::Atom(::pretty::RcDoc::text(#tok)),
            });
            continue;
        }

        match &v.fields {
            Fields::Unit => {
                let tok = vname.to_string();
                arms.push(quote! {
                    Self::#vname => crate::sexpr::Repr::Atom(::pretty::RcDoc::text(#tok)),
                });
            }
            Fields::Unnamed(fs) => {
                let binds: Vec<_> = (0..fs.unnamed.len()).map(|i| format_ident!("__f{i}")).collect();
                if let Some(path) = &vopts.with {
                    if fs.unnamed.len() != 1 {
                        return Err(Error::new_spanned(
                            v,
                            "`sexpr(with = ...)` is only supported on newtype variants",
                        ));
                    }
                    arms.push(quote! {
                        Self::#vname(__f0) => #path(__f0, __ctx),
                    });
                    continue;
                }
                // A newtype variant with no keyword and a plain (recursing)
                // field is transparent: render as the payload, keeping the
                // payload's own annotations.
                let modes = fs.unnamed.iter().map(field_mode).collect::<Result<Vec<_>>>()?;
                let transparent = vopts.transparent
                    || (vopts.keyword.is_none()
                        && fs.unnamed.len() == 1
                        && matches!(modes[0], Mode::Recurse));
                if transparent {
                    arms.push(quote! {
                        Self::#vname(__f0) =>
                            crate::sexpr::Repr::Raw(crate::sexpr::Sexpr::to_doc(__f0, __ctx)),
                    });
                    continue;
                }
                let kw = keyword_part(&vopts.keyword);
                let parts: Vec<_> = modes
                    .iter()
                    .zip(&binds)
                    .map(|(m, b)| emit_part(m, &quote!(#b)))
                    .collect();
                arms.push(quote! {
                    Self::#vname(#(#binds),*) => {
                        let mut __parts: ::std::vec::Vec<::pretty::RcDoc<'static, ()>> =
                            ::std::vec::Vec::new();
                        #kw
                        #(#parts)*
                        crate::sexpr::Repr::List(__parts)
                    }
                });
            }
            Fields::Named(fs) => {
                if vopts.with.is_some() {
                    return Err(Error::new_spanned(
                        v,
                        "`sexpr(with = ...)` is only supported on newtype variants",
                    ));
                }
                let names: Vec<_> = fs.named.iter().map(|f| f.ident.clone().unwrap()).collect();
                let kw = keyword_part(&vopts.keyword);
                let parts = fs
                    .named
                    .iter()
                    .map(|f| {
                        let b = f.ident.as_ref().unwrap();
                        Ok(emit_part(&field_mode(f)?, &quote!(#b)))
                    })
                    .collect::<Result<Vec<_>>>()?;
                arms.push(quote! {
                    Self::#vname { #(#names),* } => {
                        let mut __parts: ::std::vec::Vec<::pretty::RcDoc<'static, ()>> =
                            ::std::vec::Vec::new();
                        #kw
                        #(#parts)*
                        crate::sexpr::Repr::List(__parts)
                    }
                });
            }
        }
    }

    Ok(quote! {
        #[allow(unused_variables)]
        match self {
            #(#arms)*
        }
    })
}
