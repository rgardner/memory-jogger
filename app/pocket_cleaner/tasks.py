"""Build tasks for Pocket Cleaner."""

import os.path
from pathlib import Path
from typing import Dict

from invoke import Context, task

HEROKU_APP_NAME = "stormy-escarpment-06312"


def get_source_dir() -> Path:
    return Path(os.path.dirname(os.path.abspath(__file__)))


class BuildContext:
    def __init__(self, ctx: Context):
        self.ctx = ctx

    def run(self, command: str, *, env: Dict[str, str] = None):
        with self.ctx.cd(str(get_source_dir())):
            self.ctx.run(command)


@task
def build(ctx, fast=False, docker=False):
    """Builds the web app."""
    build_ctx = BuildContext(ctx)
    if docker:
        if fast:
            print(
                "warning: --fast is ignored when building a Docker image",
                file=sys.stderr,
            )

        build_ctx.run("docker build . --tag rgardner/pocket-cleaner/pocket_cleaner")
    else:
        if fast:
            build_ctx.run("cargo check")
        else:
            build_ctx.run("cargo build")


@task
def run(ctx, autoreload=False, docker=False):
    """Runs the web app locally."""

    build_ctx = BuildContext(ctx)
    if docker:
        build(ctx, docker=True)
        build_ctx.run("docker-compose up")
    else:
        port = 5000
        extra_env = {"PORT": str(port)}
        if autoreload:
            build_ctx.run(
                f"systemfd --no-pid -s http::{port} -- cargo watch -x run",
                env=extra_env,
            )
        else:
            build_ctx.run("cargo run", env=extra_env)


@task
def test(ctx):
    """Runs all tests."""
    BuildContext(ctx).run("cargo test")


@task
def clean(ctx):
    """Removes built artifacts."""
    BuildContext(ctx).run("cargo clean")


@task
def lint(ctx):
    """Performs static analysis on all source files."""
    BuildContext(ctx).run("cargo clippy -- -D warnings")


@task
def fmt(ctx, check=False):
    """Formats all source files."""
    build_ctx = BuildContext(ctx)
    if check:
        build_ctx.run("cargo fmt -- --check")
    else:
        build_ctx.run("cargo fmt")


@task
def deploy(ctx):
    """Deploys the web app to production."""
    build_ctx = BuildContext(ctx)
    build_ctx.run(f"heroku container:push web --app {HEROKU_APP_NAME}")
    build_ctx.run(f"heroku container:release web --app {HEROKU_APP_NAME}")
