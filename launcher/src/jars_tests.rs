use super::*;
use sha2::Digest;
use std::{cell::Cell, io::Write, path::PathBuf};

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("launcher-jars-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn source(seed: &[u8]) -> (JarRef, Vec<u8>) {
    let mut zip = zip::ZipWriter::new(io::Cursor::new(Vec::new()));
    zip.start_file(
        "assets/example/test.txt",
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    zip.write_all(seed).unwrap();
    let bytes = zip.finish().unwrap().into_inner();
    let jar = JarRef::new(
        "example".into(),
        "1".into(),
        format!("{:x}", sha2::Sha512::digest(&bytes)),
    )
    .unwrap();
    (jar, bytes)
}

fn manifest(jars: Vec<JarRef>) -> horizon_mods::manifest::Manifest {
    let mut manifest = horizon_mods::manifest::Manifest::parse(
        "id='example'\nname='Example'\nversion='1'\napi='0.2'\ncapabilities=['registries']\n",
    )
    .unwrap();
    manifest.jar = jars;
    manifest
}

#[test]
fn conflicting_metadata_for_one_exact_hash_is_refused_across_mods() {
    let (jar, _) = source(b"shared");
    for (id, version) in [("other", "1"), ("example", "2")] {
        let conflicting = JarRef::new(id.into(), version.into(), jar.sha512().into()).unwrap();
        let error = collect([
            &manifest(vec![jar.clone()]),
            &manifest(vec![conflicting.clone()]),
        ])
        .unwrap_err();
        assert_eq!(*error.first, jar);
        assert_eq!(*error.second, conflicting);
    }
}

#[test]
fn cache_hits_are_offline_and_corrupt_files_are_not_silently_replaced() {
    let home = Scratch::new();
    let (jar, bytes) = source(b"cache");
    let cache = home.0.join("jars");
    fs::create_dir_all(&cache).unwrap();
    let path = cache.join(format!("{}.jar", jar.sha512()));
    fs::write(&path, bytes).unwrap();
    prepare_with(
        &home.0,
        &[jar.clone()],
        Permission::Refuse,
        |_| panic!("cache hit must not use the network"),
        |_, _, _| panic!("download"),
    )
    .unwrap();
    fs::write(&path, b"corrupt").unwrap();
    let error = prepare_with(
        &home.0,
        &[jar],
        Permission::Allow,
        |_| panic!("corrupt cache must fail"),
        |_, _, _| panic!("download"),
    )
    .unwrap_err();
    assert!(error.contains("SHA-512 mismatch"));
    assert_eq!(fs::read(path).unwrap(), b"corrupt");
}

#[test]
fn all_missing_hashes_share_one_consent_and_each_download_once() {
    let home = Scratch::new();
    let (a, first) = source(b"first");
    let (b, second) = source(b"second");
    let prompts = Cell::new(0);
    let ask = |jars: &[&JarRef]| {
        prompts.set(prompts.get() + 1);
        assert_eq!(jars.len(), 2);
        true
    };
    let mut downloads = 0;
    let jars = collect([&manifest(vec![a.clone()]), &manifest(vec![b, a.clone()])]).unwrap();
    prepare_with(
        &home.0,
        &jars,
        Permission::Ask(&ask),
        |jar| Ok(Some(jar.sha512().to_owned())),
        |url, jar, dest| {
            assert_eq!(url, jar.sha512());
            downloads += 1;
            fs::write(
                dest,
                if jar.sha512() == a.sha512() {
                    &first
                } else {
                    &second
                },
            )
            .map_err(|e| e.to_string())
        },
    )
    .unwrap();
    assert_eq!(prompts.get(), 1);
    assert_eq!(downloads, 2);
    assert_eq!(fs::read_dir(home.0.join("jars")).unwrap().count(), 2);
}

#[test]
fn headless_refusal_is_actionable_and_never_contacts_modrinth() {
    let home = Scratch::new();
    let (jar, _) = source(b"missing");
    let expected = format!("{}.jar", jar.sha512());
    let error = prepare_with(
        &home.0,
        &[jar],
        Permission::Refuse,
        |_| panic!("headless refusal must be offline"),
        |_, _, _| panic!("download"),
    )
    .unwrap_err();
    assert!(error.contains("--allow-jar-downloads"));
    assert!(error.contains(&expected));
    assert!(!home.0.join("jars").exists());
}

#[test]
fn declined_consent_and_missing_or_failed_lookup_never_download() {
    let home = Scratch::new();
    let (jar, _) = source(b"missing");
    let ask = |_: &[&JarRef]| false;
    let error = prepare_with(
        &home.0,
        &[jar.clone()],
        Permission::Ask(&ask),
        |_| Ok(Some("exact.jar".into())),
        |_, _, _| panic!("declined"),
    )
    .unwrap_err();
    assert!(error.contains("declined"));
    for result in [Ok(None), Err("transport failed".to_owned())] {
        let ask = |_: &[&JarRef]| panic!("lookup failure must not prompt");
        let expected = if result.is_err() {
            "transport failed"
        } else {
            "no exact file"
        };
        let error = prepare_with(
            &home.0,
            &[jar.clone()],
            Permission::Ask(&ask),
            |_| result.clone(),
            |_, _, _| panic!("lookup failed"),
        )
        .unwrap_err();
        assert!(error.contains(expected));
    }
    assert!(!home.0.join("jars").exists());
}
