use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{braced, parenthesized, parse::Parse, parse_quote, Path, Token};

#[derive(Debug)]
pub(crate) struct Layout {
    pub opt_out: bool,
    pub component: Path,
    pub children: Vec<crate::RouteType>,
}

impl Layout {
    pub fn routable_match(&self) -> TokenStream {
        let comp_name = &self.component;

        quote! {
            let cx = cx.bump().alloc(Scoped {
                props: cx.bump().alloc(props),
                scope: cx,
            });
            #comp_name(cx)
        }
    }
}

impl Parse for Layout {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let inner;
        parenthesized!(inner in input);
        let component: Path = inner.parse()?;
        let _ = inner.parse::<Token![,]>();

        let content;
        braced!(content in input);
        let mut children = Vec::new();
        while !content.is_empty() {
            children.push(content.parse()?);
        }

        Ok(Self {
            opt_out: false,
            component,
            children,
        })
    }
}
