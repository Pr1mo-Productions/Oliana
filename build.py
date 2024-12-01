import os
import sys
import subprocess
import shutil
import platform


subprocess.run([
  sys.executable, os.path.join('utils', 'lookup_torch_libraries.py'), 'quiet'
], check=True)

cargo_build_env = dict(os.environ)
cargo_build_env['LIBTORCH_USE_PYTORCH'] = '1'
cargo_build_env['PYTHONPATH'] = os.pathsep.join([ os.path.abspath(os.path.join('utils', 'pyenv')) ] + os.environ.get('PYTHONPATH', '').split(os.pathsep))

if 'linux' in platform.system().lower():
  cargo_build_env['LD_LIBRARY_PATH'] = os.pathsep.join([ os.path.abspath(os.path.join('utils', 'pyenv', 'torch', 'lib')) ] + os.environ.get('LD_LIBRARY_PATH', '').split(os.pathsep))
else:
  print(f'Warning: Not modifying path for OS platform "{platform.system()}"')


cargo_cmd = 'build'
cli_args = []
for arg in sys.argv[1:]:
  cli_args.append(arg)

if len(cli_args) > 0 and cli_args[0] == 'run':
  cargo_cmd = 'run'
  cli_args.pop(0)

subprocess.run([
  'cargo', cargo_cmd, *cli_args
], check=True, env=cargo_build_env)

