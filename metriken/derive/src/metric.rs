use proc_macro2::{Punct, Spacing};
use proc_macro2::Literal;
use syn::ItemStatic;
use proc_macro2::{TokenStream, TokenTree};
use quote::quote;

pub(crate) fn metric(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> syn::Result<TokenStream> {
    let item: ItemStatic = syn::parse(item)?;
    let attr: TokenStream = attr.into();

    let mut attrs: TokenStream = TokenStream::new();

    let tokens: Vec<TokenTree> = attr.into_iter().collect();

    let mut idx = 0;
    let mut formatter = None;

    loop {
        if idx >= tokens.len() {
            break;
        }

        let token = tokens[idx].clone();

        match token {
            TokenTree::Ident(ref ident) => {
                let stringy = format!("{ident}");

                if stringy == "formatter" {
                    // swallow valid punctuation following `formatter`
                    idx += 1;

                    if let TokenTree::Punct(ref punct) = tokens[idx] {
                        if punct.as_char() != '=' {
                            panic!("expected a '=' following `formatter` argument");
                        }

                        idx += 1;

                        if punct.spacing() == Spacing::Joint {
                            while let TokenTree::Punct(_) = tokens[idx] { idx += 1; if idx >= tokens.len() { panic!("ran out of tokens")} }
                        }
                    } else {
                        panic!("expected a '=' following `formatter` argument");
                    }

                    // get the formatter ident
                    if let TokenTree::Punct(ref punct) = tokens[idx] {
                        if punct.as_char() != '&' {
                            panic!("expected a '&' following `formatter` argument");
                        }

                        idx += 1;

                        if let TokenTree::Ident(ref ident) = tokens[idx] {
                            formatter = Some([TokenTree::Punct(punct.clone()), TokenTree::Ident(ident.clone())]);
                        } else {
                            panic!("expected an ident got a {:?}", tokens[idx]);
                        }
                    }

                    idx += 1;

                    if idx >= tokens.len() {
                        break;
                    }

                    if let TokenTree::Punct(ref punct) = tokens[idx] {
                        if punct.as_char() == ',' {
                            idx += 1;
                        }
                    }

                    continue;
                }

                attrs.extend([TokenTree::Literal(Literal::string(&stringy))]);
            }
            TokenTree::Punct(punct) => {
                if punct.spacing() == Spacing::Alone && punct.as_char() == '=' {
                    attrs.extend([TokenTree::Punct(Punct::new('=', Spacing::Joint)), TokenTree::Punct(Punct::new('>', Spacing::Alone))]);
                } else {
                    attrs.extend([TokenTree::Punct(punct)]);
                }
            }
            _ => {
                attrs.extend([token]);
            }
        }

        idx += 1;
    };

    let ident = &item.ident;
    let ty = &item.ty;
    let expr = &item.expr;
    let name = format!("{ident}").to_lowercase();

    if formatter.is_none() && attrs.is_empty() {
        Ok(quote! {
            pub static #ident: #ty = {
                #[linkme::distributed_slice(metriken::STATIC_REGISTRY)]
                static __: metriken::StaticEntry = metriken::StaticEntry::new(
                    &#ident,
                    metriken::metadata!("name" => #name),
                    &metriken::default_formatter,
                );

                #expr
            };
        })
    } else if formatter.is_none() {
        Ok(quote! {
            pub static #ident: #ty = {
                #[linkme::distributed_slice(metriken::STATIC_REGISTRY)]
                static __: metriken::StaticEntry = metriken::StaticEntry::new(
                    &#ident,
                    metriken::metadata!(#attrs),
                    &metriken::default_formatter,
                );

                #expr
            };
        })
    } else if attrs.is_empty() {
        let formatter: TokenStream = formatter.unwrap().into_iter().collect();
        Ok(quote! {
            pub static #ident: #ty = {
                #[linkme::distributed_slice(metriken::STATIC_REGISTRY)]
                static __: metriken::StaticEntry = metriken::StaticEntry::new(
                    &#ident,
                    metriken::metadata!("name" => #name),
                    #formatter,
                );

                #expr
            };
        })
    } else {
        let formatter: TokenStream = formatter.unwrap().into_iter().collect();
        Ok(quote! {
            pub static #ident: #ty = {
                #[linkme::distributed_slice(metriken::STATIC_REGISTRY)]
                static __: metriken::StaticEntry = metriken::StaticEntry::new(
                    &#ident,
                    metriken::metadata!(#attrs),
                    #formatter,
                );

                #expr
            };
        })
    }
}