#!/usr/bin/env python

import os
import pathlib
import subprocess

os.chdir(pathlib.Path(__file__).parents[1])
subprocess.run(["poetry", "run", "black", "."], check=True)
subprocess.run(["poetry", "run", "isort", "."], check=True)
