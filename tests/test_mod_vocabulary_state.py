"""Behavior tests for the vocabulary's named process-state ownership gate."""
import sys
from pathlib import Path
import unittest
from unittest import mock

with mock.patch.object(sys, 'path', [*sys.path, str(Path(__file__).resolve().parents[1] / 'tools')]):
    from check_mod_vocabulary_state import declarations, violations, unsafe_violations, expansion_violations


class VocabularyStateGate(unittest.TestCase):
    def test_macro_metavariable_cannot_hide_a_new_static(self):
        mutant = 'macro_rules! slot { ($n:ident) => { static $n: AtomicBool = AtomicBool::new(false); }; } slot!(EXTRA);'
        self.assertTrue(expansion_violations({'macro.rs': mutant}))

    def test_macro_keyword_argument_cannot_hide_a_new_static(self):
        mutant = 'macro_rules! slot { ($kind:ident) => { $kind HIDDEN: AtomicBool = AtomicBool::new(false); }; } slot!(static);'
        self.assertTrue(expansion_violations({'macro.rs': mutant}))

    def test_array_static_is_inventoried_despite_its_type_semicolon(self):
        source = 'static CACHE: LazyLock<[Identifier; 3]> = make();'
        self.assertEqual(declarations(source), [('CACHE', 'LazyLock<[Identifier;3]>', 1)])
        self.assertTrue(violations({'array.rs': source}, {}))

    def test_unchecked_include_cannot_hide_generated_state_or_unsafe(self):
        for source in ('include!("extra.rs");', 'include!(concat!(env!("OUT_DIR"), "/unchecked.rs"));'):
            with self.subTest(source=source):
                self.assertTrue(expansion_violations({'lib.rs': source}))
        self.assertEqual(expansion_violations({'lib.rs': 'include!(concat!(env!("OUT_DIR"), "/wire_generated.rs"));'}), [])

    def test_added_atomic_mutant_is_rejected(self):
        owner = 'thread_local! { static CURRENT_CRASH_OWNER: Cell<CrashOwner> = todo!(); }'
        allowed = {'content/crash.rs::CURRENT_CRASH_OWNER': {'type': 'Cell<CrashOwner>'}}
        self.assertEqual(violations({'content/crash.rs': owner}, allowed), [])
        mutant = owner + '\nstatic HIDDEN_COUNTER: AtomicU64 = AtomicU64::new(0);'
        errors = violations({'content/crash.rs': mutant}, allowed)
        self.assertEqual(len(errors), 1)
        self.assertIn('unlisted process state HIDDEN_COUNTER', errors[0])

    def test_added_tls_once_lock_and_lazy_static_mutants_are_rejected(self):
        for source in (
            'thread_local! { static EXTRA: Cell<bool> = const { Cell::new(false) }; }',
            'static EXTRA: OnceLock<Mutex<Vec<u8>>> = OnceLock::new();',
            'static EXTRA: LazyLock<Mutex<Vec<u8>>> = LazyLock::new(Default::default);',
            'lazy_static! { static ref EXTRA: Mutex<()> = Mutex::new(()); }',
            'static mut EXTRA: usize = 0;',
        ):
            with self.subTest(source=source):
                self.assertEqual(len(violations({'added.rs': source}, {})), 1)

    def test_comments_literals_and_unconditional_test_items_are_not_production(self):
        source = '''// static NOT_CODE: AtomicBool = false;
        const EXAMPLE: &str = r#"thread_local! { static TEXT: Cell<bool> = x; }"#;
        #[cfg(test)] mod tests { static FIXTURE: AtomicU64 = x; }
        #[test] fn example() { static FIXTURE: AtomicU64 = x; }
        '''
        self.assertEqual(declarations(source), [])
        self.assertEqual(declarations('#![cfg(test)]\nstatic FIXTURE: AtomicU64 = x;'), [])

    def test_static_lifetimes_are_not_declarations(self):
        source = "fn read() -> &'static std::sync::OnceLock<String> { static CACHE: OnceLock<String> = OnceLock::new(); &CACHE }"
        self.assertEqual(declarations(source), [('read::CACHE', 'OnceLock<String>', 1)])
        self.assertEqual(declarations("struct Socket { callback: fn() -> &'static std::sync::atomic::AtomicBool }"), [])

    def test_feature_gate_is_not_a_test_only_exemption(self):
        for attribute in ('#[cfg(feature = "testing")]', '#[cfg(any(test, feature = "testing"))]'):
            self.assertEqual(len(declarations(attribute + '\nstatic EXTRA: AtomicBool = x;')), 1)

    def test_only_explicit_test_module_edges_exempt_fixture_files(self):
        files = {'parent.rs': '#[cfg(test)]\n#[path = "fixture.rs"] mod fixture;',
                 'fixture.rs': 'static FIXTURE: AtomicBool = x;'}
        self.assertEqual(violations(files, {}), [])
        files['parent.rs'] = '#[cfg(feature = "testing")]\n#[path = "fixture.rs"] mod fixture;'
        self.assertEqual(len(violations(files, {})), 1)
        files['parent.rs'] = '#[cfg(test)]\n#[path = "fixture.rs"] mod fixture;'
        files['other.rs'] = '#[path = "fixture.rs"] pub(crate) mod live;'
        self.assertEqual(len(violations(files, {})), 1)

    def test_unsafe_exception_cannot_spread_to_other_source(self):
        files = {'lib.rs': '#![deny(unsafe_code)]',
                 '__private/points.rs': '#[expect(unsafe_code)] unsafe impl Send for Raw {}'}
        self.assertEqual(unsafe_violations(files), [])
        files['value.rs'] = '#[allow(unsafe_code)] unsafe fn added() {}'
        self.assertEqual(len(unsafe_violations(files)), 2)
        files['value.rs'] = '// unsafe fn text() {}\nconst HELP: &str = "unsafe";'
        self.assertEqual(unsafe_violations(files), [])
        files['lib.rs'] = ''
        self.assertIn('must retain', unsafe_violations(files)[0])

    def test_changed_cache_type_and_stale_permission_are_rejected(self):
        allowed = {'value.rs::cache::VALUE': {'type': 'OnceLock<String>'}}
        source = 'fn cache() { static VALUE: OnceLock<Mutex<String>> = OnceLock::new(); }'
        self.assertIn('changed type', violations({'value.rs': source}, allowed)[0])
        self.assertIn('stale state inventory entry', violations({}, allowed)[0])


if __name__ == '__main__':
    unittest.main()
