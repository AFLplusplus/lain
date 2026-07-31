use crate::internals::symbol::*;
use crate::internals::Ctxt;
use proc_macro2::{Group, Span, TokenStream, TokenTree};
use quote::{quote, quote_spanned, ToTokens};
use std::borrow::Cow;
use std::str::FromStr;
use syn::parse::{self, Parse};
use syn::Lit::{Float, Int};
use syn::{self, parse_quote};
use syn::{Ident, LitInt};

pub struct Attr<'c, T> {
    cx: &'c Ctxt,
    name: Symbol,
    tokens: TokenStream,
    value: Option<T>,
}

impl<'c, T> Attr<'c, T> {
    fn none(cx: &'c Ctxt, name: Symbol) -> Self {
        Attr {
            cx,
            name,
            tokens: TokenStream::new(),
            value: None,
        }
    }

    fn set<A: ToTokens>(&mut self, obj: A, value: T) {
        let tokens = obj.into_token_stream();

        if self.value.is_some() {
            self.cx
                .error_spanned_by(tokens, format!("duplicate lain attribute `{}`", self.name))
        } else {
            self.tokens = tokens;
            self.value = Some(value);
        }
    }

    fn get(self) -> Option<T> {
        self.value
    }
}

struct BoolAttr<'c>(Attr<'c, ()>);

impl<'c> BoolAttr<'c> {
    fn none(cx: &'c Ctxt, name: Symbol) -> Self {
        BoolAttr(Attr::none(cx, name))
    }

    fn set_true<A: ToTokens>(&mut self, obj: A) {
        self.0.set(obj, ());
    }

    fn get(&self) -> bool {
        self.0.value.is_some()
    }
}

/// Represents a struct or enum attribute information
pub struct Container {
    serialized_size: Option<usize>,
    min_serialized_size: Option<usize>,
}

impl Container {
    /// Extract out the `#[lain()]` attributes from an item
    pub fn from_ast(cx: &Ctxt, item: &syn::DeriveInput) -> Self {
        let mut serialized_size = Attr::none(cx, SERIALIZED_SIZE);
        let mut min_serialized_size = Attr::none(cx, MIN_SERIALIZED_SIZE);

        for attr in &item.attrs {
            if *attr.path() == LAIN {
                if let Err(err) = attr.parse_nested_meta(|meta| {
                    if meta.path == SERIALIZED_SIZE {
                        let value = meta.value()?;
                        let i: LitInt = value.parse()?;
                        if let Ok(val) = i.base10_parse::<usize>() {
                            serialized_size.set(&meta.path, val);
                        } else {
                            cx.error_spanned_by(
                                &i,
                                format!(
                                    "failed to parse integer expression for {}",
                                    SERIALIZED_SIZE
                                ),
                            );
                        }
                    } else if meta.path == MIN_SERIALIZED_SIZE {
                        let value = meta.value()?;
                        let i: LitInt = value.parse()?;
                        if let Ok(val) = i.base10_parse::<usize>() {
                            min_serialized_size.set(&meta.path, val);
                        } else {
                            cx.error_spanned_by(
                                &i,
                                format!(
                                    "failed to parse integer expression for {}",
                                    MIN_SERIALIZED_SIZE
                                ),
                            );
                        }
                    } else {
                        cx.error_spanned_by(
                            &meta.path,
                            format!(
                                "unknown lain container attribute `{}`",
                                meta.path.to_token_stream()
                            ),
                        );
                    }
                    Ok(())
                }) {
                    cx.error_spanned_by(attr, err.to_string());
                }
            }
        }

        Container {
            serialized_size: serialized_size.get(),
            min_serialized_size: min_serialized_size.get(),
        }
    }

    pub fn serialized_size(&self) -> Option<usize> {
        self.serialized_size
    }

    pub fn min_serialized_size(&self) -> Option<usize> {
        self.min_serialized_size
    }

    pub fn lain_path(&self) -> Cow<'_, syn::Path> {
        Cow::Owned(parse_quote!(_lain))
    }
}

#[derive(PartialEq)]
pub enum WeightTo {
    None,
    Min,
    Max,
}

impl ToTokens for WeightTo {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match *self {
            WeightTo::None => tokens.extend(quote! {_lain::types::Weighted::None}),
            WeightTo::Min => tokens.extend(quote! {_lain::types::Weighted::Min}),
            WeightTo::Max => tokens.extend(quote! {_lain::types::Weighed::Max}),
        }
    }
}

pub fn unraw(ident: &Ident) -> String {
    ident.to_string().trim_start_matches("r#").to_owned()
}

/// Represents field attribute information
pub struct Field {
    bits: Option<usize>,
    bit_shift: Option<usize>,
    bitfield_type: Option<syn::Type>,
    min: Option<TokenStream>,
    max: Option<TokenStream>,
    ignore: bool,
    ignore_chance: Option<f64>,
    initializer: Option<TokenStream>,
    little_endian: bool,
    big_endian: bool,
    weight_to: Option<WeightTo>,
    is_last_field: bool,
}

impl Field {
    /// Extract out the `#[lain()]` attributes from an item
    pub fn from_ast(cx: &Ctxt, field: &syn::Field) -> Self {
        let mut bits = Attr::none(cx, BITS);
        let mut bitfield_type = Attr::none(cx, BITFIELD_TYPE);
        let mut min = Attr::none(cx, MIN);
        let mut max = Attr::none(cx, MAX);
        let mut ignore = BoolAttr::none(cx, IGNORE);
        let mut ignore_chance = Attr::none(cx, IGNORE_CHANCE);
        let mut initializer = Attr::none(cx, INITIALIZER);
        let mut big_endian = BoolAttr::none(cx, BIG_ENDIAN);
        let mut little_endian = BoolAttr::none(cx, LITTLE_ENDIAN);
        let mut weight_to = Attr::none(cx, WEIGHT_TO);

        for attr in &field.attrs {
            if *attr.path() == LAIN {
                if let Err(err) = attr.parse_nested_meta(|meta| {
                    if meta.path == MIN {
                        let value = meta.value()?;
                        let lit: syn::Lit = value.parse()?;
                        if let Ok(t) = parse_min_max(cx, MIN, &lit) {
                            min.set(&meta.path, t);
                        }
                    } else if meta.path == MAX {
                        let value = meta.value()?;
                        let lit: syn::Lit = value.parse()?;
                        if let Ok(t) = parse_min_max(cx, MAX, &lit) {
                            max.set(&meta.path, t);
                        }
                    } else if meta.path == BITS {
                        let value = meta.value()?;
                        let i: LitInt = value.parse()?;
                        if let Ok(val) = i.base10_parse::<usize>() {
                            bits.set(&meta.path, val);
                        } else {
                            cx.error_spanned_by(
                                &i,
                                format!("failed to parse integer expression for `{}`", BITS),
                            );
                        }
                    } else if meta.path == BITFIELD_TYPE {
                        let value = meta.value()?;
                        let lit: syn::Lit = value.parse()?;
                        if let Ok(expr) = parse_lit_into_type(cx, BITFIELD_TYPE, &lit) {
                            bitfield_type.set(&meta.path, expr);
                        } else {
                            cx.error_spanned_by(
                                &lit,
                                format!("failed to parse `{}` into a type", BITFIELD_TYPE),
                            );
                        }
                    } else if meta.path == BIG_ENDIAN {
                        if little_endian.get() {
                            cx.error_spanned_by(
                                &meta.path,
                                format!(
                                    "attribute meta items `{}` and `{}` are mutually exclusive",
                                    BIG_ENDIAN, LITTLE_ENDIAN
                                ),
                            );
                        } else {
                            big_endian.set_true(&meta.path);
                        }
                    } else if meta.path == LITTLE_ENDIAN {
                        if big_endian.get() {
                            cx.error_spanned_by(
                                &meta.path,
                                format!(
                                    "attribute meta items `{}` and `{}` are mutually exclusive",
                                    BIG_ENDIAN, LITTLE_ENDIAN
                                ),
                            );
                        } else {
                            little_endian.set_true(&meta.path);
                        }
                    } else if meta.path == IGNORE {
                        ignore.set_true(&meta.path);
                    } else if meta.path == IGNORE_CHANCE {
                        let value = meta.value()?;
                        let lit: syn::Lit = value.parse()?;
                        if let Float(ref f) = lit {
                            if let Ok(val) = f.base10_parse::<f64>() {
                                ignore_chance.set(&meta.path, val);
                            }
                        } else if let Int(ref i) = lit {
                            if let Ok(val) = i.base10_parse::<f64>() {
                                ignore_chance.set(&meta.path, val);
                            }
                        } else {
                            cx.error_spanned_by(
                                &lit,
                                format!("failed to parse float expression for `{}`", IGNORE_CHANCE),
                            );
                        }
                    } else if meta.path == INITIALIZER {
                        let value = meta.value()?;
                        let lit: syn::Lit = value.parse()?;
                        if let Ok(s) = get_lit_str(cx, INITIALIZER, INITIALIZER, &lit) {
                            if let Ok(tokens) = TokenStream::from_str(&s.value()) {
                                initializer.set(&meta.path, tokens);
                            } else {
                                cx.error_spanned_by(
                                    &lit,
                                    format!("failed to parse tokens for `{}`", INITIALIZER),
                                );
                            }
                        }
                    } else if meta.path == WEIGHT_TO {
                        let value = meta.value()?;
                        let lit: syn::Lit = value.parse()?;
                        if let Ok(s) = get_lit_str(cx, WEIGHT_TO, WEIGHT_TO, &lit) {
                            match s.value().as_ref() {
                                "min" => weight_to.set(&meta.path, WeightTo::Min),
                                "max" => weight_to.set(&meta.path, WeightTo::Max),
                                "none" => weight_to.set(&meta.path, WeightTo::None),
                                _ => cx.error_spanned_by(
                                    &lit,
                                    format!("unknown option `{}` for `{}`", s.value(), WEIGHT_TO),
                                ),
                            }
                        }
                    } else {
                        cx.error_spanned_by(
                            &meta.path,
                            format!(
                                "unknown lain field attribute `{}`",
                                meta.path.to_token_stream()
                            ),
                        );
                    }
                    Ok(())
                }) {
                    cx.error_spanned_by(attr, err.to_string());
                }
            }
        }

        Field {
            bits: bits.get(),
            bit_shift: None, // this gets fixed up later
            bitfield_type: bitfield_type.get(),
            min: min.get(),
            max: max.get(),
            ignore: ignore.get(),
            ignore_chance: ignore_chance.get(),
            initializer: initializer.get(),
            little_endian: little_endian.get(),
            big_endian: big_endian.get(),
            weight_to: weight_to.get(),
            is_last_field: false,
        }
    }

    pub fn set_is_last_field(&mut self) {
        self.is_last_field = true;
    }

    pub fn is_last_field(&self) -> bool {
        self.is_last_field
    }

    pub fn bits(&self) -> Option<usize> {
        self.bits
    }

    pub fn bit_shift(&self) -> Option<usize> {
        self.bit_shift
    }

    pub fn set_bit_shift(&mut self, shift: usize) {
        self.bit_shift = Some(shift);
    }

    pub fn bitfield_type(&self) -> Option<&syn::Type> {
        self.bitfield_type.as_ref()
    }

    pub fn min(&self) -> Option<&TokenStream> {
        self.min.as_ref()
    }

    pub fn max(&self) -> Option<&TokenStream> {
        self.max.as_ref()
    }

    pub fn ignore(&self) -> bool {
        self.ignore
    }

    pub fn ignore_chance(&self) -> Option<f64> {
        self.ignore_chance
    }

    pub fn initializer(&self) -> Option<&TokenStream> {
        self.initializer.as_ref()
    }

    pub fn little_endian(&self) -> bool {
        self.little_endian
    }

    pub fn big_endian(&self) -> bool {
        self.big_endian
    }

    pub fn weight_to(&self) -> Option<&WeightTo> {
        self.weight_to.as_ref()
    }
}

/// Represents enum variant information
pub struct Variant {
    weight: Option<u64>,
    ignore: bool,
    ignore_chance: Option<f64>,
}

impl Variant {
    /// Extract out the `#[lain()]` attributes from an enum variant
    pub fn from_ast(cx: &Ctxt, variant: &syn::Variant) -> Self {
        let mut weight = Attr::none(cx, WEIGHT);
        let mut ignore = BoolAttr::none(cx, IGNORE);
        let mut ignore_chance = Attr::none(cx, IGNORE_CHANCE);

        for attr in &variant.attrs {
            if *attr.path() == LAIN {
                if let Err(err) = attr.parse_nested_meta(|meta| {
                    if meta.path == WEIGHT {
                        let value = meta.value()?;
                        let i: LitInt = value.parse()?;
                        if let Ok(val) = i.base10_parse::<u64>() {
                            weight.set(&meta.path, val);
                        } else {
                            cx.error_spanned_by(
                                &i,
                                format!("failed to parse integer expression for {}", WEIGHT),
                            );
                        }
                    } else if meta.path == IGNORE {
                        ignore.set_true(&meta.path);
                    } else if meta.path == IGNORE_CHANCE {
                        let value = meta.value()?;
                        let lit: syn::Lit = value.parse()?;
                        if let Float(ref f) = lit {
                            if let Ok(val) = f.base10_parse::<f64>() {
                                ignore_chance.set(&meta.path, val);
                            }
                        } else if let Int(ref i) = lit {
                            if let Ok(val) = i.base10_parse::<f64>() {
                                ignore_chance.set(&meta.path, val);
                            }
                        } else {
                            cx.error_spanned_by(
                                &lit,
                                format!("failed to parse float expression for `{}`", IGNORE_CHANCE),
                            );
                        }
                    } else {
                        cx.error_spanned_by(
                            &meta.path,
                            format!(
                                "unknown lain variant attribute `{}`",
                                meta.path.to_token_stream()
                            ),
                        );
                    }
                    Ok(())
                }) {
                    cx.error_spanned_by(attr, err.to_string());
                }
            }
        }

        Variant {
            weight: weight.get(),
            ignore: ignore.get(),
            ignore_chance: ignore_chance.get(),
        }
    }

    pub fn weight(&self) -> Option<u64> {
        self.weight
    }

    pub fn ignore(&self) -> bool {
        self.ignore
    }

    pub fn ignore_chance(&self) -> Option<f64> {
        self.ignore_chance
    }
}

pub fn get_lit_str<'a>(
    cx: &Ctxt,
    attr_name: Symbol,
    meta_item_name: Symbol,
    lit: &'a syn::Lit,
) -> Result<&'a syn::LitStr, ()> {
    if let syn::Lit::Str(ref lit) = *lit {
        Ok(lit)
    } else {
        cx.error_spanned_by(
            lit,
            format!(
                "expected lain {} attribute to be a string: `{} = \"...\"`",
                attr_name, meta_item_name
            ),
        );
        Err(())
    }
}

/// Parses a `#[lain(min = ..)]` or `#[lain(max = ..)]` attribute
fn parse_min_max(cx: &Ctxt, attr_name: Symbol, lit: &syn::Lit) -> Result<TokenStream, ()> {
    // For a lit str we don't want to emit the tokens as a string, so we
    // reconstruct it as a TokenStream here
    if let syn::Lit::Str(ref s) = lit {
        if let Ok(value) = TokenStream::from_str(&s.value()) {
            Ok(quote_spanned! {lit.span() => #value})
        } else {
            cx.error_spanned_by(lit, format!("invalid tokens for {}", attr_name));
            Err(())
        }
    } else if let Int(i) = lit {
        // Reconstruct the int without any suffix. We want to use type
        // inference when we emit the tokens
        let int = LitInt::new(i.base10_digits(), lit.span());
        Ok(quote_spanned! {lit.span() => #int})
    } else {
        Ok(quote_spanned! {lit.span() => #lit})
    }
}

fn parse_lit_into_type(cx: &Ctxt, attr_name: Symbol, lit: &syn::Lit) -> Result<syn::Type, ()> {
    let string = get_lit_str(cx, attr_name, attr_name, lit)?;
    parse_lit_str(string).map_err(|_| {
        cx.error_spanned_by(lit, format!("failed to parse path: {:?}", string.value()))
    })
}

fn parse_lit_str<T>(s: &syn::LitStr) -> parse::Result<T>
where
    T: Parse,
{
    let tokens = spanned_tokens(s)?;
    syn::parse2(tokens)
}

fn spanned_tokens(s: &syn::LitStr) -> parse::Result<TokenStream> {
    let stream = syn::parse_str(&s.value())?;
    Ok(respan_token_stream(stream, s.span()))
}

fn respan_token_stream(stream: TokenStream, span: Span) -> TokenStream {
    stream
        .into_iter()
        .map(|token| respan_token_tree(token, span))
        .collect()
}

fn respan_token_tree(mut token: TokenTree, span: Span) -> TokenTree {
    if let TokenTree::Group(ref mut g) = token {
        *g = Group::new(g.delimiter(), respan_token_stream(g.stream().clone(), span));
    }
    token.set_span(span);
    token
}
