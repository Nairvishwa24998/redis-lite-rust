use bytes::Buf;
use bytes::BytesMut;
use core::error;
use mio::net::TcpStream;
use mio::{Events, Interest, Poll, Token};
use std::collections::HashMap;
use std::io::ErrorKind::WouldBlock;
use std::io::Read;
use std::io::Write;
use std::str;

use mio::net::TcpListener;
// prefere this over std TcpListener because mio TcpListener is non blocking by default
// mio TcpListener is non blocking by default so we dont need to set it to non blocking
// Unlike std TcpListener which is blocking by default

use crate::client_connection::client_connection;
use crate::constants::{BUFFER_PER_POLL_CALL, DEFAULT_BUFFER_SIZE, SERVER_TOKEN};
use crate::deserializer::deserializer;
use crate::error_response::RespErrorResponse;
use crate::message_processing::command_handler;
use crate::serializer::serializer;
use crate::{
    constants::{REDIS_DEFAULT_PORT, REDIS_DEFAULT_URL},
    error_response::CustomErrorResponse,
    store::Store,
};

pub struct Server {
    // Fields for the server, such as configuration, state, etc.
    port: u16,
    store: Store,
    // Would be the listening socket which would simply keep listening for new connections
    listening_socket: TcpListener,
    token_counter: u64, // Counter to generate unique tokens for each client connection
    // We add BytesMut so we can use that as the client buffer
    client_connections: HashMap<Token, client_connection>,
}

// We are gonna assume the server is always gonna run on one the default Redis port 6379 and default url
// Two layers of error propagation. First layer is the default Error from TcpListener::bind, which we convert to our CustomErrorResponse using the From trait. Second layer is the Result type that we return from the setup function, which can be either Ok(Server) or Err(CustomErrorResponse).
// Second layer is of the result propagated upwards
impl Server {
    pub fn setup_server_instance() -> Result<Self, CustomErrorResponse> {
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
        self.client_connections.insert(
            Token(self.token_counter as usize),
            client_connection::new(stream),
        );
        self.token_counter += 1; // Increment the token counter after registering the socket
        Ok(())
    }

    fn register_listening_socket_with_poll(
        &mut self,
        poll: &mut Poll,
    ) -> Result<(), CustomErrorResponse> {
        poll.registry().register(
            &mut self.listening_socket,
            Token(SERVER_TOKEN),
            Interest::READABLE,
        )?;
        self.token_counter += 1; // Increment the token counter after registering the listening socket
        Ok(())
    }

    fn manage_listening_socket_events(&mut self, poll: &mut Poll) {
        loop {
            // Listening socket accepts whereas client socket read and write
            // mio sets to non blocking by default so we don't need to set separately
            match self.listening_socket.accept() {
                Ok((mut stream, addr)) => {
                    // Handle the new client connection, e.g., register it with the poll instance
                    println!("New client connected: {}", addr);
                    // we don't default register for both readable and writable coz
                    // mio is edge triggered and socket becomes writable almost immediately on connection
                    // send buffer may not have anything at that point, so wastes a run for nothing
                    if let Err(e) =
                        self.register_client_socket_with_poll(poll, stream, Interest::READABLE)
                    {
                        eprintln!("Error registering client {}: {:?}", addr, e);
                    }
                }
                // No more connections to accept, break the loop and continue with the next event
                Err(ref e) if e.kind() == WouldBlock => {
                    // No more connections to accept
                    break;
                }
                // Some other kind of error has happened, so we should log it and move forward, so one client connection failure doesn't derail all the others
                Err(e) => {
                    eprintln!("Error accepting connection: {:?}", e);
                    break;
                }
            }
        }
    }

    // token implement copies so no ownership issues
    fn manage_client_socket_events(&mut self, poll: &mut Poll, client_token: &Token) {
        // if let Some((stream, buffer))
        if let Some(client_connection) = self.client_connections.get_mut(client_token) {
            let (buffer, stream) = client_connection.get_recv_buffer_and_tcp_stream();
            // read has limitations only looks at len. Not the capacity. So emtpy buffer
            // with ample capacity would be treated as a full buffer and read would return 0
            // similarly it can potentially overrite existing data in the buffer so
            // spare buffer approach used
            let mut temp_buffer = [0u8; DEFAULT_BUFFER_SIZE];
            // Read data from the client socket into the buffer
            // We loop coz a single read call may not read all the data. So we read until its
            loop {
                // avoid read_buff for now coz its unstable
                match stream.read(&mut temp_buffer) {
                    // Connection closed by the client. Don't confuse with not having any bytes to read and
                    // blocking case. That is handled below. Read(0) is a valid case returned when client has closed connection
                    Ok(0) => {
                        println!("Client disconnected: {:?}", client_token);
                        // Not super mandatory, but good practice to degister manually
                        if let Err(e) = poll.registry().deregister(stream) {
                            eprintln!("Error deregistering client {:?}: {:?}", client_token, e);
                        }
                        self.client_connections.remove(client_token);
                        break;
                    }
                    Ok(n) => {
                        // Append the read data to the buffer
                        buffer.extend_from_slice(&temp_buffer[..n]);
                        println!(
                            "Read {} bytes from client {:?}: {:?}",
                            n,
                            client_token,
                            &temp_buffer[..n]
                        );
                    }
                    Err(ref e) if e.kind() == WouldBlock => {
                        // No more data to read at the moment
                        break;
                    }
                    // Catchall for any other kind of error
                    Err(e) => {
                        eprintln!("Error reading from client {:?}: {:?}", client_token, e);
                        if let Err(e) = poll.registry().deregister(stream) {
                            eprintln!("Error deregistering client {:?}: {:?}", client_token, e);
                        }
                        self.client_connections.remove(client_token);
                        break;
                    }
                }
            }
        }

        // Calling again to avoid rust ownership issues and Coz we don't want to call if client disconnected
        self.manage_deserializer_invocation_and_message_processing(poll, client_token);
    }

    fn manage_deserializer_invocation_and_message_processing(
        &mut self,
        poll: &mut Poll,
        client_token: &Token,
    ) {
        // To avoid double mutable borrow issues.
        let mut should_close = false;

        // Calling again to avoid rust ownership issues and Coz we don't want to call if client disconnected
        if let Some(client_connection) = self.client_connections.get_mut(client_token) {
            let (buffer, send_buffer, stream) =
                client_connection.get_recv_and_send_buffer_tcp_stream();

            // labelling to exit outer loop from inside inner one, as against just the inner one if left unnamed
            'deserialize: loop {
                // Call the deserializer function with the buffer
                match deserializer(buffer) {
                    Ok((value, new_cursor)) => {
                        // otherwise we don't move the cursor and keep desrializing the same chunk again and again
                        buffer.advance(new_cursor);
                        match command_handler(&value) {
                            // We can go into command execution in this branch
                            Ok(response) => {
                                // we serialize the response, add it to send buffer and send to client
                                // need loop coz write sends upto capacity and not necessarily all of it
                                serializer(&response, send_buffer);
                                loop {
                                    // write path can also return ok(0) if buffer is genuinely empty
                                    // in which case we can just break the loop so we don't have to check below
                                    // in stream.write()
                                    if send_buffer.is_empty() {
                                        if let Err(e) = poll.registry().reregister(
                                            stream,
                                            *client_token,
                                            Interest::READABLE,
                                        ) {
                                            eprintln!(
                                                "Error while removing writable interest for client {:?}: {:?}",
                                                client_token, e
                                            );
                                        }
                                        break;
                                    }
                                    match stream.write(&send_buffer) {
                                        // two cases - complete write or partial writes
                                        Ok(n) => {
                                            // write returning 0 means write attempt failed
                                            if n == 0 {
                                                if let Err(e) = poll.registry().deregister(stream) {
                                                    eprintln!(
                                                        "Error deregistering client {:?}: {:?}",
                                                        client_token, e
                                                    );
                                                }
                                                should_close = true;
                                                break 'deserialize;
                                            }
                                            // both remaining ok cases (successful partial and complete writes) we advance the buffer
                                            send_buffer.advance(n);
                                        }
                                        // send buffer is full so would block error/ would actually block if it weren't in non blocking mode
                                        // we need to signal the kernel to add more space to send buffer
                                        // coz blocking on write means send buffer is full and only kernel can modify it.
                                        Err(ref e) if e.kind() == WouldBlock => {
                                            // Socket is not ready for writing, wait for the next writable event
                                            eprintln!(
                                                "Client's send buffer is full {:?}: {:?}",
                                                client_token, e
                                            );
                                            if let Err(e) = poll.registry().reregister(
                                                stream,
                                                *client_token,
                                                Interest::READABLE | Interest::WRITABLE,
                                            ) {
                                                eprintln!(
                                                    "Error while setting interest to writable for client {:?}: {:?}",
                                                    client_token, e
                                                );
                                            }
                                            break;
                                        }
                                        Err(e) => {
                                            eprintln!(
                                                "Error writing to client {:?}: {:?}",
                                                client_token, e
                                            );
                                            if let Err(e) = poll.registry().deregister(stream) {
                                                eprintln!(
                                                    "Error deregistering client {:?}: {:?}",
                                                    client_token, e
                                                );
                                            }
                                            should_close = true;
                                            break 'deserialize;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Error processing command: {:?}", e);
                            }
                        }
                    }
                    // If its incomplete we break the loop coz possibly more to come
                    Err(RespErrorResponse::Incomplete) => {
                        // Incomplete data, wait for more data to arrive
                        break;
                    }
                    Err(e) => {
                        eprintln!(
                            "Error deserializing input from client {:?}: {:?}",
                            client_token, e
                        );
                        if let Err(e) = poll.registry().deregister(stream) {
                            eprintln!("Error deregistering client {:?}: {:?}", client_token, e);
                        }

                        should_close = true;
                        break;
                    }
                }
            }
        }

        if should_close {
            self.client_connections.remove(client_token);
        }
    }

    pub fn commence_connection(&mut self) -> Result<(), CustomErrorResponse> {
        let mut events = Events::with_capacity(BUFFER_PER_POLL_CALL);
        // Construct a new `Poll` handle as well as the `Events` we'll store into
        let mut poll = Poll::new()?;
        const SERVER: Token = Token(SERVER_TOKEN);
        // Can use the ? here coz if registration fails we can just propagate the error and exit the function
        self.register_listening_socket_with_poll(&mut poll)?;
        loop {
            // Poll for events and block indefinitely until we get an event. Hence timeout set to None
            // poll only returns a capacity number of ready events. So we need to put it in a loop so events don't get missed
            poll.poll(&mut events, None)?;
            for event in events.iter() {
                match event.token() {
                    // listening socket is having an event. new client has potential connection and we can connect it.
                    // loop to prevent missing events due to edge triggered nature of mio
                    SERVER => {
                        self.manage_listening_socket_events(&mut poll);
                    }
                    // We can potentially read from the client sockets now
                    client_token => {
                        self.manage_client_socket_events(&mut poll, &client_token);
                    }
                }
            }
        }
    }
}
