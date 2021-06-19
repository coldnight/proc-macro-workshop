use proc_macro::TokenStream;
use quote::quote;
use syn::{self, parse, DeriveInput};


#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = parse(input).unwrap();
    let name = &ast.ident;

    let mut debug_fields = vec![];

    let mut params = vec![];
    let mut wheres = vec![];

    if let syn::Data::Struct(ds) = ast.data {
        if let syn::Fields::Named(fields) = ds.fields {
            for field in fields.named.iter() {
                let field_ty = &field.ty;
                let field_name = &field.ident;
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

                let generics = &ast.generics;

                if is_wrapped_generic_param(field_ty, &|ty| is_generic_param(ty, generics)) {
                    if is_generic_param(field_ty, generics)  || is_debug_container(field_ty) {
                        wheres.push(quote!(#field_ty: std::fmt::Debug));
                    } else if let Some(tp) = unwrap_generics_param(field_ty, &|ty| is_generic_param(ty, generics)){
                        let mut names = vec![];
                        for seg in tp.path.segments.iter() {
                            names.push(&seg.ident);
                        }
                        wheres.push(quote!(#(#names)::*: std::fmt::Debug));
                    }
                }
            }
        }
    }
    for p in ast.generics.params.iter() {
        if let syn::GenericParam::Type(t) = p {
            let name = &t.ident;
            let mut trait_bounds = vec![];
            for b in t.bounds.iter() {
                if let syn::TypeParamBound::Trait(tb) = b {
                    let mut seg_idents = vec![];
                    for seg in tb.path.segments.iter() {
                        seg_idents.push(&seg.ident);
                    }
                    trait_bounds.push(quote!(#(#seg_idents)::*))
                }
            }
            if trait_bounds.len() > 0 {
                params.push(quote!(#name: #(#trait_bounds)+*));
            } else {
                params.push(quote!(#name));
            }
        }
    }
    if params.len() > 0 {
        let tokens = quote!{
            impl<#(#params),*> std::fmt::Debug for #name<#(#params)*> where #(#wheres),* {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.debug_struct(stringify!(#name))
                        .#(#debug_fields).*
                        .finish()
                }
            }
        };
        tokens.into()
    } else {
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
                    return Err(syn::Error::new(start, "expected `debug = \"...\"`".to_owned()));
                }
            },
            _ => {
                return Err(syn::Error::new(attr.bracket_token.span, "expected `debug = `\"...\"`".to_owned()));
            },
        }
    }
    return Ok(None);
}

// returns true if generic param wrapped via others like PhantomData
fn is_wrapped_generic_param(field_ty: &syn::Type, is_generic_param: &dyn Fn(&syn::Type) -> bool) -> bool {
    if let Some(_) = unwrap_generics_param(field_ty, is_generic_param) {
        return true;
    }
    false
}


fn is_generic_param(field_ty: &syn::Type, generics: &syn::Generics) -> bool {
    if let syn::Type::Path(ty) = field_ty {
        if ty.path.segments.len() == 0 {
            return false;
        }
        let name = ty.path.segments.first().unwrap();

        for p in generics.params.iter() {
            if let syn::GenericParam::Type(t) = p {
                if t.ident == name.ident {
                    return true
                }
            }
        }
    }
    return false;
}

fn is_debug_container(ty: &syn::Type) -> bool {
    if let syn::Type::Path(tp) = ty {
        if tp.path.segments.len() == 0 {
            return false;
        }
        return tp.path.segments.first().unwrap().ident == "PhantomData";
    }
    return false;
}


fn unwrap_generics_param<'a>(field_ty: &'a syn::Type, is_generic_param: &dyn Fn(&'a syn::Type) -> bool) -> Option<&'a syn::TypePath> {
    if is_generic_param(field_ty) {
        if let syn::Type::Path(ret) = field_ty {
            return Some(ret);
        }
    }

    if let syn::Type::Path(ty) = field_ty {
        for seg in ty.path.segments.iter() {
            if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                for item in args.args.iter() {
                    if let syn::GenericArgument::Type(ty) = item {
                        return unwrap_generics_param(ty, is_generic_param);
                    }
                }
            }
        }
    }
    return None;
}
