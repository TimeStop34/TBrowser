mod base;
#[allow(unused)]
pub use base::Parser;

mod plain;
#[allow(unused)]
pub use plain::PlainParser;

mod tmkup;
#[allow(unused)]
pub use tmkup::TmkupParser;

mod factory;
pub use factory::get_parser;