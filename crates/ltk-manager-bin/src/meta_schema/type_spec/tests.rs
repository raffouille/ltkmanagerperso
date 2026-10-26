use ltk_hash::HashValue;
use ltk_meta::PropertyValueEnum;
use ltk_meta::property::values;

use super::*;

fn narrow() -> PropertyValueEnum {
    values::Hash::new(BinHash(0x1234_5678)).into()
}

fn wide() -> PropertyValueEnum {
    values::Hash::new(HashValue::wide(0x5d07_ca0d_22ff_9588)).into()
}

#[test]
fn a_hash_spec_without_a_width_matches_either_width() {
    let spec = TypeSpec::bare(Kind::Hash);
    assert!(spec.matches(&narrow()).unwrap());
    assert!(spec.matches(&wide()).unwrap());
}

#[test]
fn a_hash_spec_with_a_width_matches_that_width_only() {
    let spec = TypeSpec {
        width: Some(HashWidth::W8),
        ..TypeSpec::bare(Kind::Hash)
    };
    assert!(spec.matches(&wide()).unwrap());
    assert!(!spec.matches(&narrow()).unwrap());
    assert!(
        !spec
            .matches(&PropertyValueEnum::from(values::String::new(
                "a".to_owned()
            )))
            .unwrap()
    );
}

#[test]
fn of_reads_the_width_off_a_hash_and_nothing_else() {
    assert_eq!(TypeSpec::of(&narrow()).unwrap().width, Some(HashWidth::W4));
    assert_eq!(TypeSpec::of(&wide()).unwrap().width, Some(HashWidth::W8));
    assert_eq!(
        TypeSpec::of(&PropertyValueEnum::from(values::U32::new(1)))
            .unwrap()
            .width,
        None
    );
}

#[test]
fn an_eight_byte_hash_labels_its_width() {
    let spec = TypeSpec {
        width: Some(HashWidth::W8),
        ..TypeSpec::bare(Kind::Hash)
    };
    assert_eq!(spec.label(), "hash[u64]");
    assert_eq!(TypeSpec::bare(Kind::Hash).label(), "hash");
    let narrow = TypeSpec {
        width: Some(HashWidth::W4),
        ..TypeSpec::bare(Kind::Hash)
    };
    assert_eq!(narrow.label(), "hash");
}
