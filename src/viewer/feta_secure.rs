//! QUIC/TLS 1.3 datagrams with a bundled, pinned server certificate.
//! No insecure verifier, plaintext fallback, public CA discovery, or client-side VPN.
use quinn::{Connection, Endpoint, TransportConfig};
use rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{mpsc, Arc, Mutex},
    thread,
    time::Duration,
};
use tokio::sync::{mpsc as async_mpsc, oneshot, Semaphore};
pub const CERTIFICATE: &[u8] = include_bytes!("../../assets/network/server-cert.der");
pub const PAYLOAD: usize = 1100;

/// Server identity. The certificate is public; the PKCS#8 private key stays outside the repository.
pub struct Identity {
    pub certificate: Vec<u8>,
    pub private_key: Vec<u8>,
}
impl Identity {
    pub fn load() -> crate::Result<Self> {
        let path = std::env::var_os("FETA_TLS_KEY_FILE")
            .map(std::path::PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME")
                    .map(|h| std::path::PathBuf::from(h).join(".config/feta/server-key.der"))
            })
            .ok_or("Set FETA_TLS_KEY_FILE to the private server PKCS#8 DER key")?;
        Ok(Self {
            certificate: CERTIFICATE.to_vec(),
            private_key: std::fs::read(path)?,
        })
    }
}
fn transport() -> Arc<TransportConfig> {
    let mut t = TransportConfig::default();
    t.max_concurrent_bidi_streams(0u8.into())
        .max_concurrent_uni_streams(0u8.into())
        .datagram_receive_buffer_size(Some(PAYLOAD * 64))
        .datagram_send_buffer_size(PAYLOAD * 64)
        .max_idle_timeout(Some(Duration::from_secs(10).try_into().unwrap()))
        .keep_alive_interval(Some(Duration::from_secs(2)));
    Arc::new(t)
}
type Connections = Arc<Mutex<HashMap<SocketAddr, Connection>>>;
type Incoming = mpsc::SyncSender<(SocketAddr, Vec<u8>)>;
/// Bounded synchronous facade over one private async network thread.
pub struct SecureSocket {
    address: SocketAddr,
    send: async_mpsc::Sender<(SocketAddr, Vec<u8>)>,
    receive: mpsc::Receiver<(SocketAddr, Vec<u8>)>,
    error: Arc<Mutex<Option<String>>>,
    stop: Option<oneshot::Sender<()>>,
    worker: Option<thread::JoinHandle<()>>,
}
enum Mode {
    Server(Identity),
    Client {
        server: SocketAddr,
        certificate: Vec<u8>,
    },
}
impl SecureSocket {
    pub fn server(address: SocketAddr, identity: Identity) -> crate::Result<Self> {
        Self::start(address, Mode::Server(identity))
    }
    pub fn client(server: SocketAddr, certificate: Vec<u8>) -> crate::Result<Self> {
        Self::start(
            if server.is_ipv4() {
                "0.0.0.0:0"
            } else {
                "[::]:0"
            }
            .parse()?,
            Mode::Client {
                server,
                certificate,
            },
        )
    }
    fn start(address: SocketAddr, mode: Mode) -> crate::Result<Self> {
        let (send, mut outgoing) = async_mpsc::channel::<(SocketAddr, Vec<u8>)>(128);
        let (incoming, receive) = mpsc::sync_channel(256);
        let (stop, mut stopped) = oneshot::channel();
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let error = Arc::new(Mutex::new(None));
        let worker_error = error.clone();
        let worker=thread::Builder::new().name("feta-quic".into()).spawn(move||{
            let runtime=match tokio::runtime::Builder::new_current_thread().enable_all().build(){
                Ok(r)=>r,Err(e)=>{let _=ready_tx.send(Err(e.to_string()));return;}
            };
            runtime.block_on(async move {
                let setup=setup(address,mode);
                let (endpoint,remote)=match setup {Ok(v)=>v,Err(e)=>{let _=ready_tx.send(Err(e));return;}};
                let _=ready_tx.send(endpoint.local_addr().map_err(|e|e.to_string()));
                if let Some(server)=remote {
                    let attempt=match endpoint.connect(server,"feta.local") {Ok(c)=>c,Err(e)=>{*worker_error.lock().unwrap()=Some(e.to_string());return;}};
                    let connection=tokio::select! {
                        _=&mut stopped=>return,
                        result=tokio::time::timeout(Duration::from_secs(6),attempt)=>match result {
                            Ok(Ok(c))=>c,
                            other=>{*worker_error.lock().unwrap()=Some(format!("Secure connection failed: {other:?}. Check the server address and game version."));return;}
                        }
                    };
                    loop {
                        tokio::select! {
                            _=&mut stopped=>break,
                            packet=outgoing.recv()=>match packet {
                                Some((_,bytes))=>{let _=connection.send_datagram(bytes.into());},None=>break,
                            },
                            packet=connection.read_datagram()=>match packet {
                                Ok(bytes) if bytes.len()<=PAYLOAD=>{let _=incoming.try_send((server,bytes.to_vec()));},
                                Ok(_)=>{},
                                Err(_)=>{*worker_error.lock().unwrap()=Some("Secure connection closed. Reconnect to return to the lobby.".into());break;}
                            }
                        }
                    }
                    connection.close(0u8.into(),b"client exit");
                }else{
                    let connections:Connections=Arc::new(Mutex::new(HashMap::new()));
                    let permits=Arc::new(Semaphore::new(8));
                    loop {
                        tokio::select! {
                            _=&mut stopped=>break,
                            packet=outgoing.recv()=>match packet {
                                Some((addr,bytes))=>{if let Some(c)=connections.lock().unwrap().get(&addr){let _=c.send_datagram(bytes.into());}},None=>break,
                            },
                            candidate=endpoint.accept()=>{
                                let Some(candidate)=candidate else{break;};
                                // Stateless address validation before allocating a TLS handshake task.
                                if !candidate.remote_address_validated(){let _=candidate.retry();continue;}
                                let Ok(permit)=permits.clone().try_acquire_owned() else{candidate.refuse();continue;};
                                let connections=connections.clone();let incoming=incoming.clone();
                                tokio::spawn(async move {
                                    let _permit=permit;
                                    if let Ok(Ok(connection))=tokio::time::timeout(Duration::from_secs(5),candidate).await {
                                        receive_connection(connection,connections,incoming).await;
                                    }
                                });
                            }
                        }
                    }
                }
                endpoint.close(0u8.into(),b"shutdown");
            });
        })?;
        let address = ready_rx
            .recv()
            .map_err(|_| "Network worker stopped during startup")?
            .map_err(|e| format!("Secure network startup: {e}"))?;
        Ok(Self {
            address,
            send,
            receive,
            error,
            stop: Some(stop),
            worker: Some(worker),
        })
    }
    pub fn local_addr(&self) -> SocketAddr {
        self.address
    }
    pub fn send_to(&self, bytes: &[u8], address: SocketAddr) -> crate::Result<usize> {
        if bytes.len() > PAYLOAD {
            return Err("Secure datagram exceeds budget".into());
        }
        // Congestion drops obsolete datagrams; game input counters/state/commands recover loss.
        match self.send.try_send((address, bytes.to_vec())) {
            Ok(()) | Err(async_mpsc::error::TrySendError::Full(_)) => Ok(bytes.len()),
            Err(_) => Err("Secure connection is closed".into()),
        }
    }
    pub fn receive(&self) -> crate::Result<Vec<(SocketAddr, Vec<u8>)>> {
        if let Some(error) = self.error.lock().unwrap().take() {
            return Err(error.into());
        }
        Ok(self.receive.try_iter().take(128).collect())
    }
}
impl Drop for SecureSocket {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
fn setup(address: SocketAddr, mode: Mode) -> Result<(Endpoint, Option<SocketAddr>), String> {
    match mode {
        Mode::Server(identity) => {
            let mut config = quinn::ServerConfig::with_single_cert(
                vec![CertificateDer::from(identity.certificate)],
                PrivatePkcs8KeyDer::from(identity.private_key).into(),
            )
            .map_err(|e| e.to_string())?;
            config
                .transport_config(transport())
                .incoming_buffer_size(16 * 1024)
                .incoming_buffer_size_total(128 * 1024);
            Ok((
                Endpoint::server(config, address).map_err(|e| e.to_string())?,
                None,
            ))
        }
        Mode::Client {
            server,
            certificate,
        } => {
            let mut roots = rustls::RootCertStore::empty();
            roots
                .add(CertificateDer::from(certificate))
                .map_err(|e| e.to_string())?;
            let mut config = quinn::ClientConfig::with_root_certificates(Arc::new(roots))
                .map_err(|e| e.to_string())?;
            config.transport_config(transport());
            let mut endpoint = Endpoint::client(address).map_err(|e| e.to_string())?;
            endpoint.set_default_client_config(config);
            Ok((endpoint, Some(server)))
        }
    }
}
async fn receive_connection(connection: Connection, connections: Connections, incoming: Incoming) {
    let address = connection.remote_address();
    // Do not overwrite a live authenticated connection after a rapid endpoint reuse.
    {
        let mut map = connections.lock().unwrap();
        if map.contains_key(&address) {
            connection.close(1u8.into(), b"endpoint busy");
            return;
        }
        map.insert(address, connection.clone());
    }
    let mut window = tokio::time::Instant::now();
    let mut count = 0;
    while let Ok(Ok(bytes)) =
        tokio::time::timeout(Duration::from_secs(8), connection.read_datagram()).await
    {
        if window.elapsed() >= Duration::from_secs(1) {
            window = tokio::time::Instant::now();
            count = 0;
        }
        count += 1;
        if count > 240 || bytes.len() > PAYLOAD {
            break;
        }
        let _ = incoming.try_send((address, bytes.to_vec()));
    }
    connections.lock().unwrap().remove(&address);
    connection.close(0u8.into(), b"session ended");
}
