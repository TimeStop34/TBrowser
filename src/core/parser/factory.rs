use crate::core::parser::base::Parser;
use crate::core::parser::plain::PlainParser;
use crate::core::parser::tmkup::TmkupParser;

pub fn get_parser(content_type: &str) -> Box<dyn Parser> {
    let ct = content_type.to_ascii_lowercase();
    if ct.contains("tmkup") {
        Box::new(TmkupParser)
    } else {
        Box::new(PlainParser)
    }
}