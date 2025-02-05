#!/usr/bin/env bash

exe_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)

PATH="$exe_dir/bundle/bin:$PATH"
export PATH

# Clear the terminal for nicer UX:
reset

color_bold=$'\e[1m'
color_underline=$'\e[4m'
color_reset=$'\e[0m'

cat <<EOF

  Welcome!

  This terminal is set up for easy execution of the ‘${color_bold}blockfrost-platform${color_reset}’.

  Run one of the following commands:

   For setup: ${color_bold}blockfrost-platform --init${color_reset}
   For help:  ${color_bold}blockfrost-platform --help${color_reset}

  … and press <${color_bold}ENTER${color_reset}>.

  For more information, visit: ${color_underline}https://platform.blockfrost.io${color_reset}

EOF

exec "$SHELL" -i
