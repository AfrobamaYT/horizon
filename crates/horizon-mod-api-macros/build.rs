use std::{env, fs, path::PathBuf};

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    println!(
        "cargo:rerun-if-changed={}",
        root.join("native-hooks.json").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        root.join("tools/native-hook-fragments").display()
    );
    let hooks = horizon_mod_api_gen::hook_names(&root).expect("valid hook catalog");
    let mut output = String::from("const HOOKS: &[(&str, &str, bool)] = &[\n");
    for hook in hooks {
        output.push_str(&format!(
            "({:?}, {:?}, {}),\n",
            hook.method, hook.variant, hook.render
        ));
    }
    output.push_str("];\n");
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("hooks.rs"),
        output,
    )
    .unwrap();
}
