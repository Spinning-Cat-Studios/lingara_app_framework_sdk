//! `unions.rs` from `x-lingara-unions` and `x-lingara-app-operations` (ADR
//! 30.9.26am D6).
//!
//! - One enum per union, internally tagged on the union's tag, one variant
//!   per arm in the view's order holding typify's arm struct (S1 D4's lifted
//!   name, `CardElementHeading` …), with `From` per arm;
//! - one `<Union><Tag>` enum per union (`ContextSliceKind`, …) with `ALL`,
//!   `as_str` and `parse`: the tag values as a closed, constant list, which
//!   the manifest builder's `context` reads;
//! - `Operation`, the request discriminators (`app.render`, `app.action`) and
//!   the reply message beside them, matched exhaustively by the core, so a
//!   third operation fails the build until the core handles it;
//! - and, over typify's `models.rs`, `relax`: no request-side type keeps
//!   `deny_unknown_fields` (D6's lenient decoding).

use std::collections::BTreeSet;

use proc_macro2::{Ident, Span, TokenStream};
use quote::{ToTokens, quote};

use crate::view::{Arm, Operation, Union, View};

pub fn render(view: &View) -> TokenStream {
    let unions = view.unions.iter().map(union_enum);
    let tags = view.unions.iter().map(tag_enum);
    let operations = operation_enum(&view.operations);
    quote! {
        #(#unions)*
        #(#tags)*
        #operations
    }
}

fn union_enum(u: &Union) -> TokenStream {
    let doc = format!(" `{}`, tagged on `{}`: one variant per arm.", u.name, u.tag);
    let (name, tag, tag_enum) = (ident(&u.name), &u.tag, ident(&tag_enum_name(u)));
    let variants: Vec<Ident> = u.arms.iter().map(|a| ident(&variant_name(u, a))).collect();
    let arms: Vec<Ident> = u.arms.iter().map(|a| ident(&a.schema)).collect();
    let values = u.arms.iter().map(|a| &a.value);
    quote! {
        #[doc = #doc]
        #[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
        #[serde(tag = #tag)]
        pub enum #name {
            #( #[serde(rename = #values)] #variants(super::models::#arms), )*
        }
        #(
            impl From<super::models::#arms> for #name {
                fn from(arm: super::models::#arms) -> Self {
                    #name::#variants(arm)
                }
            }
        )*
        impl #name {
            /// This value's tag.
            pub fn tag(&self) -> #tag_enum {
                match self {
                    #( #name::#variants(_) => #tag_enum::#variants, )*
                }
            }
        }
    }
}

fn tag_enum(u: &Union) -> TokenStream {
    let doc = format!(" The `{}` values of `{}`, as a closed list.", u.tag, u.name);
    let name = ident(&tag_enum_name(u));
    let variants: Vec<Ident> = u.arms.iter().map(|a| ident(&variant_name(u, a))).collect();
    let values: Vec<&String> = u.arms.iter().map(|a| &a.value).collect();
    let count = proc_macro2::Literal::usize_unsuffixed(variants.len());
    quote! {
        #[doc = #doc]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Deserialize, serde::Serialize)]
        pub enum #name {
            #( #[serde(rename = #values)] #variants, )*
        }
        impl #name {
            pub const ALL: [#name; #count] = [ #( #name::#variants, )* ];
            pub const fn as_str(self) -> &'static str {
                match self {
                    #( #name::#variants => #values, )*
                }
            }
            pub fn parse(value: &str) -> Option<Self> {
                match value {
                    #( #values => Some(#name::#variants), )*
                    _ => None,
                }
            }
        }
    }
}

fn operation_enum(operations: &[Operation]) -> TokenStream {
    let variants: Vec<Ident> = operations.iter().map(|o| ident(&camel(&o.message))).collect();
    let messages: Vec<&String> = operations.iter().map(|o| &o.message).collect();
    let replies = operations.iter().map(|o| &o.reply_message);
    let count = proc_macro2::Literal::usize_unsuffixed(variants.len());
    quote! {
        /// The requests the relay sends an app, from `x-lingara-app-operations`.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum Operation {
            #( #variants, )*
        }
        impl Operation {
            pub const ALL: [Operation; #count] = [ #( Operation::#variants, )* ];
            /// The request's `type`.
            pub const fn message(self) -> &'static str {
                match self {
                    #( Operation::#variants => #messages, )*
                }
            }
            /// The message the app answers with.
            pub const fn reply_message(self) -> &'static str {
                match self {
                    #( Operation::#variants => #replies, )*
                }
            }
            pub fn from_message(message: &str) -> Option<Self> {
                match message {
                    #( #messages => Some(Operation::#variants), )*
                    _ => None,
                }
            }
        }
    }
}

/// `ContextSlice` + `kind` → `ContextSliceKind`.
pub(crate) fn tag_enum_name(u: &Union) -> String {
    format!("{}{}", u.name, camel(&u.tag))
}

/// The arm's name without its union's prefix: `CardElementHeading` →
/// `Heading`; an arm not lifted from its union keeps its whole name.
fn variant_name(u: &Union, arm: &Arm) -> String {
    match arm.schema.strip_prefix(&u.name) {
        Some(rest) if rest.starts_with(|c: char| c.is_ascii_uppercase()) => rest.to_owned(),
        _ => arm.schema.clone(),
    }
}

/// `app.render` → `AppRender`; `kind` → `Kind`.
fn camel(text: &str) -> String {
    text.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(|part| part[..1].to_ascii_uppercase() + &part[1..])
        .collect()
}

fn ident(name: &str) -> Ident {
    Ident::new(name, Span::call_site())
}

/// Drops `#[serde(deny_unknown_fields)]` from the request-side types (D6): an
/// unknown request field is ignored, whatever `additionalProperties` says.
pub(crate) fn relax(file: &mut syn::File, request_side: &BTreeSet<String>) {
    for item in &mut file.items {
        let (ident, attrs) = match item {
            syn::Item::Struct(s) => (&s.ident, &mut s.attrs),
            syn::Item::Enum(e) => (&e.ident, &mut e.attrs),
            _ => continue,
        };
        if request_side.contains(&ident.to_string()) {
            attrs.retain(|attr| !is_deny_unknown_fields(attr));
        }
    }
}

pub(crate) fn is_deny_unknown_fields(attr: &syn::Attribute) -> bool {
    match &attr.meta {
        syn::Meta::List(list) => list.path.is_ident("serde") && list.tokens.to_token_stream().to_string() == "deny_unknown_fields",
        _ => false,
    }
}

#[cfg(test)]
#[path = "unions_tests.rs"]
mod tests;
