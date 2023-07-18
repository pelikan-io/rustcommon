// Copyright 2021 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

use crate::args::{ArgName, Metadata, MetadataEntry, MetadataName, SingleArg, SingleArgExt};
use proc_macro2::{Span, TokenStream};
use proc_macro_crate::FoundCrate;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{parse_quote, Expr, Ident, ItemStatic, Path, Token};

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
    metadata: Option<SingleArg<Metadata>>,
    formatter: Option<SingleArg<Expr>>,
    krate: Option<SingleArg<Path>>,
    name: Option<SingleArg<Expr>>,
    description: Option<SingleArg<Expr>>,
}

impl Parse for MetricArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut args = MetricArgs::default();
        let mut first = true;

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
                "metadata" => args.metadata.insert_or_duplicate(input.parse()?)?,
                "name" => args.name.insert_or_duplicate(input.parse()?)?,
                "description" => args.description.insert_or_duplicate(input.parse()?)?,
                "formatter" => args.formatter.insert_or_duplicate(input.parse()?)?,
                "crate" => {
                    let krate = SingleArg {
                        ident: input.parse()?,
                        eq: input.parse()?,
                        value: Path::parse_mod_style(input)?,
                    };

                    args.krate.insert_or_duplicate(krate)?
                }
                _ => {
                    return Err(syn::Error::new(
                        arg.span(),
                        format!("unknown argument `{arg}`"),
                    ))
                }
            }
        }

        Ok(args)
    }
}

impl MetricArgs {
    fn crate_path(&mut self) -> Path {
        match self.krate.take() {
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
        }
    }
}

#[derive(Default)]
struct MetadataMap(BTreeMap<String, MetadataEntry>);

impl MetadataMap {
    fn insert(&mut self, entry: MetadataEntry) -> syn::Result<()> {
        match self.0.entry(entry.name.value()) {
            Entry::Occupied(_) => {
                return Err(syn::Error::new_spanned(
                    &entry.name,
                    format_args!("duplicate metadata entry `{}`", entry.name.value()),
                ))
            }
            Entry::Vacant(vacant) => {
                vacant.insert(entry);
            }
        }

        Ok(())
    }

    fn insert_arg(&mut self, arg: SingleArg<Expr>) -> syn::Result<()> {
        let entry = MetadataEntry {
            name: MetadataName::Ident(arg.ident.to_ident()),
            eq: arg.eq,
            value: arg.value,
        };

        let name = entry.name.value();

        self.insert(entry).map_err(|e| {
            syn::Error::new(
                e.span(),
                format_args!("`{name}` also specified as part of the metadata"),
            )
        })
    }
}

pub(crate) fn metric(
    attr_: proc_macro::TokenStream,
    item_: proc_macro::TokenStream,
) -> syn::Result<TokenStream> {
    let mut item: ItemStatic = syn::parse(item_)?;
    let args: MetricArgs = syn::parse(attr_)?;

    let krate = args.crate_path();

    let static_name = &item.ident;
    let static_expr = &item.expr;
    let private: Path = parse_quote!(#krate::export);

    let mut metadata = MetadataMap::default();
    if let Some(data) = args.metadata {
        for entry in data.value.entries {
            metadata.insert(entry)?;
        }
    }

    if let Some(name) = args.name {
        metadata.insert_arg(name)?;
    }

    if let Some(description) = args.description {
        metadata.insert_arg(description)?;
    }

    let formatter = args
        .formatter
        .map(|fmt| fmt.value)
        .unwrap_or_else(|| parse_quote!(&#krate::default_formatter));

    let attrs: Vec<_> = metadata
        .0
        .into_iter()
        .map(|(_, entry)| {
            let key = entry.name.to_literal();
            let value = entry.value;

            quote!( #key => #value )
        })
        .collect();

    item.expr = Box::new(parse_quote! {{
        // Rustc reserves attributes that start with "rustc". Since rustcommon
        // starts with "rustc" we can't use it directly within attributes. To
        // work around this, we first import the exports submodule and then use
        // that for the attributes.
        use #krate::export;

        #[export::linkme::distributed_slice(export::METRICS)]
        #[linkme(crate = export::linkme)]
        static __: #krate::MetricEntry = #krate::MetricEntry::_new_const(
            #krate::MetricWrapper(&#static_name.metric),
            #static_name.name(),
            #namespace,
            #description
        );

        #krate::MetricInstance::new(#static_expr, #name, #description)
    }});
    item.ty = Box::new(parse_quote! { #krate::MetricInstance<#static_type> });

    Ok(quote! { #item })
}
