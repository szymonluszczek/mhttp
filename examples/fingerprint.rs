//! what tls.peet.ws sees for each profile
//!
//! ```text
//! cargo run --example fingerprint
//! ```

#[cfg(not(mhttp_engine))]
fn main() {
    eprintln!("built without the engine; see deps/info.txt");
}

#[cfg(mhttp_engine)]
#[tokio::main]
async fn main() -> mhttp::Result<()> {
    use mhttp::{Connection, HttpVersion, Profile, Request};

    for (name, profile) in [
        ("chrome", Profile::chrome()),
        ("safari", Profile::safari_ios()),
    ] {
        for version in [HttpVersion::Http2, HttpVersion::Http1] {
            let connection = Connection::builder("https://tls.peet.ws".parse().unwrap())
                .profile(&profile)
                .version(version)
                .connect()
                .await?;

            let request = Request::get("/api/all")
                .header("user-agent", "mhttp")
                .header("accept", "*/*")
                .build();
            let response = connection.send(request).await?;

            println!("== {name} {version:?}: {}", response.status());
            println!("{}", response.text());
        }
    }
    Ok(())
}
