mod client_connection;
mod command_handling_registry;
mod constants;
mod deserializer;
mod error_response;
mod message_processing;
mod resp;
mod serializer;
mod server;
mod set_command_handling_registry;
mod store;
mod utils;
mod set_manager;

use server::Server;

fn main() {
    // Start the server
    if let Ok(mut server) = Server::setup_server_instance() {
        if let Err(e) = server.commence_connection() {
            eprintln!("Error starting server: {:?}", e);
        }
    } else {
        eprintln!("Failed to set up server instance.");
    }
}
