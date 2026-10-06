use bytes::Buf;
use bytes::BytesMut;
use core::error;
use mio::net::TcpStream;
use mio::{Events, Interest, Poll, Token};
use std::collections::HashMap;
use std::io::ErrorKind::Interrupted;
use std::io::ErrorKind::WouldBlock;
use std::io::Read;
use std::io::Write;
use std::str;

use mio::net::TcpListener;
// prefere this over std TcpListener because mio TcpListener is non blocking by default
// mio TcpListener is non blocking by default so we dont need to set it to non blocking
// Unlike std TcpListener which is blocking by default

use crate::client_connection::client_connection;
use crate::constants::MAX_BUFFER_SIZE;
use crate::constants::MAX_SEND_BUFFER_SIZE;
use crate::constants::{BUFFER_PER_POLL_CALL, DEFAULT_BUFFER_SIZE, SERVER_TOKEN};
use crate::deserializer::deserializer;
use crate::error_response::RespErrorResponse;
use crate::message_processing::command_handler;
use crate::message_processing::map_error_to_resp_object;
use crate::serializer::serializer;
use crate::{
    constants::{REDIS_DEFAULT_PORT, REDIS_DEFAULT_URL},
    error_response::ServerError,
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
// Two layers of error propagation. First layer is the default Error from TcpListener::bind, which we convert to our ServerError using the From trait. Second layer is the Result type that we return from the setup function, which can be either Ok(Server) or Err(ServerError).
// Second layer is of the result propagated upwards
impl Server {
    pub fn setup_server_instance() -> Result<Self, ServerError> {
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
    ) -> Result<(), ServerError> {
        poll.registry()
            .register(&mut stream, Token(self.token_counter as usize), interest)?;
        self.client_connections.insert(
            Token(self.token_counter as usize),
            client_connection::new(stream, interest),
        );
        self.token_counter += 1; // Increment the token counter after registering the socket
        Ok(())
    }

    fn register_listening_socket_with_poll(&mut self, poll: &mut Poll) -> Result<(), ServerError> {
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
                Ok((stream, addr)) => {
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
                // Redis usually retries if interrupted
                Err(e) if e.kind() == Interrupted => {
                    // we try again coz got interrupted
                    continue;
                }
                // Some other kind of error has happened, so we should log it and move forward, so one client connection failure doesn't derail all the others
                Err(e) => {
                    eprintln!("Error accepting connection: {:?}", e);
                    break;
                }
            }
        }
    }

    fn manage_client_socket_readable_events(&mut self, poll: &mut Poll, client_token: &Token) {
        if let Some(client_connection) = self.client_connections.get_mut(client_token) {
            let (recv_buffer, stream) = client_connection.get_recv_buffer_and_tcp_stream();
            // read has limitations only looks at len. Not the capacity. So emtpy buffer
            // with ample capacity would be treated as a full buffer and read would return 0
            // similarly it can potentially overrite existing data in the buffer so
            // spare buffer approach used
            let mut temp_buffer = [0u8; DEFAULT_BUFFER_SIZE];
            // We loop coz a single read call may not read all the data. So we read until its
            loop {
                // Read data from the client socket into the buffer
                match stream.read(&mut temp_buffer) {
                    // reached EOF or client closed connection
                    Ok(0) => {
                        println!("Client disconnected: {:?}", client_token);
                        self.disconnect_client_helper(poll, client_token);
                        // return at the point of deregister for early exit and prevent flow into deserialization
                        return;
                    }
                    // Successfully read n bytes
                    Ok(n) => {
                        if n + recv_buffer.len() > MAX_BUFFER_SIZE {
                            eprintln!(
                                "Client {:?} exceeded maximum buffer size. Disconnecting.",
                                client_token
                            );
                            self.disconnect_client_helper(poll, client_token);
                            return;
                        }
                        // Append the read data to the buffer
                        recv_buffer.extend_from_slice(&temp_buffer[..n]);
                        println!(
                            "Read {} bytes from client {:?}: {:?}",
                            n,
                            client_token,
                            &temp_buffer[..n]
                        );
                    }
                    Err(e) if e.kind() == WouldBlock => {
                        // would block if not for non blocking mode due to no data to read at the momennt
                        // so break but don't deregister
                        break;
                    }
                    // Redis usually retries if interrupted
                    Err(e) if e.kind() == Interrupted => {
                        // we try again coz got interrupted
                        continue;
                    }
                    // Any other kind of error
                    Err(e) => {
                        eprintln!("Error reading from client {:?}: {:?}", client_token, e);
                        self.disconnect_client_helper(poll, client_token);
                        return;
                    }
                }
            }
            // Calling again to avoid rust ownership issues and Coz we don't want to call if client disconnected
            self.manage_deserializer_invocation_and_message_processing(poll, client_token);
            // if there is some data in the send buffer, we can send it directly
            // can also set to writable and let next poll handle but wasted system calls
            self.flush_send_buffer(poll, client_token);
        }
    }

    fn manage_client_socket_writable_events(&mut self, poll: &mut Poll, client_token: &Token) {
        self.flush_send_buffer(poll, client_token);
    }

    // renaming it so can be invoked once within read as well without messing up responsibility segregation
    fn flush_send_buffer(&mut self, poll: &mut Poll, client_token: &Token) {
        // Calling again to avoid rust ownership issues and Coz we don't want to call if client disconnected
        if let Some(client_connection) = self.client_connections.get_mut(client_token) {
            let (_recv_buffer, send_buffer, stream, interest) = client_connection.get_all();
            loop {
                // write path can also return ok(0) if buffer is genuinely empty
                // in which case we can just break the loop so we don't have to check below
                // in stream.write()
                if send_buffer.is_empty() {
                    // if send buffer is empty then we don't need interest in writable
                    // so reregistering purely as readable after checking to avoid unncessary sys calls
                    if interest.is_writable() {
                        if let Err(e) =
                            poll.registry()
                                .reregister(stream, *client_token, Interest::READABLE)
                        {
                            eprintln!(
                                "Error while removing writable interest for client {:?}: {:?}",
                                client_token, e
                            );
                        } else {
                            *interest = Interest::READABLE;
                        }
                    }

                    break;
                }
                // this will directly pick up from the send
                match stream.write(&send_buffer) {
                    // two cases - complete write or partial writes
                    Ok(n) => {
                        // write returning 0 means write attempt failed
                        if n == 0 {
                            self.disconnect_client_helper(poll, client_token);
                            // deregister failure is not dangerous.
                            // System would handle it, when we remove the corresponding client_connction, the associated stream also loses ownership
                            // in such case, Rust autotamtically closes and the OS would deregister the client
                            // so we can remove it even if our deregistry fails
                            return;
                        }
                        // both remaining ok cases (successful partial and complete writes) we advance the buffer
                        send_buffer.advance(n);
                    }
                    // send buffer is full so would block error/ would actually block if it weren't in non blocking mode
                    // we need to signal the kernel to add more space to send buffer
                    // coz blocking on write means send buffer is full and only kernel can modify it.
                    Err(e) if e.kind() == WouldBlock => {
                        // Socket is not ready for writing, wait for the next writable event
                        eprintln!("Client's send buffer is full {:?}: {:?}", client_token, e);
                        // just to avoid unncesseary syscalls, if its already only readable, just skip and break
                        if !interest.is_writable() {
                            if let Err(e) = poll.registry().reregister(
                                stream,
                                *client_token,
                                Interest::READABLE | Interest::WRITABLE,
                            ) {
                                eprintln!(
                                    "Error while setting interest to writable for client {:?}: {:?}",
                                    client_token, e
                                );
                            } else {
                                *interest = Interest::READABLE | Interest::WRITABLE;
                            }
                        }
                        break;
                    }
                    // Redis usually retries if interrupted
                    Err(e) if e.kind() == Interrupted => {
                        // we try again coz got interrupted
                        continue;
                    }
                    Err(e) => {
                        eprintln!("Error writing to client {:?}: {:?}", client_token, e);
                        self.disconnect_client_helper(poll, client_token);
                        return;
                    }
                }
            }
        }
    }

    fn manage_deserializer_invocation_and_message_processing(
        &mut self,
        poll: &mut Poll,
        client_token: &Token,
    ) {
        // Calling again to avoid rust ownership issues and Coz we don't want to call if client disconnected
        if let Some(client_connection) = self.client_connections.get_mut(client_token) {
            let (buffer, send_buffer) = client_connection.get_rcv_and_send_buffer();
            loop {
                match deserializer(buffer) {
                    Ok((value, new_cursor)) => {
                        // to move the cursor and prevent desrializing the same chunk again and again
                        buffer.advance(new_cursor);
                        // No double mutable borrow issue for self.store coz we are only mutably borrowing the attribute not the
                        // whole self. If that had been the case borrow checker would have flagged it as double mutable borrow.
                        match command_handler(&value, &mut self.store) {
                            // command execution branch
                            Ok(response) => {
                                // we serialize the response, add it to send buffer
                                serializer(&response, send_buffer);
                            }
                            Err(e) => {
                                eprintln!("Error processing command: {:?}", e);
                                // we serialize the error response, add it to send buffer
                                let error_response = map_error_to_resp_object(&e);
                                serializer(&error_response, send_buffer);
                            }
                        }
                        // Send buffer grows potetntially when serializer si called so we need to check for happy case followed
                        // by serialization as well error followed by serialization. So placed outside the match block
                        if send_buffer.len() > MAX_SEND_BUFFER_SIZE {
                            eprintln!(
                                "Client {:?} caused maximum send buffer size to exceed. Disconnecting.",
                                client_token
                            );
                            self.disconnect_client_helper(poll, client_token);
                            return;
                        }
                    }
                    // If its incomplete we break the loop coz possibly more to come
                    Err(RespErrorResponse::Incomplete) => {
                        // Incomplete data, wait for more data to arrive
                        break;
                    }
                    // Every other error we logged in deserializer leads to disconnecting the client
                    Err(e) => {
                        eprintln!(
                            "Error deserializing input from client {:?}: {:?}",
                            client_token, e
                        );
                        self.disconnect_client_helper(poll, client_token);
                        return;
                    }
                }
            }
        }
    }

    fn disconnect_client_helper(&mut self, poll: &mut Poll, client_token: &Token) {
        if let Some(mut client_connection) = self.client_connections.remove(client_token) {
            let stream = client_connection.get_tcp_stream_mut();
            if let Err(e) = poll.registry().deregister(stream) {
                eprintln!("Error deregistering client {:?}: {:?}", client_token, e);
            }
        } // 
    }

    pub fn commence_connection(&mut self) -> Result<(), ServerError> {
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
                    // We can potentially read or write from the client sockets now
                    client_token => {
                        if event.is_readable() {
                            self.manage_client_socket_readable_events(&mut poll, &client_token);
                        }
                        if event.is_writable() {
                            self.manage_client_socket_writable_events(&mut poll, &client_token);
                        }
                    }
                }
            }
        }
    }
}
