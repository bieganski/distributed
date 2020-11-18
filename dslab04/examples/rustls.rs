use std::io::{Read, Write};
use std::sync::Arc;

use rustls::{
    Certificate, ClientSession, NoClientAuth, RootCertStore, ServerCertVerified,
    ServerCertVerifier, ServerSession, StreamOwned, TLSError,
};
use std::net::{TcpListener, TcpStream};
use webpki::DNSNameRef;

const SERVER_ADDRESS: &str = "127.0.0.1:8081";

// Generated with `openssl req -x509 -out localhost.crt -keyout localhost.key \
//   -newkey rsa:2048 -nodes -sha256 \
//   -subj '/CN=localhost' -extensions EXT -config <( \
//    printf "[dn]\nCN=localhost\n[req]\ndistinguished_name = dn\n[EXT]\nsubjectAltName=DNS:localhost\nkeyUsage=digitalSignature\nextendedKeyUsage=serverAuth")`

pub static SERVER_CERT: &str = "-----BEGIN CERTIFICATE-----
MIIDhTCCAm2gAwIBAgIUKJwAAq6JmtXRhtgLSJ0Rd6kQFs4wDQYJKoZIhvcNAQEL
BQAwUTELMAkGA1UEBhMCUEwxFDASBgNVBAgMC01hem93aWVja2llMREwDwYDVQQH
DAhXYXJzemF3YTELMAkGA1UECgwCVVcxDDAKBgNVBAsMA01JTTAgFw0yMDA3MDQw
ODA4MjBaGA8yMTIwMDYxMDA4MDgyMFowUTELMAkGA1UEBhMCUEwxFDASBgNVBAgM
C01hem93aWVja2llMREwDwYDVQQHDAhXYXJzemF3YTELMAkGA1UECgwCVVcxDDAK
BgNVBAsMA01JTTCCASIwDQYJKoZIhvcNAQEBBQADggEPADCCAQoCggEBAO2YAHxV
U3MCGwK8zHDQcqx/UBpiWkkR18udE842m9aQy7T7I54Pnrsh4dcSCqcJ9CX4O6KE
2wfXU3wVBpCY8kNjhvun8ngWGKAWZJQXbKR0nfogfTHNutr5eiM9/Ijr5fnZXMbf
yEZhFgdzWJ22psWLcXrv4RN9HAEF2p02Ig3lMyFVilrHzvaj+piE7/GcMoQKN+nc
mNlWX9KOdtAjJ5TjqHxJRPEWbP34NFx7Jg8N/oiNxq3Zya6EUaXNbIKGaVl9xlqr
x2YnROnGTJer3ZF1x5j91AsuhjEpNysTAJY/0TuRCQ2TjId0mtiYXZ51cRFqVzTw
rYnXKcpkthTjTzUCAwEAAaNTMFEwHQYDVR0OBBYEFGe9/uC2O8Lz7zLtPtZm70s7
0U0vMB8GA1UdIwQYMBaAFGe9/uC2O8Lz7zLtPtZm70s70U0vMA8GA1UdEwEB/wQF
MAMBAf8wDQYJKoZIhvcNAQELBQADggEBAK/SAF+0ihX0J6KcalYuJvtWASOJuDt0
TqyX5fWvgOawZ4bC+jsvp4UMOjYCpHUts0vjPqq1bVGl2DhNzUUkEscNt2NJSTZ1
HmUz97mrej0HV3cdW5bWKtZdxFx3NTd+0uZxtGvzwdbQD9xOP5klJwUrvqlCHfcT
AhWErM7d0x7bFv+T8IBxKX9+LME5rAMAy3NSUCcAHYt9zwDU/JrWVAOqfDi9XNHf
/FMQnMIh3dyq0d19c2S18t8sZdfjhYrwZtcjm/5YYEHGABVKD0v3TNdS1DYudOeh
k8NPdvw/ao3PHGF9FpOjbU2/IxiIirRX3bB+uokItYLc5ZYxU/tLhmk=
-----END CERTIFICATE-----
";

pub static SERVER_KEY: &str = "-----BEGIN RSA PRIVATE KEY-----
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQDtmAB8VVNzAhsC
vMxw0HKsf1AaYlpJEdfLnRPONpvWkMu0+yOeD567IeHXEgqnCfQl+DuihNsH11N8
FQaQmPJDY4b7p/J4FhigFmSUF2ykdJ36IH0xzbra+XojPfyI6+X52VzG38hGYRYH
c1idtqbFi3F67+ETfRwBBdqdNiIN5TMhVYpax872o/qYhO/xnDKECjfp3JjZVl/S
jnbQIyeU46h8SUTxFmz9+DRceyYPDf6Ijcat2cmuhFGlzWyChmlZfcZaq8dmJ0Tp
xkyXq92RdceY/dQLLoYxKTcrEwCWP9E7kQkNk4yHdJrYmF2edXERalc08K2J1ynK
ZLYU4081AgMBAAECggEAGPbNpkK43+6qs+DugjQEuIcv/XW0EtVjHSvF9vbZ7PON
09hHZ8YwYW0v848bu21aT9sDORJIiUxgozr6U5seaWI9vpdo7KE1uSxhKQZFDgqK
xss/HEqHhZaM9MMAk8plgZkLHIJK4so0rFe8SeX1I/CA3e/ycD/G/3yD+dzEOz1U
BpDWGTulPN8vuYQhpesuGMiQ20LtY6crEsmShiq29ccTqNGCXM8+PeG0P5ouZK3B
pbGgEkPIraw4IKRfRGvaWShQL4/dw0OY0rd5huol+t8ZfRv2GIG1VJKa3R1fEuwD
ehNXgvs8040R2YEkfFgtu1d/SUf1egNNxO9J1BC4yQKBgQD+AHbovo1xaLtFBaLv
1UQfGSTWWRwdFcci8BFynoN4Nkr8Lm+J7sdoepMIS6lDxI0JFyBrFf+0pJbDu9gy
PGAPXAARuUDWyaX/PQ9JMdKCL81otLwZ1/Rrz3GXyYDqInlERW49kNAKhjDPpSYb
oopeVoM+ZQb2vsbHjhRsELAD8wKBgQDvdn4+fn75+bTJxZjxFuKrsgcmrwWbtuYO
V0YrPUBSVawst4nBnVnVjeGPprN5Mntev5mbZI2ciBKqD9wZdFswPXwrJ7Lddd1k
qyRz9Hi8+FUnyaiWkTenysgTYiggZ52RHzkdyPLN0QNZ8FejtDdugB4YaVUzj1Qz
1Hn5JL8yNwKBgQDktYBcc/AiVP6C2N9s+uha5CYSR6yT1982a1oajfatUJ3WUu2m
EnZFclFgX9hqp9mifMt5ARb9DlKerk6udUS9r8Z0Lw3eGTY6DD3uV0lIZkib5lmV
H6n7RY/v78X/jMkKm0X3c6SQRWJAJmGY8pOK/HUywn8xwHh2iU2EPMZw1wKBgCiW
SOeGwTbVYBMi7r1OR7UaQ6QG3kh6Q5wCpt4FT7Wk173sjejHJsbPeX/DnX/ZUpfI
mdyAuo/hcGhqr7+QvTP79+PIosRuicvaRh9xPFWrMaPAxlZe3rQN5sOCS5LSfR5v
0FqACMdhudOwrOoOTabpCDDTCaHnlToDtXjJBa8ZAoGBAOthOQ0JiYWNcIc0Agih
w6T6FAb4RUpxrYMqTL0w4h87P6nrtrysXCauRDCtGXHsVAUECf6TVZywCf+rpciE
AaX9TNlbXgJMVU4aN64LEdk08fVHZ8aLoImyAuJJROpDek3iOTbSAYT7nymloDIE
Z3C2/JbDVPOI3Km3TE9kFgBW
-----END RSA PRIVATE KEY-----
";

// rutls by default disallows self-signed certificates, so we allow them forcefully
// for the sake of this example. In the small assignment you will not be allowed
// to override the server certificate verification.
struct VeryOptimisticServerCertVerifier {}

impl ServerCertVerifier for VeryOptimisticServerCertVerifier {
    fn verify_server_cert(
        &self,
        _roots: &RootCertStore,
        _presented_certs: &[Certificate],
        _dns_name: DNSNameRef,
        _ocsp_response: &[u8],
    ) -> Result<ServerCertVerified, TLSError> {
        Ok(ServerCertVerified::assertion())
    }
}

impl VeryOptimisticServerCertVerifier {
    fn new() -> Arc<dyn ServerCertVerifier> {
        Arc::new(Self {})
    }
}

fn client_stream(stream: TcpStream) -> StreamOwned<ClientSession, TcpStream> {
    let mut config = rustls::ClientConfig::new();
    config
        .root_store
        .add_pem_file(&mut SERVER_CERT.as_bytes())
        .unwrap();
    config
        .dangerous()
        .set_certificate_verifier(VeryOptimisticServerCertVerifier::new());
    let dns_name = webpki::DNSNameRef::try_from_ascii_str("localhost").unwrap();
    let sess = rustls::ClientSession::new(&Arc::new(config), dns_name);
    rustls::StreamOwned::new(sess, stream)
}

fn server_stream(stream: TcpStream) -> StreamOwned<ServerSession, TcpStream> {
    let mut config = rustls::ServerConfig::new(NoClientAuth::new());

    let certs = rustls::internal::pemfile::certs(&mut SERVER_CERT.as_bytes()).unwrap();
    let private_key = rustls::internal::pemfile::rsa_private_keys(&mut SERVER_KEY.as_bytes())
        .unwrap()
        .remove(0);

    config.set_single_cert(certs, private_key).unwrap();
    let sess = rustls::ServerSession::new(&Arc::new(config));
    rustls::StreamOwned::new(sess, stream)
}

fn client() {
    let client_raw_link = TcpStream::connect(SERVER_ADDRESS).unwrap();
    let mut client = client_stream(client_raw_link);

    client.write_all(b"Hello encrypted world!\n").unwrap();

    let mut data = vec![0; 5];
    client.take(5).read_to_end(data.as_mut()).unwrap();
    print!("{}", std::str::from_utf8(data.as_ref()).unwrap());
}

fn server() {
    let listener = TcpListener::bind(SERVER_ADDRESS).unwrap();
    let server_raw_stream = listener.incoming().next().unwrap().unwrap();
    let mut server = server_stream(server_raw_stream);

    let mut data = vec![0; 23];
    server.read_exact(data.as_mut()).unwrap();
    print!("{}", std::str::from_utf8(data.as_ref()).unwrap());
    server.write_all(b"Done\n").unwrap();
}

fn main() {
    let client_thread = std::thread::spawn(|| {
        client();
    });

    server();

    client_thread.join().unwrap();
}
