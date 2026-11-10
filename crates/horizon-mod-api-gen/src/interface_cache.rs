//! Build-local cache for walks that inspect signatures and module declarations, never bodies.
use quote::ToTokens;
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    fs,
    path::{Path, PathBuf},
};
use syn::visit_mut::VisitMut;

thread_local! {
    static DIRECTORY: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

pub(crate) fn with_cache<T>(
    out: &Path,
    run: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    // A new generator executable invalidates all summaries, including after parser upgrades.
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let identity = Sha256::digest(fs::read(executable).map_err(|e| e.to_string())?);
    let directory = out.join(format!("interfaces-{identity:x}"));
    fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    struct Reset(Option<PathBuf>);
    impl Drop for Reset {
        fn drop(&mut self) {
            DIRECTORY.with(|slot| *slot.borrow_mut() = self.0.take());
        }
    }
    let _reset = Reset(DIRECTORY.with(|slot| slot.replace(Some(directory))));
    run()
}

pub(crate) fn parse(path: &Path, source: &str) -> Result<syn::File, String> {
    let directory = DIRECTORY.with(|slot| slot.borrow().clone());
    let Some(directory) = directory else {
        return syn::parse_file(source).map_err(|e| format!("{}: {e}", path.display()));
    };
    let mut key = Sha256::new();
    key.update(path.as_os_str().as_encoded_bytes());
    key.update([0]);
    key.update(source.as_bytes());
    let cached = directory.join(format!("{:x}.rs", key.finalize()));
    match fs::read_to_string(&cached) {
        Ok(text) => {
            return syn::parse_file(&text).map_err(|e| format!("{}: {e}", cached.display()));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.to_string()),
    }
    let mut file = syn::parse_file(source).map_err(|e| format!("{}: {e}", path.display()))?;
    struct Signatures;
    impl VisitMut for Signatures {
        fn visit_item_fn_mut(&mut self, function: &mut syn::ItemFn) {
            function.block.stmts.clear();
        }
        fn visit_impl_item_fn_mut(&mut self, function: &mut syn::ImplItemFn) {
            function.block.stmts.clear();
        }
        fn visit_trait_item_fn_mut(&mut self, function: &mut syn::TraitItemFn) {
            if let Some(body) = &mut function.default {
                body.stmts.clear();
            }
        }
    }
    Signatures.visit_file_mut(&mut file);
    fs::write(cached, file.to_token_stream().to_string()).map_err(|e| e.to_string())?;
    Ok(file)
}
