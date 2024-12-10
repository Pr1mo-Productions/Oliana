
# This script helps connect to `opi` (pronounced Oh-Pie, the Olliana-Pi Server, yes also that)
# and more specifically aids in determining what setup steps your machine needs
# for regular connections with as little hassle as possible.

import os
import sys
import subprocess
import shutil

def die(msg):
  print(msg)
  sys.exit(1)

if not shutil.which('ssh'):
  die('you must install SSH first! Cannot find ssh[.exe], please ensure the folder holding it is on your PATH (your PATH is currently == ', os.environ.get('PATH', ''), ')')

if not 'SSH_KEY_PATH' in os.environ:
  dir('For the sake of this script, please set the variable SSH_KEY_PATH to your SSH key you want to use to login.')



