use super::*;
use crate::crossing::tests::internal;

fn entries() -> Vec<Internal> {
    vec![
        internal(
            "horizon_client::game::Game",
            "struct",
            "struct Game { value: u32 }",
        ),
        internal(
            "horizon_client::game::Game::run",
            "method",
            "impl Game { fn run(&mut self) -> Result<u32, ()> }",
        ),
        internal(
            "horizon_client::game::Game::step",
            "method",
            "impl Game { fn step(&mut self, value: u32) -> u32 }",
        ),
        internal(
            "horizon_client::game::add",
            "fn",
            "fn add(value: u32) -> u32",
        ),
    ]
}
fn declaration(selector: &str, ordinal: Option<usize>) -> Declaration {
    Declaration {
        target: "horizon_client::game::Game::run".into(),
        selector: selector.into(),
        ordinal,
        origin: "sites/test.txt:2".into(),
    }
}

#[test]
fn site_rewrite_preserves_method_path_closure_and_question_mark_results() {
    let source = "pub struct Game { pub value: u32 } impl Game { fn step(&mut self, value: u32) -> u32 { self.value += value; self.value } pub fn run(&mut self) -> Result<u32, ()> { let mut f = || self.step(2); let n = Ok::<_, ()>(f())?; Ok(n + add(3)) } } fn add(value: u32) -> u32 { value + 1 }";
    let (rewritten, sites) = rewrite(
        source,
        "horizon_client::game",
        &entries(),
        &[
            declaration("self.step", Some(0)),
            declaration("add", Some(0)),
        ],
    )
    .unwrap();
    assert_eq!(sites.len(), 2);
    syn::parse_file(&rewritten).unwrap();
    let variants = sites
        .iter()
        .map(|site| format!("{},", site.name))
        .collect::<String>();
    let fixture = format!(
        r#"
        #![forbid(unsafe_code)]
        extern crate self as mod_points;
        extern crate self as horizon_mod_api_macros;
        pub enum SiteId {{ {variants} }}
        pub mod __private {{ pub static SITES: [std::sync::atomic::AtomicBool; 2] = [const {{ std::sync::atomic::AtomicBool::new(false) }}; 2]; }}
        #[macro_export] macro_rules! site {{ ($place:ident, $listener:ident, $recv_ty:ident, $record:ident, $output:ident, $receiver:expr, $args:expr, $original:expr) => {{ ($original)($receiver, &mut $args) }}; }}
        mod game {{ {rewritten} }}
        fn main() {{
            for enabled in [false, true] {{
                for flag in &__private::SITES {{ flag.store(enabled, std::sync::atomic::Ordering::Relaxed); }}
                let mut game = game::Game {{ value: 0 }};
                assert_eq!(game.run(), Ok(6));
                assert_eq!(game.value, 2);
            }}
        }}
    "#
    );
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target")
        .join(format!("site-rewrite-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let file = directory.join("fixture.rs");
    let binary = directory.join("fixture");
    fs::write(&file, fixture).unwrap();
    let compiled = std::process::Command::new("rustc")
        .args(["--edition=2024", "-Awarnings"])
        .arg(&file)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    assert!(
        std::process::Command::new(binary)
            .status()
            .unwrap()
            .success()
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn site_rewrite_ordinals_hashes_and_refusals() {
    let source = "impl Game { fn run(&mut self) { self.step(1); self.step(2); } }";
    let declarations = [declaration(".step", Some(1))];
    let (_, sites) = rewrite(source, "horizon_client::game", &entries(), &declarations).unwrap();
    let (_, changed) = rewrite(
        &source.replace("step(2)", "step(3)"),
        "horizon_client::game",
        &entries(),
        &declarations,
    )
    .unwrap();
    assert_eq!(sites[0].ordinal, 1);
    assert_eq!(sites[0].name, changed[0].name);
    assert_ne!(sites[0].hash, changed[0].hash);
    for (selector, ordinal, reason) in [
        (".step", None, "ambiguous"),
        (".step", Some(2), "ordinal"),
        (".missing", Some(0), "matched no call"),
    ] {
        let error = rewrite(
            source,
            "horizon_client::game",
            &entries(),
            &[declaration(selector, ordinal)],
        )
        .unwrap_err();
        assert!(
            error.contains("sites/test.txt:2") && error.contains(reason),
            "{error}"
        );
    }
    let mut conflict = declaration("self.step", Some(1));
    conflict.origin = "sites/other.txt:4".into();
    let error = rewrite(
        source,
        "horizon_client::game",
        &entries(),
        &[declarations[0].clone(), conflict],
    )
    .unwrap_err();
    assert!(
        error.contains("sites/test.txt:2") && error.contains("sites/other.txt:4"),
        "{error}"
    );
    let nested = "impl Game { fn run(&mut self) { add(add(1)); } }";
    assert!(
        rewrite(
            nested,
            "horizon_client::game",
            &entries(),
            &[declaration("add", Some(0)), declaration("add", Some(1))]
        )
        .unwrap_err()
        .contains("overlap")
    );
    let bad = "impl Game { fn run(&mut self) { self.step(self.value); } }";
    assert!(
        rewrite(
            bad,
            "horizon_client::game",
            &entries(),
            &[declaration(".step", None)]
        )
        .unwrap_err()
        .contains("two-phase")
    );
}
