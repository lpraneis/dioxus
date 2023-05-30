use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use syn::parenthesized;
use syn::parse::Parse;
use syn::Ident;
use syn::Token;

#[derive(Debug)]
pub struct Render {
    pub component_name: Ident,
    pub props_name: Ident,
}

impl Parse for Render {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let inner;
        parenthesized!(inner in input);
        let component_name = inner.parse()?;
        let _ = inner.parse::<Token![,]>();
        let props_name = inner
            .parse()
            .unwrap_or_else(|_| format_ident!("{}Props", component_name));

        Ok(Self {
            component_name,
            props_name,
        })
    }
}

impl Render {
    pub fn routable_match(&self) -> TokenStream {
        let comp_name = &self.component_name;

        quote! {
            let cx = cx.bump().alloc(Scoped {
                props: cx.bump().alloc(props),
                scope: cx,
            });
            #comp_name(cx)
        }
    }
}
