use bytes::BytesMut;
use mio::net::TcpStream;

use crate::constants::DEFAULT_BUFFER_SIZE;

pub struct client_connection {
    tcp_stream: TcpStream,
    rec_buffer: BytesMut,
    send_buffer: BytesMut,
}

impl client_connection {
    pub fn new(tcp_stream: TcpStream) -> Self {
        client_connection {
            tcp_stream,
            // Note this doesn't initialize the array with that size but it does
            // ask the OS to set aside the said number of bytes so setting it too high could cause memory issues
            // 0 is fine coz when it needs more it gets reallocated anyway but keeping it at 4 as a reasonable middle ground so smaller request
            // don't need any reallocation
            rec_buffer: BytesMut::with_capacity(DEFAULT_BUFFER_SIZE),
            send_buffer: BytesMut::with_capacity(DEFAULT_BUFFER_SIZE),
        }
    }

    pub fn get_tcp_stream(&mut self) -> &mut TcpStream {
        &mut self.tcp_stream
    }

    pub fn get_rec_buffer(&mut self) -> &mut BytesMut {
        &mut self.rec_buffer
    }

    pub fn get_send_buffer(&mut self) -> &mut BytesMut {
        &mut self.send_buffer
    }

    pub fn set_send_buffer(&mut self, new_send_buffer: BytesMut) {
        self.send_buffer = new_send_buffer;
    }

    pub fn set_rec_buffer(&mut self, new_rec_buffer: BytesMut) {
        self.rec_buffer = new_rec_buffer;
    }

    pub fn set_tcp_stream(&mut self, new_tcp_stream: TcpStream) {
        self.tcp_stream = new_tcp_stream;
    }

    // just to prevent borrow checker issues when we need both of them at the same time
    pub fn get_recv_buffer_and_tcp_stream(&mut self) -> (&mut BytesMut, &mut TcpStream) {
        (&mut self.rec_buffer, &mut self.tcp_stream)
    }

    // just to prevent borrow checker issues when we need all three of them at the same time
    pub fn get_recv_and_send_buffer_tcp_stream(&mut self) -> (&mut BytesMut, &mut BytesMut, &mut TcpStream) {
        (&mut self.rec_buffer, &mut self.send_buffer, &mut self.tcp_stream)
    }
}
