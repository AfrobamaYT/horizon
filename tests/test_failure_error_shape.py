"""A `Result::unwrap` failure names its error's kind; no other payload text crosses, paths included."""
import contextlib
import io
import unittest

from tools.ci import ignored_tests as ci


class ErrorShape(unittest.TestCase):
    def render(self, value):
        with contextlib.redirect_stdout(io.StringIO()) as log:
            ci.failure_diagnostics("test load ... FAILED\n"
                                   "thread 'load' panicked at private.rs:1:1:\n"
                                   f"called `Result::unwrap()` on an `Err` value: {value}\n")
        return log.getvalue()

    def test_io_error_names_kind_and_os_error_but_no_path(self):
        log = self.render('Os { code: 2, kind: NotFound, message: "PRIVATE" } '
                          'reading /home/runner/work/horizon/.local/horizon-client/26.3/assets/minecraft/blockstates/stone.json')
        self.assertIn("panic reason: Result::unwrap on an error (kind Os, NotFound, os error 2 ENOENT); "
                      "rest of payload withheld (resource privacy)", log)
        self.assertNotIn("stone.json", log)
        self.assertNotIn("PRIVATE", log)
        self.assertNotIn("/home/runner", log)

    def test_message_without_a_kind_or_path_stays_fully_withheld(self):
        log = self.render('"PRIVATE text with a value 42"')
        self.assertIn("panic reason: Result::unwrap on an error; payload withheld (resource privacy)", log)
        self.assertNotIn("PRIVATE", log)


if __name__ == "__main__":
    unittest.main()
