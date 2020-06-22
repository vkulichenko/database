// Copyright 2020 Alex Dukhno
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::{
    messages::Message, Connection, Error, Params, Result, SslMode, VERSION_1, VERSION_2, VERSION_3, VERSION_CANCEL,
    VERSION_GSSENC, VERSION_SSL,
};
use async_trait::async_trait;
use byteorder::{ByteOrder, NetworkEndian};
use bytes::{Buf, BytesMut};
use futures::io::{self, AsyncReadExt, AsyncWriteExt};
use itertools::Itertools;
use std::net::SocketAddr;

#[async_trait]
pub trait QueryListener {
    type Socket: AsyncReadExt + AsyncWriteExt + Unpin + Send + Sync;
    type ServerSocket: ServerListener<Socket = Self::Socket> + Unpin + Send + Sync;

    #[allow(clippy::if_same_then_else)]
    async fn accept(&self) -> io::Result<Result<Connection<Self::Socket>>> {
        let (mut socket, address) = self.server_socket().tcp_connection().await?;
        log::debug!("ADDRESS {:?}", address);

        let len = read_len(&mut socket).await?;
        let mut message = read_message(len, &mut socket).await?;
        log::debug!("MESSAGE FOR TEST = {:#?}", message);
        let version = NetworkEndian::read_i32(message.bytes());
        log::debug!("VERSION FOR TEST = {:#?}", version);
        message.advance(4);

        if version == VERSION_3 {
            let parsed = message
                .bytes()
                .split(|b| *b == 0)
                .filter(|b| !b.is_empty())
                .map(|b| std::str::from_utf8(b).unwrap().to_owned())
                .tuples()
                .collect::<Params>();
            message.advance(message.remaining());
            log::debug!("Version {}\nparams = {:?}", version, parsed);
            socket.write_all(Message::AuthenticationOk.as_vec().as_slice()).await?;
            Ok(Ok(Connection::new((version, parsed, SslMode::Disable), socket)))
        } else if version == VERSION_SSL {
            if self.secure().ssl_support() {
                unimplemented!()
            } else {
                socket.write_all(Message::Notice.as_vec().as_slice()).await?;
                let len = read_len(&mut socket).await?;
                let mut message = read_message(len, &mut socket).await?;
                log::debug!("MESSAGE FOR TEST = {:#?}", message);
                let version = NetworkEndian::read_i32(message.bytes());
                message.advance(4);
                let parsed = {
                    message
                        .bytes()
                        .split(|b| *b == 0)
                        .filter(|b| !b.is_empty())
                        .map(|b| std::str::from_utf8(b).unwrap().to_owned())
                        .tuples()
                        .collect::<Params>()
                };
                message.advance(message.remaining());
                log::debug!("MESSAGE FOR TEST = {:#?}", parsed);
                socket
                    .write_all(Message::AuthenticationCleartextPassword.as_vec().as_slice())
                    .await?;
                let mut buffer = [0u8; 1];
                let tag = socket.read_exact(&mut buffer).await.map(|_| buffer[0]);
                log::debug!("client message response tag {:?}", tag);
                log::debug!("waiting for authentication response");
                let len = read_len(&mut socket).await?;
                let _message = read_message(len, &mut socket).await?;
                socket.write_all(Message::AuthenticationOk.as_vec().as_slice()).await?;
                Ok(Ok(Connection::new((version, parsed, SslMode::Require), socket)))
            }
        } else if version == VERSION_GSSENC {
            if self.secure().gssenc_support() {
                unimplemented!()
            } else {
                Ok(Err(Error::UnsupportedRequest))
            }
        } else if version == VERSION_CANCEL {
            Ok(Err(Error::UnsupportedVersion))
        } else if version == VERSION_2 {
            Ok(Err(Error::UnsupportedVersion))
        } else if version == VERSION_1 {
            Ok(Err(Error::UnsupportedVersion))
        } else {
            Ok(Err(Error::UnrecognizedVersion))
        }
    }

    fn server_socket(&self) -> &Self::ServerSocket;

    fn secure(&self) -> &Secure;
}

#[async_trait]
pub trait ServerListener {
    type Socket: AsyncReadExt + AsyncWriteExt + Unpin + Send + Sync;

    async fn tcp_connection(&self) -> io::Result<(Self::Socket, SocketAddr)>;
}

pub struct Secure {
    ssl: bool,
    gssenc: bool,
}

impl Secure {
    pub fn none() -> Secure {
        Secure {
            ssl: false,
            gssenc: false,
        }
    }

    pub fn ssl_only() -> Secure {
        Secure {
            ssl: true,
            gssenc: false,
        }
    }

    pub fn gssenc_only() -> Secure {
        Secure {
            ssl: false,
            gssenc: true,
        }
    }

    pub fn both() -> Secure {
        Secure {
            ssl: true,
            gssenc: true,
        }
    }

    fn ssl_support(&self) -> bool {
        self.ssl
    }

    fn gssenc_support(&self) -> bool {
        self.gssenc
    }
}

async fn read_len<RW>(socket: &mut RW) -> io::Result<usize>
where
    RW: AsyncReadExt + AsyncWriteExt + Unpin,
{
    let mut buffer = [0u8; 4];
    let len = socket
        .read_exact(&mut buffer)
        .await
        .map(|_| NetworkEndian::read_u32(&buffer) as usize)?;
    Ok(len - 4)
}

async fn read_message<RW>(len: usize, socket: &mut RW) -> io::Result<BytesMut>
where
    RW: AsyncReadExt + AsyncWriteExt + Unpin,
{
    let mut buffer = BytesMut::with_capacity(len);
    buffer.resize(len, b'0');
    socket.read_exact(&mut buffer).await.map(|_| buffer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};
    use test_helpers::{async_io, pg_frontend};

    struct MockQueryListener {
        server_listener: MockServerListener,
        secure: Secure,
    }

    impl MockQueryListener {
        fn new(test_case: async_io::TestCase, secure: Secure) -> MockQueryListener {
            MockQueryListener {
                server_listener: MockServerListener::new(test_case),
                secure,
            }
        }
    }

    #[async_trait]
    impl QueryListener for MockQueryListener {
        type Socket = async_io::TestCase;
        type ServerSocket = MockServerListener;

        fn server_socket(&self) -> &Self::ServerSocket {
            &self.server_listener
        }

        fn secure(&self) -> &Secure {
            &self.secure
        }
    }

    struct MockServerListener {
        test_case: async_io::TestCase,
    }

    impl MockServerListener {
        fn new(test_case: async_io::TestCase) -> MockServerListener {
            MockServerListener { test_case }
        }
    }

    #[async_trait]
    impl ServerListener for MockServerListener {
        type Socket = async_io::TestCase;

        async fn tcp_connection(&self) -> io::Result<(Self::Socket, SocketAddr)> {
            Ok((
                self.test_case.clone(),
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 5432),
            ))
        }
    }

    #[cfg(test)]
    mod hand_shake {
        use super::*;

        #[async_std::test]
        async fn trying_read_from_empty_stream() {
            let test_case = async_io::TestCase::with_content(vec![]).await;

            let error = MockQueryListener::new(test_case, Secure::none()).accept().await;

            assert!(error.is_err());
        }

        #[cfg(test)]
        mod rust_postgres {
            use super::*;
            use crate::VERSION_3;

            #[async_std::test]
            async fn trying_read_setup_message() {
                let test_case = async_io::TestCase::with_content(vec![&[0, 0, 0, 57]]).await;

                let error = MockQueryListener::new(test_case, Secure::none()).accept().await;

                assert!(error.is_err());
            }

            #[async_std::test]
            async fn successful_connection_handshake() -> io::Result<()> {
                let test_case = async_io::TestCase::with_content(vec![
                    pg_frontend::Message::SslDisabled.as_vec().as_slice(),
                    pg_frontend::Message::Setup(vec![
                        ("client_encoding", "UTF8"),
                        ("timezone", "UTC"),
                        ("user", "postgres"),
                    ])
                    .as_vec()
                    .as_slice(),
                ])
                .await;

                let connection = MockQueryListener::new(test_case.clone(), Secure::none())
                    .accept()
                    .await?
                    .expect("connection is open");

                assert_eq!(
                    connection.properties(),
                    &(
                        VERSION_3,
                        vec![
                            ("client_encoding".to_owned(), "UTF8".to_owned()),
                            ("timezone".to_owned(), "UTC".to_owned()),
                            ("user".to_owned(), "postgres".to_owned())
                        ],
                        SslMode::Disable
                    )
                );

                let actual_content = test_case.read_result().await;
                let mut expected_content = BytesMut::new();
                expected_content.extend_from_slice(Message::AuthenticationOk.as_vec().as_slice());

                assert_eq!(actual_content, expected_content);

                Ok(())
            }
        }

        #[cfg(test)]
        mod psql_client {
            use super::*;

            #[async_std::test]
            async fn trying_read_only_length_of_ssl_message() {
                let test_case = async_io::TestCase::with_content(vec![&[0, 0, 0, 8]]).await;

                let error = MockQueryListener::new(test_case, Secure::none()).accept().await;

                assert!(error.is_err());
            }

            #[async_std::test]
            async fn sending_notice_after_reading_ssl_message() {
                let test_case =
                    async_io::TestCase::with_content(vec![pg_frontend::Message::SslRequired.as_vec().as_slice()]).await;

                let error = MockQueryListener::new(test_case.clone(), Secure::none()).accept().await;

                assert!(error.is_err());

                let actual_content = test_case.read_result().await;
                let mut expected_content = BytesMut::new();
                expected_content.extend_from_slice(Message::Notice.as_vec().as_slice());

                assert_eq!(actual_content, expected_content);
            }

            #[async_std::test]
            async fn successful_connection_handshake() -> io::Result<()> {
                let test_case = async_io::TestCase::with_content(vec![
                    pg_frontend::Message::SslRequired.as_vec().as_slice(),
                    pg_frontend::Message::Setup(vec![
                        ("user", "username"),
                        ("database", "database_name"),
                        ("application_name", "psql"),
                        ("client_encoding", "UTF8"),
                    ])
                    .as_vec()
                    .as_slice(),
                    pg_frontend::Message::Password("123").as_vec().as_slice(),
                ])
                .await;

                let connection = MockQueryListener::new(test_case.clone(), Secure::none())
                    .accept()
                    .await?;

                assert!(connection.is_ok());

                let actual_content = test_case.read_result().await;
                let mut expected_content = BytesMut::new();
                expected_content.extend_from_slice(Message::Notice.as_vec().as_slice());
                expected_content.extend_from_slice(Message::AuthenticationCleartextPassword.as_vec().as_slice());
                expected_content.extend_from_slice(Message::AuthenticationOk.as_vec().as_slice());

                assert_eq!(actual_content, expected_content);

                Ok(())
            }
        }
    }
}