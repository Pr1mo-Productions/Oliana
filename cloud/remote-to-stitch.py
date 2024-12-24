
# This script helps connect to `stitch` (pronounced Oh-Pie, the Olliana-Pi Server, yes also that)
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

ssh_key_path = None

github_ssh_config = subprocess.check_output(['ssh', '-G', 'github.com'])
if not isinstance(github_ssh_config, str):
  github_ssh_config = github_ssh_config.decode('utf-8')
identity_file_list = []
for line in github_ssh_config.splitlines(keepends=False):
  if line.lower().startswith('identityfile'):
    identity_file_list.append(line.split(' ', 1)[1])

for possible_id_file in identity_file_list:
  possible_id_file = os.path.expanduser(possible_id_file)
  if os.path.exists(possible_id_file):
    ssh_key_path = possible_id_file
    break

if ssh_key_path is None:
  print('Warning ssh_key_path could not be found because all keys configured to authenticate to github.com do not exist!')
  print('Falling back to reading key path from environment variable SSH_KEY_PATH')
  if not 'SSH_KEY_PATH' in os.environ or not os.path.exists(os.environ.get('SSH_KEY_PATH', '')):
    die('No configuration in ~/.ssh/config identified a github.com key to use (default, simplest config) AND no file from SSH_KEY_PATH exists!')
  ssh_key_path = os.environ['SSH_KEY_PATH']

print(f'We are using your private key located at {ssh_key_path} to authenticate; if this does not work make sure Jeff has added the public key from that (generally denoted with a .pub suffix) to the stitch server!')

server_name = 'stitch'+'.jmcateer'+'.com'
if 'SERVER_NAME' in os.environ:
  server_name = os.environ.get('SERVER_NAME', server_name)

server_port = '92'
if 'SERVER_PORT' in os.environ:
  server_port = os.environ.get('SERVER_PORT', server_port)

cmd = [
  'ssh',
  '-i', ssh_key_path,
  '-p', f'{server_port}',
  '-L', '8010:127.0.0.1:8010', # Forward :8010 on the box to 127.0.0.1:8010 in stitch
  '-L', '8011:127.0.0.1:8011', # Forward :8011 on the box to 127.0.0.1:8011 in stitch
  '-L', '8012:127.0.0.1:8012', # Forward :8012 on the box to 127.0.0.1:8012 in stitch
  f'user@{server_name}'
]

if shutil.which('waypipe') is not None and not 'NO_WAYPIPE' in os.environ:
  cmd = [ 'waypipe' ] + cmd

print(f'>>> {" ".join(cmd)}')

subprocess.run(cmd, shell=False)
