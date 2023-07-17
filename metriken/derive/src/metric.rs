// Copyright 2021 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use std::collections::HashMap;

use crate::args::ArgName;
use proc_macro2::{Span, TokenStream};
use proc_macro_crate::FoundCrate;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{parse_quote, Error, Expr, Ident, ItemStatic, Path, Token};

/// A single argument to an attribute macro.
///
/// ```text
/// #[macro(name = value, a = "string")]
///         ^^^^^^^^^^^^  ^^^^^^^^^^^^
/// ```
struct SingleArg<T> {
    ident: ArgName,
    eq: Token![=],
    value: T,
}

impl<T: Parse> Parse for SingleArg<T> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            ident: input.parse()?,
            eq: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl<T: ToTokens> ToTokens for SingleArg<T> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.ident.to_tokens(tokens);
        self.eq.to_tokens(tokens);
        self.value.to_tokens(tokens);
    }
}

/// All arguments to the metric attribute macro
///
/// ```text
/// #[metric(formatter = &fmt, name = "metric")]
///          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
/// ```
///
/// The parse implementation for this type separates out the `formatter` and
/// `krate` arguments. All others are passed verbatim to the
/// `metriken::metadata!` macro.
#[derive(Default)]
struct MetricArgs {
    formatter: Option<SingleArg<Expr>>,
    krate: Option<SingleArg<Path>>,
    attrs: HashMap<String, Expr>,
}

impl Parse for MetricArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut args = MetricArgs::default();
        let mut first = true;

        fn duplicate_arg_error(
            span: Span,
            arg: &impl std::fmt::Display,
        ) -> syn::Result<MetricArgs> {
            Err(Error::new(
                span,
                format!("Unexpected duplicate argument '{}'", arg),
            ))
        }

        // # How parsing works
        // We first peek at the next token and use that to determine which
        // argument to parse. `formatter` and `crate` are handled specially
        // while anything else is put into the attrs map.
        while !input.is_empty() {
            if !first {
                let _: Token![,] = input.parse()?;
            }
            first = false;

            let arg: ArgName = input.fork().parse()?;
            match &*arg.to_string() {
                "formatter" => {
                    let formatter = input.parse()?;
                    match args.formatter {
                        None => args.formatter = Some(formatter),
                        Some(_) => return duplicate_arg_error(formatter.span(), &arg),
                    }
                }
                "crate" => {
                    let krate = SingleArg {
                        ident: input.parse()?,
                        eq: input.parse()?,
                        value: Path::parse_mod_style(input)?,
                    };
                    match args.krate {
                        None => args.krate = Some(krate),
                        Some(_) => return duplicate_arg_error(krate.span(), &arg),
                    }
                }
                _ => {
                    let entry: SingleArg<Expr> = input.parse()?;
                    let ident = entry.ident.to_string();
                    if args.attrs.contains_key(&ident) {
                        return duplicate_arg_error(entry.span(), &entry.ident);
                    }

                    args.attrs.insert(ident, entry.value);
                }
            }
        }

        Ok(args)
    }
}

pub(crate) fn metric(
    attr_: proc_macro::TokenStream,
    item_: proc_macro::TokenStream,
) -> syn::Result<TokenStream> {
    let mut item: ItemStatic = syn::parse(item_)?;
    let mut args: MetricArgs = syn::parse(attr_)?;

    let krate: Path = match args.krate {
        Some(krate) => krate.value,
        None => proc_macro_crate::crate_name("metriken")
            .map(|krate| match krate {
                FoundCrate::Name(name) => {
                    assert_ne!(name, "");
                    Ident::new(&name, Span::call_site()).into()
                }
                FoundCrate::Itself => parse_quote! { metriken },
            })
            .unwrap_or(parse_quote! { metriken }),
    };

    let static_name = &item.ident;
    let static_expr = &item.expr;
    let private: Path = parse_quote!(#krate::__private);

    if !args.attrs.contains_key("name") {
        args.attrs
            .insert("name".to_string(), parse_quote!(stringify!(#static_name)));
    }

    let formatter = args
        .formatter
        .map(|fmt| fmt.value)
        .unwrap_or_else(|| parse_quote!(&#krate::default_formatter));

    let attrs: Vec<_> = args
        .attrs
        .iter()
        .map(|(key, value)| quote!( #key => #value ))
        .collect();

    item.expr = Box::new(parse_quote! {{
        #[#private::linkme::distributed_slice(#krate::STATIC_REGISTRY)]
        #[linkme(crate = #private::linkme)]
        static __: #krate::StaticEntry = #krate::StaticEntry::new(
            &#static_name,
            #krate::metadata!(#( #attrs ),*),
            #formatter
        );

        #static_expr
    }});

    Ok(quote! { #item })
}
