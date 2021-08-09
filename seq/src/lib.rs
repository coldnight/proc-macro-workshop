use proc_macro::{Group, Literal, TokenStream, TokenTree};
use proc_macro::token_stream::IntoIter;
use syn::{self, Ident, LitInt, Token};

struct Header {
    name: Ident,
    start: i32,
    end: i32,
}

#[proc_macro]
pub fn seq(input: TokenStream) -> TokenStream {
    match process(input) {
        Ok(v) => v,
        Err(e) => e.to_compile_error().into(),
    }
}

fn process(input: TokenStream) -> Result<TokenStream, syn::Error> {
    let mut result = TokenStream::new();
    let mut iter = input.clone().into_iter();
    let header = parse_header(&mut iter)?;
    let body = braced_body(&mut iter)?;
    for lit in header.start .. header.end {
        interrupt_ident_to_literal(&header.name, lit, body.clone().into_iter(), &mut result);
    }
    Ok(result)
}

fn parse_header(iter: &mut IntoIter) -> Result<Header, syn::Error> {
    let name = parse_header_name(iter)?;
    parse_token::<Token![in]>(iter)?;
    let start = parse_header_lit(iter)?;
    parse_token::<Token![.]>(iter)?;
    parse_token::<Token![.]>(iter)?;
    let end = parse_header_lit(iter)?;
    Ok(Header{
        name,
        start,
        end,
    })
}


fn parse_header_name(iter: &mut IntoIter) -> Result<Ident, syn::Error> {
    if let Some(tt) = iter.next() {
        let ts: TokenStream = tt.into();
        let ident: Ident = syn::parse(ts)?;
        return Ok(ident);
    }
    Err(syn::Error::new(proc_macro::Span::call_site().into(), "unexpected eof"))
}

fn parse_token<T: syn::parse::Parse>(iter: &mut IntoIter) -> Result<(), syn::Error> {
    if let Some(tt) = iter.next() {
        let ts: TokenStream = tt.into();
        let _: T = syn::parse(ts)?;
        return Ok(());
    }
    Err(syn::Error::new(proc_macro::Span::call_site().into(), "unexpected eof"))

}

fn parse_header_lit(iter: &mut IntoIter) -> Result<i32, syn::Error> {
    if let Some(tt) = iter.next() {
        let ts: TokenStream = tt.into();
        let lit: LitInt = syn::parse(ts)?;
        let v = lit.base10_parse::<i32>()?;
        return Ok(v);
    }
    Err(syn::Error::new(proc_macro::Span::call_site().into(), "unexpected eof"))
}

fn braced_body(iter: &mut IntoIter) -> Result<TokenStream, syn::Error> {
    if let Some(tt) = iter.next() {
        if let proc_macro::TokenTree::Group(g) = tt {
            match g.delimiter() {
                proc_macro::Delimiter::Brace => return Ok(g.stream()),
                _ => {
                    let err = syn::Error::new(g.span().into(), "unexpected delimiter");
                    return Err(err);
                }
            }
        }
        return Err(syn::Error::new(tt.span().into(), "a group is expected"));
    }
    return Err(syn::Error::new(proc_macro::Span::call_site().into(), "unexpected eof"));
}

fn interrupt_ident_to_literal(name: &Ident, lit: i32, iter: IntoIter, output: &mut TokenStream) {
    for tt in iter {
        if let TokenTree::Group(g) = &tt {
            let mut tmp = TokenStream::new();
            interrupt_ident_to_literal(name, lit, g.stream().into_iter(), &mut tmp);
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
