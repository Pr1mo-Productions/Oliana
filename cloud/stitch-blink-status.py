
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

try:
  import inotify.adapters
except:
  subprocess.run([
    'yay', '-S', 'python-inotify'
  ])
  import inotify.adapters

try:
  if not os.path.exists('/tmp/last_bash_cmd'):
    with open('/tmp/last_bash_cmd', 'w') as fd:
      fd.write('0')
except:
  traceback.print_exc()

# See https://arvydas.github.io/blinkstick-python/
bstick = blinkstick.blinkstick.find_first()
print(f'bstick={bstick}')

allowed_errors = 3
while allowed_errors > 0:
  try:
    io = inotify.adapters.Inotify()
    io.add_watch('/tmp/last_bash_cmd')

    should_blink_led = False
    for event in io.event_gen(yield_nones=False, timeout_s=0.8):
      #print(f'event={event}')
      should_blink_led = True
      break # either 800ms passed or file processed

    user_sessions = psutil.users()
    num_user_sessions = len(user_sessions)
    print(f'num_user_sessions={num_user_sessions}')

    for i in range(0,8):
      if i < num_user_sessions:
        bstick.set_color(channel=0, index=i, red=128, green=0, blue=0)
      else:
        bstick.set_color(channel=0, index=i, red=0, green=0, blue=0) # "Off"

    led_to_blink = -1
    #if (time.time() - os.path.getmtime('/tmp/last_bash_cmd')) < 0.5:
    if should_blink_led:
      tty_name = ''
      with open('/tmp/last_bash_cmd', 'r') as fd:
        tty_name = fd.read()
      led_to_blink = int(''.join( c for c in tty_name if c.isdigit() ))
      print(f'led_to_blink={led_to_blink}')
      bstick.set_color(channel=0, index=led_to_blink, red=0, green=0, blue=0) # "Off"
      time.sleep(0.5)
      bstick.set_color(channel=0, index=led_to_blink, red=128, green=0, blue=0)


  except:
    traceback.print_exc()
    allowed_errors -= 1
    time.sleep(0.25)

# Note:
# We have added the following to the guest .bashrc in support of the blinking lights!
# function process_command() {
#   #echo "$BASH_COMMAND" > /tmp/last_bash_cmd
#   tty > /tmp/last_bash_cmd
# }
# trap process_command DEBUG

