extern crate serde;

#[cfg(feature="withserde")]
extern crate serde_json;

#[allow(unused_imports)] 
#[macro_use]
extern crate serde_derive;

extern crate molysite;

#[allow(unused_imports)] 
use std::collections::HashMap;

#[allow(unused_imports)] 
use molysite::hcl::parse_hcl;

#[cfg(feature="withserde")]
#[test]
fn hcl_serde_basic() {
    #[derive(Deserialize, Debug)]
    struct User {
        fingerprint: String,
        location: String,
    }

    let test = "fingerprint = \"foo\"
location = \"bar\"";

    if let Ok(j) = parse_hcl(test) {
        let u: User = serde_json::from_value(j).unwrap();
        assert_eq!("foo", u.fingerprint);
        assert_eq!("bar", u.location);
    }
}

#[cfg(not(feature="arraynested"))]
#[cfg(feature="withserde")]
#[test]
fn hcl_serde_policy() {
    #[derive(Deserialize, Debug)]
    struct PolicyResp {
        key: HashMap<String, Policy>,
    }

    #[derive(Deserialize, Debug)]
    struct Policy {
        policy: String,
        options: Option<Vec<String>>,
    }

    let test = "key \"\" {
	policy = \"read\"
}

key \"foo/\" {
	policy = \"write\"
    options = [\"list\", \"edit\"]
}

key \"foo/bar/\" {
	policy = \"read\"
}

key \"foo/bar/baz\" {
	policy = \"deny\"
}";

    if let Ok(j) = parse_hcl(test) {
        let u: PolicyResp = serde_json::from_value(j).unwrap();
        println!("{:?}", u);
        if let Some(policy) = u.key.get("") {
            assert_eq!(policy.policy, "read");
        } else {
            panic!("missing key");
        }
        if let Some(policy) = u.key.get("foo/") {
            assert_eq!(policy.policy, "write");
        } else {
            panic!("missing key");
        }
        if let Some(policy) = u.key.get("foo/bar/") {
            assert_eq!(policy.policy, "read");
        } else {
            panic!("missing key");
        }
        if let Some(policy) = u.key.get("foo/bar/baz") {
            assert_eq!(policy.policy, "deny");
        } else {
            panic!("missing key");
        }
    }
}
