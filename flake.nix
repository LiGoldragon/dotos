{
  description = "nota-next — structural NOTA reader for the schema-derived stack";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
  };

  outputs = { self, nixpkgs, flake-utils, fenix, crane }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        toolchain = fenix.packages.${system}.stable.withComponents [
          "cargo"
          "rustc"
          "rustfmt"
          "clippy"
          "rust-src"
        ];
        craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;
        src = craneLib.cleanCargoSource ./.;
        commonArguments = { inherit src; strictDeps = true; };
        cargoArtifacts = craneLib.buildDepsOnly commonArguments;
      in
      {
        packages.default = craneLib.buildPackage (commonArguments // { inherit cargoArtifacts; });
        checks = {
          build = craneLib.cargoBuild (commonArguments // { inherit cargoArtifacts; });
          test = craneLib.cargoTest (commonArguments // { inherit cargoArtifacts; });
          design-examples = pkgs.runCommand "nota-next-design-examples" { } ''
            grep -R "design_example_source_spans_propagate_through_nested_blocks" ${src}/tests/design_examples.rs >/dev/null
            grep -R "design_example_reader_exposes_candidates_not_schema_semantics" ${src}/tests/design_examples.rs >/dev/null
            grep -R "design_example_pipe_delimiters_are_recursive_blocks" ${src}/tests/design_examples.rs >/dev/null
            grep -R "design_example_structure_header_captures_first_two_levels" ${src}/tests/design_examples.rs >/dev/null
            grep -R "design_example_structure_header_marks_child_count_overflow" ${src}/tests/design_examples.rs >/dev/null
            grep -R "design_example_structure_header_marks_slot_truncation" ${src}/tests/design_examples.rs >/dev/null
            touch $out
          '';
          no-escaped-newline-nota-fixtures = pkgs.runCommand "nota-next-no-escaped-newline-nota-fixtures" { } ''
            if grep -R -n -E 'let source = ".*\\n' ${src}/tests; then
              echo 'inline NOTA fixtures must use spaces or real newlines in raw strings, not \n escapes' >&2
              exit 1
            fi
            touch $out
          '';
          no-production-free-functions = pkgs.runCommand "nota-next-no-production-free-functions" { } ''
            if grep -R -n -E '^(pub(\([^)]*\))? )?fn ' ${src}/src; then
              echo "production Rust must not use module-level free functions" >&2
              exit 1
            fi
            touch $out
          '';
          doc = craneLib.cargoDoc (commonArguments // {
            inherit cargoArtifacts;
            RUSTDOCFLAGS = "-D warnings";
          });
          fmt = craneLib.cargoFmt { inherit src; };
          clippy = craneLib.cargoClippy (commonArguments // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- -D warnings";
          });
        };
        devShells.default = pkgs.mkShell {
          name = "nota-next";
          packages = [ pkgs.jujutsu toolchain ];
        };
      });
}
