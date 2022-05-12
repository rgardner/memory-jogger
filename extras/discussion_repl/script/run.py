#!/usr/bin/env python

import os
import pathlib
import subprocess
import sys

os.chdir(pathlib.Path(__file__).parents[1])
subprocess.run(["poetry", "run", "discussion_repl"] + sys.argv[1:], check=True)
