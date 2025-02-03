#!/usr/bin/env bash

exe_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)

PATH="$exe_dir/bundle/bin:$PATH"
export PATH

# Clear the terminal for nicer UX:
reset

color_bold=$'\e[1m'
color_reset=$'\e[0m'

cat <<EOF

  Hello!

  This terminal has been set up to allow easy running of ‘${color_bold}blockfrost-platform${color_reset}’.

  Try typing in one of:

    ${color_bold}blockfrost-platform --help${color_reset}
    ${color_bold}blockfrost-platform --init${color_reset}

  … and pressing <${color_bold}ENTER${color_reset}>.

EOF

exec "$SHELL" -i
