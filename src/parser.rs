use regex::Regex;
use std::{net::Ipv6Addr, u16};

use crate::handler::Error;

#[derive(PartialEq, Debug)]
pub enum Token {
	Octet(u8),
	Filler,
	HexBlock(u16),
	MappedModifier,
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
			"map" => Token::MappedModifier,
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

	// pub fn print_tokens(&self) {
	// 	for token in &self.tokens {
	// 		println!("{:?}", token);
	// 	}
	// }

	pub fn to_address(&self) -> Result<Ipv6Addr, Error> {
		let mut total_octets: i8 = 0;
		let mut filler_start: i8 = -1;
		let mut octets: [u8; 16] = [0; 16];
		let mut is_valid = true;
		let mut last_was_octet = false;

		for token in &self.tokens {
			match token {
				Token::Octet(octet) => {
					if (total_octets % 2 == 1 && !last_was_octet) || total_octets == 16 {
						is_valid = false;
						break;
					}

					octets[total_octets as usize] = *octet;
					total_octets += 1;
					last_was_octet = total_octets % 2 == 1;
				},
				Token::Filler => {
					if filler_start != -1 || total_octets == 16 { // can only have one filler in the address
						is_valid = false;
						break;
					}
					filler_start = total_octets;
				},
				Token::HexBlock(hexblock) => {
					if total_octets % 2 != 0 || total_octets == 16 {
						is_valid = false;
						break;
					}

					octets[total_octets as usize] = (*hexblock >> 8) as u8;
					octets[(total_octets + 1) as usize] = (*hexblock & 0xFF) as u8;
					total_octets += 2;
				},
				Token::MappedModifier => {
					if total_octets != 0 || filler_start != -1 { // needs to be at the start
						is_valid = false;
						break;
					}

					total_octets = 12;
					octets[10] = 0xFF;
					octets[11] = 0xFF;
				},
				Token::LocalModifier => {
					if total_octets != 0 || filler_start != -1 { // needs to be at the start
						is_valid = false;
						break;
					}

					total_octets = 8;
					octets[0] = 0xFE;
					octets[1] = 0x80;
				}
				Token::Unknown => {
					if total_octets != 16 && filler_start == -1 { // needs to be at the end
						is_valid = false;
						break;
					}
				}
			}
		}

		if !is_valid || last_was_octet {
			return Err(Error::InvalidAddress);
		}
		if total_octets != 16 && filler_start == -1 {
			return Err(Error::NotEnoughOctets);
		}

		if filler_start == -1 {
			return Ok(Ipv6Addr::from(octets));
		} else {
			let mut address: [u8; 16] = [0; 16];

			address[..filler_start as usize].copy_from_slice(&octets[..filler_start as usize]);
			address[(16 - total_octets + filler_start) as usize..].copy_from_slice(&octets[filler_start as usize..total_octets as usize]);

			return Ok(Ipv6Addr::from(address));
		}
	}
}