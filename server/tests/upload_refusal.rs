//! A refused upload must still read the photo the client is sending. If the server
//! answers and closes with the body half-sent, nginx in front of it reports 502
//! instead of the server's own status (seen in the W3b CI rehearsal: a 2 MiB upload
//! without a session came back 502, not 401). A phone whose session had expired would
//! then retry forever instead of signing out.

use std::time::Duration;

use capsnap_server::{AppState, Config, serve_listener};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[tokio::test]
async fn a_refused_upload_reads_the_whole_photo_before_answering() {
    let dir = tempfile::tempdir().unwrap();
    let state = AppState::open(Config::for_dir(dir.path(), "https://guests.example"))
        .await
        .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(serve_listener(listener, state));

    let photo = vec![0u8; 8 * 1024 * 1024];
    let mut conn = TcpStream::connect(addr).await.unwrap();
    let head = format!(
        "POST /api/v1/media HTTP/1.1\r\nHost: capsnap.test\r\nContent-Type: image/jpeg\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n",
        photo.len()
    );
    conn.write_all(head.as_bytes()).await.unwrap();
    // Time for the server to refuse (no session) before the photo arrives, as a
    // proxy's buffered body arrives after the request head.
    tokio::time::sleep(Duration::from_millis(300)).await;
    conn.write_all(&photo)
        .await
        .expect("the server closed the connection before reading the photo");

    let mut response = String::new();
    conn.read_to_string(&mut response).await.unwrap();
    assert!(
        response.starts_with("HTTP/1.1 401"),
        "{}",
        &response[..response.len().min(200)]
    );
}
