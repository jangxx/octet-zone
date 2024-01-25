use regex::Regex;
use std::u16;

#[derive(PartialEq, Debug)]
pub enum Token {
	Octet(u8),
	Filler,
	HexBlock(u16),
	LocalModifier,
	Unknown,
}

pub struct Parser {
	tokens: Vec<Token>,
	octet_re: Regex,
	hexblock_re: Regex,
	short_hexblock_re: Regex,
}

impl Parser {
	pub fn new() -> Self {
		Parser {
			tokens: Vec::new(),
			octet_re: Regex::new(r"^[0-9]{1,3}$").unwrap(),
			hexblock_re: Regex::new(r"^[0-9a-fA-F]{4}$").unwrap(),
			short_hexblock_re: Regex::new(r"^x([0-9a-fA-F]{1,3})$").unwrap(),
		}
	}

	pub fn add_token_from_label(&mut self, label: &str) {
		let mut token = match label {
			"_" => Token::Filler,
			"local" => Token::LocalModifier,
			_ => Token::Unknown,
		};

		// Check if label is an octet
		if token == Token::Unknown {
			token = match self.octet_re.captures(&label) {
				Some(caps) => {
					if let Ok(octet) = caps.get(0).unwrap().as_str().parse::<u16>() {
						if octet > 255 {
							Token::Unknown
						} else {
							Token::Octet(octet as u8)
						}
					} else {
						Token::Unknown
					}
				}
				None => Token::Unknown,
			}
		}

		// Check if label is a hexblock
		if token == Token::Unknown {
			token = match self.hexblock_re.captures(&label) {
				Some(caps) => {
					if let Ok(hexblock) = u16::from_str_radix(caps.get(0).unwrap().as_str(), 16) {
						Token::HexBlock(hexblock)
					} else {
						Token::Unknown
					}
				}
				None => Token::Unknown,
			}
		}

		// Check if label is a short hexblock
		if token == Token::Unknown {
			token = match self.short_hexblock_re.captures(&label) {
				Some(caps) => {
					if let Ok(hexblock) = u16::from_str_radix(&caps.get(1).unwrap().as_str(), 16) {
						Token::HexBlock(hexblock)
					} else {
						Token::Unknown
					}
				}
				None => Token::Unknown,
			}
		}

		self.tokens.push(token);
	}

	pub fn print_tokens(&self) {
		for token in &self.tokens {
			println!("{:?}", token);
		}
	}
}