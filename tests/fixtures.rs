extern crate molysite;

use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use molysite::hcl::parse_hcl;
use molysite::json::parse_json;

#[cfg(feature = "arraynested")]
macro_rules! fixture_tests {
    ($($name:ident: $value:expr,)*) => {
    $(
        #[test]
        fn $name() {
            let (case, expect_pass) = $value;
            test_fixture(case, expect_pass);
        }
    )*
    }
}

#[cfg(feature = "arraynested")]
fixture_tests! {
    test_fixture_assign_deep: ("assign_deep", true),
    test_fixture_basic: ("basic", true),
    test_fixture_basic_int_string: ("basic_int_string", true),
    test_fixture_basic_squish: ("basic_squish", true),
    //test_fixture_block_assign: ("block_assign", false),
    test_fixture_decode_policy: ("decode_policy", true),
    test_fixture_decode_tf_variable: ("decode_tf_variable", true),
    test_fixture_empty: ("empty", true),
    test_fixture_escape: ("escape", true),
    test_fixture_escape_backslash: ("escape_backslash", true),
    test_fixture_flat: ("flat", true),
    test_fixture_float: ("float", true),
    //test_fixture_git_crypt: ("git_crypt", false),
    test_fixture_list_of_lists: ("list_of_lists", true),
    test_fixture_list_of_maps: ("list_of_maps", true),
    test_fixture_multiline: ("multiline", true),
    //test_fixture_multiline_bad: ("multiline_bad", false),
    test_fixture_multiline_indented: ("multiline_indented", true),
    //test_fixture_multiline_literal: ("multiline_literal", false),
    test_fixture_multiline_literal_with_hil: ("multiline_literal_with_hil", true),
    test_fixture_multiline_no_eof: ("multiline_no_eof", true),
    test_fixture_multiline_no_hanging_indent: ("multiline_no_hanging_indent", true),
    //test_fixture_multiline_no_marker: ("multiline_no_marker", false),
    test_fixture_nested_block_comment: ("nested_block_comment", true),
    //test_fixture_nested_provider_bad: ("nested_provider_bad", false),
    test_fixture_object_with_bool: ("object_with_bool", true),
    test_fixture_scientific: ("scientific", true),
    test_fixture_slice_expand: ("slice_expand", true),
    //test_fixture_structure2: ("structure2", false),
    test_fixture_structure: ("structure", true),
    test_fixture_structure_flatmap: ("structure_flatmap", true),
    test_fixture_structure_list: ("structure_list", true),
    test_fixture_structure_multi: ("structure_multi", true),
    test_fixture_terraform_heroku: ("terraform_heroku", true),
    test_fixture_tfvars: ("tfvars", true),
    //test_fixture_unterminated_block_comment: ("unterminated_block_comment", false),
    //test_fixture_unterminated_brace: ("unterminated_brace", false),
}

#[cfg(feature = "arraynested")]
#[allow(dead_code)]
fn test_fixture(case: &str, expect_pass: bool) {
    let mut hcl = String::new();
    let mut json = String::new();

    let hcl_path = format!("tests/test-fixtures/{}.hcl", case);
    let json_path = format!("tests/test-fixtures/{}.hcl.json", case);

    let path = Path::new(&hcl_path);
    let mut file = File::open(&path).unwrap();
    file.read_to_string(&mut hcl).unwrap();

    if expect_pass {
        let path = Path::new(&json_path);
        let mut file = File::open(&path).unwrap();
        file.read_to_string(&mut json).unwrap();
    }

    let parsed_hcl = parse_hcl(&hcl);
    if let Ok(parsed_hcl) = parsed_hcl {
        if expect_pass {
            let parsed_json = parse_json(&json).unwrap();
            assert_eq!(parsed_hcl, parsed_json);
        } else {
            panic!("Expected failure")
        }
    } else {
        if expect_pass {
            panic!("Expected success")
        }
    }
}
