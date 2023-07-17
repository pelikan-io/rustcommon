// Copyright 2021 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use proc_macro2::TokenStream;
use quote::ToTokens;
use std::fmt::{Display, Formatter, Result};
use syn::parse::{Parse, ParseStream};
use syn::{Ident, Token};

/// The name of an attribute macro argument.
///
/// ```text
/// #[macro(name = value)]
///         ^^^^
/// ```
///
/// This can be either a normal identifier or the `crate` token.
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
