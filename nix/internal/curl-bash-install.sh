#!/usr/bin/env bash
set -euo pipefail

project_name="@projectName@"
version="@projectVersion@"
short_rev="@shortRev@"
base_url="@baseUrl@"

color_bold=$'\e[1m'
color_reset=$'\e[0m'

isa="$(uname -m)"
kernel="$(uname -s)"
case "${isa}-${kernel}" in
"x86_64-Linux")
  target_system="x86_64-linux"
  expected_sha256="@sha256_x86_64_linux@"
  ;;
"aarch64-Linux")
  target_system="aarch64-linux"
  expected_sha256="@sha256_aarch64_linux@"
  ;;
"x86_64-Darwin")
  target_system="x86_64-darwin"
  expected_sha256="@sha256_x86_64_darwin@"
  ;;
# Apple Silicon can appear as "arm64-Darwin" rather than "aarch64-Darwin":
"arm64-Darwin")
  target_system="aarch64-darwin"
  expected_sha256="@sha256_aarch64_darwin@"
  ;;
"aarch64-Darwin")
  target_system="aarch64-darwin"
  expected_sha256="@sha256_aarch64_darwin@"
  ;;
*)
  echo >&2 "fatal: no matching installer found for ${color_bold}${isa}-${kernel}${color_reset}" >&2
  exit 1
  ;;
esac
unset isa
unset kernel

archive="${project_name}-${version}-${short_rev}-${target_system}.tar.bz2"
archive_url="${base_url}/${archive}"

download_path="$(mktemp -d)/${archive}"
# shellcheck disable=SC2016
opt_dir_expr='$HOME/.local/opt'
opt_dir="${HOME}/.local/opt"
install_dir="${opt_dir}/${project_name}"
install_dir_expr="${opt_dir_expr}/${project_name}"
bin_dir="${install_dir}/bin"
bin_dir_expr="${install_dir_expr}/bin"

if ! command -v bzip2 >/dev/null 2>&1; then
  echo >&2 "fatal: ${color_bold}bzip2${color_reset} not found, please install it, and try again"
  exit 1
fi

if command -v sha256sum >/dev/null 2>&1; then
  sha256_cmd="sha256sum"
elif command -v shasum >/dev/null 2>&1; then
  sha256_cmd="shasum -a 256"
else
  echo >&2 "fatal: neither ${color_bold}sha256sum${color_reset} nor ${color_bold}shasum${color_reset} were found, please install one, and try again"
  exit 1
fi

echo >&2 "info: downloading ${color_bold}${archive_url}${color_reset}"
echo >&2 "info: saving to ${color_bold}${download_path}${color_reset}"

if command -v curl >/dev/null 2>&1; then
  curl -fsSL -o "${download_path}" "$archive_url"
elif command -v wget >/dev/null 2>&1; then
  wget -O "${download_path}" "$archive_url"
else
  echo >&2 "fatal: neither ${color_bold}curl${color_reset} nor ${color_bold}wget${color_reset} were found, please install one, and try again"
  exit 1
fi

calculated_sha256=$($sha256_cmd "${download_path}" | cut -d' ' -f1)
if [ "$calculated_sha256" != "$expected_sha256" ]; then
  echo >&2 "fatal: archive checksum mismatch (network issues?), please restart the installer"
  exit 1
fi

if [ -e "$install_dir" ]; then
  old_installation="${install_dir}.$(date '+%Y-%m-%dT%H:%M:%S%z')"
  echo >&2 "warn: moving previous installation to ${color_bold}${old_installation}${color_reset}"
  chmod +w "$install_dir"
  mv "$install_dir" "$old_installation"
  chmod -w "$old_installation"
  unset old_installation
fi

echo >&2 "info: installing to ${color_bold}${install_dir}${color_reset}"
mkdir -p "$opt_dir"
(cd "$opt_dir" && tar -xjf "$download_path")

echo >&2 "info: adding ${color_bold}${bin_dir}${color_reset} to ${color_bold}\$PATH${color_reset}"

add_to_path_script="$install_dir/add-to-path.sh"
add_to_path_expr="source \"${install_dir_expr}/add-to-path.sh\""
chmod +w "$install_dir"
cat >"$add_to_path_script" <<EOF
case ":\${PATH}:" in
  *:"$bin_dir_expr":*) ;;
  *) export PATH="${bin_dir_expr}:\$PATH" ;;
esac
EOF
chmod -w "$add_to_path_script" "$install_dir"

at_least_one=
for rcname in .profile .bash_profile .bash_login .bashrc .zshrc .zshenv .zprofile; do
  rcfile="$HOME/$rcname"
  if [ -e "$rcfile" ]; then
    if grep -qF "$add_to_path_expr" "$rcfile"; then
      at_least_one=1
    else
      if echo $'\n'"$add_to_path_expr" >>"$rcfile"; then
        echo >&2 "info: sucessfully added to ${color_bold}${rcname}${color_reset}"
        at_least_one=1
      fi
    fi
  fi
done
if [ -z "$at_least_one" ]; then
  rcname=".profile"
  rcfile="$HOME/$rcname"
  if echo "$add_to_path_expr" >"$rcfile"; then
    echo >&2 "info: sucessfully added to ${color_bold}${rcname}${color_reset}"
    at_least_one=1
  fi
fi
if [ -z "$at_least_one" ]; then
  echo >&2 "warn: unable to add to ${color_bold}\$PATH${color_reset}, you'll have to do that yourself"
  echo >&2
  echo >&2 "Add the following line to your shell config, and restart it, or just run:"
else
  echo >&2
  echo >&2 "Now, either restart your shell, or run:"
fi

echo >&2
echo >&2 "    ${color_bold}${add_to_path_expr}${color_reset}"
echo >&2
echo >&2 "To be able to later run one of:"
echo >&2
echo >&2 "    ${color_bold}${project_name} --init${color_reset}"
echo >&2 "    ${color_bold}${project_name} --help${color_reset}"
