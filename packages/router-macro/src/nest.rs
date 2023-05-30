use std::fmt::Display;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{braced, parenthesized, parse::Parse, Ident, LitStr, Path, Token};

use crate::{segment::RouteSegment, RouteType};

fn print_route_segment<'a, I: Iterator<Item = (usize, &'a RouteSegment)>>(
    mut s: std::iter::Peekable<I>,
    sucess_tokens: TokenStream,
    error_enum_name: &Ident,
    enum_varient: &Ident,
    varient_parse_error: &Ident,
) -> TokenStream {
    if let Some((i, route)) = s.next() {
        let children = print_route_segment(
            s,
            sucess_tokens,
            error_enum_name,
            enum_varient,
            varient_parse_error,
        );

        route.try_parse(
            i,
            error_enum_name,
            enum_varient,
            varient_parse_error,
            children,
        )
    } else {
        quote! {
            #sucess_tokens
        }
    }
}

#[derive(Debug)]
pub(crate) struct Nest {
    pub attrs: Vec<syn::Attribute>,
    pub name: Ident,
    pub path: RoutePath,
    pub children: Vec<RouteType>,
}

impl Parse for Nest {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let inner;
        parenthesized!(inner in input);
        let attrs = inner.call(syn::Attribute::parse_outer)?;
        let name = inner.parse()?;
        inner.parse::<Token![,]>()?;
        let path = inner.parse()?;

        let content;
        braced!(content in input);
        let mut children = Vec::new();
        while !content.is_empty() {
            children.push(content.parse()?);
        }

        Ok(Self {
            attrs,
            name,
            path,
            children,
        })
    }
}

impl Nest {
    pub fn dynamic_segments(&self) -> impl Iterator<Item = TokenStream> + '_ {
        self.dynamic_segments_names().map(|i| quote! {#i})
    }

    pub fn dynamic_segments_names(&self) -> impl Iterator<Item = Ident> + '_ {
        self.path.segments.iter().filter_map(|seg| seg.name())
    }

    pub fn write(&self) -> TokenStream {
        let write_segments = self.path.segments.iter().map(|s| s.write_segment());
        let write_children = self.children.iter().map(|s| s.display_match());

        quote! {
            {
                #(#write_segments)*
            }
            #(#write_children)*
        }
    }

    pub fn parse_impl(
        &self,
        error_enum_name: &Ident,
        parent_route_name: Option<Ident>,
    ) -> TokenStream {
        let parse_children: TokenStream = self
            .children
            .iter()
            .map(|s| s.parse_impl_inner(error_enum_name, Some(self.name.clone())))
            .collect();

        let error_name = self.error_ident();
        let error_variant = self.error_variant();
        let name = &self.name;
        let fields = self.dynamic_segments_names();
        let parent_route_name = parent_route_name.into_iter();

        let success_tokens = quote! {
            let name = #name {
                #(parent: #parent_route_name,)*
                #(#fields,)*
            };
            #parse_children
        };

        print_route_segment(
            self.path.segments.iter().enumerate().peekable(),
            success_tokens,
            &error_enum_name,
            &error_variant,
            &error_name,
        )
    }

    pub fn error_ident(&self) -> Ident {
        format_ident!("Nest{}ParseError", self.name)
    }

    pub fn error_variant(&self) -> Ident {
        format_ident!("Nest{}", self.name)
    }

    pub fn error_type(&self) -> TokenStream {
        let error_name = self.error_ident();

        let mut error_variants = Vec::new();
        let mut display_match = Vec::new();

        for (i, segment) in self.path.segments.iter().enumerate() {
            let error_name = segment.error_name(i);
            match segment {
                RouteSegment::Static(index) => {
                    error_variants.push(quote! { #error_name });
                    display_match.push(quote! { Self::#error_name => write!(f, "Static segment '{}' did not match", #index)? });
                }
                RouteSegment::Dynamic(ident, ty) => {
                    let missing_error = segment.missing_error_name().unwrap();
                    error_variants.push(quote! { #error_name(<#ty as dioxus_router::routable::FromRouteSegment>::Err) });
                    display_match.push(quote! { Self::#error_name(err) => write!(f, "Dynamic segment '({}:{})' did not match: {}", stringify!(#ident), stringify!(#ty), err)? });
                    error_variants.push(quote! { #missing_error });
                    display_match.push(quote! { Self::#missing_error => write!(f, "Dynamic segment '({}:{})' was missing", stringify!(#ident), stringify!(#ty))? });
                }
                _ => todo!(),
            }
        }

        quote! {
            #[allow(non_camel_case_types)]
            #[derive(Debug, PartialEq)]
            pub enum #error_name {
                #(#error_variants,)*
            }

            impl std::fmt::Display for #error_name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    match self {
                        #(#display_match,)*
                    }
                    Ok(())
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct RoutePath {
    pub segments: Vec<RouteSegment>,
    pub query: Option<QuerySegment>,
}

impl Display for RoutePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for seg in &self.segments {
            write!(f, "/{}", seg)?;
        }
        if let Some(query) = &self.query {
            write!(f, "?{}", query)?;
        }

        Ok(())
    }
}

impl Parse for RoutePath {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // parse all segments first
        let mut segments = Vec::new();
        // remove any leading slash
        if input.peek(syn::Token![/]) {
            input.parse::<syn::Token![/]>()?;
        }

        while !input.is_empty() {
            let peak = input.lookahead1();
            // check if the next segment is a query
            if peak.peek(syn::Token![?]) {
                break;
            } else if peak.peek(syn::Token![/]) {
                input.parse::<syn::Token![/]>()?;
            } else if peak.peek(syn::Ident) || peak.peek(syn::Token![...]) || peak.peek(syn::LitStr)
            {
                // parse the segment
                segments.push(input.parse()?);
            } else {
                return Err(peak.error());
            }
        }
        // then parse the query
        let query = if input.peek(syn::Token![?]) {
            Some(input.parse()?)
        } else {
            None
        };
        Ok(Self { segments, query })
    }
}

impl Parse for RouteSegment {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(Token![...]) {
            input.parse::<Token![...]>()?;
            let name: Ident = input.parse()?;
            input.parse::<Token![:]>()?;
            let type_: Path = input.parse()?;

            // parse the /
            let _ = input.parse::<Token![/]>();

            Ok(RouteSegment::CatchAll(name, type_))
        } else if lookahead.peek(LitStr) {
            let lit: LitStr = input.parse()?;

            // parse the /
            let _ = input.parse::<Token![/]>();

            Ok(RouteSegment::Static(lit.value()))
        } else if lookahead.peek(Ident) {
            let ident: Ident = input.parse()?;
            input.parse::<Token![:]>()?;
            let type_: Path = input.parse()?;

            // parse the /
            let _ = input.parse::<Token![/]>();

            Ok(RouteSegment::Dynamic(ident, type_))
        } else {
            Err(lookahead.error())
        }
    }
}

#[derive(Debug)]
pub struct QuerySegment {
    spread: bool,
    name: Ident,
    type_: Path,
}

impl Display for QuerySegment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.spread {
            write!(f, "...")?;
        }
        let type_ = &self.type_;
        write!(f, "{}: {}", self.name, quote! {#type_})
    }
}

impl Parse for QuerySegment {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        input.parse::<syn::Token![?]>()?;
        let lookahead = input.lookahead1();
        let spread = if lookahead.peek(syn::Token![...]) {
            input.parse::<syn::Token![...]>()?;
            true
        } else {
            if !lookahead.peek(syn::Ident) {
                return Err(lookahead.error());
            }
            false
        };
        let name = input.parse()?;
        input.parse::<syn::Token![:]>()?;
        let type_ = input.parse()?;
        Ok(Self {
            spread,
            name,
            type_,
        })
    }
}
