use proc_macro::{TokenStream};
use regex::Regex;
use quote::{format_ident, quote};
use syn::{parse::Parse, parse::ParseStream, parse_macro_input, ItemFn, LitStr, Token};
use std::path::PathBuf;

struct GenTestsArgs {
    path: String,
    matches: String,
}

impl Parse for GenTestsArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut path = None;
        let mut matches = None;

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let value: LitStr = input.parse()?;

            match ident.to_string().as_str() {
                "path" => path = Some(value.value()),
                "matches" => matches = Some(value.value()),
                other => {
                    return Err(syn::Error::new(ident.span(), format!("unknown arg: {other}")))
                }
            }

            // consume optional trailing comma
            let _ = input.parse::<Token![,]>();
        }

        Ok(GenTestsArgs {
            path: path
                .ok_or_else(|| syn::Error::new(proc_macro2::Span::call_site(), "missing `path`"))?,
            matches: matches.ok_or_else(|| {
                syn::Error::new(proc_macro2::Span::call_site(), "missing `matches`")
            })?,
        })
    }
}

/// Sanitize a filename into a valid Rust identifier suffix.
fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

#[proc_macro_attribute]
pub fn generate_tests(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as GenTestsArgs);
    let func = parse_macro_input!(item as ItemFn);

    let func_name = &func.sig.ident;
    let func_block = &func.block;

    let re = Regex::new(&args.matches).unwrap_or_else(|e| {
        panic!("invalid regex in `matches`: {e}");
    });

    // Resolve the directory relative to the workspace/crate root (CARGO_MANIFEST_DIR).
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let base = PathBuf::from(&manifest_dir).join(&args.path);

    let mut entries: Vec<PathBuf> = std::fs::read_dir(&base)
        .unwrap_or_else(|e| panic!("cannot read dir {}: {e}", base.display()))
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.is_file() {
                let name = path.file_name()?.to_str()?;
                if re.is_match(name) {
                    return Some(path);
                }
            }
            None
        })
        .collect();

    entries.sort();

    let tests = entries.iter().map(|full_path| {
        let file_stem = full_path.file_stem().unwrap().to_str().unwrap();
        let test_name = format_ident!("{}_{}", func_name, sanitize(file_stem));

        // Store as a string literal so the path is resolved at runtime.
        let path_str = full_path.to_str().unwrap();

        quote! {
            #[test]
            fn #test_name() {
                let input = ::std::path::Path::new(#path_str);
                // Inline the body — `input` is bound to the path.
                #func_block
            }
        }
    });

    TokenStream::from(quote! { #(#tests)* })
}
