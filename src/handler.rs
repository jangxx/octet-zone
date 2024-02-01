use std::{collections::HashMap, str::FromStr};

use crate::Args;
use crate::parser::Parser;
use tracing::*;
use hickory_server::{
	authority::MessageResponseBuilder,
	proto::op::{Header, MessageType, OpCode},
	proto::rr::{IntoName, LowerName, Name, RData, Record, rdata::AAAA, rdata::{txt, TXT}, rdata::A},
	server::{Request, RequestHandler, ResponseHandler, ResponseInfo}
};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Invalid OpCode {0:}")]
    InvalidOpCode(OpCode),
    #[error("Invalid MessageType {0:}")]
    InvalidMessageType(MessageType),
    #[error("Invalid Zone {0:}")]
    InvalidZone(LowerName),
    #[error("IO error: {0:}")]
    Io(#[from] std::io::Error),
	#[error("Invalid address")]
	InvalidAddress,
	#[error("Not enough octets")]
	NotEnoughOctets,
}

/// DNS Request Handler
#[derive(Clone, Debug)]
pub struct Handler {
	root_zone: LowerName,
	root_ipv4: Vec<std::net::Ipv4Addr>,
	root_ipv6: Vec<std::net::Ipv6Addr>,
	txt_records: HashMap<LowerName, Vec<Record>>,
}

impl Handler {
    /// Create new handler from command-line options.
    pub fn from_options(args: &Args) -> Self {
		let domain = &args.domain;
		let mut txt_records = HashMap::new();

		match &args.additional_txt {
			Some(additional_txt) => {
				for txt_def in additional_txt {
					let mut parts = txt_def.splitn(2, '=');

					let name = parts.next().unwrap();
					let value = parts.next().unwrap();

					let name = LowerName::from_str(name).unwrap();

					let txt = txt::TXT::new(vec![value.to_string()]);

					let record = Record::from_rdata(name.clone().into(), 3600, RData::TXT(txt));

					let records = txt_records.entry(name).or_insert(vec![]);

					records.push(record);
				}
			},
			_ => {}
		}

        Handler {
			root_zone: LowerName::from(Name::from_str(domain).unwrap()),
			root_ipv4: match &args.root_ipv4 {
				Some(ipv4_addrs) => ipv4_addrs.into_iter().map(|addr| std::net::Ipv4Addr::from_str(&addr).unwrap()).collect(),
				None => vec![]
			},
			root_ipv6: match &args.root_ipv6 {
				Some(ipv6_addrs) => ipv6_addrs.into_iter().map(|addr| std::net::Ipv6Addr::from_str(&addr).unwrap()).collect(),
				None => vec![]
			},
			txt_records: txt_records,
		}
    }

	fn handle_root_request(
		&self,
		request: &Request,
	) -> Result<Vec<Record>, Error> {
		let mut records = Vec::new();

		for ipv4 in &self.root_ipv4 {
			records.push(Record::from_rdata(request.query().name().into(), 3600, RData::A(A(*ipv4))));
		}

		for ipv6 in &self.root_ipv6 {
			records.push(Record::from_rdata(request.query().name().into(), 3600, RData::AAAA(AAAA(*ipv6))));
		}

		if let Some(txt_records) = self.txt_records.get(request.query().name()) {
			for txt_record in txt_records {
				records.push(txt_record.clone());
			}
		}

		return Ok(records);
	}

	fn do_handle_request(
		&self,
		request: &Request,
	) -> Result<Vec<Record>, Error> {
		if request.op_code() != OpCode::Query {
			return Err(Error::InvalidOpCode(request.op_code()));
		}

		if request.message_type() != MessageType::Query {
			return Err(Error::InvalidMessageType(request.message_type()));
		}

		let name = request.query().name();

		if !self.root_zone.zone_of(name) {
			return Err(Error::InvalidZone(name.clone()));
		}

		if self.root_zone.zone_of(name) && self.root_zone.num_labels() == name.num_labels() {
			return self.handle_root_request(request);
		}

		if let Some(records) = self.txt_records.get(name) {
			return Ok(records.clone());
		}

		let mut parser = Parser::new();

		for label in name.into_name().unwrap().iter() {
			parser.add_token_from_label(std::str::from_utf8(label).unwrap());
		}

		// parser.print_tokens();

		let address_result = parser.to_address();

		match address_result {
			Ok(address) => {
				let records = vec![Record::from_rdata(request.query().name().into(), 3600, RData::AAAA(AAAA(address)))];
        		Ok(records)
			}
			Err(e) => {
				Err(e)
			}
		}
	}
}

#[async_trait::async_trait]
impl RequestHandler for Handler {
    async fn handle_request<R: ResponseHandler>(
        &self,
        request: &Request,
        mut responder: R,
    ) -> ResponseInfo {
        let result = self.do_handle_request(request);

		let records = match result {
			Ok(records) => records,
			Err(e) => {
				let response_str = match e {
					Error::InvalidAddress => "Invalid address",
					Error::NotEnoughOctets => "Not enough octets",
					_ => "Unknown error",
				};

				vec![Record::from_rdata(request.query().name().into(), 60, RData::TXT(TXT::new(vec![response_str.to_string()])))]
			}
		};

		let builder = MessageResponseBuilder::from_message_request(request);
		let mut header = Header::response_from_request(request.header());
		header.set_authoritative(true);

		// only respond with the records that match the query type
		let response = builder.build(header, records.iter().filter(|record| {
			record.record_type() == request.query().query_type()
		}), &[], &[], &[]);

		responder.send_response(response).await.unwrap()
    }
}