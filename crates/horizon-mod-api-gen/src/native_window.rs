//! Rule-driven native window projection generation.
use crate::{
    crossing::{Collection, Rule, Scalar, Shape, Wrapper},
    native_schema::{self, Exposure},
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Type;

/// Expression constructing a borrowed native node. The expression borrows `place` and is valid
/// only while that native value remains live. Unsupported shapes fail generation explicitly.
pub fn generate(
    rule: &Rule,
    module: &str,
    owner: Option<&str>,
    ty: &Type,
    place: TokenStream,
    exposure: &dyn Exposure,
) -> Result<TokenStream, String> {
    fn emit(
        rule: &Rule,
        shape: &Shape,
        place: TokenStream,
        exposure: &dyn Exposure,
        depth: usize,
    ) -> Result<TokenStream, String> {
        if depth > 128 {
            return Err("unimplemented: recursive native projection".into());
        }
        let identity = native_schema::identity(rule, shape, exposure)?;
        let key = quote!(mod_points::snapshot::key(#identity));
        Ok(match shape {
            Shape::Scalar(Scalar::String | Scalar::Str) => {
                let unit = native_schema::identity(
                    rule,
                    &Shape::Scalar(Scalar::Unsigned("u16")),
                    exposure,
                )?;
                quote!(mod_points::native_nodes::Text((#place).as_ref(), #key, mod_points::snapshot::key(#unit)))
            }
            Shape::Scalar(Scalar::Signed("isize")) => {
                return Err("unimplemented: isize has no Wire encoding".into());
            }
            Shape::Scalar(_) | Shape::Unit => {
                quote!(mod_points::native_nodes::Scalar(&(#place), #key))
            }
            Shape::Ref { to, .. }
            | Shape::Wrapped(Wrapper::Box | Wrapper::Arc | Wrapper::Rc, to) => {
                return emit(rule, to, quote!(**(#place)), exposure, depth + 1);
            }
            Shape::Wrapped(Wrapper::Cow, to) => {
                return emit(rule, to, quote!(*(#place).as_ref()), exposure, depth + 1);
            }
            Shape::Array(_, _) | Shape::Slice(_) | Shape::Collection(Collection::Vec, _) => {
                let to = match shape {
                    Shape::Collection(_, args) => {
                        args.first().ok_or("unimplemented: malformed vector")?
                    }
                    Shape::Array(to, _) | Shape::Slice(to) => to,
                    _ => unreachable!(),
                };
                let child = emit(rule, to, quote!(*__horizon_item), exposure, depth + 1)?;
                quote!(mod_points::native_nodes::Sequence { items: &(#place), key: #key, wrap: |__horizon_item, __horizon_visit| __horizon_visit(mod_points::locators::Step::Index(0), 0, &#child) })
            }
            Shape::Tuple(parts) => {
                let mut arms = Vec::new();
                for (index, part) in parts.iter().enumerate() {
                    let member = syn::Index::from(index);
                    let field = index as u32;
                    let ordinal = index as u64;
                    let child = emit(rule, part, quote!((#place).#member), exposure, depth + 1)?;
                    arms.push(quote!(#ordinal => __horizon_visit(mod_points::locators::Step::Field(#field), #field, &#child),));
                }
                let count = parts.len() as u64;
                quote!(mod_points::native_nodes::Record { key: #key, count: #count, visit: |__horizon_index, __horizon_visit| match __horizon_index { #(#arms)* _ => Err(mod_api::RowError::Bounds) } })
            }
            Shape::Named(path, _) => {
                let named = rule
                    .named(path)
                    .ok_or("unimplemented: missing named layout")?;
                if !matches!(named.item, syn::Item::Struct(_)) {
                    return Err(format!("unimplemented: enum {path}"));
                }
                let mut arms = Vec::new();
                for (index, field) in rule.fields(path).iter().enumerate() {
                    if !exposure.field(path, None, index as u32) {
                        continue;
                    }
                    let member = &field.member;
                    let shape = rule.shape(&named.module, Some(path), field.ty);
                    let child = emit(rule, &shape, quote!((#place).#member), exposure, depth + 1)?;
                    let field_id = index as u32;
                    let ordinal = arms.len() as u64;
                    arms.push(quote!(#ordinal => __horizon_visit(mod_points::locators::Step::Field(#field_id), #field_id, &#child),));
                }
                let count = arms.len() as u64;
                quote!(mod_points::native_nodes::Record { key: #key, count: #count, visit: |__horizon_index, __horizon_visit| match __horizon_index { #(#arms)* _ => Err(mod_api::RowError::Bounds) } })
            }
            _ => {
                return Err(format!(
                    "unimplemented: native row projection for {shape:?}"
                ));
            }
        })
    }
    emit(rule, &rule.shape(module, owner, ty), place, exposure, 0)
}
