use proc_macro::{Delimiter, Group, Literal, TokenStream, TokenTree};
use proc_macro::token_stream::IntoIter;
use quote::ToTokens;
use syn::{self, Ident, LitInt, Token};

struct Header {
    name: Ident,
    start: i32,
    end: i32,
}

#[derive(Debug)]
enum Segment {
    Normal(TokenStream),
    Repeat(TokenStream),
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
    let mut segments = vec![];
    if find_repeat_section(body.clone().into_iter(), &mut segments) {
        for seg in segments {
            match seg {
                Segment::Normal(n) => result.extend(n),
                Segment::Repeat(r) => {
                    for lit in header.start .. header.end {
                        interrupt_ident_to_literal(&header.name, lit, r.clone().into_iter(), &mut result);
                    }
                }
            }
        }
        eprintln!("{}", result);
    } else {
        for lit in header.start .. header.end {
            interrupt_ident_to_literal(&header.name, lit, body.clone().into_iter(), &mut result);
        }
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

// interrupt N to lit
fn interrupt_ident_to_literal(name: &Ident, lit: i32, iter: IntoIter, output: &mut TokenStream) {
    let mut sharp = None;
    let mut ident_before_sharp: Option<TokenTree> = None;
    for tt in iter {
        match &tt {
            TokenTree::Group(g) => {
                if let Some(ident0) = &ident_before_sharp {
                    output.extend::<TokenStream>(ident0.clone().into());
                    ident_before_sharp = None;
                }

                let mut tmp = TokenStream::new();
                interrupt_ident_to_literal(name, lit, g.stream().into_iter(), &mut tmp);
                let mut new_g = Group::new(g.delimiter(), tmp);
                new_g.set_span(g.span());
                let new_tt: TokenTree = new_g.into();
                output.extend::<TokenStream>(new_tt.into());
            }
            TokenTree::Punct(p) => {
                // 如果检测到 ident_before_sharp#name 则合并 ident_before_sharp 和 name 追加到结果；
                // 否则需要将 ident_before_sharp 作为单独的 ident 追加到结果。
                if p.as_char() == '#' {
                    sharp = Some(tt.clone());
                    continue
                }


                if let Some(ident0) = &ident_before_sharp {
                    output.extend::<TokenStream>(ident0.clone().into());
                    ident_before_sharp = None;
                }
                // append the sharp if has any
                if let Some(s) = &sharp {
                    output.extend::<TokenStream>(s.clone().into());
                    sharp = None;
                }
                output.extend::<TokenStream>(tt.into());
            }
            TokenTree::Ident(i) => {
                if i.to_string() == name.to_string() {
                    let t: TokenTree = Literal::i32_unsuffixed(lit).into();
                    let mut ts: TokenStream = t.into();
                    if let Some(_) = sharp {
                        if let Some(ident0) = &ident_before_sharp {
                            let new_name = format!("{}{}", ident0, lit);
                            let new_ident = syn::Ident::new(&&new_name, ident0.span().into());
                            ts = new_ident.to_token_stream().into();
                            ident_before_sharp = None; // avoid duplicated extend to result
                        }
                        sharp = None;
                    }
                    output.extend(ts);
                    continue
                }

                if let Some(ident0) = &ident_before_sharp {
                    output.extend::<TokenStream>(ident0.clone().into());
                }
                ident_before_sharp = Some(tt.clone());
            }
            _ => {
                if let Some(ident0) = &ident_before_sharp {
                    output.extend::<TokenStream>(ident0.clone().into());
                    ident_before_sharp = None;
                }
                // append the sharp if has any
                if let Some(s) = &sharp {
                    output.extend::<TokenStream>(s.clone().into());
                    sharp = None;
                }
                output.extend::<TokenStream>(tt.into());
            }
        }

    }
}

fn find_repeat_section(iter: IntoIter, segments: &mut Vec<Segment>) -> bool {
    let mut found = false;
    let mut sharp: Option<TokenTree> = None;
    let mut repeat: Option<TokenStream> = None;
    for tt in iter {
        match &tt {
            TokenTree::Group(g) => {
                if sharp.is_some() && g.delimiter() == Delimiter::Parenthesis{
                    repeat = Some(g.stream());
                } else {
                    if let Some(s) = &sharp {
                        segments.push(Segment::Normal(s.clone().into()));
                        sharp = None;
                    }
                    repeat = None;
                    if find_repeat_section(g.stream().into_iter(), segments) {
                        found = true;
                        continue;
                    }
                    segments.push(Segment::Normal(tt.clone().into()));
                }
            }
            TokenTree::Punct(p) => {
                match p.as_char() {
                    '#' => sharp = Some(tt.clone()),
                    '*' => {
                        if sharp.is_some() && repeat.is_some() {
                            if let Some(r) = &repeat {
                                segments.push(Segment::Repeat(r.clone().into()));
                                found = true;
                            }
                            continue;
                        }

                        if let Some(s) = &sharp {
                            segments.push(Segment::Normal(s.clone().into()));
                        }
                        if let Some(r) = &repeat {
                            segments.push(Segment::Normal(r.clone().into()));
                        }
                        sharp = None;
                        repeat = None;
                        segments.push(Segment::Normal(tt.clone().into()));
                    }
                    _ => {
                        if let Some(s) = &sharp {
                            segments.push(Segment::Normal(s.clone().into()));
                        }
                        if let Some(r) = &repeat {
                            segments.push(Segment::Normal(r.clone().into()));
                        }
                        sharp = None;
                        repeat = None;
                        segments.push(Segment::Normal(tt.clone().into()));
                    }
                }
            }
            _ => {
                if let Some(s) = &sharp {
                    segments.push(Segment::Normal(s.clone().into()));
                }
                if let Some(r) = &repeat {
                    segments.push(Segment::Normal(r.clone().into()));
                }
                sharp = None;
                repeat = None;
                segments.push(Segment::Normal(tt.clone().into()))
            }
        }
    }
    found
}
