//! List-driven call-expression slots. The staging coordinator activates these after widening.
use crate::{
    crossing::{Rule, Shape},
    internals::Internal,
    points::plain_path,
};
use quote::{ToTokens, quote};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
use syn::{Expr, FnArg, Type};

/// One requested call expression, with its list-file diagnostic location.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Declaration {
    pub target: String,
    pub selector: String,
    pub ordinal: Option<usize>,
    pub origin: String,
}

/// Read the sorted union of per-mod site files. Missing directories mean no sites.
pub fn listed(directory: &Path) -> Result<Vec<Declaration>, String> {
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut files = fs::read_dir(directory)
        .map_err(|e| e.to_string())?
        .map(|e| e.map(|e| e.path()).map_err(|e| e.to_string()))
        .collect::<Result<Vec<PathBuf>, _>>()?;
    files.sort();
    let mut union = BTreeMap::new();
    for file in files
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e == "txt"))
    {
        let text = fs::read_to_string(&file).map_err(|e| e.to_string())?;
        for (i, line) in text.lines().enumerate() {
            let line = line.split('#').next().unwrap_or_default().trim();
            if line.is_empty() {
                continue;
            }
            let origin = format!("{}:{}", file.display(), i + 1);
            let parts: Vec<_> = line.split('|').map(str::trim).collect();
            let [target, selector, ordinal] = parts.as_slice() else {
                return Err(format!("{origin}: expected target | selector | ordinal"));
            };
            syn::parse_str::<syn::Path>(target).map_err(|e| format!("{origin}: target: {e}"))?;
            let call = if selector.starts_with('.') {
                format!("__receiver{selector}()")
            } else {
                format!("{selector}()")
            };
            if !matches!(
                syn::parse_str::<Expr>(&call),
                Ok(Expr::Call(_) | Expr::MethodCall(_))
            ) {
                return Err(format!(
                    "{origin}: selector must name a method or path call"
                ));
            }
            let ordinal = if ordinal.is_empty() {
                None
            } else {
                Some(
                    ordinal
                        .parse()
                        .map_err(|_| format!("{origin}: ordinal must be a nonnegative integer"))?,
                )
            };
            let target = plain_path(target);
            let selector = selector.replace(' ', "");
            union
                .entry((target.clone(), selector.clone(), ordinal))
                .or_insert(Declaration {
                    target,
                    selector,
                    ordinal,
                    origin,
                });
        }
    }
    Ok(union.into_values().collect())
}

/// A compiled call site's raw form, written to `points.json` beside the head/return points.
#[derive(Debug, serde::Serialize)]
pub struct Site {
    pub name: String,
    pub target: String,
    pub selector: String,
    pub ordinal: usize,
    pub hash: String,
    pub listener: String,
    pub record: String,
    pub receiver: String,
    pub args: Vec<(String, String)>,
    pub output: String,
}

fn named(shape: Shape) -> Result<String, String> {
    match shape {
        Shape::Named(path, _) => Ok(path),
        Shape::Ref { to, .. } => named(*to),
        _ => Err("unresolved receiver type".into()),
    }
}

/// Resolve a receiver through the internals catalog's import and field type information.
fn receiver(
    expr: &Expr,
    module: &str,
    this: Option<&str>,
    rule: &Rule,
    sig: &syn::Signature,
) -> Result<String, String> {
    match expr {
        Expr::Path(path) if path.path.is_ident("self") => this
            .map(str::to_owned)
            .ok_or_else(|| "self outside a method".into()),
        Expr::Path(path) => {
            let ty = sig
                .inputs
                .iter()
                .find_map(|arg| match arg {
                    FnArg::Typed(arg)
                        if arg.pat.to_token_stream().to_string()
                            == path.to_token_stream().to_string() =>
                    {
                        Some(&*arg.ty)
                    }
                    _ => None,
                })
                .ok_or("unresolved receiver: local has no parameter type annotation")?;
            named(rule.shape(module, this, ty))
        }
        Expr::Field(field) => {
            let owner = receiver(&field.base, module, this, rule, sig)?;
            let field_ty = rule
                .fields(&owner)
                .into_iter()
                .find(|f| f.case.is_none() && f.member == field.member)
                .ok_or("unresolved receiver field")?;
            let scope = &rule
                .named(&owner)
                .ok_or("unresolved receiver owner")?
                .module;
            named(rule.shape(scope, Some(&owner), field_ty.ty))
        }
        _ => Err(
            "unresolved receiver: chained calls and computed receivers are not supported".into(),
        ),
    }
}

/// A resolved call, using the same catalog and type rule as the internals view.
pub struct Resolved<'a> {
    pub callee: &'a Internal,
    pub signature: syn::Signature,
    pub receiver: Option<&'a Expr>,
    pub receiver_type: proc_macro2::TokenStream,
    pub arguments: Vec<&'a Expr>,
    pub fields: Vec<syn::Ident>,
    pub types: Vec<Type>,
    pub output: proc_macro2::TokenStream,
}

/// Resolve a selected call and refuse shapes that cannot have a repeatable raw continuation.
pub fn resolve<'a>(
    expr: &'a Expr,
    enclosing: &Internal,
    enclosing_sig: &syn::Signature,
    entries: &'a [Internal],
    rule: &Rule,
) -> Result<Resolved<'a>, String> {
    let (scope, this) = crate::internals_table::function_scope(rule, enclosing)
        .ok_or("unresolved enclosing receiver")?;
    let (callee, recv, arguments) = match expr {
        Expr::MethodCall(call) => {
            if call.turbofish.is_some() {
                return Err("generic callee".into());
            }
            let owner = receiver(&call.receiver, &scope, this.as_deref(), rule, enclosing_sig)?;
            let mut candidates = entries
                .iter()
                .filter(|e| {
                    e.kind == "method"
                        && e.cfg.is_empty()
                        && e.path.ends_with(&format!("::{}", call.method))
                })
                .filter(|e| {
                    crate::internals_table::function_scope(rule, e)
                        .is_some_and(|(_, ty)| ty.as_deref() == Some(&owner))
                });
            let callee = candidates
                .next()
                .ok_or("callee not in the internals catalog")?;
            if candidates.next().is_some() {
                return Err("ambiguous callee".into());
            }
            (
                callee,
                Some(&*call.receiver),
                call.args.iter().collect::<Vec<_>>(),
            )
        }
        Expr::Call(call) => {
            let Expr::Path(path) = &*call.func else {
                return Err("indirect call".into());
            };
            if path.path.segments.iter().any(|s| !s.arguments.is_empty()) {
                return Err("generic callee".into());
            }
            let path = path.to_token_stream().to_string().replace(' ', "");
            let callee = entries
                .iter()
                .find(|e| {
                    matches!(e.kind, "fn" | "method")
                        && e.cfg.is_empty()
                        && (plain_path(&e.path) == path
                            || plain_path(&e.path) == format!("{scope}::{path}"))
                })
                .ok_or("callee not in the internals catalog")?;
            (callee, None, call.args.iter().collect())
        }
        _ => unreachable!(),
    };
    let sig =
        crate::internals_table::signature(&callee.signature).ok_or("generic or bounded callee")?;
    if sig.generics.params.len() != sig.generics.lifetimes().count()
        || sig.unsafety.is_some()
        || sig.asyncness.is_some()
        || sig.variadic.is_some()
    {
        return Err("generic, unsafe, async or variadic callee".into());
    }
    let (callee_module, callee_this) =
        crate::internals_table::function_scope(rule, callee).ok_or("unresolved callee scope")?;
    let mut types = Vec::new();
    let mut fields = Vec::new();
    for arg in &sig.inputs {
        if let FnArg::Typed(arg) = arg {
            let syn::Pat::Ident(pat) = &*arg.pat else {
                return Err("destructured callee parameter".into());
            };
            let shape = rule.shape(&callee_module, callee_this.as_deref(), &arg.ty);
            // Moving an owned field out of &mut Args cannot implement a repeatable
            // original. Borrowing fields need a separate alias analysis, not guesses.
            if !matches!(
                shape,
                Shape::Scalar(
                    crate::crossing::Scalar::Bool
                        | crate::crossing::Scalar::Char
                        | crate::crossing::Scalar::Signed(_)
                        | crate::crossing::Scalar::Unsigned(_)
                        | crate::crossing::Scalar::Float(_)
                        | crate::crossing::Scalar::Wide(_)
                ) | Shape::Unit
            ) {
                return Err("argument record requires a borrowing or non-Copy field".into());
            }
            types.push((*arg.ty).clone());
            fields.push(pat.ident.clone());
        }
    }
    if arguments.len() != fields.len() {
        return Err("callee argument count differs".into());
    }
    if arguments.iter().any(|arg| {
        arg.to_token_stream()
            .to_string()
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .any(|s| s == "self")
    }) {
        return Err(
            "argument borrows or reads the receiver: two-phase borrowing is not record-safe".into(),
        );
    }
    let output = match &sig.output {
        syn::ReturnType::Default => quote!(()),
        syn::ReturnType::Type(_, ty) => {
            let shape = rule.shape(&callee_module, callee_this.as_deref(), ty);
            if matches!(shape, Shape::Ref { .. } | Shape::Opaque(_)) {
                return Err("borrowed or unresolved call result".into());
            }
            crate::internals_table::spell(&shape).ok_or("unresolved call result")?
        }
    };
    let recv_type = match (sig.receiver(), callee_this.as_ref()) {
        (Some(r), Some(owner)) if r.reference.is_some() => {
            let shape = rule.shape(&callee_module, Some(owner), &syn::parse_quote!(Self));
            let ty = crate::internals_table::spell(&shape)
                .ok_or("unresolved receiver spelling")?
                .to_string()
                .replace("'static", "'g")
                .parse::<proc_macro2::TokenStream>()
                .map_err(|e| e.to_string())?;
            if r.mutability.is_some() {
                quote!(&mut #ty)
            } else {
                quote!(&#ty)
            }
        }
        (None, _) => quote!(()),
        _ => return Err("by-value receiver".into()),
    };

    Ok(Resolved {
        callee,
        signature: sig,
        receiver: recv,
        receiver_type: recv_type,
        arguments,
        fields,
        types,
        output,
    })
}

mod rewrite;
pub use rewrite::rewrite;
mod stage;
pub use stage::{ids, stage, tracked};

#[cfg(test)]
mod tests;
