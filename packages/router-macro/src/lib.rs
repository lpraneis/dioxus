extern crate proc_macro;

use layout::Layout;
use nest::Nest;
use proc_macro::TokenStream;
use quote::{__private::Span, quote, ToTokens};
use redirect::Redirect;
use route::Render;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, Ident, Token,
};

use proc_macro2::TokenStream as TokenStream2;

mod layout;
mod nest;
mod query;
mod redirect;
mod route;
mod segment;

#[proc_macro]
pub fn routes(input: TokenStream) -> TokenStream {
    let route_enum = parse_macro_input!(input as RouteEnum);

    let error_type = route_enum.error_type();
    let parse_impl = route_enum.parse_impl();
    let display_impl = route_enum.impl_display();
    let routable_impl = route_enum.routable_impl();
    let name = &route_enum.name;
    let vis = &route_enum.vis;

    quote! {
        #route_enum

        #error_type

        #parse_impl

        #display_impl

        #routable_impl

        #vis fn Outlet(cx: dioxus::prelude::Scope) -> dioxus::prelude::Element {
            dioxus_router::prelude::GenericOutlet::<#name>(cx)
        }

        #vis fn Router(cx: dioxus::prelude::Scope<dioxus_router::prelude::GenericRouterProps<#name>>) -> dioxus::prelude::Element {
            dioxus_router::prelude::GenericRouter(cx)
        }

        #vis fn Link<'a>(cx: dioxus::prelude::Scope<'a, dioxus_router::prelude::GenericLinkProps<'a, #name>>) -> dioxus::prelude::Element<'a> {
            dioxus_router::prelude::GenericLink(cx)
        }

        #vis fn use_router<R: dioxus_router::prelude::Routable + Clone>(cx: &dioxus::prelude::ScopeState) -> &dioxus_router::prelude::GenericRouterContext<R> {
            dioxus_router::prelude::use_generic_router::<R>(cx)
        }
    }
    .into()
}

struct RouteEnum {
    vis: syn::Visibility,
    name: Ident,
    attrs: Vec<syn::Attribute>,
    roots: Vec<RouteType>,
}

impl ToTokens for RouteEnum{
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let name = &self.name;
        let vis = &self.vis;
        let attrs = &self.attrs;
        let roots = self.roots.iter().flat_map(|root| root.variants().into_iter());
        tokens.extend(quote! {
            #(#attrs)*
            #vis enum #name {
                #(#roots,)*
            }
        });
    }
}

impl Parse for RouteEnum {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let vis: syn::Visibility = syn::Visibility::Inherited;
        let attrs = input.call(syn::Attribute::parse_outer)?;
        let name = input.parse()?;
        let _ = input.parse::<Token![,]>();

        let mut roots = Vec::new();

        while !input.is_empty() {
            roots.push(input.parse()?);
        }

        Ok(Self { vis, name, attrs,roots })
    }
}

impl RouteEnum {
    fn impl_display(&self) -> TokenStream2 {
        let mut display_match = Vec::new();

        for route in &self.roots {
            display_match.push(route.display_match());
        }

        let name = &self.name;

        quote! {
            impl std::fmt::Display for #name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    #[allow(unused)]
                    match self {
                        #(#display_match)*
                    }
                    Ok(())
                }
            }
        }
    }

    fn parse_impl(&self) -> TokenStream2 {
        let name = &self.name;

        let error_name = self.error_name();
        let tokens = self.roots.iter().map(|root| {
            root.parse_impl(&error_name)
        });

        quote! {
            impl<'a> core::convert::TryFrom<&'a str> for #name {
                type Error = <Self as std::str::FromStr>::Err;

                fn try_from(s: &'a str) -> Result<Self, Self::Error> {
                    s.parse()
                }
            }

            impl std::str::FromStr for #name {
                type Err = dioxus_router::routable::RouteParseError<#error_name>;

                fn from_str(s: &str) -> Result<Self, Self::Err> {
                    let route = s.strip_prefix('/').unwrap_or(s);
                    let (route, query) = route.split_once('?').unwrap_or((route, ""));
                    let mut segments = route.split('/');
                    let mut errors = Vec::new();

                    #(#tokens)*

                    Err(dioxus_router::routable::RouteParseError {
                        attempted_routes: errors,
                    })
                }
            }
        }
    }

    fn error_name(&self) -> Ident {
        Ident::new(&(self.name.to_string() + "MatchError"), Span::call_site())
    }

    fn error_type(&self) -> TokenStream2 {
        let match_error_name = self.error_name();

        let mut type_defs = Vec::new();
        let mut error_variants = Vec::new();
        let mut display_match = Vec::new();

        for root in &self.roots {
            root.with_nests_pre_order(&mut |nest|{
                let error_variant = nest.error_variant();
                let error_name = nest.error_ident();
                let route_str = nest.path.to_string();
    
                error_variants.push(quote! { #error_variant(#error_name) });
                display_match.push(quote! { Self::#error_variant(err) => write!(f, "Route '{}' ('{}') did not match:\n{}", stringify!(#error_name), #route_str, err)? });
                type_defs.push(nest.error_type());
            });
        }

        quote! {
            #(#type_defs)*

            #[allow(non_camel_case_types)]
            #[derive(Debug, PartialEq)]
            pub enum #match_error_name {
                #(#error_variants),*
            }

            impl std::fmt::Display for #match_error_name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    match self {
                        #(#display_match),*
                    }
                    Ok(())
                }
            }
        }
    }

    fn site_map(&self) -> Vec<TokenStream2> {
        let mut site_map = Vec::new();

        for root in &self.roots {
            site_map.append(&mut root.site_map())
        }

        site_map
    }

    fn routable_impl(&self) -> TokenStream2 {
        let name = &self.name;
        let site_map = self.site_map().into_iter();

        let mut layers: Vec<Vec<TokenStream2>> = Vec::new();

        for root in &self.roots {
            root.add_routable_layers(&mut layers);
        }

        let index_iter = 0..layers.len();
        let layers = layers.into_iter().map(|layer| {
            quote! {
                #(#layer)*
            }
        });

        quote! {
            impl dioxus_router::routable::Routable for #name where Self: Clone {
                const SITE_MAP: &'static [dioxus_router::routable::SiteMapSegment] = &[
                    #(#site_map,)*
                ];

                fn render<'a>(&self, cx: &'a ScopeState, level: usize) -> Element<'a> {
                    let myself = self.clone();
                    match level {
                        #(
                            #index_iter => {
                                match myself {
                                    #layers
                                    _ => None
                                }
                            },
                        )*
                        _ => None
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub(crate) enum RouteType {
    Nest(Nest),
    Layout(Layout),
    Render(Render),
    Redirect(Redirect),
}

impl RouteType {
    pub fn site_map(&self) -> Vec<TokenStream2> {
        match self {
            RouteType::Nest(nest) => {
                let mut segments = nest.path.segments.iter().rev().peekable();
                let mut current_segment = {
                    let first_segment = segments
                        .next()
                        .expect("Routes must have at least one segment");
                    let ty = first_segment.to_site_map_type();
                    let children = nest
                        .children
                        .iter()
                        .flat_map(|child| child.site_map().into_iter());
                    quote! {
                        dioxus_router::routable::SiteMapSegment {
                            segment_type: #ty,
                            children: &[#(#children,)*],
                        }
                    }
                };
                for segment in segments {
                    let ty = segment.to_site_map_type();
                    current_segment = quote! {
                        dioxus_router::routable::SiteMapSegment {
                            segment_type: #ty,
                            children: &[#current_segment],
                        }
                    };
                }
                vec![current_segment]
            }
            RouteType::Layout(layout) => layout
                .children
                .iter()
                .flat_map(|child| child.site_map().into_iter())
                .collect(),
            _ => Vec::new(),
        }
    }

    pub fn with_nests_pre_order(&self, f: &mut impl FnMut(&Nest)) {
        match self {
            Self::Nest(nest) => {
                f(nest);
                for child in &nest.children {
                    child.with_nests_pre_order(f);
                }
            }
            Self::Layout(layout) => {
                for child in &layout.children {
                    child.with_nests_pre_order(f);
                }
            }
            _ => {}
        }
    }

    pub fn variants(&self) -> Vec<TokenStream2>
    {
        self.variants_inner(None)
    }

    fn variants_inner(&self, parent_route: Option<Ident>) -> Vec<TokenStream2>
    {
        match self{
            RouteType::Nest(nest) => {
                let name = &nest.name;
                let mut variants=Vec::new();
                for child in &nest.children {
                    variants.append(&mut child.variants_inner(Some(name.clone())));
                }
                variants
            }
            RouteType::Redirect(_)=>Vec::new(),
            RouteType::Layout(layout) => {
                let mut variants=Vec::new();
                for child in &layout.children {
                    variants.append(&mut child.variants_inner(parent_route.clone()));
                }
                variants
            }
            RouteType::Render(render) => {
                let Some(name) = parent_route else{
                    let error = syn::Error::new_spanned(&render.component_name,"Render must have a route parent").to_compile_error();
                    return vec![error];
                };
                let comp_name = &render.component_name;
                let variant = quote! {
                    #comp_name(#name)
                };
                vec![variant]
            }
            
        }
    }

    pub fn has_render_children(&self) -> bool {
        match self {
            Self::Nest(nest) => nest
                .children
                .iter()
                .any(|child| child.has_render_children()),
            Self::Layout(layout) => layout
                .children
                .iter()
                .any(|child| child.has_render_children()),
            Self::Render(_) => true,
            _ => false,
        }
    }

    fn display_match(&self) -> TokenStream2 {
        if !self.has_render_children() {
            return quote! {};
        }
        match self {
            Self::Nest(nest) => {
                let name = &nest.name;
                let display = &nest.write();
                quote! {
                    Self::#name(#name) => {
                        #display
                    }
                }
            }
            Self::Layout(layout) => layout
                .children
                .iter()
                .map(|child| child.display_match())
                .collect(),
            Self::Render(_) => {
                quote! {}
            }
            Self::Redirect(_) => {
                quote! {}
            }
        }
    }

    fn parse_impl(&self, error_enum_name: &Ident) -> TokenStream2 {
        self.parse_impl_inner(error_enum_name, None)
    }

    fn parse_impl_inner(
        &self,
        error_enum_name: &Ident,
        parent_route_ident: Option<Ident>,
    ) -> TokenStream2 {
        match self {
            RouteType::Nest(nest) => nest.parse_impl(error_enum_name, parent_route_ident),
            RouteType::Layout(layout) => layout
                .children
                .iter()
                .map(|child| child.parse_impl_inner(error_enum_name, parent_route_ident.clone()))
                .collect(),
            RouteType::Render(render) => {
                let Some(name) = parent_route_ident else{
                    return syn::Error::new_spanned(&render.component_name,"Render must have a route parent").to_compile_error();
                };
                quote! {
                    return Ok(#name);
                }
            }
            RouteType::Redirect(redirect) => {
                let Some(name) = parent_route_ident else{
                    return syn::Error::new_spanned(&redirect.function,"Redirect must have a route parent").to_compile_error();
                };
                redirect.parse_impl(name)
            }
        }
    }

    pub fn add_routable_layers(&self, layers: &mut Vec<Vec<TokenStream2>>) {
        self.add_routable_layers_inner(None, Vec::new(), layers);
    }

    pub fn add_routable_layers_inner(
        &self,
        parent_route: Option<Ident>,
        current_layouts: Vec<(usize, TokenStream2)>,
        layers: &mut Vec<Vec<TokenStream2>>,
    ) {
        match self {
            RouteType::Nest(nest) => {
                let mut new_layouts = current_layouts;
                // increment the depth of all layouts
                for (i, _) in &mut new_layouts {
                    *i += 1;
                }
                for child in &nest.children {
                    child.add_routable_layers_inner(
                        Some(nest.name.clone()),
                        new_layouts.clone(),
                        layers,
                    );
                }
            }
            RouteType::Layout(layout) => {
                let self_component = layout.routable_match();
                let mut new_layouts = current_layouts;
                new_layouts.push((0, self_component));
                for child in &layout.children {
                    child.add_routable_layers_inner(
                        parent_route.clone(),
                        new_layouts.clone(),
                        layers,
                    );
                }
            }
            RouteType::Render(render) => {
                let Some(name) = parent_route else{
                    let error = syn::Error::new_spanned(&render.component_name,"Render must have a route parent").to_compile_error();
                    layers.push(vec![error]);
                    return;
                };
                while layers.len() <= current_layouts.len() {
                    layers.push(Vec::new());
                }
                // first render the current route's layouts
                for (i, (depth, layout)) in current_layouts.iter().enumerate() {
                    let navigate_parent = (0..(depth-1)).map(|_|quote!(let props = props.parent;));
                    let tokens = quote! {
                        Self::#name(#name) => {
                            let props = #name;
                            #(
                                #navigate_parent
                            )*
                            #layout
                        }
                    };
                    layers[i].push(tokens);
                }

                // then render the current route
                let final_index=current_layouts.len();
                let render =render.routable_match();
                let tokens = quote! {
                    Self::#name(#name) => {
                        let props = #name;
                        #render
                    }
                };
                layers[final_index].push(tokens);
            }
            RouteType::Redirect(_) => {
               
            }
        }
    }
}

impl Parse for RouteType {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(syn::Token![!]) {
            input.parse::<Token![!]>()?;
            let ident: Ident = input.parse()?;
            if ident == "layout" {
                let mut layout: Layout = input.parse()?;
                layout.opt_out = true;
                Ok(RouteType::Layout(layout))
            } else {
                Err(lookahead.error())
            }
        } else if lookahead.peek(syn::Ident) {
            let ident: Ident = input.parse()?;
            if ident == "route" {
                let route = input.parse()?;
                Ok(RouteType::Nest(route))
            } else if ident == "layout" {
                let layout = input.parse()?;
                Ok(RouteType::Layout(layout))
            } else if ident == "render" {
                let render = input.parse()?;
                Ok(RouteType::Render(render))
            } else if ident == "redirect" {
                let redirect = input.parse()?;
                Ok(RouteType::Redirect(redirect))
            } else {
                Err(lookahead.error())
            }
        } else {
            Err(lookahead.error())
        }
    }
}
