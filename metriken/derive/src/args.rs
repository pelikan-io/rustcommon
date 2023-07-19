// Copyright 2021 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use proc_macro2::TokenStream;
use quote::ToTokens;
use std::fmt::{Display, Formatter, Result};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Ident, Token};

#[derive(Clone)]
pub(crate) enum ArgName {
    Ident(Ident),
    Crate(Token![crate]),
}

impl Parse for ArgName {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        Ok(match () {
            _ if lookahead.peek(Ident) => Self::Ident(input.parse()?),
            _ if lookahead.peek(Token![crate]) => Self::Crate(input.parse()?),
            _ => return Err(lookahead.error()),
        })
    }
}

impl ToTokens for ArgName {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        use self::ArgName::*;

        match self {
            Ident(ident) => ident.to_tokens(tokens),
            Crate(krate) => krate.to_tokens(tokens),
        }
    }
}

impl Display for ArgName {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        use self::ArgName::*;

        match self {
            Ident(ident) => ident.fmt(f),
            Crate(_) => f.write_str("crate"),
        }
    }
}

/// A single argument to an attribute macro.
///
/// ```text
/// #[macro(name = value, a = "string")]
///         ^^^^^^^^^^^^  ^^^^^^^^^^^^
/// ```
pub(crate) struct SingleArg<T> {
    pub ident: ArgName,
    pub eq: Token![=],
    pub value: T,
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

/// An identifier for metadata. This can be either an ident or a string literal.
#[derive(Clone)]
pub(crate) enum MetadataName {
    Ident(syn::Ident),
    String(syn::LitStr),
}

impl MetadataName {
    pub fn value(&self) -> String {
        match self {
            Self::Ident(ident) => ident.to_string(),
            Self::String(string) => string.value(),
        }
    }

    pub fn to_literal(&self) -> syn::LitStr {
        match self {
            Self::Ident(ident) => syn::LitStr::new(&ident.to_string(), ident.span()),
            Self::String(string) => string.clone(),
        }
    }
}

impl Parse for MetadataName {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        Ok(match () {
            _ if lookahead.peek(syn::Ident) => Self::Ident(input.parse()?),
            _ if lookahead.peek(syn::LitStr) => Self::String(input.parse()?),
            _ => return Err(lookahead.error()),
        })
    }
}

impl ToTokens for MetadataName {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Ident(ident) => ident.to_tokens(tokens),
            Self::String(string) => string.to_tokens(tokens),
        }
    }
}

/// A single key-value metadata entry.
///
/// ```text
/// #[macro(metadata = { "arg.val" = "b", name = "thing" })]
///                      ^^^^^^^^^^^^^^^  ^^^^^^^^^^^^^^
/// ```
pub(crate) struct MetadataEntry {
    pub name: MetadataName,
    pub eq: Token![=],
    pub value: syn::Expr,
}

impl Parse for MetadataEntry {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            name: input.parse()?,
            eq: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl ToTokens for MetadataEntry {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.name.to_tokens(tokens);
        self.eq.to_tokens(tokens);
        self.value.to_tokens(tokens);
    }
}

/// A set of key-value entries surrounded by braces.
///
/// ```text
/// #[macro(metadata = { "arg.val" = "b", name = "thing" })]
///                    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
/// ```
pub(crate) struct Metadata {
    pub brace: syn::token::Brace,
    pub entries: Punctuated<MetadataEntry, Token![,]>,
}

impl Parse for Metadata {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;

        Ok(Self {
            brace: syn::braced!(content in input),
            entries: Punctuated::parse_terminated(&content)?,
        })
    }
}

impl ToTokens for Metadata {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.brace
            .surround(tokens, |tokens| self.entries.to_tokens(tokens))
    }
}

pub(crate) trait SingleArgExt {
    type Inner;

    fn insert_or_duplicate(&mut self, arg: SingleArg<Self::Inner>) -> syn::Result<()>;
}

impl<T> SingleArgExt for Option<SingleArg<T>> {
    type Inner = T;

    fn insert_or_duplicate(&mut self, arg: SingleArg<Self::Inner>) -> syn::Result<()> {
        match self {
            None => {
                *self = Some(arg);
                Ok(())
            }
            Some(_) => Err(syn::Error::new_spanned(
                arg.ident.clone(),
                format_args!("unexpected duplicate argument `{}`", arg.ident),
            )),
        }
    }
}
