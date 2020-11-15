#![forbid(unsafe_code)]
#![warn(clippy::all)]
//! A simple, safe HTTP client.
//!
//! Ureq's first priority is being easy for you to use. It's great for
//! anyone who wants a low-overhead HTTP client that just gets the job done. Works
//! very well with HTTP APIs. Its features include cookies, JSON, HTTP proxies,
//! HTTPS, and charset decoding.
//!
//! Ureq is in pure Rust for safety and ease of understanding. It avoids using
//! `unsafe` directly. It uses blocking I/O instead of async I/O, because that keeps
//! the API simple and and keeps dependencies to a minimum. For TLS, ureq uses
//! [rustls].
//!
//! ## Usage
//!
//! In its simplest form, ureq looks like this:
//!
//! ```rust
//! # fn main() -> Result<(), ureq::Error> {
//! # ureq::is_test(true);
//! let body: String = ureq::get("http://example.com")
//!     .set("Accept", "text/html")
//!     .call()?
//!     .into_string()?;
//! # Ok(())
//! # }
//! ```
//!
//! For more involved tasks, you'll want to create an [Agent]. An Agent
//! holds a connection pool for reuse, and a cookie store if you use the
//! "cookies" feature. An Agent can be cheaply cloned due to an internal
//! [Arc](std::sync::Arc) and all clones of an Agent share state among each other. Creating
//! an Agent also allows setting options like the TLS configuration.
//!
//! ```no_run
//! # fn main() -> std::result::Result<(), ureq::Error> {
//! # ureq::is_test(true);
//!   use ureq::{Agent, AgentBuilder};
//!   use std::time::Duration;
//!
//!   let agent: Agent = ureq::AgentBuilder::new()
//!       .timeout_read(Duration::from_secs(5))
//!       .timeout_write(Duration::from_secs(5))
//!       .build();
//!   let body: String = agent.get("http://example.com/page")
//!       .call()?
//!       .into_string()?;
//!
//!   // Reuses the connection from previous request.
//!   let response: String = agent.put("http://example.com/upload")
//!       .set("Authorization", "example-token")
//!       .call()?
//!       .into_string()?;
//! # Ok(())
//! # }
//! ```
//!
//! Ureq supports sending and receiving json, if you enable the "json" feature:
//!
//! ```rust
//! # #[cfg(feature = "json")]
//! # fn main() -> std::result::Result<(), ureq::Error> {
//! # ureq::is_test(true);
//!   // Requires the `json` feature enabled.
//!   let resp: String = ureq::post("http://myapi.example.com/ingest")
//!       .set("X-My-Header", "Secret")
//!       .send_json(ureq::json!({
//!           "name": "martin",
//!           "rust": true
//!       }))?
//!       .into_string()?;
//! # Ok(())
//! # }
//! # #[cfg(not(feature = "json"))]
//! # fn main() {}
//! ```
//!
//! ## Features
//!
//! To enable a minimal dependency tree, some features are off by default.
//! You can control them when including ureq as a dependency.
//!
//! `ureq = { version = "*", features = ["json", "charset"] }`
//!
//! * `tls` enables https. This is enabled by default.
//! * `cookies` enables cookies.
//! * `json` enables [Response::into_json()] and [Request::send_json()] via serde_json.
//! * `charset` enables interpreting the charset part of the Content-Type header
//!    (e.g.  `Content-Type: text/plain; charset=iso-8859-1`). Without this, the
//!    library defaults to Rust's built in `utf-8`.
//!
//! # Plain requests
//!
//! Most standard methods (GET, POST, PUT etc), are supported as functions from the
//! top of the library ([get()], [post()], [put()], etc).
//!
//! These top level http method functions create a [Request] instance
//! which follows a build pattern. The builders are finished using:
//!
//! * [`.call()`][Request::call()] without a request body.
//! * [`.send()`][Request::send()] with a request body as [Read][std::io::Read] (chunked encoding support for non-known sized readers).
//! * [`.send_string()`][Request::send_string()] body as string.
//! * [`.send_bytes()`][Request::send_bytes()] body as bytes.
//! * [`.send_form()`][Request::send_form()] key-value pairs as application/x-www-form-urlencoded.
//!
//! # JSON
//!
//! By enabling the `ureq = { version = "*", features = ["json"] }` feature,
//! the library supports serde json.
//!
//! * [`request.send_json()`][Request::send_json()] send body as serde json.
//! * [`response.into_json()`][Response::into_json()] transform response to json.
//!
//! # Content-Length and Transfer-Encoding
//!
//! The library will send a Content-Length header on requests with bodies of
//! known size, in other words, those sent with
//! [`.send_string()`][Request::send_string()],
//! [`.send_bytes()`][Request::send_bytes()],
//! [`.send_form()`][Request::send_form()], or
//! [`.send_json()`][Request::send_json()]. If you send a
//! request body with [`.send()`][Request::send()],
//! which takes a [Read][std::io::Read] of unknown size, ureq will send Transfer-Encoding:
//! chunked, and encode the body accordingly. Bodyless requests
//! (GETs and HEADs) are sent with [`.call()`][Request::call()]
//! and ureq adds neither a Content-Length nor a Transfer-Encoding header.
//!
//! If you set your own Content-Length or Transfer-Encoding header before
//! sending the body, ureq will respect that header by not overriding it,
//! and by encoding the body or not, as indicated by the headers you set.
//!
//! ```
//! let resp = ureq::post("http://my-server.com/ingest")
//!     .set("Transfer-Encoding", "chunked")
//!     .send_string("Hello world");
//! ```
//!
//! # Character encoding
//!
//! By enabling the `ureq = { version = "*", features = ["charset"] }` feature,
//! the library supports sending/receiving other character sets than `utf-8`.
//!
//! For [`response.into_string()`][Response::into_string()] we read the
//! header `Content-Type: text/plain; charset=iso-8859-1` and if it contains a charset
//! specification, we try to decode the body using that encoding. In the absence of, or failing
//! to interpret the charset, we fall back on `utf-8`.
//!
//! Similarly when using [`request.send_string()`][Request::send_string()],
//! we first check if the user has set a `; charset=<whatwg charset>` and attempt
//! to encode the request body using that.
//!
//! ------------------------------------------------------------------------------
//!
//! Ureq is inspired by other great HTTP clients like
//! [superagent](http://visionmedia.github.io/superagent/) and
//! [the fetch API](https://developer.mozilla.org/en-US/docs/Web/API/Fetch_API).
//!
//! If ureq is not what you're looking for, check out these other Rust HTTP clients:
//! [surf](https://crates.io/crates/surf), [reqwest](https://crates.io/crates/reqwest),
//! [isahc](https://crates.io/crates/isahc), [attohttpc](https://crates.io/crates/attohttpc),
//! [actix-web](https://crates.io/crates/actix-web), and [hyper](https://crates.io/crates/hyper).
//!

mod agent;
mod body;
mod error;
mod header;
mod pool;
mod proxy;
mod request;
mod resolve;
mod response;
mod stream;
mod unit;

#[cfg(feature = "cookies")]
mod cookies;

#[cfg(feature = "json")]
pub use serde_json::json;

#[cfg(test)]
mod test;
#[doc(hidden)]
mod testserver;

pub use crate::agent::Agent;
pub use crate::agent::AgentBuilder;
pub use crate::error::Error;
pub use crate::header::Header;
pub use crate::proxy::Proxy;
pub use crate::request::Request;
pub use crate::resolve::Resolver;
pub use crate::response::Response;

// re-export
#[cfg(feature = "cookies")]
pub use cookie::Cookie;
#[cfg(feature = "json")]
pub use serde_json::{to_value as serde_to_value, Map as SerdeMap, Value as SerdeValue};

use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicBool, Ordering};

/// Creates an agent builder.
pub fn builder() -> AgentBuilder {
    AgentBuilder::new()
}

// is_test returns false so long as it has only ever been called with false.
// If it has ever been called with true, it will always return true after that.
// This is a public but hidden function used to allow doctests to use the test_agent.
// Note that we use this approach for doctests rather the #[cfg(test)], because
// doctests are run against a copy of the crate build without cfg(test) set.
// We also can't use #[cfg(doctest)] to do this, because cfg(doctest) is only set
// when collecting doctests, not when building the crate.
#[doc(hidden)]
pub fn is_test(is: bool) -> bool {
    static IS_TEST: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));
    if is {
        IS_TEST.store(true, Ordering::SeqCst);
    }
    let x = IS_TEST.load(Ordering::SeqCst);
    return x;
}

/// Agents are used to keep state between requests.
pub fn agent() -> Agent {
    #[cfg(not(test))]
    if is_test(false) {
        return testserver::test_agent();
    } else {
        return AgentBuilder::new().build();
    }
    #[cfg(test)]
    return testserver::test_agent();
}

/// Make a request setting the HTTP method via a string.
///
/// ```
/// # fn main() -> Result<(), ureq::Error> {
/// # ureq::is_test(true);
/// ureq::request("GET", "http://example.com").call()?;
/// # Ok(())
/// # }
/// ```
pub fn request(method: &str, path: &str) -> Request {
    agent().request(method, path)
}

/// Make a GET request.
pub fn get(path: &str) -> Request {
    request("GET", path)
}

/// Make a HEAD request.
pub fn head(path: &str) -> Request {
    request("HEAD", path)
}

/// Make a POST request.
pub fn post(path: &str) -> Request {
    request("POST", path)
}

/// Make a PUT request.
pub fn put(path: &str) -> Request {
    request("PUT", path)
}

/// Make a DELETE request.
pub fn delete(path: &str) -> Request {
    request("DELETE", path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connect_http_google() {
        let agent = Agent::new();

        let resp = agent.get("http://www.google.com/").call().unwrap();
        assert_eq!(
            "text/html; charset=ISO-8859-1",
            resp.header("content-type").unwrap()
        );
        assert_eq!("text/html", resp.content_type());
    }

    #[test]
    #[cfg(feature = "tls")]
    fn connect_https_google() {
        let agent = Agent::new();

        let resp = agent.get("https://www.google.com/").call().unwrap();
        assert_eq!(
            "text/html; charset=ISO-8859-1",
            resp.header("content-type").unwrap()
        );
        assert_eq!("text/html", resp.content_type());
    }

    #[test]
    #[cfg(feature = "tls")]
    fn connect_https_invalid_name() {
        let result = get("https://example.com{REQUEST_URI}/").call();
        assert!(matches!(result.unwrap_err(), Error::DnsFailed(_)));
    }
}
