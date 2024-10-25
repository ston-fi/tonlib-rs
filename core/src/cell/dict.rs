mod builder;
mod leading_bit_utils;
mod parser;
pub mod predefined_readers;
pub mod predefined_writers;
mod types;

pub(crate) use builder::DictBuilder;
pub(crate) use parser::DictParser;
pub use types::{KeyReader, SnakeFormatDict, ValReader, ValWriter};

#[cfg(test)]
mod tests;
