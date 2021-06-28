use proc_macro::TokenStream;

use syn::{self, Ident, Token, braced, parse_macro_input};
use syn::parse::{Parse};


struct Seq {
    name: Ident,
    start: u32,
    end: u32,
    tokens: TokenStream,
}

impl Parse for Seq {
    fn parse(input: &syn::parse::ParseBuffer<'_>) -> std::result::Result<Self, syn::Error> {
        let name: Ident = input.parse()?;
        input.parse::<Token![in]>()?;
        let start: u32 = {
            let lit: syn::LitInt = input.parse()?;
            lit.base10_parse::<u32>()?
        };
        input.parse::<Token![..]>()?;
        let end: u32 = {
            let lit: syn::LitInt = input.parse()?;
            lit.base10_parse::<u32>()?
        };
        let content;
        let _ = braced!(content in input);

        Ok(Seq{
            name: name,
            start: start,
            end: end,
            tokens: content.cursor().token_stream().into(),
        })
    }
}

#[proc_macro]
pub fn seq(input: TokenStream) -> TokenStream {
    let s = parse_macro_input!(input as Seq);
    let result = TokenStream::new();
    result
}
