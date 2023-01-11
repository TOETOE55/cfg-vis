#![doc = include_str!("../README.md")]

use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::parse_macro_input;
use syn::spanned::Spanned;

struct CfgVisAttrArgs {
    cfg: syn::NestedMeta,
    vis: syn::Visibility,
}

impl Parse for CfgVisAttrArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let cfg = input.parse()?;
        let comma: Option<syn::Token![,]> = input.parse()?;

        if comma.is_none() || input.is_empty() {
            return Ok(Self {
                cfg,
                vis: syn::Visibility::Inherited,
            });
        }

        let vis = input.parse()?;

        Ok(Self { cfg, vis })
    }
}


///
/// # Rules
///
/// ```ignore
/// #[cfg_vis($cfg: meta, $vis: vis)]
/// $default_vis:vis $($item:tt)*
/// ```
///
/// will expend to
///
/// ```ignore
/// #[cfg($cfg)]
/// $vis $($item)*
///
/// #[cfg(not($cfg))]
/// $default_vis $($item)*
/// ```
///
/// # Example
///
/// ```rust
/// use cfg_vis::cfg_vis;
///
/// // default visibility is `pub`, while the target is linux, the visibility is `pub(super)`.
/// #[cfg_vis(target_os = "linux", pub(super))]
/// pub fn foo() {}
/// ```
///
#[proc_macro_attribute]
pub fn cfg_vis(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let CfgVisAttrArgs { cfg, vis } = parse_macro_input!(attr as CfgVisAttrArgs);
    let mut item = parse_macro_input!(item as syn::Item);
    let default_item = item.clone();

    let default_vis = match &mut item {
        syn::Item::Const(i) => &mut i.vis,
        syn::Item::Enum(i) => &mut i.vis,
        syn::Item::ExternCrate(i) => &mut i.vis,
        syn::Item::Fn(i) => &mut i.vis,
        syn::Item::Macro2(i) => &mut i.vis,
        syn::Item::Mod(i) => &mut i.vis,
        syn::Item::Static(i) => &mut i.vis,
        syn::Item::Struct(i) => &mut i.vis,
        syn::Item::Trait(i) => &mut i.vis,
        syn::Item::TraitAlias(i) => &mut i.vis,
        syn::Item::Type(i) => &mut i.vis,
        syn::Item::Union(i) => &mut i.vis,
        syn::Item::Use(i) => &mut i.vis,
        _ => {
            let err = syn::Error::new(
                item.span(),
                "`cfg_vis` can only apply on item with visibility",
            );
            return proc_macro::TokenStream::from(err.into_compile_error());
        }
    };

    *default_vis = vis;

    let tokens = quote! {
        #[cfg(#cfg)]
        #item

        #[cfg(not(#cfg))]
        #default_item
    };

    proc_macro::TokenStream::from(tokens)
}
