//! Specification: <https://specifications.freedesktop.org/mime-apps-spec/mime-apps-spec-latest.html>
//!
//! Reference implementation:
//! <https://cgit.freedesktop.org/xdg/xdg-utils/tree/scripts/xdg-utils-common.in>

mod utilities;

pub use utilities::*;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        use utilities::*;
        eprintln!("{:?}", query_default_app("image/jpeg"));
        eprintln!("{:?}", query_default_app("text/html"));
        eprintln!("{:?}", query_default_app("video/mp4"));
        eprintln!("{:?}", query_default_app("application/pdf"));
    }
}
