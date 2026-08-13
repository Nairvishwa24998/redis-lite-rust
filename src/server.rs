use mio::event::Source;
use mio::net::TcpStream;
use mio::{Events, Interest, Poll, Token};
use std::collections::HashMap;
use std::io::ErrorKind::WouldBlock;

use mio::net::TcpListener;
// mio TcpListener is non blocking by default so we dont need to set it to non blocking
// Unlike std TcpListener which is blocking by default

use crate::constants::{BUFFER_PER_POLL_CALL, SERVER_TOKEN};
use crate::{
    constants::{REDIS_DEFAULT_PORT, REDIS_DEFAULT_URL},
    error_response::CustomErrorResponse,
    store::Store,
};

struct Server {
    // Fields for the server, such as configuration, state, etc.
    port: u16,
    store: Store,
    // Would be the listening socket which would simply keep listening for new connections
    listening_socket: TcpListener,
    token_counter: u64, // Counter to generate unique tokens for each client connection
    client_connections: HashMap<Token, TcpStream>,
}

// We are gonna assume the server is always gonna run on one the default Redis port 6379 and default url
// Two layers of error propagation. First layer is the default Error from TcpListener::bind, which we convert to our CustomErrorResponse using the From trait. Second layer is the Result type that we return from the setup function, which can be either Ok(Server) or Err(CustomErrorResponse).
// Second layer is of the result propagated upwards
impl Server {
    fn setup_server_instance() -> Result<Self, CustomErrorResponse> {
        Ok(Self {
            port: REDIS_DEFAULT_PORT,
            store: Store::new(),
            // Bind the listening socket to default Redis URL and Port
            listening_socket: TcpListener::bind(REDIS_DEFAULT_URL.parse().unwrap())?,
            token_counter: SERVER_TOKEN as u64, // Initialize the token counter to 0,
            client_connections: HashMap::new(),
        })
    }

    fn register_client_socket_with_poll(
        &mut self,
        poll: &mut Poll,
        // dont forget to make stream mutable
        mut stream: TcpStream,
        interest: Interest,
    ) -> Result<(), CustomErrorResponse> {
        poll.registry()
            .register(&mut stream, Token(self.token_counter as usize), interest)?;
        self.client_connections
            .insert(Token(self.token_counter as usize), stream);
        self.token_counter += 1; // Increment the token counter after registering the socket
        Ok(())
    }

    fn commence_connection(&mut self) -> Result<(), CustomErrorResponse> {
        let mut events = Events::with_capacity(BUFFER_PER_POLL_CALL);
        // Construct a new `Poll` handle as well as the `Events` we'll store into
        let mut poll = Poll::new()?;
        const SERVER: Token = Token(SERVER_TOKEN);
        poll.registry()
            .register(&mut self.listening_socket, SERVER, Interest::READABLE)?;
        self.token_counter += 1; // Increment the token counter after registering the listening socket
        loop {
            // Poll for events and block indefinitely until we get an event. Hence timeout set to None
            poll.poll(&mut events, None)?;
            for event in events.iter() {
                match event.token() {
                    // This is the listening socket having an event. This means a new client has potential connection and we can connect it.
                    SERVER => loop {
                        match self.listening_socket.accept() {
                            Ok((mut stream, addr)) => {
                                // Handle the new client connection, e.g., register it with the poll instance
                                println!("New client connected: {}", addr);
                                self.register_client_socket_with_poll(
                                    &mut poll,
                                    stream,
                                    Interest::READABLE.add(Interest::WRITABLE),
                                )?;
                            }
                            Err(ref e) if e.kind() == WouldBlock => {
                                // No more connections to accept
                                break;
                            }
                            Err(e) => {
                                eprintln!("Error accepting connection: {}", e);
                                break;
                            }
                        }
                    },
                    // We can potentially read from the client sockets now
                    _clients => {}
                }
            }
        }
        Ok(())
    }
}
