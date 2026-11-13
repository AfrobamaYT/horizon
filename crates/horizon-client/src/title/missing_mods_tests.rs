use super::super::tests::{Fixture, only_world};
use super::*;
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;

/// A mod the fixture's world was saved with and no test loads.
fn gone() -> MissingMod {
    MissingMod {
        id: "gone".into(),
        saved_version: "1.0.0".into(),
        code: true,
        content: false,
        blocks: true,
    }
}

/// Every file under `dir`, by its path relative to `dir`, with its bytes.
fn files(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let name = path.strip_prefix(root).unwrap().to_string_lossy();
                out.insert(name.replace('\\', "/"), fs::read(&path).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

/// A world with a region file beside its level.dat, asked about `gone`.
fn asked(f: &mut Fixture) -> (WorldEntry, PathBuf) {
    f.world("Gone", "Gone");
    let dir = f.title.saves.join("Gone");
    fs::create_dir_all(dir.join("region")).unwrap();
    fs::write(dir.join("region/r.0.0.mca"), b"chunk bytes").unwrap();
    // What the game keeps when it asks: the hook's answer for this world.
    level_mods::remember_answer(&dir, Vec::new(), Vec::new());
    f.title.ask_missing_mods("Gone", vec![gone()]);
    (only_world(f), dir)
}

fn finish_backup(f: &mut Fixture) {
    f.title.backup_shown.set(true);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while matches!(f.title.mode, Mode::Backup { .. }) {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(10));
        f.title.poll_backup();
    }
}

/// The question lists the mod; Don't Open, the default, leaves every file of the world as it
/// was and keeps it closed, and the next open asks again.
#[test]
fn dont_open_leaves_the_world_closed_and_unchanged() {
    let _answer = level_mods::TEST_ANSWER
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut f = Fixture::new();
    let (_, dir) = asked(&mut f);
    let before = files(&dir);
    assert!(matches!(
        &f.title.mode,
        Mode::MissingMods { missing, .. } if *missing == [gone()]
    ));
    assert_eq!(
        f.click(&lang::get("horizon.missing_mods.dont_open")),
        Action::None
    );
    assert!(matches!(f.title.mode, Mode::Select { .. }));
    assert!(!level_mods::is_confirmed(&dir));
    assert_eq!(f.title.take_open_request(), None);
    assert_eq!(files(&dir), before);
    // Escape answers the same.
    level_mods::remember_answer(&dir, Vec::new(), Vec::new());
    f.title.ask_missing_mods("Gone", vec![gone()]);
    f.title.escape();
    assert!(matches!(f.title.mode, Mode::Select { .. }));
    assert!(!level_mods::is_confirmed(&dir));
    assert_eq!(files(&dir), before);
}

/// Open Without a Backup opens the world, and its open goes ahead without asking again.
#[test]
fn open_without_a_backup_opens_at_once() {
    let _answer = level_mods::TEST_ANSWER
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut f = Fixture::new();
    let (_, dir) = asked(&mut f);
    assert_eq!(
        f.click(&lang::get("horizon.missing_mods.open")),
        Action::Play {
            folder: "Gone".into()
        }
    );
    assert!(level_mods::is_confirmed(&dir));
    level_mods::forget_answer();
}

/// Back Up and Open copies the whole world before anything is written, then opens it and says
/// where the copy is.
#[test]
fn back_up_and_open_copies_the_world_first() {
    let _answer = level_mods::TEST_ANSWER
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut f = Fixture::new();
    let (_, dir) = asked(&mut f);
    let before = files(&dir);
    assert_eq!(
        f.click(&lang::get("horizon.missing_mods.backup_open")),
        Action::None
    );
    finish_backup(&mut f);
    assert_eq!(f.title.take_open_request(), Some("Gone".into()));
    assert!(level_mods::is_confirmed(&dir));
    let Some(WorldToast::Created { folder: path, .. }) = &f.title.pending_toast else {
        panic!("no backup toast: {:?}", f.title.pending_toast.is_some());
    };
    let mut zip = zip::ZipArchive::new(fs::File::open(path).unwrap()).unwrap();
    let mut copied = BTreeMap::new();
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).unwrap();
        if entry.is_dir() {
            continue;
        }
        let name = entry
            .name()
            .unwrap()
            .strip_prefix("Gone/")
            .unwrap()
            .to_owned();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        copied.insert(name, bytes);
    }
    assert_eq!(copied, before);
    assert_eq!(files(&dir), before);
    level_mods::forget_answer();
}

/// A backup that cannot be written cancels the open and says why.
#[test]
fn a_failed_backup_cancels_the_open() {
    let _answer = level_mods::TEST_ANSWER
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut f = Fixture::new();
    let (_, dir) = asked(&mut f);
    // The backups folder's place is taken by a file, so it cannot be made.
    let backups = world_management::backups_path(&f.title.saves);
    fs::write(&backups, b"not a folder").unwrap();
    f.click(&lang::get("horizon.missing_mods.backup_open"));
    finish_backup(&mut f);
    assert_eq!(f.title.take_open_request(), None);
    assert!(!level_mods::is_confirmed(&dir));
    assert!(matches!(&f.title.mode, Mode::Error { message, .. }
        if message.starts_with(&lang::tr("horizon.missing_mods.backup_failed", &[&""]))));
}

/// However many mods are missing, the text fits between the heading and the buttons: the mods
/// that don't fit become one line counting them, and a short list is shown whole.
#[test]
fn a_long_list_keeps_the_heading_and_buttons_on_screen() {
    let line = |text: &str| vec![text.to_owned()];
    let description = vec!["a".to_owned(), "b".to_owned()];
    let mods: Vec<Vec<String>> = (0..30).map(|i| line(&format!("mod {i}"))).collect();
    // A 240 high window: the column must end on it.
    let max = max_lines(240.0);
    let top = confirm_top(240.0, max);
    assert!(top >= 0.0 && top + 85.0 + max as f32 * 9.0 <= 240.0);
    let shown = fit(description.clone(), mods.clone(), max);
    assert_eq!(shown.len(), max);
    assert_eq!(shown[..2], description[..]);
    let kept = max - 3;
    assert_eq!(
        shown[2..2 + kept],
        (0..kept).map(|i| format!("mod {i}")).collect::<Vec<_>>()[..]
    );
    assert_eq!(
        shown.last().unwrap(),
        &lang::tr("horizon.missing_mods.more", &[&(30 - kept)])
    );
    // A list that fits is shown whole, with no count.
    let short = fit(description.clone(), mods[..2].to_vec(), max);
    assert_eq!(short, ["a", "b", "mod 0", "mod 1"]);
}
