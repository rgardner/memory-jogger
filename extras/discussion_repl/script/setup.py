#!/usr/bin/env python

import os
import pathlib
import subprocess

main_dir = pathlib.Path(__file__).parent.absolute()
os.chdir(main_dir)
subprocess.run(["poetry", "install"], check=True)
subprocess.run(["cargo", "install", "--path", main_dir.parents[2]], check=True)
