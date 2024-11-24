
import os
import sys
import subprocess
import shutil

def bincheck(bin_name):
  if shutil.which(bin_name) is None:
    print(f'Fatal Error: this tool needs {bin_name} installed and available on the PATH, but it was not found!')
    sys.exit(1)

def cmd(*parts):
  cmd_parts = [x for x in parts if not (x is None)]
  cmd_parts[0] = shutil.which(cmd_parts[0])
  print(f'> {" ".join(cmd_parts)}')
  subprocess.run(cmd_parts, check=True)

def cmd_in(directory, *parts):
  cmd_parts = [x for x in parts if not (x is None)]
  cmd_parts[0] = shutil.which(cmd_parts[0])
  print(f'> cd {directory} ; {" ".join(cmd_parts)}')
  subprocess.run(cmd_parts, check=True, cwd=directory)

def clone_or_update(git_url, target_dir):
  if not os.path.exists(target_dir):
    cmd('git', 'clone', git_url, target_dir)
  elif os.path.exists(os.path.join(target_dir, '.git')):
    cmd_in(target_dir, 'git', 'lfs', 'fetch', '--all')
    cmd_in(target_dir, 'git', 'lfs', 'pull')
    cmd_in(target_dir, 'git', 'lfs', 'checkout')
  else:
    raise Exception(f'Refusing to clone from {git_url} to {target_dir} because {target_dir} already exists and does NOT have a .git folder within!')


if len(sys.argv) < 2:
  print(f'Pass the URL to a huggingface-formatted repo as the first argument; for example:')
  print(f'  python utils/oneshot-convert-hf-model-to-onnx.py https://huggingface.co/Qwen/Qwen2.5-7B-Instruct')
  sys.exit(1)

hf_url_to_convert = sys.argv[1]
print(f'Converting: {hf_url_to_convert}')
hf_url_name = hf_url_to_convert.rsplit('/', 1)[1]
print(f'Folder name: {hf_url_name}')

cwd = os.path.dirname(__file__)
os.chdir(cwd)

bincheck('git')
cmd('git', 'lfs', 'install')

workdir = os.path.join(cwd, 'work')
os.makedirs(workdir, exist_ok=True)

qwen2_export_dir = os.path.join(workdir, 'qwen2-export-onnx')
clone_or_update('https://github.com/w3ng-git/qwen2-export-onnx.git', qwen2_export_dir)

qwen2_export_pyenv = os.path.join(workdir, 'qwen2-export-onnx-pyenv')
os.makedirs(qwen2_export_pyenv, exist_ok=True)

if not os.path.exists(os.path.join(qwen2_export_pyenv, 'transformers')) or not os.path.exists(os.path.join(qwen2_export_pyenv, 'onnx')):
  cmd('python', '-m', 'pip', 'install', f'--target={qwen2_export_pyenv}', 'transformers', 'onnx')
if not os.path.exists(os.path.join(qwen2_export_pyenv, 'torch')):
  cmd('python', '-m', 'pip', 'install', f'--target={qwen2_export_pyenv}', '--pre', '--upgrade', 'torch', 'torchvision', 'torchaudio', '--index-url', 'https://download.pytorch.org/whl/nightly/cpu')

os.environ['PYTHONPATH'] = qwen2_export_pyenv

hf_local_repo = os.path.join(workdir, hf_url_name)
clone_or_update(hf_url_to_convert, hf_local_repo)

output_onnx_folder = os.path.join(workdir, 'output_onnx')
if os.path.exists(output_onnx_folder):
  yn = input(f'{output_onnx_folder} already exists, delete before export? ')
  if not 'y' in yn.lower():
    print(f'Exiting w/o deleting {output_onnx_folder}...')
    sys.exit(1)
  else:
    # y was in yn.lower()
    shutil.rmtree(output_onnx_folder)

os.makedirs(output_onnx_folder, exist_ok=True)

cmd_in(workdir, 'python',
  os.path.join(qwen2_export_dir, 'export_onnx_qwen.py'),
  hf_local_repo
)

print()
print(f'The model from {hf_local_repo} now has ONNX runtime files located at {output_onnx_folder}')
print()

