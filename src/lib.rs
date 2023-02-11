#![doc = include_str!("../README.md")]

use proc_macro2::{Span, TokenStream};
use quote::quote;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
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
    let CfgVisAttrArgs { cfg, vis } = parse_macro_input!(attr as CfgVisAttrArgs);
    let item = parse_macro_input!(item as syn::Item);
    cfg_vis_impl(cfg, vis, item)
        .unwrap_or_else(|err| proc_macro::TokenStream::from(err.into_compile_error()))
}

fn cfg_vis_impl(
    cfg: syn::NestedMeta,
    vis: syn::Visibility,
    mut item: syn::Item,
) -> syn::Result<proc_macro::TokenStream> {
    let default_item = item.clone();

    let (default_vis, _) = match &mut item {
        syn::Item::Const(i) => (&mut i.vis, &i.attrs),
        syn::Item::Enum(i) => (&mut i.vis, &i.attrs),
        syn::Item::ExternCrate(i) => (&mut i.vis, &i.attrs),
        syn::Item::Fn(i) => (&mut i.vis, &i.attrs),
        syn::Item::Macro2(i) => (&mut i.vis, &i.attrs),
        syn::Item::Mod(i) => (&mut i.vis, &i.attrs),
        syn::Item::Static(i) => (&mut i.vis, &i.attrs),
        syn::Item::Struct(i) => (&mut i.vis, &i.attrs),
        syn::Item::Trait(i) => (&mut i.vis, &i.attrs),
        syn::Item::TraitAlias(i) => (&mut i.vis, &i.attrs),
        syn::Item::Type(i) => (&mut i.vis, &i.attrs),
        syn::Item::Union(i) => (&mut i.vis, &i.attrs),
        syn::Item::Use(i) => (&mut i.vis, &i.attrs),
        _ => {
            return Err(syn::Error::new(
                item.span(),
                "`cfg_vis` can only apply on item with visibility",
            ));
        }
    };

    *default_vis = vis;

    let check_unique = assert_cfg_vis_is_unique(&item);

    let tokens = quote! {
        #[cfg(#cfg)]
        #item

        #[cfg(not(#cfg))]
        #default_item

        #check_unique
    };

    Ok(proc_macro::TokenStream::from(tokens))
}

fn assert_cfg_vis_is_unique(item: &syn::Item) -> TokenStream {
    let mut hasher = DefaultHasher::new();

    PartialHashItemHelper(item).hash(&mut hasher);

    // different version of package make a different accumulator
    env!("CARGO_PKG_VERSION").hash(&mut hasher);

    let name = format!("__CFG_VIS_MUST_CALL_ONCE_{}", hasher.finish());
    let check_unique = syn::Ident::new(&name, Span::call_site());

    quote! {
        const #check_unique: () = ();
    }
}

struct PartialHashItemHelper<'a>(&'a syn::Item);

impl Hash for PartialHashItemHelper<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self.0).hash(state);
        match &self.0 {
            syn::Item::Const(v0) => {
                v0.ident.hash(state);
            }
            syn::Item::Enum(v0) => {
                v0.ident.hash(state);
            }
            syn::Item::ExternCrate(v0) => {
                v0.ident.hash(state);
                v0.rename.hash(state);
            }
            syn::Item::Fn(v0) => {
                v0.sig.ident.hash(state);
            }
            syn::Item::Macro(v0) => {
                v0.ident.hash(state);
            }
            syn::Item::Macro2(v0) => {
                v0.ident.hash(state);
            }
            syn::Item::Mod(v0) => {
                v0.ident.hash(state);
            }
            syn::Item::Static(v0) => {
                v0.ident.hash(state);
            }
            syn::Item::Struct(v0) => {
                v0.ident.hash(state);
            }
            syn::Item::Trait(v0) => {
                v0.ident.hash(state);
            }
            syn::Item::TraitAlias(v0) => {
                v0.ident.hash(state);
            }
            syn::Item::Type(v0) => {
                v0.ident.hash(state);
            }
            syn::Item::Union(v0) => {
                v0.ident.hash(state);
            }
            syn::Item::Use(v0) => {
                v0.tree.hash(state);
            }
            _ => self.0.hash(state),
        }
    }
}

///
/// # cfg visibility on fields
///
/// ## Rules
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
    let fields = match &mut item {
        syn::Item::Struct(s) => match &mut s.fields {
            syn::Fields::Named(f) => &mut f.named,
            syn::Fields::Unnamed(f) => &mut f.unnamed,
            syn::Fields::Unit => {
                return Ok(item);
            }
        },

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

fn find_replace_cfg_vis(
    fields: Punctuated<syn::Field, syn::Token![,]>,
) -> syn::Result<Punctuated<syn::Field, syn::Token![,]>> {
    let mut fields_replaced = Punctuated::new();
    for mut field in fields {
        if let Some(pos) = guard_cfg_vis_unique(&field.attrs)? {
            let attr = &field.attrs[pos].tokens;
            let CfgVisAttrArgsWithParens(CfgVisAttrArgs { cfg, vis }) = parse_quote!(#attr);

            let mut field_replaced = field.clone();
            field_replaced.attrs[pos] = parse_quote! { #[cfg(#cfg)] };
            field_replaced.vis = vis;
            fields_replaced.push(field_replaced);

            field.attrs[pos] = parse_quote! { #[cfg(not(#cfg))] };
        }
        fields_replaced.push(field);
    }

    Ok(fields_replaced)
}

fn guard_cfg_vis_unique(attrs: &[syn::Attribute]) -> syn::Result<Option<usize>> {
    let mut count = 0;
    let mut pos = None;

    for (i, attr) in attrs.iter().enumerate() {
        let has_cfg_vis = attr
            .path
            .get_ident()
            .filter(|&ident| ident == "cfg_vis")
            .is_some();

        if has_cfg_vis {
            count += 1;
            pos = Some(i);
        }

        if count > 1 {
            return Err(syn::Error::new(
                attr.span(),
                "multiple `cfg_vis` is not allowed",
            ));
        }
    }

    Ok(pos)
}
