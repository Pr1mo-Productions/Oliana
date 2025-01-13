
import os
import sys
import subprocess
import traceback
import time


try:
  import blinkstick
except:
  subprocess.run([
    'yay', '-S', 'python-blinkstick-git' # See https://aur.archlinux.org/packages/python-blinkstick-git
  ])
  subprocess.run([
    'sudo', 'blinkstick', '--add-udev-rule'
  ])
  subprocess.run([
    'sudo', 'udevadm', 'control', '--reload-rules'
  ])
  subprocess.run([
    'sudo', 'udevadm', 'trigger'
  ])
  import blinkstick

import blinkstick.blinkstick


try:
  import psutil
except:
  subprocess.run([
    'yay', '-S', 'python-psutil'
  ])
  import psutil


# See https://arvydas.github.io/blinkstick-python/
bstick = blinkstick.blinkstick.find_first()
print(f'bstick={bstick}')

allowed_errors = 3
while allowed_errors > 0:
  try:

    time.sleep(0.75)

    user_sessions = psutil.users()
    num_user_sessions = len(user_sessions)
    print(f'num_user_sessions={num_user_sessions}')

    for i in range(0,8):
      if i < num_user_sessions:
        bstick.set_color(channel=0, index=i, red=128, green=0, blue=0)
      else:
        bstick.set_color(channel=0, index=i, red=0, green=0, blue=0) # "Off"

  except:
    traceback.print_exc()
    allowed_errors -= 1
    time.sleep(0.25)

