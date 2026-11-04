use serde::{Deserialize, Serialize};

/// An original Java jar used as a passive asset/data layer, in manifest order.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(try_from = "Fields")]
pub struct JarRef {
    id: String,
    version: String,
    sha512: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Fields {
    id: String,
    version: String,
    sha512: String,
}

impl TryFrom<Fields> for JarRef {
    type Error = String;

    fn try_from(fields: Fields) -> Result<Self, Self::Error> {
        Self::new(fields.id, fields.version, fields.sha512)
    }
}

impl JarRef {
    /// Validate the supported loader ID syntax and the exact file's SHA-512.
    /// The metadata reader, not this loader-independent manifest, checks the
    /// original loader's more specific ID rule. IDs are never normalized.
    pub fn new(id: String, version: String, sha512: String) -> Result<Self, String> {
        let bytes = id.as_bytes();
        let valid_id = (2..=64).contains(&bytes.len())
            && bytes
                .iter()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_-".contains(b))
            && (!bytes.contains(&b'-') || bytes[0].is_ascii_lowercase());
        if !valid_id {
            return Err(format!(
                "jar id {id:?} is not a supported NeoForge or Fabric ID"
            ));
        }
        if version.is_empty() {
            return Err(format!("jar {id}: version is empty"));
        }
        super::wasm::sha512_hex(&sha512).map_err(|e| format!("jar {id}: sha512 {e}"))?;
        Ok(Self {
            id,
            version,
            sha512,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn sha512(&self) -> &str {
        &self.sha512
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_jar_list_defaults_empty_and_survives_package_roundtrip_in_order() {
        use super::super::{Manifest, Package};
        let base = "id = 'port'\nname = 'Port'\nversion = '1'\napi = '0.2'\ncapabilities = ['registries']\n";
        assert!(Manifest::parse(base).unwrap().jar.is_empty());
        let text = format!(
            "{base}[[jar]]\nid = 'first_jar'\nversion = '1'\nsha512 = '{}'\n[[jar]]\nid = 'fabric-api'\nversion = '2'\nsha512 = '{}'\n",
            "0".repeat(128),
            "1".repeat(128)
        );
        let manifest = Manifest::parse(&text).unwrap();
        assert_eq!(
            manifest.jar.iter().map(JarRef::id).collect::<Vec<_>>(),
            ["first_jar", "fabric-api"]
        );
        let bytes = Package::write_files(&text, &[]).unwrap();
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        assert_eq!(Package::read_zip(&mut archive).unwrap().manifest, manifest);
        assert!(Manifest::parse(&format!("{text}loader = 'fabric'\n")).is_err());
    }

    #[test]
    fn manifest_refs_preserve_original_ids_and_refuse_invalid_boundary_values() {
        let hash = "0123456789abcdef".repeat(8);
        for id in ["fabric-api", "neo_mod", "_1", "1mod", &"a".repeat(64)] {
            let text = format!("id = {id:?}\nversion = '1.2+build'\nsha512 = {hash:?}\n");
            let jar: JarRef = toml::from_str(&text).unwrap();
            assert_eq!(jar.id, id);
            assert_eq!(
                toml::from_str::<JarRef>(&toml::to_string(&jar).unwrap()).unwrap(),
                jar
            );
            assert!(toml::from_str::<JarRef>(&format!("{text}loader = 'fabric'\n")).is_err());
        }
        for id in [
            "",
            "a",
            "Fabric",
            "-bad",
            "1-mod",
            "bad.id",
            "bad/id",
            &"a".repeat(65),
        ] {
            assert!(
                JarRef::new(id.into(), "1".into(), hash.clone()).is_err(),
                "{id}"
            );
        }
        assert!(JarRef::new("good".into(), String::new(), hash.clone()).is_err());
        for bad_hash in [
            "a".repeat(127),
            "a".repeat(129),
            "A".repeat(128),
            "g".repeat(128),
        ] {
            let text = format!("id = 'good'\nversion = '1'\nsha512 = {bad_hash:?}\n");
            assert!(toml::from_str::<JarRef>(&text).is_err());
        }
    }
}
