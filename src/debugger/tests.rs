use super::dbglanguage;

#[cfg(test)]
#[test]
fn number_test() {
    assert!(dbglanguage::parse_Number("12").is_ok());
    assert!(dbglanguage::parse_Number("-02").is_ok());
}

#[test]
fn hexnumber_test() {
    assert!(dbglanguage::parse_Number("0x12").is_ok());
}
