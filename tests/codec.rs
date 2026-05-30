use std::collections::BTreeMap;

use nota_next::{NotaEncode, NotaSource};

#[test]
fn codec_decodes_and_encodes_scalars() {
    assert_eq!(
        NotaSource::new("[schema owns strings]")
            .parse::<String>()
            .expect("string decodes"),
        "schema owns strings"
    );
    assert_eq!(
        NotaSource::new("42")
            .parse::<u64>()
            .expect("integer decodes"),
        42
    );
    assert!(
        NotaSource::new("True")
            .parse::<bool>()
            .expect("boolean decodes")
    );

    assert_eq!(
        "schema owns strings".to_owned().to_nota(),
        "[schema owns strings]"
    );
    assert_eq!(42_u64.to_nota(), "42");
    assert_eq!(false.to_nota(), "False");
}

#[test]
fn codec_decodes_and_encodes_collection_values() {
    let vector = NotaSource::new("[alpha beta gamma]")
        .parse::<Vec<String>>()
        .expect("vector decodes");
    assert_eq!(vector, vec!["alpha", "beta", "gamma"]);
    assert_eq!(vector.to_nota(), "[[alpha] [beta] [gamma]]");

    let option = NotaSource::new("(Some [cache entry])")
        .parse::<Option<String>>()
        .expect("option decodes");
    assert_eq!(option, Some("cache entry".to_owned()));
    assert_eq!(option.to_nota(), "(Some [cache entry])");

    let none = NotaSource::new("None")
        .parse::<Option<String>>()
        .expect("none decodes");
    assert_eq!(none, None);
    assert_eq!(none.to_nota(), "None");
}

#[test]
fn codec_decodes_and_encodes_ordered_map_values() {
    let map = NotaSource::new("{alpha 1 beta 2}")
        .parse::<BTreeMap<String, u64>>()
        .expect("map decodes");

    assert_eq!(map.get("alpha"), Some(&1));
    assert_eq!(map.get("beta"), Some(&2));
    assert_eq!(map.to_nota(), "{[alpha] 1 [beta] 2}");
}

#[test]
fn codec_decodes_and_encodes_boxed_values_without_shape_noise() {
    let boxed = NotaSource::new("[recursive reference]")
        .parse::<Box<String>>()
        .expect("boxed value decodes");

    assert_eq!(*boxed, "recursive reference");
    assert_eq!(boxed.to_nota(), "[recursive reference]");
}

#[test]
fn codec_rejects_multi_root_source_for_typed_parse() {
    let error = NotaSource::new("alpha beta")
        .parse::<String>()
        .expect_err("multi-root source rejects");

    assert!(
        error
            .to_string()
            .contains("expected exactly one NOTA root object")
    );
}
