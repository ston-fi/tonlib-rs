mod error;

use std::cmp;
use std::collections::HashMap;

pub use error::*;
use hmac::{Hmac, Mac};
use lazy_static::lazy_static;
use nacl::sign::generate_keypair;
use pbkdf2::password_hash::Output;
use pbkdf2::{pbkdf2_hmac, Params};
use sha2::Sha512;

const WORDLIST_EN: &str = include_str!("mnemonic/wordlist.EN");
const PBKDF_ITERATIONS: u32 = 100000;

lazy_static! {
    pub static ref WORDLIST_EN_SET: HashMap<&'static str, usize> = {
        let words: HashMap<&'static str, usize> = WORDLIST_EN
            .split('\n')
            .filter(|w| !w.is_empty())
            .enumerate()
            .map(|(i, w)| (w.trim(), i))
            .collect();
        words
    };
}

/// A Rust port of https://github.com/tonwhales/ton-crypto/blob/master/src/mnemonic/mnemonic.ts
pub struct Mnemonic {
    words: Vec<String>,
    password: Option<String>,
}

#[derive(PartialEq, Eq, Clone, Hash)]
pub struct KeyPair {
    pub public_key: Vec<u8>,
    pub secret_key: Vec<u8>,
}

impl Mnemonic {
    pub fn new(words: Vec<&str>, password: &Option<String>) -> Result<Mnemonic, MnemonicError> {
        let normalized_words: Vec<String> = words.iter().map(|w| w.trim().to_lowercase()).collect();

        // Check words
        if normalized_words.len() != 24 {
            return Err(MnemonicError::UnexpectedWordCount(normalized_words.len()));
        }
        for word in &normalized_words {
            if !WORDLIST_EN_SET.contains_key(word.as_str()) {
                return Err(MnemonicError::InvalidWord(word.clone()));
            }
        }

        // Check password validity
        match password {
            Some(s) if !s.is_empty() => {
                let passless_entropy = to_entropy(&normalized_words, &None)?;
                let seed = pbkdf2_sha512(passless_entropy, "TON fast seed version", 1, 64)?;
                if seed[0] != 1 {
                    return Err(MnemonicError::InvalidFirstByte(seed[0]));
                }
                // Make that this also is not a valid passwordless mnemonic
                let entropy = to_entropy(&normalized_words, password)?;
                let seed = pbkdf2_sha512(
                    entropy,
                    "TON seed version",
                    cmp::max(1, PBKDF_ITERATIONS / 256),
                    64,
                )?;
                if seed[0] == 0 {
                    return Err(MnemonicError::InvalidFirstByte(seed[0]));
                }
            }
            _ => {
                let entropy = to_entropy(&normalized_words, &None)?;
                let seed = pbkdf2_sha512(
                    entropy,
                    "TON seed version",
                    cmp::max(1, PBKDF_ITERATIONS / 256),
                    64,
                )?;
                if seed[0] != 0 {
                    return Err(MnemonicError::InvalidPasswordlessMenmonicFirstByte(seed[0]));
                }
            }
        }

        let mnemonic = Mnemonic {
            words: normalized_words,
            password: password.clone(),
        };
        Ok(mnemonic)
    }

    pub fn from_str(s: &str, password: &Option<String>) -> Result<Mnemonic, MnemonicError> {
        let words: Vec<&str> = s
            .split(' ')
            .map(|w| w.trim())
            .filter(|w| !w.is_empty())
            .collect();
        Mnemonic::new(words, password)
    }

    pub fn to_key_pair(&self) -> Result<KeyPair, MnemonicError> {
        let entropy = to_entropy(&self.words, &self.password)?;
        let seed = pbkdf2_sha512(entropy, "TON default seed", PBKDF_ITERATIONS, 64)?;
        let key_pair = generate_keypair(&seed.as_slice()[0..32]);
        Ok(KeyPair {
            public_key: key_pair.pkey.to_vec(),
            secret_key: key_pair.skey.to_vec(),
        })
    }
}

fn to_entropy(words: &[String], password: &Option<String>) -> Result<Vec<u8>, MnemonicError> {
    let mut mac = Hmac::<Sha512>::new_from_slice(words.join(" ").as_bytes())?;
    if let Some(s) = password {
        mac.update(s.as_bytes());
    }
    let result = mac.finalize();
    let code_bytes = result.into_bytes().to_vec();
    Ok(code_bytes)
}

fn pbkdf2_sha512(
    key: Vec<u8>,
    salt: &str,
    rounds: u32,
    output_length: usize,
) -> Result<Vec<u8>, MnemonicError> {
    let params = Params {
        rounds,
        output_length,
    };

    let output = Output::init_with(params.output_length, |out| {
        pbkdf2_hmac::<Sha512>(key.as_slice(), salt.as_bytes(), params.rounds, out);
        Ok(())
    })
    .map_err(MnemonicError::PasswordHashError)?;
    Ok(output.as_bytes().to_vec())
}

///Based on https://github.com/tonwhales/ton-crypto/blob/master/src/mnemonic/mnemonic.spec.ts
#[cfg(test)]
mod tests {
    use crate::mnemonic::Mnemonic;

    #[test]
    fn mnemonic_parse_works() -> anyhow::Result<()> {
        let words = "dose ice enrich trigger test dove century still betray gas diet dune use other base gym mad law immense village world example praise game";
        let mnemonic = Mnemonic::from_str(words, &None);
        assert!(mnemonic.is_ok());

        let words = " dose ice enrich trigger test dove \
        century still betray gas diet       dune use other base gym mad law \
        immense village world example praise game ";
        let mnemonic = Mnemonic::from_str(words, &None);
        assert!(mnemonic.is_ok());
        Ok(())
    }

    #[test]
    fn mnemonic_validate_works() -> anyhow::Result<()> {
        let mnemonic = Mnemonic::new(
            vec![
                "dose", "ice", "enrich", "trigger", "test", "dove", "century", "still", "betray",
                "gas", "diet", "dune",
            ],
            &None,
        );
        assert!(mnemonic.is_err());
        let mnemonic = Mnemonic::new(vec!["a"], &None);
        assert!(mnemonic.is_err());
        Ok(())
    }

    #[test]
    fn mnemonic_to_private_key_works() -> anyhow::Result<()> {
        let mnemonic = Mnemonic::new(
            vec![
                "dose", "ice", "enrich", "trigger", "test", "dove", "century", "still", "betray",
                "gas", "diet", "dune", "use", "other", "base", "gym", "mad", "law", "immense",
                "village", "world", "example", "praise", "game",
            ],
            &None,
        )?;
        let expected = "119dcf2840a3d56521d260b2f125eedc0d4f3795b9e627269a4b5a6dca8257bdc04ad1885c127fe863abb00752fa844e6439bb04f264d70de7cea580b32637ab";

        let kp = mnemonic.to_key_pair()?;
        println!("{:?} {:?}", kp.public_key, kp.secret_key);

        let res = hex::encode(kp.secret_key);

        assert_eq!(res, expected);

        Ok(())
    }
}
