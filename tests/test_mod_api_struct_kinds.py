"""The mod API's public structs are built by mods or only received (#2862)."""
import unittest

from tools.mod_api_struct_kinds import census, defaults_and_constructors, structs

# Public structs a mod can build with a brace literal that have neither
# `Default` nor a constructor and are not `#[non_exhaustive]`. The number only
# goes down: each batch that classifies some lowers it to the new count.
NEITHER_LIMIT = 128


def kinds(source):
    """Name -> (non_exhaustive, derives_default, literal) for a fixture."""
    return {name: rest for name, _, *rest in structs(source)}


class StructKindFixtures(unittest.TestCase):
    def test_a_struct_of_public_fields_is_built_by_literal(self):
        self.assertEqual(kinds('pub struct A { pub x: u8, pub y: Vec<(u8, u8)> }'),
                         {'A': [False, False, True]})

    def test_a_private_or_crate_field_stops_a_literal(self):
        found = kinds('pub struct A { pub x: u8, y: u8 } pub struct B { pub(crate) x: u8 }')
        self.assertEqual(found['A'][2], False)
        self.assertEqual(found['B'][2], False)

    def test_tuple_and_unit_structs_are_not_counted_as_literals(self):
        found = kinds('pub struct A(pub u8); pub struct B;')
        self.assertEqual([found['A'][2], found['B'][2]], [False, False])

    def test_attributes_say_non_exhaustive_and_default(self):
        found = kinds('#[non_exhaustive]\n#[derive(Clone)] pub struct A { pub x: u8 }\n'
                      '#[derive(Clone, Default, Wire)] pub struct B { pub x: u8 }\n'
                      '#[derive(Clone)] pub struct C { #[doc = "x"] pub x: u8 }')
        self.assertEqual(found['A'][:2], [True, False])
        self.assertEqual(found['B'][:2], [False, True])
        self.assertEqual(found['C'], [False, False, True])

    def test_generic_and_lifetime_structs_are_read(self):
        found = kinds("pub struct A<'a, T: Into<u8>> where T: Clone { pub x: &'a T }")
        self.assertEqual(found['A'], [False, False, True])

    def test_built_mirrors_are_left_out(self):
        self.assertEqual(kinds('pub(crate) mod built { pub struct A { pub x: u8 } }'), {})

    def test_test_only_structs_are_left_out(self):
        self.assertEqual(kinds('#[cfg(test)] mod tests { pub struct A { pub x: u8 } }'), {})

    def test_default_impls_and_constructors_are_found(self):
        defaults, constructors = defaults_and_constructors(
            'impl Default for A { fn default() -> Self { A { x: 0 } } }\n'
            'impl B { pub fn new(x: u8) -> Self { B { x } } pub fn x(&self) -> u8 { self.x } }\n'
            'impl C { pub fn x(&self) -> Self { todo!() } }\n'
            'impl D { pub fn parse(s: &str) -> Option<Self> { None } }')
        self.assertEqual(defaults, {'A'})
        self.assertEqual(constructors, {'B', 'D'})


class StructKindRepository(unittest.TestCase):
    def test_no_new_struct_is_neither_kind(self):
        counts, neither = census()
        listed = '\n'.join(neither)
        self.assertLessEqual(
            counts['neither'], NEITHER_LIMIT,
            'a public mod API struct is built by literal but has neither Default nor a '
            'constructor: add Default (kind 1, built by mods) or #[non_exhaustive] (kind 2, '
            f'only received; see docs/modding). Neither kind now:\n{listed}')
        self.assertEqual(
            counts['neither'], NEITHER_LIMIT,
            f'fewer structs are neither kind: lower NEITHER_LIMIT to {counts["neither"]}')


if __name__ == '__main__':
    unittest.main()
