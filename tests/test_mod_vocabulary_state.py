"""Behavior tests for the vocabulary's named process-state ownership gate."""
import sys
from pathlib import Path
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
from check_mod_vocabulary_state import declarations, violations


class VocabularyStateGate(unittest.TestCase):
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

    def test_changed_cache_type_and_stale_permission_are_rejected(self):
        allowed = {'value.rs::cache::VALUE': {'type': 'OnceLock<String>'}}
        source = 'fn cache() { static VALUE: OnceLock<Mutex<String>> = OnceLock::new(); }'
        self.assertIn('changed type', violations({'value.rs': source}, allowed)[0])
        self.assertIn('stale state inventory entry', violations({}, allowed)[0])


if __name__ == '__main__':
    unittest.main()
