#![doc = include_str!("../README.md")]

use proc_macro2::{Span, TokenStream};
use proc_macro_crate::{crate_name, FoundCrate};
use quote::{quote, ToTokens};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{parse_macro_input, parse_quote};

struct CfgVisAttrArgs {
    cfg: syn::NestedMeta,
    _comma: Option<syn::Token![,]>,
    vis: syn::Visibility,
}

impl Parse for CfgVisAttrArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let cfg = input.parse()?;
        let comma: Option<syn::Token![,]> = input.parse()?;

        if comma.is_none() || input.is_empty() {
            return Ok(Self {
                cfg,
                _comma: comma,
                vis: syn::Visibility::Inherited,
            });
        }

        let vis = input.parse()?;

        Ok(Self {
            cfg,
            _comma: comma,
            vis,
        })
    }
}

impl ToTokens for CfgVisAttrArgs {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.cfg.to_tokens(tokens);
        self._comma.to_tokens(tokens);
        self.vis.to_tokens(tokens);
    }
}

struct CfgVisAttrArgsAccumulator {
    version: String,
    _semi: Option<syn::Token![;]>,
    acc: Punctuated<CfgVisAttrArgs, syn::Token![;]>,
}

impl Parse for CfgVisAttrArgsAccumulator {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let version_str: syn::LitStr = input.parse()?;
        Ok(Self {
            version: version_str.value(),
            _semi: input.parse()?,
            acc: Punctuated::parse_terminated(input)?,
        })
    }
}

impl ToTokens for CfgVisAttrArgsAccumulator {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.version.to_tokens(tokens);
        self._semi.to_tokens(tokens);
        self.acc.to_tokens(tokens);
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
    let version = env!("CARGO_PKG_VERSION");
    let __cfg_vis_accumulator_declare_path = __cfg_vis_accumulator_declare_path();

    let (_, attrs) = proj_item(&mut item);
    let mut accumulator= attrs
        .iter_mut()
        .filter_map(|acc_attr| {
            if acc_attr.path == __cfg_vis_accumulator_declare_path {
                Some((acc_attr.parse_args(), acc_attr))
            } else {
                None
            }
        })
        .map(|(acc, attr)| {
            let acc: CfgVisAttrArgsAccumulator = acc?;
            if acc.version == version {
                Ok((acc, attr))
            } else {
                Err(syn::Error::new(
                    Span::call_site(),
                    format!("multiple versions of cfg-vis conflict, current version: {:?}, other version: {:?}", version, acc.version))
                )
            }
        })
        .collect::<syn::Result<Vec<_>>>()?;

    match &mut accumulator[..] {
        [] => {
            // the last attr
            attrs.push(
                parse_quote!(#[#__cfg_vis_accumulator_declare_path(#version; #cfg_vis_attr)]),
            );
        }
        [(acc, attr)] => {
            acc.acc.push(cfg_vis_attr);
            **attr = parse_quote!(#[#__cfg_vis_accumulator_declare_path(#acc)]);
        }
        _ => {
            return Err(syn::Error::new(
                Span::call_site(),
                "multiple cfg-vis accumulators exist, it's a bug.",
            ))
        }
    }

    Ok(item.into_token_stream().into())
}

// expend after all `cfg_vis` were expended
#[doc(hidden)]
#[proc_macro_attribute]
pub fn __cfg_vis_accumulator(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let accumulator = parse_macro_input!(attr as CfgVisAttrArgsAccumulator);
    let item = parse_macro_input!(item as syn::Item);

    let version = env!("CARGO_PKG_VERSION");
    if accumulator.version != version {
        return syn::Error::new(
            Span::call_site(),
            format!(
                "multiple versions of cfg-vis conflict, current version: {:?}, other version: {:?}",
                version, accumulator.version
            ),
        )
        .into_compile_error()
        .into();
    }
    // generate:
    //
    // #[cfg($cond_n)]
    // $vis_n $($item)*
    let mut token_stream = TokenStream::new();
    for cfg_vis_args in &accumulator.acc {
        let vis = cfg_vis_args.vis.clone();
        let cfg = &cfg_vis_args.cfg;

        let mut tmp_item = item.clone();
        let (tmp_vis, tmp_attrs) = proj_item(&mut tmp_item);
        *tmp_vis = vis;
        tmp_attrs.push(parse_quote!(#[cfg(#cfg)]));
        tmp_item.to_tokens(&mut token_stream);
    }

    // generate
    //
    // #[cfg(not($cond_1))]
    // #[cfg(not($cond_2))]
    // ..
    // #[cfg(not($cond_n))]
    // $default_vis $($item)*
    let cfgs = accumulator.acc.iter().map(|cfg_vis_args| &cfg_vis_args.cfg);
    let default_item = quote! {
        #( #[cfg(not(#cfgs))] )*
        #item
    };
    token_stream.extend(default_item);

    // check_unique
    let check_unique = assert_accumulator_is_unique(&item);
    token_stream.extend(check_unique);

    token_stream.into()
}

/// `$crate::__cfg_vis_accumulator`
fn __cfg_vis_accumulator_declare_path() -> syn::Path {
    let found_name = crate_name("cfg-vis").expect("cfg-vis is present in `Cargo.toml`");

    match found_name {
        FoundCrate::Itself => {
            parse_quote!(::cfg_vis::__cfg_vis_accumulator)
        }
        FoundCrate::Name(cfg_vis) => {
            let cfg_vis = syn::Ident::new(&cfg_vis, Span::call_site());
            parse_quote!(::#cfg_vis::__cfg_vis_accumulator)
        }
    }
}

fn proj_item(item: &mut syn::Item) -> (&mut syn::Visibility, &mut Vec<syn::Attribute>) {
    match item {
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
            panic!("`cfg_vis` can only apply on item with visibility");
        }
    }
}

fn assert_accumulator_is_unique(item: &syn::Item) -> TokenStream {
    let mut hasher = DefaultHasher::new();

    PartialHashItemHelper(item).hash(&mut hasher);

    // different version of package make a different accumulator
    env!("CARGO_PKG_VERSION").hash(&mut hasher);

    let name = format!(
        "__CFG_VIS_ACCUMULATOR_MUST_EXPAND_ONCE_OTHERWISE_IS_A_BUG_{}",
        hasher.finish()
    );
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

fn take_all_cfg_vis(attrs: &mut Vec<syn::Attribute>) -> syn::Result<Vec<CfgVisAttrArgs>> {
    let (cfg_vis_attrs, remain_attrs): (Vec<_>, Vec<_>) = std::mem::take(attrs)
        .into_iter()
        .partition(|attr| attr.path.is_ident("cfg_vis"));

    *attrs = remain_attrs;

    cfg_vis_attrs
        .into_iter()
        .map(|attr| attr.parse_args::<CfgVisAttrArgs>())
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
