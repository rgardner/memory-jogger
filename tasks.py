"""Build tasks for Pocket Cleaner."""

import os
import pathlib
import sys
from typing import Dict

import invoke

HEROKU_APP_NAME = "stormy-escarpment-06312"


def get_source_dir() -> pathlib.Path:
    return pathlib.Path(os.path.dirname(os.path.abspath(__file__)))


class BuildContext:
    def __init__(self, ctx: invoke.Context):
        self.ctx = ctx

    def run(self, command: str, *, env: Dict[str, str] = None):
        with self.ctx.cd(str(get_source_dir())):
            self.ctx.run(command)


@invoke.task
def build(ctx, fast=False, docker=False):
    """Builds Pocket Cleaner."""
    build_ctx = BuildContext(ctx)
    if docker:
        if fast:
            print(
                "warning: --fast is ignored when building a Docker image",
                file=sys.stderr,
            )

        build_ctx.run("docker-compose build")
    else:
        if fast:
            build_ctx.run("cargo check")
        else:
            build_ctx.run("cargo build")


@invoke.task
def test(ctx):
    """Runs all tests."""
    BuildContext(ctx).run("cargo test")


@invoke.task
def clean(ctx):
    """Removes built artifacts."""
    BuildContext(ctx).run("cargo clean")


@invoke.task
def lint(ctx):
    """Performs static analysis on all source files."""
    BuildContext(ctx).run("cargo clippy -- -D warnings")


@invoke.task
def fmt(ctx, check=False):
    """Formats all source files."""
    build_ctx = BuildContext(ctx)
    if check:
        build_ctx.run("cargo fmt -- --check")
    else:
        build_ctx.run("cargo fmt")


@invoke.task
def deploy(ctx):
    """Deploys Pocket Cleaner to production."""
    build_ctx = BuildContext(ctx)
    build_ctx.run(f"heroku container:push web --app {HEROKU_APP_NAME}")
    build_ctx.run(f"heroku container:release web --app {HEROKU_APP_NAME}")
