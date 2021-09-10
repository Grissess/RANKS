
use std::net::{TcpListener, SocketAddr};
use std::thread::{self, JoinHandle};
use std::sync::{Mutex, MutexGuard, Arc, mpsc::{self, RecvError}};

use std::io::Result as IoResult;

use websocket::{OwnedMessage, server::{NoTlsAcceptor, WsServer}, result::WebSocketError};

use bus::Bus;

use sim::Team;

pub struct TankServer {
    // TODO: use a TLS acceptor, but it's almost midnight and I don't want to do it now
    wsserv: Arc<Mutex<WsServer<NoTlsAcceptor, TcpListener>>>,
    broadcaster: Arc<Mutex<Bus<OwnedMessage>>>,
    stm: Arc<OwnedMessage>,
    receiver: Option<mpsc::Receiver<ClientMessage>>,
    tx: mpsc::Sender<ClientMessage>,
}

impl TankServer {
    pub fn server(&self) -> Arc<Mutex<WsServer<NoTlsAcceptor, TcpListener>>> {
        self.wsserv.clone()
    }

    pub fn broadcaster(&self) -> MutexGuard<Bus<OwnedMessage>> {
        self.broadcaster.lock().unwrap()
    }

    pub fn receiver(&mut self) -> Option<mpsc::Receiver<ClientMessage>> {
        let mut rv = None;
        core::mem::swap(&mut rv, &mut self.receiver);
        rv
    }

    pub fn new(startup_message: Arc<OwnedMessage>) -> std::io::Result<Self> {
        WsServer::<NoTlsAcceptor, TcpListener>::bind(SocketAddr::from(([0, 0, 0, 0], 7446))).map(|wsserv| {
            let ws = Arc::new(Mutex::new(wsserv));
            let bus = Arc::new(Mutex::new(Bus::new(10)));
            let (tx, receiver) = mpsc::channel();
            TankServer { wsserv: ws, broadcaster: bus, stm: startup_message, receiver: Some(receiver), tx }
        })
    }

    pub fn init(&mut self) -> JoinHandle<()> {
        let wsc = self.wsserv.clone();
        let stm = self.stm.clone();
        let tx = self.tx.clone();
        let rxsource = self.broadcaster.clone();
        thread::spawn(move || {
            let mut team: Team = 0;
            loop {
                match wsc.lock().unwrap().accept() {
                    Ok(u) => match u.accept() {
                        Ok(mut client) => {
                            let my_team = team.clone();
                            let to_send = stm.clone();
                            let mut rx = rxsource.lock().unwrap().add_rx();
                            let my_tx = tx.clone();
                            let dc_tx = tx.clone();
                            match my_tx.send(ClientMessage::Connect(my_team, client.peer_addr())) {
                                _ => ()  // TODO: maybe do something if the channel is closed?
                            };
                            thread::spawn(move || {
                                match || -> Result<(), WebSocketError> {
                                    client.send_message(&*to_send)?;
                                    let (mut reader, mut writer) = client.split().map_err(|e| WebSocketError::IoError(e))?;
                                    let jh1 = thread::spawn(move || {
                                        loop {
                                            match rx.recv() {
                                                Ok(message) => {
                                                    match writer.send_message(&message) {
                                                        Ok(()) => (),
                                                        Err(_) => break,
                                                    }
                                                }
                                                Err(RecvError) => {
                                                    core::mem::drop(writer.shutdown_all());
                                                    break;
                                                }
                                            }
                                        }
                                    });
                                    let jh2 = thread::spawn(move || {
                                        loop {
                                            match reader.recv_message() {
                                                Ok(_message) => {
                                                    // TODO
                                                },
                                                Err(_) => break,
                                            }
                                        }
                                    });
                                    core::mem::drop((jh1.join(), jh2.join()));
                                    Ok(())
                                }() {
                                    _ => {
                                        core::mem::drop(dc_tx.send(ClientMessage::Disconnect(my_team)));
                                    },
                                }
                            });
                        },
                        Err(_) => continue,
                    },
                    Err(_) => continue,
                }
                team = team.wrapping_add(1);
            }
        })
    }
}

pub enum ClientMessage {
    Connect(Team, IoResult<SocketAddr>),
    Disconnect(Team),
    Message(Team, OwnedMessage),
}

