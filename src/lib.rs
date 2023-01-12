#![doc = include_str!("../README.md")]

use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::visit_mut::VisitMut;
use syn::{parenthesized, parse_macro_input, parse_quote};

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

struct CfgVisAttrArgsWithParens(CfgVisAttrArgs);

impl Parse for CfgVisAttrArgsWithParens {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        parenthesized!(content in input);
        Ok(Self(content.parse()?))
    }
}

struct FieldVisibilityReplace;

impl VisitMut for FieldVisibilityReplace {
    fn visit_fields_mut(&mut self, fields: &mut syn::Fields) {
        let new_fields = (&mut *fields)
            .into_iter()
            .flat_map(|field| {
                field
                    .attrs
                    .iter()
                    .position(|attr| {
                        attr.path
                            .get_ident()
                            .filter(|&ident| ident == "cfg_vis")
                            .is_some()
                    })
                    .and_then(|ind| {
                        let attr = &field.attrs[ind].tokens;
                        let CfgVisAttrArgsWithParens(CfgVisAttrArgs { cfg, vis }) =
                            parse_quote!(#attr);

                        let mut field_if_cfg = field.clone();
                        field_if_cfg.vis = vis;

                        field_if_cfg.attrs[ind] = parse_quote! { #[cfg(#cfg)] };
                        field.attrs[ind] = parse_quote! { #[cfg(not(#cfg))] };

                        Some(vec![field_if_cfg, field.clone()])
                    })
                    .unwrap_or_else(|| vec![field.clone()])
            })
            .collect();

        match fields {
            syn::Fields::Named(fields) => fields.named = new_fields,
            syn::Fields::Unnamed(fields) => fields.unnamed = new_fields,
            syn::Fields::Unit => (),
        }
    }
}

///
/// # Rules
///
/// # Example
///
/// ```rust
/// use cfg_vis::cfg_vis_fields;
///
/// #[cfg_vis_fields]
/// struct Foo {
///     // while the target is linux, the visibility is `pub`.
///     #[cfg_vis(target_os = "linux", pub)]
///     foo: i32,
/// }
/// ```
///
#[proc_macro_attribute]
pub fn cfg_vis_fields(
    _attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut item = parse_macro_input!(item as syn::Item);

    match item {
        syn::Item::Struct(_) | syn::Item::Enum(_) | syn::Item::Union(_) => (),
        _ => {
            let err = syn::Error::new(
                item.span(),
                "`cfg_vis_fields` can only apply on struct, enum or union",
            );
            return proc_macro::TokenStream::from(err.into_compile_error());
        }
    }

    FieldVisibilityReplace.visit_item_mut(&mut item);

    let tokens = quote! { #item };
    proc_macro::TokenStream::from(tokens)
}
