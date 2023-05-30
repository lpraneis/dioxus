use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{parenthesized, parse::Parse, Expr};

#[derive(Debug)]
pub struct Redirect {
    pub function: Expr,
}

impl Redirect {
    pub fn parse_impl(&self, parent_route_name: Ident) -> TokenStream {
        let function = &self.function;
        quote! {
            if let Some(new_route) = (#function)(#parent_route_name) {
                return Ok(new_route);
            }
        }
    }
}

impl Parse for Redirect {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let inner;
        parenthesized!(inner in input);
        let function = inner.parse()?;
        Ok(Self { function })
    }
}
