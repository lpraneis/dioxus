use quote::quote;
use syn::{Ident, Type};

use proc_macro2::TokenStream as TokenStream2;

#[derive(Debug, Clone)]
pub enum Query {
    None,
    Segment(QuerySegment),
    Segments(QuerySegments),
}

impl Query {
    pub fn parse(&self) -> TokenStream2 {
        match self {
            Query::None => quote! {},
            Query::Segment(query_segment) => query_segment.parse(),
            Query::Segments(query_segments) => query_segments.parse(),
        }
    }

    pub fn write(&self) -> TokenStream2 {
        match self {
            Query::None => quote! {},
            Query::Segment(query_segment) => query_segment.write(),
            Query::Segments(query_segments) => query_segments.write(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct QuerySegment {
    pub ident: Ident,
    pub ty: Type,
}

impl QuerySegment {
    pub fn parse(&self) -> TokenStream2 {
        let ident = &self.ident;
        let ty = &self.ty;
        quote! {
            let #ident = <#ty as dioxus_router::routable::FromQuerySegment>::from_query(query);
        }
    }

    pub fn write(&self) -> TokenStream2 {
        let ident = &self.ident;
        quote! {
            write!(f, "?{}", #ident)?;
        }
    }
}

#[derive(Debug, Clone)]
pub struct QuerySegments {
    pub ident: Ident,
    pub ty: Type,
}

impl QuerySegments {
    pub fn parse(&self) -> TokenStream2 {
        let ident = &self.ident;
        let ty = &self.ty;
        quote! {
            let #ident = <#ty as dioxus_router::routable::FromQuerySegments>::from_query(query);
        }
    }

    pub fn write(&self) -> TokenStream2 {
        let ident = &self.ident;
        quote! {
            write!(f, "?{}", #ident)?;
        }
    }
}
