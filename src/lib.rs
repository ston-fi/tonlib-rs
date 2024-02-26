extern crate core;

pub mod address;
pub mod cell;
#[cfg(feature = "interactive")] pub mod client;
pub mod config;
#[cfg(feature = "interactive")] pub mod contract;
pub mod message;
pub mod meta;
pub mod mnemonic;
#[cfg(feature = "interactive")] pub mod tl;
pub mod types;
pub mod wallet;

#[doc = include_str!("../README.md")]
#[cfg(feature = "interactive")]
#[cfg(doctest)]
pub struct ReadmeDoctests;
