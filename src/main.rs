use std::ops::Not;
use std::io::{Read, Write};
use std::collections::HashMap;

#[macro_use] extern crate rouille;
#[macro_use] extern crate serde_derive;

use multipart::server::save::BufReader;
use multipart::server::Multipart;

#[derive(Serialize)]
struct FileInfo {
    name: String,
    size: f32,
}

fn main() {
    let upload_path = std::env::temp_dir()
        .join("uploat")
        .to_str()
        .map(|i| i.to_string())
        .unwrap();
    let upload_path = &*String::leak(upload_path);

    std::path::Path::new(&upload_path)
        .exists()
        .not()
        .then(|| std::fs::create_dir(upload_path));

    println!("Listening on 0.0.0.0:8888");
    rouille::start_server("0.0.0.0:8888", move |request| {
        router! {
            request,

            (GET) (/) => {
                rouille::Response::html(include_str!("../res/index.html"))
            },

            (GET) (/uploaded) => {
            let dir = std::fs::read_dir(upload_path).unwrap();
            let x = dir
                .into_iter()
                .map(|i| {
                    let filename = i
                        .as_ref()
                        .unwrap()
                        .file_name()
                        .as_os_str()
                        .to_str()
                        .unwrap()
                        .to_string();
                    let megabytes = i.unwrap().metadata().unwrap().size() as f32 / 1024.0 / 1024.0;

                    FileInfo {
                        name: filename.split_once(":").unwrap().1.to_owned(),
                        size: megabytes,
                    }
                })
                .collect::<Vec<_>>();

                rouille::Response::json(&x)
            },

            (DELETE) (/delete/{file: String}) => {
                let dir = std::fs::read_dir(upload_path).unwrap();
                let (actual_path, _) = dir
                .into_iter()
                .map(|i| {
                    let actual_name = i
                        .unwrap()
                        .file_name()
                        .as_os_str()
                        .to_str()
                        .unwrap()
                        .to_string();


                    let filename = actual_name.split_once(":").unwrap().1.to_owned();
                    (actual_name, filename)
                })
                .find(|(_, filename)| *filename == file)
                .unwrap();

                std::fs::remove_file(format!("{upload_path}/{actual_path}")).expect("Failed to remove file");
                rouille::Response::text("success")
            },

            (GET) (/download/{file: String}) => {
                let dir = std::fs::read_dir(upload_path).unwrap();
                    let (actual_name, mime_type, _) = dir
                    .into_iter()
                    .map(|i| {
                        let actual_name = i
                            .unwrap()
                            .file_name()
                            .as_os_str()
                            .to_str()
                            .unwrap()
                            .to_string();

                        let binding = actual_name.clone();
                        let (mime_type, filename) = binding.split_once(":").unwrap();
                        let mime_type = mime_type.replace("_", "/");

                        (actual_name, mime_type, filename.to_owned())
                    })
                    .find(|(_, _, filename)| *filename == file)
                    .unwrap();

                let contents = std::fs::File::open(format!("{upload_path}/{actual_name}")).expect("Failed to open index.html file");
                rouille::Response::from_file(mime_type, contents)
            },

            (POST) (/upload) => {
                let headers: HashMap<String, String> = HashMap::from_iter(request.headers().map(|(k,v)| (k.to_string(), v.to_string())));
                let content_len = &headers["Content-Length"];

                // Log the content length (for debugging purposes)
                println!("Content-Length: {}", content_len);

                let boundary = headers.get("Content-Type").and_then(|ct| ct.split("boundary=").nth(1))
                .unwrap_or("");

                // // Parse the multipart form data
                let mut mp = Multipart::with_body(request.data().unwrap(), boundary);

                let mut file_content = Vec::new();
                let mut mime_type = String::new();
                let mut file_name = String::new();

                mp.foreach_entry(|field| {
                    if &*field.headers.name == "file" {
                        mime_type = field
                            .headers
                            .content_type
                            .map(|ct| ct.to_string()) // Extract MIME type from the field
                            .unwrap_or_else(|| "application/octet-stream".to_string()); // Fallback MIME type

                        file_name = field.headers.filename.unwrap();

                        let mut bufreader = BufReader::new(field.data);
                        bufreader.read_to_end(&mut file_content).unwrap();
                    }
                })
                .unwrap();

                // Respond with the received data
                let response_body = format!(
                    "Received file: {file_name}\n with MIME type: {mime_type}\nFile size: {} bytes",
                    file_content.len()
                );

                println!("{response_body}");

                // save the file to `uploaded` directory
                let mut new_file = std::fs::File::create(format!(
                    "{upload_path}/{}:{file_name}",
                    mime_type.replace("/", "_")
                ))
                .expect("Failed to create file");
                new_file
                    .write_all(&file_content) .expect("Failed to write contents");

                rouille::Response::text("success")
            },

            _ => rouille::Response::empty_404()
        }
    });
}
