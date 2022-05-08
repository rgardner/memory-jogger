"""Codebase tests."""

import pathlib
import subprocess
import sys

MODULE_NAME = "discussion_repl"
ROOT_DIR = pathlib.Path(__file__).parents[1]


def test_format_black():
    """Verifies Python source files are formatted correctly."""
    # verifies exception not thrown
    subprocess.run([sys.executable, "-m", "black", "--check", ROOT_DIR], check=True)


def test_format_isort():
    """Verifies Python imports are formatted correctly."""
    # verifies exception not thrown
    subprocess.run(
        [sys.executable, "-m", "isort", "--check-only", ROOT_DIR], check=True
    )


def test_lint_mypy():
    """Verifies mypy linting."""
    subprocess.run([sys.executable, "-m", "mypy", ROOT_DIR], check=True)


def test_lint_pylint():
    """Verifies pylint linting."""
    subprocess.run([sys.executable, "-m", "pylint", MODULE_NAME], check=True)
