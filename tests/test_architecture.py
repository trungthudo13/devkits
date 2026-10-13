"""Guard the three-layer dependency boundaries as new tools are added."""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parent.parent

class ArchitectureTests(unittest.TestCase):
    def test_business_has_no_presentation_or_direct_platform_dependencies(self):
        for path in (ROOT / "src/business").rglob("*.rs"):
            if path.name == "tests.rs":
                continue
            text = path.read_text(encoding="utf-8")
            for forbidden in (r"\begui\b", r"\beframe\b", r"\brfd::", r"\barboard::", r"std::fs", r"std::process", r"crate::ui"):
                self.assertIsNone(re.search(forbidden, text), f"{path}: {forbidden}")

    def test_adapters_do_not_depend_on_business_or_ui(self):
        for path in (ROOT / "src/adapter").rglob("*.rs"):
            self.assertNotRegex(path.read_text(encoding="utf-8"), r"(?:crate|super)::(?:business|ui)")

    def test_ui_uses_adapters_for_native_io(self):
        for path in (ROOT / "src/ui").rglob("*.rs"):
            self.assertNotRegex(path.read_text(encoding="utf-8"), r"std::(?:fs|process)|\brfd::|\barboard::")

if __name__ == "__main__":
    unittest.main()
