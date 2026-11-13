use super::*;

const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// An index listing `files` (name, bytes), as the writer serializes it.
fn index(files: &[(&str, &[u8])]) -> Index {
    let assets = files
        .iter()
        .map(|(name, bytes)| {
            let (kind, codec, target, side) = Asset::parts_of(name).unwrap();
            let asset = Asset {
                sha256: sha256_hex(bytes),
                size: bytes.len() as u64,
                kind,
                codec,
                target,
                side,
            };
            (name.to_string(), asset)
        })
        .collect();
    Index {
        schema: SCHEMA,
        build: Build {
            commit: COMMIT.into(),
            game: GAME.into(),
            date: "2026-11-13".into(),
            nightly: None,
        },
        package_format: PACKAGE_FORMAT,
        toolchain: Toolchain {
            rust: "1.99.1".into(),
        },
        content_api: crate::manifest::API.into(),
        surface: Surface {
            hash: "ab".repeat(16),
        },
        mixins: Mixins {
            rev: crate::mixins_tool::REV.into(),
            patch: crate::mixins_tool::patch_hash(),
        },
        assets,
        inputs: inputs(),
    }
}

fn json(index: &Index) -> serde_json::Value {
    serde_json::to_value(index).unwrap()
}

fn read(value: &serde_json::Value) -> Result<Index, Refusal> {
    Index::read(&serde_json::to_vec(value).unwrap(), COMMIT)
}

const ENGINE: &str = "engine-x86_64-unknown-linux-gnu.tar.xz";

#[test]
fn written_index_reads_back_and_unknown_fields_are_ignored() {
    let written = index(&[("surface.json", b"{}"), (ENGINE, b"engine")]);
    let mut value = json(&written);
    value["later"] = serde_json::json!({"field": 1});
    value["assets"][ENGINE]["later"] = serde_json::json!(true);
    assert_eq!(read(&value).unwrap(), written);
}

#[test]
fn each_bad_index_is_refused_by_name() {
    let good = json(&index(&[(ENGINE, b"engine")]));
    let cases: [(&str, fn(&mut serde_json::Value), Refusal); 7] = [
        (
            "unknown schema",
            |v| v["schema"] = 2.into(),
            Refusal::Schema(2),
        ),
        (
            "wrong build",
            |v| v["build"]["commit"] = "f".repeat(40).into(),
            Refusal::Build {
                expected: COMMIT.into(),
                found: "f".repeat(40),
            },
        ),
        (
            "unknown package format",
            |v| v["package_format"] = 5.into(),
            Refusal::PackageFormat(5),
        ),
        (
            "unknown codec",
            |v| v["assets"][ENGINE]["codec"] = "zstd".into(),
            Refusal::Codec {
                asset: ENGINE.into(),
                codec: "zstd".into(),
            },
        ),
        (
            "unknown kind",
            |v| v["assets"][ENGINE]["kind"] = "slice".into(),
            Refusal::Kind {
                asset: ENGINE.into(),
                kind: "slice".into(),
            },
        ),
        (
            "a path for a name",
            |v| {
                let asset = v["assets"].as_object_mut().unwrap().remove(ENGINE).unwrap();
                v["assets"]["../engine-x86_64-unknown-linux-gnu.tar.xz"] = asset;
            },
            Refusal::AssetName("../engine-x86_64-unknown-linux-gnu.tar.xz".into()),
        ),
        (
            "a name that says another side",
            |v| v["assets"][ENGINE]["side"] = "server".into(),
            Refusal::AssetName(ENGINE.into()),
        ),
    ];
    for (case, change, refusal) in cases {
        let mut value = good.clone();
        change(&mut value);
        assert_eq!(read(&value).unwrap_err(), refusal, "{case}");
    }
    for broken in [&b"not json"[..], br#"{"schema": 1}"#, b"[]"] {
        assert!(
            matches!(Index::read(broken, COMMIT), Err(Refusal::Malformed(_))),
            "{}",
            String::from_utf8_lossy(broken)
        );
    }
}

#[test]
fn an_asset_is_checked_by_size_then_checksum() {
    let listed = index(&[(ENGINE, b"engine")]);
    listed.verify(ENGINE, &b"engine"[..]).unwrap();
    assert_eq!(
        listed.verify(ENGINE, &b"engine and more"[..]).unwrap_err(),
        Refusal::Size {
            asset: ENGINE.into(),
            expected: 6,
            found: 7,
        }
    );
    assert_eq!(
        listed.verify(ENGINE, &b"ENGINE"[..]).unwrap_err(),
        Refusal::Checksum {
            asset: ENGINE.into()
        }
    );
    assert_eq!(
        listed.verify("surface.json", &b"{}"[..]).unwrap_err(),
        Refusal::Unlisted("surface.json".into())
    );
}

#[test]
fn a_folder_with_a_missing_or_unlisted_asset_is_refused() {
    let listed = index(&[("surface.json", b"{}"), (ENGINE, b"engine")]);
    let folder = tempfile::tempdir().unwrap();
    let write = |name: &str, bytes: &[u8]| std::fs::write(folder.path().join(name), bytes).unwrap();
    write("surface.json", b"{}");
    write(INDEX, b"index");
    assert_eq!(
        listed.verify_folder(folder.path()).unwrap_err(),
        Refusal::Missing(vec![ENGINE.into()])
    );
    write(ENGINE, b"engine");
    listed.verify_folder(folder.path()).unwrap();
    write("api-source.tar.xz", b"source");
    assert_eq!(
        listed.verify_folder(folder.path()).unwrap_err(),
        Refusal::Unlisted("api-source.tar.xz".into())
    );
}

#[test]
fn a_release_folders_index_lists_each_asset_and_reads_back() {
    let folder = tempfile::tempdir().unwrap();
    let write = |name: &str, bytes: &[u8]| std::fs::write(folder.path().join(name), bytes).unwrap();
    let surface = format!(r#"{{"hash":"{}","hooks":[]}}"#, "ab".repeat(16));
    write("surface.json", surface.as_bytes());
    write(ENGINE, b"engine");
    write("SHA256SUMS", b"sums");
    let build = Build {
        commit: COMMIT.into(),
        game: GAME.into(),
        date: "2026-11-13".into(),
        nightly: None,
    };
    let written = Index::of_folder(folder.path(), build.clone(), "1.99.1".into()).unwrap();
    assert_eq!(
        written,
        index(&[("surface.json", surface.as_bytes()), (ENGINE, b"engine")])
    );
    assert_eq!(
        Index::read(written.to_json().as_bytes(), COMMIT).unwrap(),
        written
    );
    write("notes.txt", b"stray");
    assert_eq!(
        Index::of_folder(folder.path(), build, "1.99.1".into()).unwrap_err(),
        Refusal::AssetName("notes.txt".into())
    );
}

#[test]
fn a_route_names_its_assets_for_the_target_and_every_missing_one() {
    let listed = index(&[(ENGINE, b"engine")]);
    let target = "x86_64-unknown-linux-gnu";
    assert_eq!(
        listed.route_assets("api", target, Side::Client).unwrap(),
        [ENGINE]
    );
    assert_eq!(
        listed
            .route_assets("engine", target, Side::Server)
            .unwrap_err(),
        Refusal::Missing(vec!["engine-x86_64-unknown-linux-gnu-server.tar.xz".into()])
    );
}

#[test]
fn asset_names_state_kind_target_and_side() {
    for (name, parts) in [
        (
            "surface.json",
            Some((Kind::Surface, Codec::None, None, None)),
        ),
        (
            "build-data.tar.xz",
            Some((Kind::BuildData, Codec::Xz, None, None)),
        ),
        (
            "runtime-aarch64-apple-darwin.tar.xz",
            Some((
                Kind::Runtime,
                Codec::Xz,
                Some("aarch64-apple-darwin".into()),
                Some(Side::Client),
            )),
        ),
        (
            "engine-x86_64-unknown-linux-gnu-server.tar.xz",
            Some((
                Kind::Engine,
                Codec::Xz,
                Some("x86_64-unknown-linux-gnu".into()),
                Some(Side::Server),
            )),
        ),
        ("engine-.tar.xz", None),
        ("build-data-x.tar.xz", None),
        ("sdk/engine-x.tar.xz", None),
        ("surface.json.xz", None),
    ] {
        assert_eq!(Asset::parts_of(name), parts, "{name}");
    }
}

#[test]
fn removing_each_required_input_names_it_before_work() {
    let inputs = inputs();
    for route in ["package", "package-server"] {
        let folder = tempfile::tempdir().unwrap();
        let checkout = folder.path().join("checkout");
        let program = folder.path().join("bin/horizon-client.exe");
        let listed = inputs.route(route);
        let files: Vec<(String, PathBuf)> = listed
            .program_files(&program)
            .into_iter()
            .map(|path| (path.file_name().unwrap().to_string_lossy().into(), path))
            .chain(
                listed
                    .checkout
                    .iter()
                    .map(|name| (name.clone(), checkout.join(name))),
            )
            .collect();
        assert!(
            files
                .iter()
                .any(|(name, _)| name == "horizon-wasm-std.wasm")
        );
        assert!(files.iter().any(|(name, _)| name == "horizon-server.exe"));
        for (_, path) in &files {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, b"").unwrap();
        }
        inputs.check(route, &program, &checkout).unwrap();
        for (name, path) in &files {
            std::fs::remove_file(path).unwrap();
            let Err(Refusal::MissingInputs { missing, how }) =
                inputs.check(route, &program, &checkout)
            else {
                panic!("{route}: {name} missing was not refused");
            };
            assert_eq!(missing, std::slice::from_ref(name), "{route}");
            assert_eq!(how.len(), 1, "{route}: {name} has no way to make it");
            std::fs::write(path, b"").unwrap();
        }
    }
}

#[test]
fn every_named_input_has_a_way_to_make_it_and_the_book_lists_it() {
    let inputs = inputs();
    let page = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/modding/src/how-to/native-mods/07-local-developer-builds.md"),
    )
    .unwrap();
    for route in inputs.routes.values() {
        for name in route.program.iter().chain(&route.checkout) {
            assert!(inputs.how.contains_key(name), "{name}");
            assert!(page.contains(&format!("`{name}`")), "the book omits {name}");
        }
        for kind in &route.sdk {
            assert!(inputs.how.contains_key(kind.name()), "{}", kind.name());
        }
    }
}

#[cfg(feature = "prebuilt")]
mod unpack {
    use super::*;

    fn tar(entries: &[(&str, &[u8])], link: Option<&str>) -> Vec<u8> {
        let mut builder = tar::Builder::new(Vec::new());
        for (path, bytes) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_size(bytes.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append_data(&mut header, path, *bytes).unwrap();
        }
        if let Some(target) = link {
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(tar::EntryType::Symlink);
            header.set_size(0);
            builder.append_link(&mut header, "link", target).unwrap();
        }
        builder.into_inner().unwrap()
    }

    fn xz(bytes: &[u8]) -> Vec<u8> {
        let mut packed = Vec::new();
        lzma_rs::xz_compress(&mut &bytes[..], &mut packed).unwrap();
        packed
    }

    #[test]
    fn an_archive_unpacks_within_its_limit_and_a_bomb_is_refused() {
        let packed = xz(&tar(&[("deps/a.rmeta", &[0u8; 1 << 16])], None));
        let listed = index(&[(ENGINE, &packed)]);
        let folder = tempfile::tempdir().unwrap();
        let into = folder.path().join("sdk");
        unpack(&listed, ENGINE, &packed[..], &into, 1 << 20).unwrap();
        assert_eq!(
            std::fs::read(into.join("deps/a.rmeta")).unwrap().len(),
            1 << 16
        );
        // Only the destination is left: the scratch tar is gone.
        assert_eq!(std::fs::read_dir(folder.path()).unwrap().count(), 1);
        let small = folder.path().join("small");
        assert_eq!(
            unpack(&listed, ENGINE, &packed[..], &small, 1 << 12).unwrap_err(),
            Refusal::TooLarge {
                asset: ENGINE.into(),
                limit: 1 << 12,
            }
        );
    }

    #[test]
    fn an_archive_the_packager_wrote_unpacks() {
        // Written by `tools/prebuilt_game.py`'s `tar_xz` (tests/test_prebuilt_game.py checks it still is).
        let packed = include_bytes!("fixture.tar.xz");
        let listed = index(&[(ENGINE, packed)]);
        let folder = tempfile::tempdir().unwrap();
        unpack(&listed, ENGINE, &packed[..], folder.path(), 1 << 20).unwrap();
        assert_eq!(
            std::fs::read_to_string(folder.path().join("deps/a.rmeta")).unwrap(),
            "metadata"
        );
    }

    #[test]
    fn links_and_damaged_archives_are_refused() {
        let folder = tempfile::tempdir().unwrap();
        let packed = xz(&tar(&[("a", b"a")], Some("/etc/passwd")));
        let listed = index(&[(ENGINE, &packed)]);
        let refused = unpack(
            &listed,
            ENGINE,
            &packed[..],
            &folder.path().join("l"),
            1 << 20,
        );
        assert!(
            matches!(&refused, Err(Refusal::Archive { why, .. }) if why.contains("not a file or folder")),
            "{refused:?}"
        );
        let refused = unpack(
            &listed,
            ENGINE,
            &b"not xz"[..],
            &folder.path().join("d"),
            1 << 20,
        );
        assert!(
            matches!(refused, Err(Refusal::Archive { .. })),
            "{refused:?}"
        );
    }
}
