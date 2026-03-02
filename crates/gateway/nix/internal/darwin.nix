{
  targetSystem,
  unix,
  ...
}:
assert builtins.elem targetSystem [
  "x86_64-darwin"
  "aarch64-darwin"
]; unix
