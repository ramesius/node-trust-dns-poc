use std::{net::IpAddr, str::FromStr};

use neon::prelude::*;
use once_cell::sync::OnceCell;
use tokio::runtime::Runtime;
use trust_dns_resolver::{
    config::{ResolverConfig, ResolverOpts},
    error::ResolveError,
    lookup_ip::LookupIp,
    Name, TokioAsyncResolver,
};

fn runtime<'a, C: Context<'a>>(cx: &mut C) -> NeonResult<&'static Runtime> {
    static RUNTIME: OnceCell<Runtime> = OnceCell::new();
    RUNTIME.get_or_try_init(|| Runtime::new().or_else(|err| cx.throw_error(err.to_string())))
}

fn async_resolver<'a, C: Context<'a>>(cx: &mut C) -> NeonResult<&'static TokioAsyncResolver> {
    static ASYNC_RESOLVER: OnceCell<TokioAsyncResolver> = OnceCell::new();

    ASYNC_RESOLVER.get_or_try_init(|| {
        runtime(cx)?.block_on(async {
            TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default())
                .or_else(|err| cx.throw_error(err.to_string()))
        })
    })
}

fn build_lookup_address_response<'a, C: Context<'a>>(
    cx: &mut C,
    address: IpAddr,
) -> NeonResult<Handle<'a, JsObject>> {
    let js_lookup_address: Handle<JsObject> = cx.empty_object();
    match address {
        IpAddr::V4(v4) => {
            let js_address_string = cx.string(v4.to_string());
            js_lookup_address
                .set(cx, "address", js_address_string)
                .or_else(|err| cx.throw_error(err.to_string()))?;
            let js_addr_family = JsNumber::new(cx, 4);
            js_lookup_address
                .set(cx, "family", js_addr_family)
                .or_else(|err| cx.throw_error(err.to_string()))?;
        }
        IpAddr::V6(v6) => {
            let js_address_string = cx.string(v6.to_string());
            js_lookup_address
                .set(cx, "address", js_address_string)
                .or_else(|err| cx.throw_error(err.to_string()))?;
            let js_addr_family = JsNumber::new(cx, 6);
            js_lookup_address
                .set(cx, "family", js_addr_family)
                .or_else(|err| cx.throw_error(err.to_string()))?;
        }
    }

    return Ok(js_lookup_address);
}

fn async_lookup(mut cx: FunctionContext) -> JsResult<JsPromise> {
    let runtime = runtime(&mut cx)?;
    let channel = cx.channel();
    let async_resolver = async_resolver(&mut cx).expect("Unable to connect to resolver");

    let options: Option<Handle<JsValue>> = cx.argument_opt(1);

    let mut all = false;

    if options.is_some() {
        let options = options.unwrap().downcast::<JsObject, _>(&mut cx).unwrap();

        all = options
            .get_opt::<JsBoolean, _, _>(&mut cx, "all")?
            .map(|s| s.value(&mut cx))
            .unwrap_or(false);
    }

    let host = cx
        .argument::<JsString>(0)
        .or_else(|err| cx.throw_error(err.to_string()))?;
    let name =
        Name::from_str(&host.value(&mut cx)).or_else(|err| cx.throw_error(err.to_string()))?;

    let (deferred, promise) = cx.promise();

    runtime.spawn(async move {
        let lookup_ip_future = async_resolver.lookup_ip(name);
        let lookup_result = lookup_ip_future.await;

        if all {
            lookup_all(deferred, channel, lookup_result);
        } else {
            lookup_one(deferred, channel, lookup_result);
        }
    });

    return Ok(promise);
}

fn lookup_one(
    deferred: neon::types::Deferred,
    channel: Channel,
    lookup_result: Result<LookupIp, ResolveError>,
) {
    deferred.settle_with(
        &channel,
        move |mut cx| -> Result<Handle<JsValue>, neon::result::Throw> {
            match lookup_result {
                Err(err) => cx.throw_error(err.to_string()),
                Ok(lookup) => {
                    let ip_addr = lookup.iter().next();
                    if ip_addr.is_some() {
                        let js_lookup_address =
                            build_lookup_address_response(&mut cx, ip_addr.unwrap())?;
                        return Ok(js_lookup_address.as_value(&mut cx));
                    } else {
                        return Ok(cx.undefined().as_value(&mut cx));
                    }
                }
            }
        },
    );
}

fn lookup_all(
    deferred: neon::types::Deferred,
    channel: Channel,
    lookup_result: Result<LookupIp, ResolveError>,
) {
    deferred.settle_with(&channel, move |mut cx| match lookup_result {
        Err(err) => cx.throw_error(err.to_string()),
        Ok(lookup) => {
            let records: Vec<IpAddr> = lookup.iter().collect();
            let js_address_array = JsArray::new(&mut cx, records.len() as u32);
            for (i, ip_addr) in records.iter().enumerate() {
                let js_lookup_address = build_lookup_address_response(&mut cx, *ip_addr)?;
                js_address_array
                    .set(&mut cx, i as u32, js_lookup_address)
                    .or_else(|err| cx.throw_error(err.to_string()))?;
            }
            return Ok(js_address_array);
        }
    });
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("lookup", async_lookup)?;
    Ok(())
}
