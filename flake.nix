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
        notaFilter = path: type:
          type == "regular" && pkgs.lib.hasSuffix ".nota" path;
        sourceFilter = path: type:
          type == "directory" || (craneLib.filterCargoSources path type) || (notaFilter path type);
        src = pkgs.lib.cleanSourceWith {
          src = ./.;
          filter = sourceFilter;
          name = "source";
        };
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
          operator-271-closed-claims = pkgs.runCommand "nota-next-operator-271-closed-claims" { } ''
            # Architectural-truth witnesses for operator 271 claims 2 and 3.
            # The Rust test file at tests/operator_271_closed_claims.rs runs
            # through cargo test; this Nix check verifies each named witness
            # is present and that the live derive source carries the present
            # canonical shape.
            test -f ${src}/tests/operator_271_closed_claims.rs
            # Claim 2 — FieldEncode ZST holder CLOSED. Current shape:
            #   struct FieldEncode<'field> { field: &'field Field }
            grep -R "struct FieldEncode<'field>" ${src}/derive/src/lib.rs >/dev/null
            grep -R "field: &'field Field," ${src}/derive/src/lib.rs >/dev/null
            grep -R "impl<'field> FieldEncode<'field>" ${src}/derive/src/lib.rs >/dev/null
            grep -R "fn body_named(&self)" ${src}/derive/src/lib.rs >/dev/null
            grep -R "FieldEncode::new(field).body_named()" ${src}/derive/src/lib.rs >/dev/null
            # The retired ZST and free-style call site are both gone.
            ! grep -R "struct FieldEncode;" ${src}/derive/src/lib.rs
            ! grep -R ".map(FieldEncode::body_named)" ${src}/derive/src/lib.rs
            # Claim 3 — CodecDerive single-field wrapper resolved-not-bug.
            grep -R "struct CodecDerive {" ${src}/derive/src/lib.rs >/dev/null
            grep -R "input: DeriveInput," ${src}/derive/src/lib.rs >/dev/null
            grep -R "CodecDerive::new(input).expand_decode()" ${src}/derive/src/lib.rs >/dev/null
            grep -R "CodecDerive::new(input).expand_encode()" ${src}/derive/src/lib.rs >/dev/null
            # Named witness functions present in the cargo-test surface.
            grep -R "field_encode_carries_field_data" ${src}/tests/operator_271_closed_claims.rs >/dev/null
            grep -R "field_encode_call_sites_construct_and_dispatch" ${src}/tests/operator_271_closed_claims.rs >/dev/null
            grep -R "codec_derive_wraps_syn_derive_input_with_methods" ${src}/tests/operator_271_closed_claims.rs >/dev/null
            grep -R "codec_derive_is_constructed_by_both_proc_macro_entry_points" ${src}/tests/operator_271_closed_claims.rs >/dev/null
            grep -R "field_decode_and_field_encode_mirror_each_other_as_data_bearing_wrappers" ${src}/tests/operator_271_closed_claims.rs >/dev/null
            grep -R "derive_crate_carries_no_zst_method_holders" ${src}/tests/operator_271_closed_claims.rs >/dev/null
            touch $out
          '';
          derive-crate-no-zst-method-holders = pkgs.runCommand "nota-next-derive-crate-no-zst-method-holders" { } ''
            # Pair-rule sweep per skills/architectural-truth-tests.md
            # §"Pair-rule sweeps". The derive crate hosts the single-field
            # wrapper pattern (validated by designer 448); the same scope
            # must remain free of unit-struct ZST namespace holders.
            if grep -R -n -E '^struct [A-Za-z][A-Za-z0-9_]*;' ${src}/derive/src; then
              echo "derive crate must not declare ZST namespace structs" >&2
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
