{ julia-bin, writeShellScriptBin }:
let
  julia = julia-bin.withPackages [
    "Runic"
  ];
in
writeShellScriptBin "runic" ''
  exec ${julia}/bin/julia \
    --startup-file=no \
    --history-file=no \
    -e 'using Runic; exit(Runic.main(ARGS))' \
    -- "$@"
''
