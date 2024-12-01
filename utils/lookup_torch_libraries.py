
import os
import sys
import subprocess

pyenv = os.path.join(os.path.dirname(__file__), 'pyenv')
os.makedirs(pyenv, exist_ok=True)
sys.path.append(pyenv)

try:
  import torch
except:
  subprocess.run([
    sys.executable, '-m', 'pip', 'install',
      f'--target={pyenv}',
      # 'torch', 'torchvision', 'torchaudio'
      #'torch==2.5.0', 'torchvision==0.20.0', 'torchaudio==2.5.0', '--index-url', 'https://download.pytorch.org/whl/cu124'
      'torch==2.2.0', 'torchvision==0.17.0', 'torchaudio==2.2.0', '--index-url', 'https://download.pytorch.org/whl/cu121'
  ])
  import torch

print(f'torch={torch}')
print(f'torch.cuda.is_available() = {torch.cuda.is_available()}')
first_cuda_device = torch.device('cuda')
print(f'first_cuda_device = {first_cuda_device}')
print(f'memory info of device = {torch.cuda.mem_get_info()}')

# The following lists _ALL_ open filed, including the copy of libtorch_cuda.so we've just loaded.
if not 'quiet' in sys.argv:
  subprocess.run([
    'lsof', '-p', str(os.getpid())
  ])
