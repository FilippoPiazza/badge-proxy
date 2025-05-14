use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{Method, Request, Response, StatusCode, header};
use hyper::body::Incoming;
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::sync::RwLock;

// Async function to read the URL
async fn read_url(shared_url: Arc<RwLock<Option<String>>>) -> Option<String> {
    // Get read lock (shared access with other readers)
    let url = shared_url.read().await;
    
    // Clone the string to return it (avoid holding the lock longer than needed)
    let result = url.clone();
    
    // Lock is dropped here when url goes out of scope
    
    // Return the URL
    result
}

// Async function to write/update the URL
async fn write_url(shared_url: Arc<RwLock<Option<String>>>, new_url: String) {
    // Get write lock (exclusive access)
    let mut url = shared_url.write().await;
    
    // Update the URL
    *url = Some(new_url);
    
    // Lock is dropped here when url goes out of scope
}

// Helper function to create a full body response
fn full<T: Into<Bytes>>(body: T) -> Full<Bytes> {
    Full::new(body.into())
}

// HTTP request handler
async fn handle_request(
    req: Request<Incoming>,
    shared_url: Arc<RwLock<Option<String>>>,
    update_password: Arc<Option<String>>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    match (req.method(), req.uri().path()) {
        // GET / - Proxy to the URL if set, otherwise return an error
        (&Method::GET, "/") => {
            match read_url(shared_url).await {
                Some(url) => {
                    // Proxy to the URL
                    match proxy_request(&url).await {
                        Ok(proxy_response) => Ok(proxy_response),
                        Err(e) => {
                            // Error occurred during proxying
                            let response = Response::builder()
                                .status(StatusCode::BAD_GATEWAY)
                                .body(full(format!("Error proxying request: {}", e)))
                                .unwrap();
                            Ok(response)
                        }
                    }
                },
                None => {
                    // No URL is set, return an error
                    let response = Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body(full("No URL has been set"))
                        .unwrap();
                    Ok(response)
                }
            }
        },
        
        // POST /url - Update the URL with the request body (keeping /url for updates)
        (&Method::POST, "/url") | (&Method::POST, "/") => {
            // Check if update password is set
            if let Some(required_password) = update_password.as_ref() {
                // Password is set, so check for authorization
                let auth_header = req.headers().get(header::AUTHORIZATION);
                let is_authorized = match auth_header {
                    Some(header_value) => {
                        if let Ok(auth_str) = header_value.to_str() {
                            // Check if the header starts with "Bearer " and the rest matches our password
                            if auth_str.starts_with("Bearer ") {
                                let provided_password = &auth_str[7..]; // Skip "Bearer " prefix
                                provided_password == required_password
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    },
                    None => false
                };
                
                // If not authorized, return 401 Unauthorized
                if !is_authorized {
                    return Ok(Response::builder()
                        .status(StatusCode::UNAUTHORIZED)
                        .header(header::WWW_AUTHENTICATE, "Bearer")
                        .body(full("Unauthorized: Valid password required to update URL"))
                        .unwrap());
                }
            }
            // If no password is set or authorization passed, proceed with the update
            
            // Read the request body
            let body_bytes = match req.collect().await {
                Ok(collected) => collected.to_bytes(),
                Err(_) => {
                    return Ok(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(full("Failed to read request body"))
                        .unwrap());
                }
            };
            
            // Convert bytes to string
            let new_url = match String::from_utf8(body_bytes.to_vec()) {
                Ok(s) => s,
                Err(_) => {
                    return Ok(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(full("Request body is not valid UTF-8"))
                        .unwrap());
                }
            };
            
            // Update the URL
            write_url(shared_url, new_url).await;
            
            // Return success response
            let response = Response::builder()
                .status(StatusCode::OK)
                .body(full("URL updated successfully"))
                .unwrap();
            Ok(response)
        },
        
        // All other routes - Return 404 Not Found
        _ => {
            let response = Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(full("Not Found"))
                .unwrap();
            Ok(response)
        }
    }
}

// Function to proxy a request to the target URL (assuming it's a shields.io badge image)
async fn proxy_request(url: &str) -> Result<Response<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync>> {
    // Use reqwest to fetch the image
    let client = reqwest::Client::new();
    let resp = client.get(url).send().await?;
    
    // Get the image data as bytes
    let image_bytes = resp.bytes().await?;
    
    // Create a response with the image data
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "image/svg+xml")
        .header(header::CACHE_CONTROL, "no-cache, no-store, must-revalidate")
        .header(header::PRAGMA, "no-cache")
        .header(header::EXPIRES, "0")
        .body(full(image_bytes))?;
    
    Ok(response)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Read the update password from environment variable
    let update_password = std::env::var("URL_UPDATE_PASSWORD").ok();
    
    // Read the default URL from environment variable
    let default_url = std::env::var("DEFAULT_URL").ok();
    
    // Create a shared URL wrapped in Arc<RwLock<T>> - initialize with default URL if available
    let shared_url = Arc::new(RwLock::new(default_url));
    
    // Log startup information
    if let Some(ref url) = *shared_url.read().await {
        println!("Server started with default URL: {}", url);
    } else {
        println!("Server started with no default URL");
    }
    
    if update_password.is_some() {
        println!("URL update password is set - authentication required for updates");
    } else {
        println!("No URL update password set - any update will be accepted");
    }
    
    // Create a clone of the password for the request handler
    let update_password = Arc::new(update_password);
    
    // Set up the server address
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    
    // Create a TCP listener
    let listener = TcpListener::bind(addr).await?;
    println!("Server listening on {}", addr);
    
    // Accept and process incoming connections
    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        
        // Clone the shared URL and password for this connection
        let url_clone = Arc::clone(&shared_url);
        let password_clone = Arc::clone(&update_password);
        
        // Spawn a new task to handle this connection
        tokio::spawn(async move {
            // Create a service function that will handle each request
            let service = hyper::service::service_fn(move |req| {
                let url_clone = Arc::clone(&url_clone);
                let password_clone = Arc::clone(&password_clone);
                handle_request(req, url_clone, password_clone)
            });
            
            // Process HTTP1 connections
            if let Err(err) = hyper::server::conn::http1::Builder::new()
                .serve_connection(io, service)
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}
