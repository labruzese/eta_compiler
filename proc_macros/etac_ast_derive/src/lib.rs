use proc_macro::TokenStream;
use proc_macro2::TokenStream as Ts2;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DataEnum, DataStruct, DeriveInput, Error, Field, Fields, Result};

#[proc_macro_derive(Ast, attributes(ast))]
pub fn derive_ast(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand(&input).unwrap_or_else(Error::into_compile_error).into()
}

#[proc_macro_attribute]
/// Attaches the following attributes:
///     * `#[derive(Ast, AstNode, Clone, Debug)]`
/// Implements:
///     * `fn new() -> Self`
/// Does some checks on the well-formedness of an AstNode.
pub fn node(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as DeriveInput);
    let name = input.ident.clone();

    let fields = match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(named) => named.named.clone(),
            _ => return Error::new_spanned(input, "#[node] requires a struct with named fields").into_compile_error().into(),
        },
        _ => return Error::new_spanned(input, "#[node] can only be applied to structs").into_compile_error().into(),
    };

    let node_id_field = fields
        .iter()
        .find(|f| f.ident.as_ref().map(|i| i == "node_id").unwrap_or(false));

    let node_id_field = match node_id_field {
        Some(f) => f,
        None => {
            return Error::new_spanned(
                input,
                "struct must declare `node_id: NodeId<Self>` to be a #[node]",
            ).into_compile_error().into();
        }
    };
    let ty = &node_id_field.ty;
    let ty_str = quote!(#ty).to_string().replace(' ', "");
    if !ty_str.ends_with("NodeId<Self>") {
        return Error::new_spanned(
            &node_id_field.ty,
            "the `node_id` field of a #[node] must have type `NodeId<Self>`",
        ).into_compile_error().into();
    }

    // Prepend the derives that used to live inside the macro_rules body.
    let derive_attr: syn::Attribute = syn::parse_quote!(#[derive(Ast, Clone, Debug)]);
    input.attrs.insert(0, derive_attr);

    let f_names: Vec<_> = fields.iter().map(|f| f.ident.clone().unwrap()).collect();
    let f_types: Vec<_> = fields.iter().map(|f| f.ty.clone()).collect();

    let expanded = quote! {
        #input
        impl AstNode<#name> for #name {
            fn node_id(&self) -> NodeId<#name> {
                self.node_id
            }
        }
        impl #name {
            pub fn new(#(#f_names: #f_types),*) -> Self {
                #name { #(#f_names),* }
            }
        }
    };

    expanded.into()
}

/// Attaches the following attributes:
///     * `#[derive(Ast, Clone, Debug)]`
///     * `#[ast(transparent)]`
/// Does some checks on the well-formedness of an discriminater in the ast.
/// Importantly checks that all non-Error variants contain a field, because otherwise we lose information.
#[proc_macro_attribute]
pub fn kind(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as DeriveInput);

    let data_enum = match &input.data {
        Data::Enum(data_enum) => data_enum,
        _ => return syn::Error::new_spanned(&input.ident, "`#[kind]` should be only be on a discriminater (enum)")
                .to_compile_error()
                .into()
    };

    for variant in &data_enum.variants {
        if variant.fields.is_empty() {
            return syn::Error::new_spanned(
                &variant.ident, format!(
                    "consider adding a field to variant `{}`, otherwise this node can not be referred to because #[kinds] are transparent (i.e. have no node-id)",
                    variant.ident
                ),
            )
            .to_compile_error()
            .into();
        }
    }

    let derive_attr: syn::Attribute = syn::parse_quote!(#[derive(Ast, Clone, Debug)]);
    let ast_transparent_attr: syn::Attribute = syn::parse_quote!(#[ast(transparent)]);
    input.attrs.insert(0, derive_attr);
    input.attrs.insert(1, ast_transparent_attr);
    quote! {
        #input
    }
    .into()
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

fn emit_part(mode: &Mode, dispatcher: &Ts2) -> Result<Ts2> {
    Ok(match mode {
        Mode::Skip => quote! { (&#dispatcher).recurse(visitor); },
        Mode::Recurse => {
            quote! { (&#dispatcher).visit(visitor); }
        }
        Mode::List => {
            quote! { (&#dispatcher).iter().for_each(|__x| __x.visit(visitor)); }
        }
        Mode::Optional => {
            quote! {
                if let Some(__x) = (&#dispatcher) {
                    __x.visit(visitor);
                }
            }
        }
    })
}

// ---- expansion ----

fn expand(input: &DeriveInput) -> Result<Ts2> {
    let copts = parse_container(&input.attrs)?;
    let name = &input.ident;
    let (impl_g, ty_g, where_c) = input.generics.split_for_impl();

    let body = match &input.data {
        Data::Struct(s) => expand_struct(input, &copts, s)?,
        Data::Enum(e) => expand_enum(input, &copts, e)?,
        Data::Union(_) => return Err(Error::new_spanned(name, "Ast cannot be derived for unions")),
    };

    let visitor_fn_name = format_ident!("visit_{}", ident_case::RenameRule::SnakeCase.apply_to_variant(input.ident.to_string()));
    // if transparent then we just want to recurse because there's no visitor_fn to call
    let visit_body = if copts.transparent {
        quote!(self.recurse(visitor))
    } else {
        quote!(visitor.#visitor_fn_name(self))
    };
    eprintln!("fn {}(&mut self, {}: &{});", visitor_fn_name, ident_case::RenameRule::SnakeCase.apply_to_variant(input.ident.to_string()), input.ident);

    Ok(quote! {
        #[automatically_derived]
        impl #impl_g crate::ast::interface::Ast for #name #ty_g #where_c {
            fn recurse<V: Visitor>(&self, visitor: &mut V) {
                #body
            }
            fn visit<V: Visitor>(&self, visitor: &mut V) {
                #visit_body
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
    let parts = fields.named
        .iter()
        .map(|f| {
            let ident = f.ident.as_ref().unwrap();
            let mode = if copts.transparent { Mode::Skip } else { field_mode(f)? };
            emit_part(&mode, &quote!(self.#ident))
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(
        quote! {
            #(#parts)*
        },
    )
}

fn expand_enum(_input: &DeriveInput, copts: &ContainerOpts, e: &DataEnum) -> Result<Ts2> {
    let mut arms: Vec<Ts2> = Vec::new();
    for v in &e.variants {
        let vopts = parse_variant(&v.attrs)?;
        let vname = &v.ident;
        match &v.fields {
            Fields::Unit => {
                arms.push(quote! {
                    Self::#vname => ()
                });
            }
            Fields::Unnamed(fs) => {
                let binds: Vec<_> = fs.unnamed.iter().enumerate().map(|(i, _f)| format_ident!("__f{i}")).collect();
                let modes = fs.unnamed.iter()
                    .map(|f| if copts.transparent || vopts.transparent { Ok(Mode::Skip) } else { field_mode(f) })
                    .collect::<Result<Vec<_>>>()?;
                let parts: Vec<_> = modes
                    .iter()
                    .zip(&binds)
                    .map(|(m, b)| emit_part(m, &quote!(#b)))
                    .collect::<Result<_>>()?;
                arms.push(quote! {
                    Self::#vname(#(#binds),*) => {
                        #(#parts)*
                    }
                });
            }
            Fields::Named(fs) => {
                let names: Vec<_> = fs.named.iter().map(|f| f.ident.clone().unwrap()).collect();
                let modes = fs.named.iter()
                    .map(|f| if copts.transparent || vopts.transparent { Ok(Mode::Skip) } else { field_mode(f) })
                    .collect::<Result<Vec<_>>>()?;
                let parts: Vec<_> = modes
                    .iter()
                    .zip(fs.named.iter())
                    .map(|(m, Field { ident, .. })| emit_part(m, &quote!(#ident)))
                    .collect::<Result<_>>()?;
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
