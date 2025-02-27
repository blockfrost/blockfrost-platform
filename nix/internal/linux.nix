{
  inputs,
  targetSystem,
  unix,
}:
assert __elem targetSystem ["x86_64-linux" "aarch64-linux"]; unix
