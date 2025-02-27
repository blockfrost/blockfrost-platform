{
  inputs,
  targetSystem,
  unix,
}:
assert __elem targetSystem ["x86_64-darwin" "aarch64-darwin"]; unix
