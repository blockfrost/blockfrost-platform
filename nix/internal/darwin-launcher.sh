#!/usr/bin/env bash

exe_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)

init_script="$exe_dir/darwin-terminal-init.sh"

escaped_for_apple_script=$(sed <<<"$init_script" -e 's/\\/\\\\/g' -e 's/"/\\"/g' -e "s/'/\\\\'/g")

# If the Terminal app is running, `activate` will not add a new window,
# but just move the app to the foreground instead. So we need to add
# a new window.
#
# If the app is not running, we activate it, which also brings it to the
# foreground, and creates a new window. So to prevent having 2 windows,
# we run the script in front window.
#
# We also have to escape the path to the init script properly.
osascript <<END
  set initScript to "${escaped_for_apple_script}"

  if application "Terminal" is running then
    tell application "Terminal"
      activate
      do script ("exec " & quoted form of initScript)
    end tell
  else
    tell application "Terminal"
      activate
      do script ("exec " & quoted form of initScript) in front window
    end tell
  end if
END
