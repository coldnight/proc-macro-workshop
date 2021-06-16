use proc_macro::TokenStream;
use quote::quote;
use syn::{self, parse, DeriveInput};

#[proc_macro_derive(CustomDebug)]
pub fn derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = parse(input).unwrap();
    let name = &ast.ident;

    let mut debug_fields = vec![];

    if let syn::Data::Struct(ds) = ast.data {
        if let syn::Fields::Named(fields) = ds.fields {
            for field in fields.named.iter() {
                let field_name = field.ident.clone().unwrap();
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
