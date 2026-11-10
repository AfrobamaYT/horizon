"""Every Python test that runs `rustc` or `cargo` must skip when the tool is missing.

`verify` runs the Python suite before it installs any Rust toolchain, and a fork's pull request can run where none is
installed at all, so an unguarded `subprocess.run(["rustc", ...])` fails there with FileNotFoundError (#2868, and
`tests/test_prebuilt_game.py` after #2622).

The rule, checked statically over `tests/*.py`: a test that starts `rustc` or `cargo`, either itself or through a
helper function or method of the same module, carries a guard: `unittest.skipUnless(shutil.which(...))` (or
`skipIf`) on the test, its class or the module, or a `shutil.which(...)` check with `self.skipTest(...)` in the test
or helper body. Calls inside a `mock.patch` of the subprocess module are not launches and don't count.
"""
import ast
from pathlib import Path
import unittest

TESTS = Path(__file__).resolve().parent
TOOLS = {"rustc", "cargo"}
LAUNCHERS = {"run", "check_output", "check_call", "call", "Popen"}


def _str(node):
    return node.value if isinstance(node, ast.Constant) and isinstance(node.value, str) else None


def _which_tool(node):
    """`shutil.which("rustc")` or `which("cargo")`: the tool it looks up, if one of ours."""
    if isinstance(node, ast.Call):
        func = node.func
        name = func.attr if isinstance(func, ast.Attribute) else getattr(func, "id", None)
        if name == "which" and node.args and _str(node.args[0]) in TOOLS:
            return _str(node.args[0])
    return None


def _mentions_which(node):
    return any(_which_tool(n) for n in ast.walk(node))


def _is_guard_decorator(node):
    # unittest.skipUnless(shutil.which("rustc"), ...) / skipIf(shutil.which("cargo") is None, ...)
    if not isinstance(node, ast.Call):
        return False
    func = node.func
    name = func.attr if isinstance(func, ast.Attribute) else getattr(func, "id", None)
    return name in {"skipUnless", "skipIf"} and _mentions_which(node)


def _guarded_body(func):
    """The body checks for the tool and calls skipTest (directly, before or after other setup)."""
    has_skip = any(isinstance(n, ast.Call) and isinstance(n.func, ast.Attribute) and n.func.attr == "skipTest"
                   for n in ast.walk(func))
    return has_skip and _mentions_which(func)


def _which_names(func):
    """Local names bound to `shutil.which(tool)`: `compiler = shutil.which("rustc")`."""
    names = set()
    for node in ast.walk(func):
        if isinstance(node, ast.Assign) and _which_tool(node.value):
            names |= {t.id for t in node.targets if isinstance(t, ast.Name)}
    return names


def _launches(func):
    """Whether this function body itself starts rustc or cargo through subprocess."""
    which_names = _which_names(func)
    for node in ast.walk(func):
        if not (isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute)
                and node.func.attr in LAUNCHERS and node.args):
            continue
        command = node.args[0]
        if isinstance(command, (ast.List, ast.Tuple)) and command.elts:
            first = command.elts[0]
            if _str(first) in TOOLS or (isinstance(first, ast.Name) and first.id in which_names):
                return True
    return False


def _called_names(func):
    names = set()
    for node in ast.walk(func):
        if isinstance(node, ast.Call):
            if isinstance(node.func, ast.Name):
                names.add(node.func.id)
            elif isinstance(node.func, ast.Attribute) and isinstance(node.func.value, ast.Name) \
                    and node.func.value.id in {"self", "cls"}:
                names.add(node.func.attr)
    return names


def unguarded(source, filename="<test>"):
    """Test names in this module that can launch rustc or cargo without a guard."""
    tree = ast.parse(source, filename)
    module_guarded = any(isinstance(n, ast.FunctionDef) and n.name == "setUpModule" and _guarded_body(n)
                         for n in tree.body)
    # Every function and method, with its owning class (None for module level).
    functions = []
    for node in tree.body:
        if isinstance(node, ast.FunctionDef):
            functions.append((None, node))
        elif isinstance(node, ast.ClassDef):
            functions += [(node, n) for n in node.body if isinstance(n, ast.FunctionDef)]
    by_name = {}
    for owner, func in functions:
        by_name.setdefault(func.name, []).append((owner, func))

    def guarded(owner, func):
        return (module_guarded or _guarded_body(func) or any(_is_guard_decorator(d) for d in func.decorator_list)
                or (owner is not None and any(_is_guard_decorator(d) for d in owner.decorator_list))
                or (owner is not None and any(n.name == "setUp" and _guarded_body(n)
                                              for n in owner.body if isinstance(n, ast.FunctionDef))))

    # A function launches unguarded if it launches itself without a guard, or calls (within this module) a
    # function that does; a guard anywhere on the way protects the call.
    memo = {}

    def unguarded_launch(owner, func, seen=()):
        key = (owner.name if owner else None, func.name)
        if key in memo:
            return memo[key]
        if key in seen or guarded(owner, func):
            return False
        result = _launches(func) or any(
            unguarded_launch(o, f, seen + (key,)) for name in _called_names(func) for o, f in by_name.get(name, []))
        memo[key] = result
        return result

    return sorted(f"{owner.name}.{func.name}" if owner else func.name
                  for owner, func in functions if func.name.startswith("test") and unguarded_launch(owner, func))


class ToolchainGuardRuleTests(unittest.TestCase):
    def test_every_test_that_starts_rustc_or_cargo_skips_without_it(self):
        offenders = []
        for path in sorted(TESTS.glob("*.py")):
            offenders += [f"{path.name}: {name}" for name in unguarded(path.read_text(encoding="utf-8"), str(path))]
        self.assertEqual([], offenders, "guard these with shutil.which(...) and a skip; see this module's docstring")

    def test_an_unguarded_launch_is_found_directly_and_through_a_helper(self):
        direct = 'import subprocess, unittest\nclass T(unittest.TestCase):\n' \
                 '    def test_a(self):\n        subprocess.run(["rustc", "x.rs"], check=True)\n'
        self.assertEqual(["T.test_a"], unguarded(direct))
        helper = 'import subprocess, unittest\ndef build():\n    subprocess.check_output(["cargo", "build"])\n' \
                 'class T(unittest.TestCase):\n    def test_b(self):\n        build()\n' \
                 '    def compile(self):\n        subprocess.Popen(["rustc"])\n' \
                 '    def test_c(self):\n        self.compile()\n'
        self.assertEqual(["T.test_b", "T.test_c"], unguarded(helper))

    def test_each_guard_form_is_accepted(self):
        guarded = '''
import shutil, subprocess, unittest

@unittest.skipUnless(shutil.which("cargo"), "requires Cargo")
class Whole(unittest.TestCase):
    def test_a(self):
        subprocess.run(["cargo", "build"])

class Parts(unittest.TestCase):
    @unittest.skipUnless(shutil.which("rustc"), "requires Rust")
    def test_decorated(self):
        subprocess.run(["rustc"])

    def test_in_body(self):
        compiler = shutil.which("rustc")
        if compiler is None:
            self.skipTest("requires Rust")
        subprocess.run([compiler, "x.rs"])

    def helper(self):
        if not shutil.which("cargo"):
            self.skipTest("requires Cargo")
        subprocess.run(["cargo", "test"])

    def test_through_a_guarded_helper(self):
        self.helper()

class Mocked(unittest.TestCase):
    def test_only_builds_the_command(self):
        command = ["cargo", "build"]
        self.assertEqual("cargo", command[0])
'''
        self.assertEqual([], unguarded(guarded))


if __name__ == "__main__":
    unittest.main()
