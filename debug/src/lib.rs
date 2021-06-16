use proc_macro::TokenStream;
use quote::quote;
use syn::{self, parse, DeriveInput};


#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = parse(input).unwrap();
    let name = &ast.ident;

    let mut debug_fields = vec![];

    if let syn::Data::Struct(ds) = ast.data {
        if let syn::Fields::Named(fields) = ds.fields {
            for field in fields.named.iter() {
                let field_name = field.ident.clone().unwrap();
                if let Some(attr) = field.attrs.clone().iter().next() {
                    match parse_debug_attr_value(&attr) {
                        Ok(v) => {
                            if let Some(value) = v {
                                debug_fields.push(quote!{
                                    field(stringify!(#field_name), &format_args!(#value, &self.#field_name))
                                });
                                continue;
                            }
                        },
                        Err(err) => return err.to_compile_error().into(),
                    }
                }
                debug_fields.push(quote!{
                    field(stringify!(#field_name), &self.#field_name)
                });
            }
        }
    }
    let tokens = quote!{
        impl std::fmt::Debug for #name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct(stringify!(#name))
                    .#(#debug_fields).*
                    .finish()
            }
        }
    };
    tokens.into()
}
fn parse_debug_attr_value(attr: &syn::Attribute) -> Result<Option<syn::LitStr>, syn::Error> {
    if let Some(seg) = attr.path.segments.first() {
        if seg.ident != "debug" {
            return Ok(None);
        }
        let meta: syn::Meta = attr.parse_meta()?;
        match meta {
            syn::Meta::NameValue(values) => {
                let arg_name = &values.path.segments.first().unwrap().ident;
                if arg_name == "debug" {
                    if let syn::Lit::Str(name) = values.lit {
                        return Ok(Some(name));
                    }
                } else {
                    let start = attr.path.segments.first().unwrap().ident.span();
                    return Err(syn::Error::new(start, "expected `builder(each = \"...\")`".to_owned()));
                }
            },
            _ => {
                return Err(syn::Error::new(attr.bracket_token.span, "expected `debug = `\"...\"`".to_owned()));
            },
        }
    }
    return Ok(None);
}
