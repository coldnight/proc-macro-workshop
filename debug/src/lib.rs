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

            for p in ast.generics.params.iter() {
                if let syn::GenericParam::Type(t) = p {
                    let name = &t.ident;
                    params.push(name);
                    if is_param_wrapped_phantom(name, &fields) {
                        wheres.push(quote!(std::marker::PhantomData<#name>: std::fmt::Debug))
                    } else {
                        wheres.push(quote!(#name: std::fmt::Debug))
                    }
                }
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


fn is_param_wrapped_phantom(name: &syn::Ident, fields: &syn::FieldsNamed) -> bool {
    let mut ret = false;
    for field in fields.named.iter() {
        let field_ty = &field.ty;
        if let syn::Type::Path(ty_pth) = field_ty {
            for seg in ty_pth.path.segments.iter() {
                // type directly use
                if &seg.ident == name {
                    return false;
                }
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    for item in args.args.iter() {
                        if let syn::GenericArgument::Type(ty) = item {
                            if let syn::Type::Path(ty_path) = ty {
                                if &(ty_path.path.segments.iter().next().unwrap().ident) == name {
                                    // if container is PhantomData then T is wrapped via PhantomData,
                                    // otherwise we return false immedialy.
                                    if seg.ident == "PhantomData" {
                                        ret = true;
                                    } else {
                                        return false;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    return ret;
}
