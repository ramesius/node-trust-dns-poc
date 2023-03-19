use std::str::FromStr;

use napi::bindgen_prelude::*;
use napi_derive::napi;
use once_cell::sync::OnceCell;
use trust_dns_resolver::{
    config::{ResolverConfig, ResolverOpts},
    proto::rr::RecordType,
    Name, TokioAsyncResolver,
};

#[napi(object)]
#[doc = "DNS response to a DNS request."]
pub struct AddressResponse {
    pub address: String,
    pub family: i32,
}

#[doc = "Setup an async DNS resolver. Wraps the resolver in a OnceCell for subsequent access."]
fn async_resolver<'a>() -> Result<&'static TokioAsyncResolver> {
    static ASYNC_RESOLVER: OnceCell<TokioAsyncResolver> = OnceCell::new();

    ASYNC_RESOLVER.get_or_try_init(|| {
        TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default())
            .or_else(|err| Err(Error::new(Status::GenericFailure, err.to_string())))
    })
}

#[napi]
#[doc = "Perform a DNS lookup for the hostname, returning addresses in the request family."]
async fn lookup(hostname: String, family: Option<i32>) -> Result<AddressResponse> {
    let async_resolver = async_resolver().expect("Unable to connect to resolver");

    // Prepare the hostname String for lookup
    let name = Name::from_str(&hostname)
        .or_else(|err| Err(Error::new(Status::InvalidArg, err.to_string())))?;
    // Convert family int into RecordType
    let record_type = match family {
        Some(6) => RecordType::AAAA,
        _ => RecordType::A,
    };

    // Lookup the hostname and immediately wait for the DNS response.
    let lookup_ip_future = async_resolver.lookup(name, record_type);
    let lookup_result = lookup_ip_future.await;

    // Prepare the response to javascript
    match lookup_result {
        Ok(lookup_ip) => {
            let ip_addr = lookup_ip.iter().next().unwrap();
            Ok(AddressResponse {
                address: ip_addr.to_string(),
                family: match ip_addr.to_record_type() {
                    RecordType::A => 4,
                    RecordType::AAAA => 6,
                    _ => unreachable!("No other record type is supported"),
                },
            })
        }
        Err(err) => Err(Error::new(Status::GenericFailure, err.to_string())),
    }
}
