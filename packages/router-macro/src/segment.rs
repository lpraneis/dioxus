use std::fmt::Display;

use quote::{format_ident, quote};
use syn::{Ident, Path};

use proc_macro2::TokenStream as TokenStream2;

#[derive(Debug, Clone)]
pub enum RouteSegment {
    Static(String),
    Dynamic(Ident, Path),
    CatchAll(Ident, Path),
}

impl Display for RouteSegment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RouteSegment::Static(segment) => write!(f, "{}", segment),
            RouteSegment::Dynamic(ident, ty) => write!(f, "{}: {}", ident, quote! {#ty}),
            RouteSegment::CatchAll(ident, ty) => write!(f, "...{}: {}", ident, quote! {#ty}),
        }
    }
}

impl RouteSegment {
    pub fn to_site_map_type(&self) -> TokenStream2 {
        match self {
            RouteSegment::Static(segment) => {
                let segment_as_str = segment.to_string();
                quote! { dioxus_router::routable::SegmentType::Static(#segment_as_str) }
            }
            RouteSegment::Dynamic(ident, _) => {
                let ident_as_str = ident.to_string();
                quote! { dioxus_router::routable::SegmentType::Dynamic(#ident_as_str) }
            }
            RouteSegment::CatchAll(ident, _) => {
                let ident_as_str = ident.to_string();
                quote! { dioxus_router::routable::SegmentType::CatchAll(#ident_as_str) }
            }
        }
    }

    pub fn name(&self) -> Option<Ident> {
        match self {
            Self::Static(_) => None,
            Self::Dynamic(ident, _) => Some(ident.clone()),
            Self::CatchAll(ident, _) => Some(ident.clone()),
        }
    }

    pub fn write_segment(&self) -> TokenStream2 {
        match self {
            Self::Static(segment) => quote! { write!(f, "/{}", #segment)?; },
            Self::Dynamic(ident, _) => quote! { write!(f, "/{}", #ident)?; },
            Self::CatchAll(ident, _) => quote! { #ident.display_route_segements(f)?; },
        }
    }

    pub fn error_name(&self, idx: usize) -> Ident {
        match self {
            Self::Static(_) => static_segment_idx(idx),
            Self::Dynamic(ident, _) => format_ident!("{}ParseError", ident),
            Self::CatchAll(ident, _) => format_ident!("{}ParseError", ident),
        }
    }

    pub fn missing_error_name(&self) -> Option<Ident> {
        match self {
            Self::Dynamic(ident, _) => Some(format_ident!("{}MissingError", ident)),
            _ => None,
        }
    }

    pub fn try_parse(
        &self,
        idx: usize,
        error_enum_name: &Ident,
        error_enum_varient: &Ident,
        inner_parse_enum: &Ident,
        parse_children: TokenStream2,
    ) -> TokenStream2 {
        let error_name = self.error_name(idx);
        match self {
            Self::Static(segment) => {
                quote! {
                    {
                        let mut segments = segments.clone();
                        let parsed = if let Some(#segment) = segments.next() {
                            Ok(())
                        } else {
                            Err(#error_enum_name::#error_enum_varient(#inner_parse_enum::#error_name))
                        };
                        match parsed {
                            Ok(_) => {
                                #parse_children
                            }
                            Err(err) => {
                                errors.push(err);
                            }
                        }
                    }
                }
            }
            Self::Dynamic(name, ty) => {
                let missing_error_name = self.missing_error_name().unwrap();
                quote! {
                    {
                        let mut segments = segments.clone();
                        let parsed = if let Some(segment) = segments.next() {
                            <#ty as dioxus_router::routable::FromRouteSegment>::from_route_segment(segment).map_err(|err| #error_enum_name::#error_enum_varient(#inner_parse_enum::#error_name(err)))
                        } else {
                            Err(#error_enum_name::#error_enum_varient(#inner_parse_enum::#missing_error_name))
                        };
                        match parsed {
                            Ok(#name) => {
                                #parse_children
                            }
                            Err(err) => {
                                errors.push(err);
                            }
                        }
                    }
                }
            }
            Self::CatchAll(name, ty) => {
                quote! {
                    {
                        let parsed = {
                            let mut segments = segments.clone();
                            let segments: Vec<_> = segments.collect();
                            <#ty as dioxus_router::routable::FromRouteSegments>::from_route_segments(&segments).map_err(|err| #error_enum_name::#error_enum_varient(#inner_parse_enum::#error_name(err)))
                        };
                        match parsed {
                            Ok(#name) => {
                                #parse_children
                            }
                            Err(err) => {
                                errors.push(err);
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn static_segment_idx(idx: usize) -> Ident {
    format_ident!("StaticSegment{}ParseError", idx)
}
