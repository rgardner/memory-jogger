"""Build tasks for Memory Jogger."""

import os
import pathlib
import shlex
import sys
from typing import Dict, List, Union

import invoke


def get_source_dir() -> pathlib.Path:
    return pathlib.Path(os.path.dirname(os.path.abspath(__file__)))


class BuildContext:
    def __init__(self, ctx: invoke.Context):
        self.ctx = ctx

    def run(self, command: Union[str, List[str]], *, env: Dict[str, str] = None):
        with self.ctx.cd(str(get_source_dir())):
            command_str = command if isinstance(command, str) else shlex.join(command)
            self.ctx.run(command_str)


def cargo_features(backends=None):
    if backends == ["sqlite"]:
        return ["--no-default-features", "--features", "sqlite"]
    elif backends == ["postgres"]:
        return ["--no-default-features", "--features", "postgres"]
    return []


def get_heroku_app_name() -> str:
    return os.environ["HEROKU_APP_NAME"]


@invoke.task(iterable=["backends"])
def build(ctx, backends=None, fast=False, docker=False):
    """Builds Memory Jogger."""
    build_ctx = BuildContext(ctx)
    if docker:
        if backends is not None:
            print(
                "warning: backends is ignored when building a Docker image",
                file=sys.stderr,
            )
        if fast:
            print(
                "warning: --fast is ignored when building a Docker image",
                file=sys.stderr,
            )

        build_ctx.run("docker-compose build")
    else:
        args = ["cargo", "check" if fast else "build", *cargo_features(backends)]
        build_ctx.run(args)


@invoke.task(iterable=["backends"])
def test(ctx, backends=None):
    """Runs all tests."""
    BuildContext(ctx).run(["cargo", "test", *cargo_features(backends)])


@invoke.task
def clean(ctx):
    """Removes built artifacts."""
    BuildContext(ctx).run("cargo clean")


@invoke.task(iterable=["backends"])
def lint(ctx, backends=None):
    """Runs clippy on all source files."""
    BuildContext(ctx).run(
        ["cargo", "clippy", *cargo_features(backends), "--", "-D", "warnings"]
    )


@invoke.task
def fmt(ctx, check=False):
    """Runs rustfmt on all source files."""
    build_ctx = BuildContext(ctx)
    if check:
        build_ctx.run("cargo fmt -- --check")
    else:
        build_ctx.run("cargo fmt")


@invoke.task
def deploy(ctx):
    """Deploys Docker container to Heroku."""
    build_ctx = BuildContext(ctx)
    app_name = get_heroku_app_name()
    build_ctx.run(f"heroku container:push web --app {app_name}")
    build_ctx.run(f"heroku container:release web --app {app_name}")
