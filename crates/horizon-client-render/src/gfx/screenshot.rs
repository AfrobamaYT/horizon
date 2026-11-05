//! Read-back screenshots waiting for the game's client hook, then PNG encoding.
use super::{Result, hdr_icc, hdr_screenshot_path};
use std::path::{Path, PathBuf};

/// CPU-side frame returned to the game before any screenshot file is written.
/// Pixels stay in the renderer crate, never in the mod API.
pub struct PendingScreenshot {
    pub path: PathBuf,
    pub(super) size: [u32; 2],
    pub(super) pixels: Vec<u8>,
    pub(super) hdr: Option<HdrScreenshot>,
}

pub(super) struct HdrScreenshot {
    pub size: [u32; 2],
    pub pixels: Vec<u8>,
    pub peak: f32,
    pub paper_white: f32,
}

impl PendingScreenshot {
    /// Save an agent capture or world icon to its requested destination.
    pub fn save(&self) -> Result<[u32; 2]> {
        if let Some(hdr) = &self.hdr {
            let path = hdr_screenshot_path(&self.path);
            match hdr.save(&path) {
                Ok(()) => log::info!("Saved HDR screenshot to {}", path.display()),
                Err(e) => log::warn!("Horizon renderer: HDR screenshot {}: {e}", path.display()),
            }
        }
        let mut encoder = png::Encoder::new(
            std::io::BufWriter::new(std::fs::File::create(&self.path)?),
            self.size[0],
            self.size[1],
        );
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header()?.write_image_data(&self.pixels)?;
        Ok(self.size)
    }

    /// Validate a mod's relative file name at the write boundary, then save
    /// both copies inside `root`. No destination may traverse a symbolic link.
    pub fn save_in(&mut self, root: &Path, file_name: &str) -> Result<[u32; 2]> {
        let path = screenshot_destination(root, file_name)?;
        if self.hdr.is_some() {
            let hdr_name = hdr_screenshot_path(Path::new(file_name));
            screenshot_destination(root, &hdr_name.to_string_lossy())?;
        }
        std::fs::create_dir_all(path.parent().expect("destination has a parent"))?;
        self.path = path;
        self.save()
    }
}

/// A relative, portable path. Refuse Windows separators/prefixes on every OS,
/// not just on Windows, and inspect every component including the output file.
fn screenshot_destination(root: &Path, file_name: &str) -> Result<PathBuf> {
    if file_name.is_empty()
        || file_name.contains(['\\', ':', '\0'])
        || file_name.split('/').any(|part| {
            let stem = part
                .split('.')
                .next()
                .unwrap_or_default()
                .to_ascii_uppercase();
            part.is_empty()
                || part.ends_with(['.', ' '])
                || matches!(
                    stem.as_str(),
                    "CON"
                        | "PRN"
                        | "AUX"
                        | "NUL"
                        | "COM1"
                        | "COM2"
                        | "COM3"
                        | "COM4"
                        | "COM5"
                        | "COM6"
                        | "COM7"
                        | "COM8"
                        | "COM9"
                        | "LPT1"
                        | "LPT2"
                        | "LPT3"
                        | "LPT4"
                        | "LPT5"
                        | "LPT6"
                        | "LPT7"
                        | "LPT8"
                        | "LPT9"
                )
        })
    {
        return Err("screenshot file name must be a relative path inside screenshots".into());
    }
    let mut path = root.to_path_buf();
    for part in file_name.split('/') {
        path.push(part);
        match std::fs::symlink_metadata(&path) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err("screenshot destination must not traverse a symbolic link".into());
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => break,
            Err(e) => return Err(e.into()),
        }
    }
    Ok(root.join(file_name))
}

impl HdrScreenshot {
    /// HDR10 as a 16-bit PNG, retaining BT.2020/PQ, cLLI and the ICC profile.
    fn save(&self, path: &Path) -> Result<()> {
        let mut info = png::Info::with_size(self.size[0], self.size[1]);
        info.color_type = png::ColorType::Rgb;
        info.bit_depth = png::BitDepth::Sixteen;
        info.icc_profile = Some(hdr_icc::profile().into());
        let encoder =
            png::Encoder::with_info(std::io::BufWriter::new(std::fs::File::create(path)?), info)?;
        let mut writer = encoder.write_header()?;
        writer.write_chunk(png::chunk::cICP, &[9, 16, 0, 1])?;
        let nits = |n: f32| ((n * 10_000.0) as u32).to_be_bytes();
        let peak = self.peak.max(self.paper_white);
        writer.write_chunk(
            png::chunk::cLLI,
            &[nits(peak), nits(self.paper_white.min(peak))].concat(),
        )?;
        writer.write_image_data(&self.pixels)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(path: PathBuf, hdr: bool) -> PendingScreenshot {
        PendingScreenshot {
            path,
            size: [1, 1],
            pixels: vec![12, 34, 56, 255],
            hdr: hdr.then(|| HdrScreenshot {
                size: [1, 1],
                pixels: vec![0, 0, 255, 255, 128, 0],
                peak: 1000.0,
                paper_white: 203.0,
            }),
        }
    }

    #[test]
    fn pending_screenshot_writes_only_on_save_and_renames_both_copies() {
        let root = tempfile::tempdir().unwrap();
        let original = root.path().join("original.png");
        let mut pending = frame(original.clone(), true);
        assert!(!original.exists());
        assert_eq!(
            pending.save_in(root.path(), "album/changed.png").unwrap(),
            [1, 1]
        );
        assert!(!original.exists());
        let file = std::fs::File::open(root.path().join("album/changed.png")).unwrap();
        let mut reader = png::Decoder::new(std::io::BufReader::new(file))
            .read_info()
            .unwrap();
        let mut bytes = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut bytes).unwrap();
        assert_eq!(&bytes[..info.buffer_size()], &[12, 34, 56, 255]);
        let file = std::fs::File::open(root.path().join("album/changed-hdr.png")).unwrap();
        let reader = png::Decoder::new(std::io::BufReader::new(file))
            .read_info()
            .unwrap();
        assert_eq!(reader.info().bit_depth, png::BitDepth::Sixteen);
        assert!(reader.info().icc_profile.is_some());
    }

    #[test]
    fn pending_screenshot_noop_saves_the_requested_name() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("normal.png");
        let pending = frame(path.clone(), false);
        assert_eq!(pending.save().unwrap(), [1, 1]);
        assert!(path.exists());
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    }

    #[test]
    fn pending_screenshot_refuses_paths_outside_the_folder() {
        let root = tempfile::tempdir().unwrap();
        let mut pending = frame(root.path().join("unused.png"), true);
        for name in [
            "",
            "/absolute.png",
            "../escape.png",
            "a/../escape.png",
            "./dot.png",
            "a//b.png",
            "a/",
            "C:drive.png",
            "a\\escape.png",
            "nul\0.png",
            ".. /escape.png",
            "album./image.png",
            "NUL.png",
            "com1.png",
            "LPT9/log.png",
        ] {
            assert!(pending.save_in(root.path(), name).is_err(), "{name:?}");
        }
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }

    #[cfg(unix)]
    #[test]
    fn pending_screenshot_refuses_symlinks_including_the_hdr_sibling() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        symlink(outside.path(), root.path().join("album")).unwrap();
        symlink(outside.path().join("sdr.png"), root.path().join("sdr.png")).unwrap();
        symlink(
            outside.path().join("hdr.png"),
            root.path().join("image-hdr.png"),
        )
        .unwrap();
        let mut pending = frame(root.path().join("unused.png"), true);
        for name in ["album/escape.png", "sdr.png", "image.png"] {
            assert!(pending.save_in(root.path(), name).is_err());
        }
        assert!(!root.path().join("image.png").exists());
        assert_eq!(std::fs::read_dir(outside.path()).unwrap().count(), 0);
    }
}
