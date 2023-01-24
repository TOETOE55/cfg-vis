#![doc = include_str!("../README.md")]

use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{parenthesized, parse_macro_input, parse_quote};

struct CfgVisAttrArgs {
    cfg: syn::NestedMeta,
    vis: syn::Visibility,
}

struct CfgVisAttrArgsWithParens(CfgVisAttrArgs);

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
impl Parse for CfgVisAttrArgsWithParens {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        parenthesized!(content in input);
        Ok(Self(content.parse()?))
    }
}

///
/// # cfg visibility on items
///
/// ## Rules
///
/// ```ignore
/// #[cfg_vis($cond1:meta, $vis1:vis)]
/// #[cfg_vis($cond2:meta, $vis2:vis)]
/// #[cfg_vis($cond3:meta, $vis3:vis)]
/// $default_vis:vis $($item:tt)*
/// ```
///
/// will expend to
///
/// ```ignore
/// #[cfg($cond1)]
/// $vis1 $($item)*
///
/// #[cfg($cond2)]
/// $vis2 $($item)*
///
/// #[cfg($cond3)]
/// $vis3 $($item)*
///
/// #[cfg(not($cond1))]
/// #[cfg(not($cond2))]
/// #[cfg(not($cond3))]
/// $default_vis $($item)*
/// ```
///
/// ## Example
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
    let cfg_vis_attr = parse_macro_input!(attr as CfgVisAttrArgs);
    let item = parse_macro_input!(item as syn::Item);
    cfg_vis_impl(cfg_vis_attr, item)
        .unwrap_or_else(|err| proc_macro::TokenStream::from(err.into_compile_error()))
}

fn cfg_vis_impl(
    cfg_vis_attr: CfgVisAttrArgs,
    mut item: syn::Item,
) -> syn::Result<proc_macro::TokenStream> {
    fn proj_item(
        item: &mut syn::Item,
    ) -> syn::Result<(&mut syn::Visibility, &mut Vec<syn::Attribute>)> {
        let proj = match item {
            syn::Item::Const(i) => (&mut i.vis, &mut i.attrs),
            syn::Item::Enum(i) => (&mut i.vis, &mut i.attrs),
            syn::Item::ExternCrate(i) => (&mut i.vis, &mut i.attrs),
            syn::Item::Fn(i) => (&mut i.vis, &mut i.attrs),
            syn::Item::Macro2(i) => (&mut i.vis, &mut i.attrs),
            syn::Item::Mod(i) => (&mut i.vis, &mut i.attrs),
            syn::Item::Static(i) => (&mut i.vis, &mut i.attrs),
            syn::Item::Struct(i) => (&mut i.vis, &mut i.attrs),
            syn::Item::Trait(i) => (&mut i.vis, &mut i.attrs),
            syn::Item::TraitAlias(i) => (&mut i.vis, &mut i.attrs),
            syn::Item::Type(i) => (&mut i.vis, &mut i.attrs),
            syn::Item::Union(i) => (&mut i.vis, &mut i.attrs),
            syn::Item::Use(i) => (&mut i.vis, &mut i.attrs),
            _ => {
                return Err(syn::Error::new(
                    item.span(),
                    "`cfg_vis` can only apply on item with visibility",
                ));
            }
        };

        Ok(proj)
    }

    let (default_vis, attrs) = proj_item(&mut item)?;

    let mut cfg_vis_attrs = take_all_cfg_vis(attrs)?;
    cfg_vis_attrs.push(cfg_vis_attr);

    let mut token_stream = proc_macro2::TokenStream::new();
    for (mut cfgs, vis) in expend_cfg_vis_attrs(cfg_vis_attrs, default_vis.clone()) {
        let mut item = item.clone();
        let (item_vis, attrs) = proj_item(&mut item)?;
        *item_vis = vis;
        attrs.append(&mut cfgs);

        item.to_tokens(&mut token_stream);
    }

    Ok(proc_macro::TokenStream::from(token_stream))
}

///
/// # cfg visibility on fields
///
/// ## Rules
///
/// `#[cfg_vis]` on field as same as it on item.
///
/// ## Example
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
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    if !attr.is_empty() {
        let err = syn::Error::new(
            Span::call_site(),
            format!("unsupported arg \"{}\" for `cfg_vis_fields`", attr),
        );
        return proc_macro::TokenStream::from(err.into_compile_error());
    }

    let item = parse_macro_input!(item as syn::Item);

    let toks = cfg_vis_fields_impl(item)
        .map(|item| quote! { #item })
        .unwrap_or_else(|err| err.to_compile_error());

    proc_macro::TokenStream::from(toks)
}

fn cfg_vis_fields_impl(mut item: syn::Item) -> syn::Result<syn::Item> {
    fn find_replace_cfg_vis(
        fields: Punctuated<syn::Field, syn::Token![,]>,
    ) -> syn::Result<Punctuated<syn::Field, syn::Token![,]>> {
        let mut fields_replaced = vec![];

        for mut field in fields {
            let cfg_vis_attrs = take_all_cfg_vis(&mut field.attrs)?;
            for (mut cfgs, vis) in expend_cfg_vis_attrs(cfg_vis_attrs, field.vis.clone()) {
                let mut field = field.clone();
                field.vis = vis;
                field.attrs.append(&mut cfgs);
                fields_replaced.push(field);
            }
        }

        Ok(Punctuated::from_iter(fields_replaced))
    }

    let fields = match &mut item {
        syn::Item::Struct(s) => {
            let fields = match &mut s.fields {
                syn::Fields::Named(f) => &mut f.named,
                syn::Fields::Unnamed(f) => &mut f.unnamed,
                syn::Fields::Unit => {
                    return Ok(item);
                }
            };

            fields
        }

        syn::Item::Union(u) => &mut u.fields.named,
        _ => {
            return Err(syn::Error::new(
                item.span(),
                "`cfg_vis_fields` can only apply on struct or union",
            ))
        }
    };

    *fields = find_replace_cfg_vis(std::mem::take(fields))?;

    Ok(item)
}

fn take_all_cfg_vis(attrs: &mut Vec<syn::Attribute>) -> syn::Result<Vec<CfgVisAttrArgs>> {
    let (cfg_vis_attrs, remain_attrs): (Vec<_>, Vec<_>) =
        std::mem::take(attrs).into_iter().partition(|attr| {
            attr.path
                .get_ident()
                .filter(|&ident| ident == "cfg_vis")
                .is_some()
        });

    *attrs = remain_attrs;

    cfg_vis_attrs
        .into_iter()
        .map(|attr| {
            let cfg_vis_attr = syn::parse2::<CfgVisAttrArgsWithParens>(attr.tokens)?;
            Ok(cfg_vis_attr.0)
        })
        .collect()
}

fn expend_cfg_vis_attrs(
    cfg_vis_attrs: Vec<CfgVisAttrArgs>,
    default_vis: syn::Visibility,
) -> impl Iterator<Item = (Vec<syn::Attribute>, syn::Visibility)> {
    let default_cfg_attrs = cfg_vis_attrs
        .iter()
        .map(|attr| {
            let cfg = &attr.cfg;
            parse_quote! {
                #[cfg(not(#cfg))]
            }
        })
        .collect::<Vec<_>>();

    cfg_vis_attrs
        .into_iter()
        .map(|cfg_vis| {
            let cfg = cfg_vis.cfg;
            let cfgs = vec![parse_quote! {
                #[cfg(#cfg)]
            }];

            (cfgs, cfg_vis.vis)
        })
        .chain(Some((default_cfg_attrs, default_vis)))
}
