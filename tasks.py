"""Build tasks for Memory Jogger."""

import os
import pathlib
import shlex
import sys
from typing import Dict, List, Optional, TypeVar, Union

import invoke  # type: ignore


def get_source_dir() -> pathlib.Path:
    return pathlib.Path(os.path.dirname(os.path.abspath(__file__)))


class BuildContext:
    def __init__(self, ctx: invoke.Context):
        self.ctx = ctx

    def run(self, command: Union[str, List[str]], *, env: Dict[str, str] = None):
        with self.ctx.cd(str(get_source_dir())):
            command_str = command if isinstance(command, str) else shlex.join(command)
            self.ctx.run(command_str)


def cargo_features(backends: Optional[List[str]] = None, large=False):
    features = []
    if backends == ["sqlite"]:
        features.extend(["--no-default-features", "--features", "sqlite"])
    elif backends == ["postgres"]:
        features.extend(["--no-default-features", "--features", "postgres"])

    if large:
        features.extend(["--features", "large_tests"])

    return features


def get_heroku_app_name() -> str:
    return os.environ["HEROKU_APP_NAME"]


@invoke.task(iterable=["backends"])
def build(
    ctx, release=False, all_targets=False, backends=None, fast=False, docker=False
):
    """Builds Memory Jogger."""
    build_ctx = BuildContext(ctx)
    if docker:

        T = TypeVar("T")

        def warn_if_supplied(name: str, value: Union[bool, Optional[T]]):
            if value:
                print(
                    f"warning: --{name} is ignored when building a Docker image",
                    file=sys.stderr,
                )

        for name, value in [
            ("release", release),
            ("all-targets", all_targets),
            ("backends", backends),
            ("fast", fast),
        ]:
            warn_if_supplied(name, value)

        build_ctx.run("docker-compose build")
    else:
        args = ["cargo", "check" if fast else "build", *cargo_features(backends)]
        if release:
            args.append("--release")
        if all_targets:
            args.append("--all-targets")
        build_ctx.run(args)


@invoke.task(iterable=["backends"])
def test(ctx, release=False, backends=None, large=False):
    """Runs all tests."""
    args = ["cargo", "test", *cargo_features(backends, large)]
    if release:
        args.append("--release")
    BuildContext(ctx).run(args)


@invoke.task
def clean(ctx):
    """Removes built artifacts."""
    BuildContext(ctx).run("cargo clean")


@invoke.task(iterable=["backends"])
def lint(ctx, backends=None):
    """Runs clippy on all source files."""
    BuildContext(ctx).run(
        [
            "cargo",
            "clippy",
            "--all-features",
            *cargo_features(backends),
            "--",
            "-D",
            "warnings",
        ]
    )


@invoke.task
def fmt(ctx, check=False):
    """Runs rustfmt on all source files."""
    build_ctx = BuildContext(ctx)
    if check:
        build_ctx.run("cargo fmt -- --check")
    else:
        build_ctx.run("cargo fmt")
