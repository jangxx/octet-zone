use std::{io, str::FromStr, io::Write};

use crate::Args;
use crate::parser::Parser;
use tracing::*;
use hickory_server::{
	server::{Request, RequestHandler, ResponseHandler, ResponseInfo},
	proto::op::{Header, ResponseCode, MessageType, OpCode},
	proto::rr::{LowerName, Name, IntoName},
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
	#[error("Whatever")]
	Whatever,
}

/// DNS Request Handler
#[derive(Clone, Debug)]
pub struct Handler {
	root_zone: LowerName,
}

impl Handler {
    /// Create new handler from command-line options.
    pub fn from_options(args: &Args) -> Self {
		let domain = &args.domain;
        Handler {
			root_zone: LowerName::from(Name::from_str(domain).unwrap()),
		}
    }

	async fn do_handle_request<R: ResponseHandler>(
		&self,
		request: &Request,
		response: R,
	) -> Result<ResponseInfo, Error> {
		if request.op_code() != OpCode::Query {
			return Err(Error::InvalidOpCode(request.op_code()));
		}

		if request.message_type() != MessageType::Query {
			return Err(Error::InvalidMessageType(request.message_type()));
		}

		let name = request.query().name();

		// println!("name: {}", name);

		if !self.root_zone.zone_of(name) {
			return Err(Error::InvalidZone(name.clone()));
		}

		let mut parser = Parser::new();

		for label in name.into_name().unwrap().iter() {
			// print!("label: {}", label);
			// io::stdout().write(label).unwrap();
			// println!();

			parser.add_token_from_label(std::str::from_utf8(label).unwrap());
		}

		parser.print_tokens();

		return Err(Error::Whatever);
	}
}

#[async_trait::async_trait]
impl RequestHandler for Handler {
    async fn handle_request<R: ResponseHandler>(
        &self,
        _request: &Request,
        _response: R,
    ) -> ResponseInfo {
        match self.do_handle_request(_request, _response).await {
			Ok(info) => info,
			Err(e) => {
				tracing::error!("Error handling request: {}", e);
				let mut header = Header::new();
				header.set_response_code(ResponseCode::ServFail);
				header.into()
			}
		}
    }
}