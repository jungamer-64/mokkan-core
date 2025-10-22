use std::time::SystemTime;

fn main() {
    let now = httpdate::fmt_http_date(SystemTime::now());
    println!("cargo:rustc-env=BUILD_DATE={now}");
}
