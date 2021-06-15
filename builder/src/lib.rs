use proc_macro::{TokenStream};
use quote::quote;
use syn::{parse, DeriveInput, Data, parse_str, Ident, self};

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = parse(input).unwrap();
    let name = &ast.ident;
    let bn = format!("{}Builder", name);
    let builder_name: Ident = parse_str(&bn).unwrap();
    let mut tokens = TokenStream::new();
    match ast.data {
        Data::Struct(s) => {
            tokens.extend(construct_builder(&builder_name, &s));
            tokens.extend(add_builder_method_to_target(&name, &builder_name, &s));
            tokens.extend(impl_builder(&name, &builder_name, &s));
        }
        _ => unimplemented!()
    }
    tokens
}


fn construct_builder(builder_name: &Ident, s: &syn::DataStruct) -> TokenStream {
    let mut builder_fields = vec![];
    for f in s.fields.iter() {
        let ty = &f.ty;
        let field_name = &f.ident.as_ref().unwrap();

        if let Some(ty) = try_extract_option(ty) {
            builder_fields.push(quote!{
                #field_name: Option<#ty>,
            });
        } else if let Some(_) = try_extract_vec(ty) {
            builder_fields.push(quote!{
                #field_name: #ty,
            });
        } else {
            builder_fields.push(quote!{
                #field_name: Option<#ty>,
            });
        }
    }

    let builder = quote! {
        #[derive(Clone)]
        pub struct #builder_name {
            #(#builder_fields)*
        }
    };
    builder.into()
}

fn add_builder_method_to_target(name: &Ident, builder_name: &Ident, s: &syn::DataStruct)  -> TokenStream {
    let mut builder_init = vec![];
    for f in s.fields.iter() {
        let ty = &f.ty;
        let field_name = &f.ident.as_ref().unwrap();
        if let Some(_) = try_extract_vec(ty) {
            builder_init.push(quote!{
                #field_name: vec![],
            });
        } else {
            builder_init.push(quote!{
                #field_name: None,
            });
        }

    }

    let builder = quote! {
        impl #name {
            pub fn builder() -> #builder_name {
                #builder_name {
                    #(#builder_init)*
                }
            }
        }
    };
    builder.into()
}


// Option<T> -> T
fn try_extract_option(ty: &syn::Type) -> Option<&syn::Type> {
    return try_extract_first_generic_param(ty, "Option");
}


// Vec<T> -> T
fn try_extract_vec(ty: &syn::Type) -> Option<&syn::Type> {
    return try_extract_first_generic_param(ty, "Vec");
}

fn try_extract_first_generic_param<'a, 'b>(ty: &'a syn::Type, ident: &'b str) -> Option<&'a syn::Type> {
    match ty {
        syn::Type::Path(pth) => {
            if let Some(i) = pth.path.segments.first() {
                if  i.ident == ident {
                    match &i.arguments {
                        syn::PathArguments::AngleBracketed(args) => {
                            if let Some(a) = args.args.first() {
                                match a {
                                    syn::GenericArgument::Type(ty) => {
                                        return Some(ty)
                                    },
                                    _ => {},
                                }
                            }
                        },
                        _ => {},
                    };
                }
            }
        },
        _ => {},
    }
    return None;
}

fn impl_builder(name: &Ident, builder_name: &Ident, s: &syn::DataStruct) -> TokenStream {
    let mut tokens = TokenStream::new();
    tokens.extend(impl_builder_set_funcs(builder_name, s));
    tokens.extend(impl_builder_dot_build(name, builder_name, s));
    tokens
}

fn impl_builder_set_funcs(builder_name: &Ident, s: &syn::DataStruct) -> TokenStream {
    let mut tokens = TokenStream::new();
    for f in s.fields.iter() {
        if let Some(t) = try_impl_field_repeat(builder_name, f) {
            tokens.extend(t);
        } else {
            tokens.extend(impl_field_setter(builder_name, f));
        }
    }
    tokens
}

fn impl_field_setter(builder_name: &Ident, f: &syn::Field) -> TokenStream {
    let ty = if let Some(ty) = try_extract_option(&f.ty) {
        ty
    } else {
        &f.ty
    };
    let field_name = &f.ident.as_ref().unwrap();
    if let Some(_) = try_extract_vec(ty) {
        let expanded = quote! {
            impl #builder_name {
                pub fn #field_name(&mut self, #field_name: #ty) -> Self {
                    self.#field_name = #field_name;
                    self.clone()
                }
            }
        };
        expanded.into()
    } else {
        let expanded = quote! {
            impl #builder_name {
                pub fn #field_name(&mut self, #field_name: #ty) -> Self {
                    self.#field_name = Some(#field_name);
                    self.clone()
                }
            }
        };
        expanded.into()
    }
}

fn try_impl_field_repeat(builder_name: &Ident, f: &syn::Field) -> Option<TokenStream> {
    let field_name = &f.ident.as_ref().unwrap();
    if let Some(ty) = try_extract_vec(&f.ty) {
        for attr in f.attrs.iter() {
            match parse_each_attr_value(attr) {
                Ok(v) => {
                    if let Some(field_each) = v {
                        let expanded = quote!{
                            impl #builder_name {
                                pub fn #field_each(&mut self, #field_each: #ty) -> Self {
                                    self.#field_name.push(#field_each);
                                    self.clone()
                                }
                            }
                        };
                        return Some(expanded.into());
            }
                },
                Err(v) => {
                    return Some(v.to_compile_error().into());
                },
            };
        }
    }
    None
}

fn parse_each_attr_value(attr: &syn::Attribute) -> Result<Option<Ident>, syn::Error> {
    if let Some(seg) = attr.path.segments.first() {
        if seg.ident == "builder" {
            let args = attr.parse_args()?;
            if let syn::Meta::NameValue(values) = args {
                let arg_name = &values.path.segments.first().unwrap().ident;
                if arg_name == "each" {
                    if let syn::Lit::Str(name) = values.lit {
                        let ident: Ident = parse_str(&name.value())?;
                        return Ok(Some(ident));
                    }
                } else {
                    return Err(syn::Error::new_spanned(arg_name, "expected `builder(each = \"...\")`".to_owned()));
                }
            }
        }
    }
    return Ok(None)
}

// pub fn build() -> Result<T, String>
fn impl_builder_dot_build(name: &Ident, builder_name: &Ident, s: &syn::DataStruct) -> TokenStream {
    let mut checks = vec![];
    let mut fields = vec![];

    for f in s.fields.iter() {
        let ty = &f.ty;
        let field_name = &f.ident.as_ref().unwrap();
        if let Some(_) = try_extract_option(ty) {
            checks.push(quote!{
                let #field_name = self.#field_name;
            });
        } else if let Some(_) = try_extract_vec(ty) {
            checks.push(quote!{
                let #field_name = self.#field_name;
            });
        } else {
            checks.push(quote!{
                let #field_name = match self.#field_name {
                    Some(f) => f,
                    None => return None,
                }
            });
        }
        fields.push(quote!{
           #field_name: #field_name,
        });
    }

    let tokens = quote! {
        impl #builder_name {
            pub fn build(self) -> Option<#name> {
                #(#checks);*;

                let ret = #name {
                    #(#fields)*
                };
                Some(ret)

            }
        }
    };
    tokens.into()
}
