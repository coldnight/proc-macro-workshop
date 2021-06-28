use proc_macro::{TokenStream, TokenTree, Group, Literal};
use syn::{self, Ident, Token, braced, parse_macro_input, Expr};
use syn::punctuated::Punctuated;
use syn::parse::{Parse};
use quote::{ToTokens};

struct Seq {
    name: Ident,
    start: i32,
    end: i32,
    exprs: Punctuated<Expr, Token![;]>,
}

impl Parse for Seq {
    fn parse(input: &syn::parse::ParseBuffer<'_>) -> std::result::Result<Self, syn::Error> {
        let name: Ident = input.parse()?;
        input.parse::<Token![in]>()?;
        let start: i32 = {
            let lit: syn::LitInt = input.parse()?;
            lit.base10_parse::<i32>()?
        };
        input.parse::<Token![..]>()?;
        let end: i32 = {
            let lit: syn::LitInt = input.parse()?;
            lit.base10_parse::<i32>()?
        };
        let content;
        let _ = braced!(content in input);

        Ok(Seq{
            name: name,
            start: start,
            end: end,
            exprs: content.parse_terminated(Expr::parse)?,
        })
    }
}

#[proc_macro]
pub fn seq(input: TokenStream) -> TokenStream {
    let s = parse_macro_input!(input as Seq);
    let mut result = TokenStream::new();
    let p: TokenStream = syn::parse_str::<Token![;]>(";").unwrap().to_token_stream().into();
    for lit in s.start .. s.end {
        for expr in s.exprs.clone().iter() {
            let ts: TokenStream = expr.into_token_stream().into();
            interrupt_ident_to_literal(&s.name, lit, ts, &mut result);
            result.extend(p.clone());
        }
    }
    result
}


fn interrupt_ident_to_literal(name: &Ident, lit: i32, input: TokenStream, output: &mut TokenStream) {
    for tt in input.into_iter() {
        if let TokenTree::Group(g) = &tt {
            let mut tmp = TokenStream::new();
            interrupt_ident_to_literal(name, lit, g.stream(), &mut tmp);
            let mut new_g = Group::new(g.delimiter(), tmp);
            new_g.set_span(g.span());
            let new_tt: TokenTree = new_g.into();
            let new_ts: TokenStream = new_tt.into();
            output.extend(new_ts);
            continue
        }
        if let TokenTree::Ident(i) = &tt {
            if i.to_string() == name.to_string() {
                let ts: TokenTree = Literal::i32_unsuffixed(lit).into();
                output.extend::<TokenStream>(ts.into());
                continue
            }
        }
        let ts: TokenStream = tt.into();
        output.extend(ts);
    }
}
