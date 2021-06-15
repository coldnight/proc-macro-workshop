use proc_macro::{TokenStream,};
use quote::quote;
use syn::{parse, DeriveInput, Data, parse_str, Ident, self};

#[proc_macro_derive(Builder)]
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
        let field_name = &f.ident.as_ref().unwrap();
        builder_init.push(quote!{
            #field_name: None,
        });

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

fn try_extract_option(ty: &syn::Type) -> Option<&syn::Type> {
    match ty {
        syn::Type::Path(pth) => {
            if let Some(i) = pth.path.segments.first() {
                if  i.ident == "Option" {
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
        let ty = if let Some(ty) = try_extract_option(&f.ty) {
            ty
        } else {
            &f.ty
        };
        let field_name = &f.ident.as_ref().unwrap();
        let expanded = quote! {
            impl #builder_name {
                pub fn #field_name(&mut self, #field_name: #ty) -> Self {
                    self.#field_name = Some(#field_name);
                    self.clone()
                }
            }
        };
        let token: TokenStream = expanded.into();
        tokens.extend(token);
    }
    tokens
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
