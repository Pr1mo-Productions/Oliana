
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
      f'--target={pyenv}', 'torch', 'torchvision', 'torchaudio'
  ])
  import torch

print(f'torch={torch}')
print(f'torch.cuda.is_available() = {torch.cuda.is_available()}')
first_cuda_device = torch.device('cuda')
print(f'first_cuda_device = {first_cuda_device}')
print(f'memory info of device = {torch.cuda.mem_get_info(first_cuda_device)}')

# The following lists _ALL_ open filed, including the copy of libtorch_cuda.so we've just loaded.
subprocess.run([
  'lsof', '-p', str(os.getpid())
])
