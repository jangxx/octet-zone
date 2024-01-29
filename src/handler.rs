use std::str::FromStr;

use crate::Args;
use crate::parser::Parser;
use tracing::*;
use hickory_server::{
	authority::MessageResponseBuilder,
	proto::op::{Header, MessageType, OpCode},
	proto::rr::{IntoName, LowerName, Name, RData, Record, rdata::AAAA, rdata::TXT, rdata::A},
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
	root_ipv4: Option<std::net::Ipv4Addr>,
	root_ipv6: Option<std::net::Ipv6Addr>,
}

impl Handler {
    /// Create new handler from command-line options.
    pub fn from_options(args: &Args) -> Self {
		let domain = &args.domain;
        Handler {
			root_zone: LowerName::from(Name::from_str(domain).unwrap()),
			root_ipv4: match &args.root_ipv4 { Some(ipv4) => Some(std::net::Ipv4Addr::from_str(&ipv4).unwrap()), None => None },
			root_ipv6: match &args.root_ipv6 { Some(ipv6) => Some(std::net::Ipv6Addr::from_str(&ipv6).unwrap()), None => None },
		}
    }

	async fn handle_root_request<R: ResponseHandler>(
		&self,
		request: &Request,
		responder: &mut R,
	) -> Result<ResponseInfo, Error> {
		let builder = MessageResponseBuilder::from_message_request(request);
		let mut header = Header::response_from_request(request.header());
		header.set_authoritative(true);

		let mut records = Vec::new();

		if let Some(ipv4) = self.root_ipv4 {
			records.push(Record::from_rdata(request.query().name().into(), 3600, RData::A(A(ipv4))));
		}

		if let Some(ipv6) = self.root_ipv6 {
			records.push(Record::from_rdata(request.query().name().into(), 3600, RData::AAAA(AAAA(ipv6))));
		}

		let response = builder.build(header, records.iter(), &[], &[], &[]);
		return Ok(responder.send_response(response).await?);
	}

	async fn do_handle_request<R: ResponseHandler>(
		&self,
		request: &Request,
		responder: &mut R,
	) -> Result<ResponseInfo, Error> {
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
			return self.handle_root_request(request, responder).await;
		}

		let mut parser = Parser::new();

		for label in name.into_name().unwrap().iter() {
			parser.add_token_from_label(std::str::from_utf8(label).unwrap());
		}

		// parser.print_tokens();

		let address_result = parser.to_address();

		match address_result {
			Ok(address) => {
				let builder = MessageResponseBuilder::from_message_request(request);
				let mut header = Header::response_from_request(request.header());
				header.set_authoritative(true);

				let records = vec![Record::from_rdata(request.query().name().into(), 3600, RData::AAAA(AAAA(address)))];
				let response = builder.build(header, records.iter(), &[], &[], &[]);
        		Ok(responder.send_response(response).await?)
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
        match self.do_handle_request(request, &mut responder).await {
			Ok(info) => info,
			Err(e) => {
				let builder = MessageResponseBuilder::from_message_request(request);
				let mut header = Header::response_from_request(request.header());
				header.set_authoritative(true);

				let response_str = match e {
					Error::InvalidAddress => "Invalid address",
					Error::NotEnoughOctets => "Not enough octets",
					_ => "Unknown error",
				};

				let rdata = RData::TXT(TXT::new(vec![response_str.to_string()]));
				let records = vec![Record::from_rdata(request.query().name().into(), 60, rdata)];
				let response = builder.build(header, records.iter(), &[], &[], &[]);
				responder.send_response(response).await.unwrap()
			}
		}
    }
}