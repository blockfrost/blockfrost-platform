{
  targetSystem,
  unix,
  ...
}:
assert builtins.elem targetSystem ["x86_64-linux" "aarch64-linux"]; unix
