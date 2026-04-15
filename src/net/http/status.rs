// http status: Go's http.StatusXxx constants.

use crate::types::int;

pub const StatusOK: int = 200;
pub const StatusCreated: int = 201;
pub const StatusAccepted: int = 202;
pub const StatusNoContent: int = 204;
pub const StatusPartialContent: int = 206;

pub const StatusMovedPermanently: int = 301;
pub const StatusFound: int = 302;
pub const StatusSeeOther: int = 303;
pub const StatusNotModified: int = 304;
pub const StatusTemporaryRedirect: int = 307;
pub const StatusPermanentRedirect: int = 308;

pub const StatusBadRequest: int = 400;
pub const StatusUnauthorized: int = 401;
pub const StatusForbidden: int = 403;
pub const StatusNotFound: int = 404;
pub const StatusMethodNotAllowed: int = 405;
pub const StatusConflict: int = 409;
pub const StatusGone: int = 410;
pub const StatusTeapot: int = 418;
pub const StatusTooManyRequests: int = 429;

pub const StatusInternalServerError: int = 500;
pub const StatusNotImplemented: int = 501;
pub const StatusBadGateway: int = 502;
pub const StatusServiceUnavailable: int = 503;
pub const StatusGatewayTimeout: int = 504;

/// `http.StatusText(code)` — returns the English text for the status code,
/// matching Go's implementation.
#[allow(non_snake_case)]
pub fn StatusText(code: int) -> crate::types::string {
    let s = match code {
        100 => "Continue",
        101 => "Switching Protocols",
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        204 => "No Content",
        206 => "Partial Content",
        301 => "Moved Permanently",
        302 => "Found",
        303 => "See Other",
        304 => "Not Modified",
        307 => "Temporary Redirect",
        308 => "Permanent Redirect",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        410 => "Gone",
        418 => "I'm a teapot",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "",
    };
    s.to_owned()
}
