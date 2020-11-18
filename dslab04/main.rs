use std::borrow::Borrow;
use std::net::{TcpListener, TcpStream};

use crate::keys::{ROOT_CERT, SERVER_FULL_CHAIN, SERVER_PRIVATE_KEY};
use crate::solution::{MacKeyProvider, SecureClient, SecureServer, SecureServerError};

mod public_test;
mod solution;

pub(crate) mod keys {
    pub static ROOT_CERT: &str = "-----BEGIN CERTIFICATE-----
MIIFFzCCAv+gAwIBAgIUWygmFFF/Tk4kwsadVgMhIZ+cOxQwDQYJKoZIhvcNAQEL
BQAwGjEYMBYGA1UEAwwPcG9ueXRvd24gUlNBIENBMCAXDTIwMDcxMTEwMTIyNVoY
DzMwMTkxMTEyMTAxMjI1WjAaMRgwFgYDVQQDDA9wb255dG93biBSU0EgQ0EwggIi
MA0GCSqGSIb3DQEBAQUAA4ICDwAwggIKAoICAQCzGv2VGP3OfgMbJ1/2heDzzxFn
VuEbRlqCyIStg0Ury42EVs89tCorN2O2aIn2ezoJkBEl8HFj/zPVUzwOERXJTmam
FIr2ddlxi5MtwcyXiAiYRsfsFQN07+VZHfasu0POeIyDihBNijvp+WmEimYowJoC
Ame6FvvH0HpXsruzDw3rKqJk/+IWJDlQBw0AkRHNIlqZcNnz1j6AVghNTETVVpD4
ihV9o38+m/yz0vmmvqBTD0NVyLSuxRqBSg4VFpkhY4pg3zvoGGEevlQyt8uoyv0I
XqSLsAKlrHA12JS3t/2H1gu+fn8738Vhhj9s6q2gJagmT1ZumQt4LVhvt3JkKqzi
rq9uQCK1oz5cVvdkdLm4alrZIAPJNdXPE6T8uwbpUTRT1tcvNK/yIcu9UPFSZTaR
X+e+RV5VTk07WbJhfh01+k/NMGq070csdwn6lAUqe6C8YwcwfVAPCZDnhBz/VyY3
k9bua9LbOKfK8vdaZazA3T0D8Bu459rdk3imIurKw5jaou/U0xulyir87/lXfPjE
9uzeMDFZ+b7zOibNEwQYBVZQ/14WXvOGrg56mpI4k+k5dOn3vk99Tc/GTv5NRX0D
hdf9aViehXquAa8FUNohOyhkFC9860qV6JhrVtJ/3aYKGp5n8ahIOko/DdEBQsLp
FAplfRsQV2TveCII4wIDAQABo1MwUTAdBgNVHQ4EFgQUjXZZJbe+5aU1vsLTUKXO
cWlf+DMwHwYDVR0jBBgwFoAUjXZZJbe+5aU1vsLTUKXOcWlf+DMwDwYDVR0TAQH/
BAUwAwEB/zANBgkqhkiG9w0BAQsFAAOCAgEATG7L+6WnRpb5w6TUV1GaXVLKT4vD
LTSLM0qzjzkNOcFgGKNp5vsxakhQPWAHotopzFiAsKoOdz2iWO5VzSqQFLJyG5Zm
OKFbIpCJc7hfmSR30jLcnBwpSP2CVQtyuz25P8gY2/hNXl2n/uM1AjVdpvv3MyoN
JZkfc/78MPGC61d6LNCEVn87zJQ5/jD3HufWI43oFxeXB+b/yfaogtI8wT/Di55i
PA60fhJog/UT4GTlnaQLU+GAhdkpN7fWKH2atAO81g73LM7ruDjkFTAAWWWwl9C1
hmbNU69VYZ+4Ub/vPKsApwAp0VNDNJSEoY62HieUGTu6yHyJSmi+s1Dl08u2HVz6
nm8iJ1xa+JnMY+wObQn9hFlXFnIEj5RIZwIVpwCurY6FQ5MmdCyONBXEhQIteuQU
AoxNG2/UCvnddjUK2tTuSO69JyRNrhdyUo6Jx38wEkaSkqoWItt1x2qDAmFYJTpX
XT9JOTqdx/JG4gt2ZcatHfKnonwsMqU3K9o7QvKIpIlB7ziA++kV+qEGtj4gWtYt
l9VNt922UlBPXBQXmy2/BNIYT4eVKd7N5v4MIvvnAH1c618Gg1xWxGXyo0VhIHFD
i1jOwPTn+kd2Va4tHM9ANUW6WWIJVwP5rGSYdKZEXPnRxnLyZ7/u6SQGvrUQkBOG
eRnas+VZ9AcT0T0=
-----END CERTIFICATE-----
";

    pub static SERVER_FULL_CHAIN: &str = "-----BEGIN CERTIFICATE-----
MIIEWjCCAkKgAwIBAgIBezANBgkqhkiG9w0BAQsFADAaMRgwFgYDVQQDDA9wb255
dG93biBSU0EgQ0EwIBcNMjAwNzExMTAxMjI1WhgPMzAxOTExMTIxMDEyMjVaMBgx
FjAUBgNVBAMMDWxvY2FsaG9zdC5jb20wggEiMA0GCSqGSIb3DQEBAQUAA4IBDwAw
ggEKAoIBAQC2x11QeB3b05m9F8H1tnjTmiUYkBcwYZeHLKbtZ5Shz+TGEC6CmsCk
inofcd1H1CQJr5wLCHamR1tf9ysU3lO4anHCv0o4YGTHf5IxMf7bp/H15xNOYreY
fjKl01Bvs17kI0M4dcpFmyCvi9qscyTakSxUsOwpc2yZrLpqmeHmYu9jaJBUbOnY
Ehyk+vuRsz9+IOHpZf7fwL+cqw0Pz49Dlnn7q8O7wtCEoUuvkJclGF9HX/yDrV00
ASyA8PaDtmJ7gcHT1bd19U5I1TncpuwM68CkuOl/ujOZK30HrgyEYURbbG4hdNdZ
eM1JM7uDQfc2WpNfvBo/bFzv0pA00pSDAgMBAAGjgaowgacwDAYDVR0TAQH/BAIw
ADALBgNVHQ8EBAMCBsAwHQYDVR0OBBYEFL7lIgxiXwYcrphbOtFaqbeRYerBMFUG
A1UdIwROMEyAFI12WSW3vuWlNb7C01ClznFpX/gzoR6kHDAaMRgwFgYDVQQDDA9w
b255dG93biBSU0EgQ0GCFFsoJhRRf05OJMLGnVYDISGfnDsUMBQGA1UdEQQNMAuC
CWxvY2FsaG9zdDANBgkqhkiG9w0BAQsFAAOCAgEAkVo8qvC2029uYYMHCOdj5Hlq
o0tceFqwQ0P6cVudfSXn+Gx8oO/tU6a33SuIy5DzDf4HfHhmyzdx5tL2DF8YI7eb
hywHWTe0r1nqBu4fsq0pc0V2owhx6ySBT7HR+Ux75FrDRV7lLo9ywzZTSumpWZAf
yv9ikhA/n+x3yHwrBtjU8yxDcEGsK+7pFweTqQWBCeLvptF9QkdzAmod6Qxkj/Le
Hw2cAOTTAtnuWcf1WC8cIrb/RPh/sCpa0hSWQw7tyAM8vQXeF+chz9/C6XSAU/V+
9xlWM6tsJGcqLZGUj+3aLryS8r8BBaRyeL8uo8CVpa5FcjVlkCmw6Vu18DSMC1Gk
BfxGKCDnuSYKZvSsdDTP4JQsykgeuI1VBb6S7l7CtUcy+fofWxJOTJ9Fj959btY4
mjbxcQZyx1Ckq7MHemBTcrSLU+FZAzxyaCo7SUyYFzxL5tZUNc19UU9qfBK3J90H
beH/rmmeDtLcfLiP3L5O/HQwy6wk10bsoZWzufF4gw9vpuQSCRvoQyE06OwrGIGg
PAyXEOigLk2pQex6SH57CepUhpKq5EID/xm80GNKGSf5dHS6XwnApaIhCZ1TtXgR
TV+4dUQMoK3LUsF105sz/LKwDKLuUUHh5ytKdVuP+0BZYv8uw5rISZ2Vyc94DQmw
j56VaPhYMQZ7XkLQR/U=
-----END CERTIFICATE-----
-----BEGIN CERTIFICATE-----
MIIFFzCCAv+gAwIBAgIUWygmFFF/Tk4kwsadVgMhIZ+cOxQwDQYJKoZIhvcNAQEL
BQAwGjEYMBYGA1UEAwwPcG9ueXRvd24gUlNBIENBMCAXDTIwMDcxMTEwMTIyNVoY
DzMwMTkxMTEyMTAxMjI1WjAaMRgwFgYDVQQDDA9wb255dG93biBSU0EgQ0EwggIi
MA0GCSqGSIb3DQEBAQUAA4ICDwAwggIKAoICAQCzGv2VGP3OfgMbJ1/2heDzzxFn
VuEbRlqCyIStg0Ury42EVs89tCorN2O2aIn2ezoJkBEl8HFj/zPVUzwOERXJTmam
FIr2ddlxi5MtwcyXiAiYRsfsFQN07+VZHfasu0POeIyDihBNijvp+WmEimYowJoC
Ame6FvvH0HpXsruzDw3rKqJk/+IWJDlQBw0AkRHNIlqZcNnz1j6AVghNTETVVpD4
ihV9o38+m/yz0vmmvqBTD0NVyLSuxRqBSg4VFpkhY4pg3zvoGGEevlQyt8uoyv0I
XqSLsAKlrHA12JS3t/2H1gu+fn8738Vhhj9s6q2gJagmT1ZumQt4LVhvt3JkKqzi
rq9uQCK1oz5cVvdkdLm4alrZIAPJNdXPE6T8uwbpUTRT1tcvNK/yIcu9UPFSZTaR
X+e+RV5VTk07WbJhfh01+k/NMGq070csdwn6lAUqe6C8YwcwfVAPCZDnhBz/VyY3
k9bua9LbOKfK8vdaZazA3T0D8Bu459rdk3imIurKw5jaou/U0xulyir87/lXfPjE
9uzeMDFZ+b7zOibNEwQYBVZQ/14WXvOGrg56mpI4k+k5dOn3vk99Tc/GTv5NRX0D
hdf9aViehXquAa8FUNohOyhkFC9860qV6JhrVtJ/3aYKGp5n8ahIOko/DdEBQsLp
FAplfRsQV2TveCII4wIDAQABo1MwUTAdBgNVHQ4EFgQUjXZZJbe+5aU1vsLTUKXO
cWlf+DMwHwYDVR0jBBgwFoAUjXZZJbe+5aU1vsLTUKXOcWlf+DMwDwYDVR0TAQH/
BAUwAwEB/zANBgkqhkiG9w0BAQsFAAOCAgEATG7L+6WnRpb5w6TUV1GaXVLKT4vD
LTSLM0qzjzkNOcFgGKNp5vsxakhQPWAHotopzFiAsKoOdz2iWO5VzSqQFLJyG5Zm
OKFbIpCJc7hfmSR30jLcnBwpSP2CVQtyuz25P8gY2/hNXl2n/uM1AjVdpvv3MyoN
JZkfc/78MPGC61d6LNCEVn87zJQ5/jD3HufWI43oFxeXB+b/yfaogtI8wT/Di55i
PA60fhJog/UT4GTlnaQLU+GAhdkpN7fWKH2atAO81g73LM7ruDjkFTAAWWWwl9C1
hmbNU69VYZ+4Ub/vPKsApwAp0VNDNJSEoY62HieUGTu6yHyJSmi+s1Dl08u2HVz6
nm8iJ1xa+JnMY+wObQn9hFlXFnIEj5RIZwIVpwCurY6FQ5MmdCyONBXEhQIteuQU
AoxNG2/UCvnddjUK2tTuSO69JyRNrhdyUo6Jx38wEkaSkqoWItt1x2qDAmFYJTpX
XT9JOTqdx/JG4gt2ZcatHfKnonwsMqU3K9o7QvKIpIlB7ziA++kV+qEGtj4gWtYt
l9VNt922UlBPXBQXmy2/BNIYT4eVKd7N5v4MIvvnAH1c618Gg1xWxGXyo0VhIHFD
i1jOwPTn+kd2Va4tHM9ANUW6WWIJVwP5rGSYdKZEXPnRxnLyZ7/u6SQGvrUQkBOG
eRnas+VZ9AcT0T0=
-----END CERTIFICATE-----
";

    pub static SERVER_PRIVATE_KEY: &str = "-----BEGIN RSA PRIVATE KEY-----
MIIEpAIBAAKCAQEAtsddUHgd29OZvRfB9bZ405olGJAXMGGXhyym7WeUoc/kxhAu
gprApIp6H3HdR9QkCa+cCwh2pkdbX/crFN5TuGpxwr9KOGBkx3+SMTH+26fx9ecT
TmK3mH4ypdNQb7Ne5CNDOHXKRZsgr4varHMk2pEsVLDsKXNsmay6apnh5mLvY2iQ
VGzp2BIcpPr7kbM/fiDh6WX+38C/nKsND8+PQ5Z5+6vDu8LQhKFLr5CXJRhfR1/8
g61dNAEsgPD2g7Zie4HB09W3dfVOSNU53KbsDOvApLjpf7ozmSt9B64MhGFEW2xu
IXTXWXjNSTO7g0H3NlqTX7waP2xc79KQNNKUgwIDAQABAoIBAQCmlKJuIVsNKxe1
p75lQ21xZM4kScZomLkyQDbOrZVfuApHnR3WeFXUYESW/f5vZV+G2F6+C3Ofbe9Q
UgMYnNvueky98fsh0HBpBdJmNauR48l4lKYazYuIVpOwYBuyAeccwyqTfX7g21X5
x9t/Sd2vuZmOgkRqLsEueabYWvDwN3aAJMeEMPqm5cjhhhcjYi078EYVebvHQhNm
nrBA9jvDIcSRKkIoLyZXKe8vC3DhFBoYJrUEUNbyqz7g2SSpRGkQyO6f6aIB3YQo
DnZNSyv5VYLj7lTksYV3cVv9LIKv4OTpCkF6RHci9d51r/UJtc+t2dSK+wUeeJQE
IijTbq9xAoGBAN7bGE4+lsyodHkfdvZaSKgKT5ltSHgVlaS71bHmLvaQkCu+xnzJ
Xyyguyjs9pwGwqOZG0TVlIANPRU4IkjoHSzIEnAy5oRprVYItNSBvmAEsuwOeoGT
EKNk7BBtqKgAjQtEkbXnPQlUH46yYkbl0+9PPMjxif3CL+se4sV8xWF7AoGBANH2
ZQM1nXmQL8apTdv8tq/1m+nYnxRBGhdHgB/vfm8Kt/4elwI22CQD7EfLuUg3sx3G
Hvpzs12JgbCDgR22GUH0Lfa7NfzcpfZQpptiwzxqwooZISRIRSQbPVJ9KCYLYol+
0dzMPjyCLdenUDUTeMzLsTPeE2N9SNikt44blFaZAoGAGt+y06SqjK13ti98KgMD
JfhwVuEdzxVTQVVBVL2cRjFyoUPVLbEe4APV7f59Up1iFVZeOnPC/5oZFpj5UW9k
LUVHK+6Ha8pOk8RjAglPSsbmSJ8KWNvCMuH1sZl3sCK433X6WEQ1UQ2q7ItIXKJU
Z1RX0SeHa2liW+kSkZwVNUcCgYAJMdcemtx4lF3jP0rPlXOSpRjc1sWwp2EzH8h2
nZBV4IxKLqDCUhCJEzrnsf49MWNArIpywVpbgEgTqM6gtHbKspzIr04f8rG55bJG
H78ZCDvYvFz9L8UHXcIDuMNnVxxLlSgvmSookDLdvNAAYwfpQApUkSccNkJYam43
ZPHMEQKBgQCwt4/nX2IXSW4w07eIUQp4BcXsOUQDkJQdAmFe3kIGjL5zOtMIIYhU
ImaJ0/v958zxXcd7JLYHshGKzxJ6JHq1PsHoRJ1BguZ2gfHYgXKYefAz9SYpVymB
ONEtvmqUN5kyin0FTpcEQG92SQVHhIadWDSPRoI4Yg8f6kq3PvvEqw==
-----END RSA PRIVATE KEY-----
";
}

#[derive(Clone)]
struct MacKeyProviderType {
    key: Vec<u8>,
}

impl MacKeyProviderType {
    fn new(key: Vec<u8>) -> Self {
        MacKeyProviderType { key }
    }
}

impl MacKeyProvider for MacKeyProviderType {
    fn key(&self) -> &[u8] {
        self.key.borrow()
    }
}

fn setup_process(
    mac_key_provider: &dyn MacKeyProvider,
) -> (SecureClient<TcpStream>, SecureServer<TcpStream>) {
    let listener = TcpListener::bind("127.0.0.1:2222").unwrap();

    println!("Server listening on: {}", listener.local_addr().unwrap());
    println!("port: {}", listener.local_addr().unwrap().port());
    let client = SecureClient::new(
        TcpStream::connect(("127.0.0.1", listener.local_addr().unwrap().port())).unwrap(),
        mac_key_provider,
        ROOT_CERT,
    );

    let server = SecureServer::new(
        listener.incoming().next().unwrap().unwrap(),
        mac_key_provider,
        SERVER_PRIVATE_KEY,
        SERVER_FULL_CHAIN,
    );
    (client, server)
}

/// Server rejecting messages from client because of invalid HMAC tag.
fn invalid_hmac_example() {
    let first_key_provider = MacKeyProviderType::new(vec![1, 2, 3, 4]);
    let second_key_provider = MacKeyProviderType::new(vec![1, 2, 3, 4, 5]);

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();

    let mut invalid_hmac_client = SecureClient::new(
        TcpStream::connect(("127.0.0.1", listener.local_addr().unwrap().port())).unwrap(),
        &first_key_provider,
        ROOT_CERT,
    );
    // Notice that here is a different hmac key used for the server:
    let mut invalid_hmac_server = SecureServer::new(
        listener.incoming().next().unwrap().unwrap(),
        &second_key_provider,
        SERVER_PRIVATE_KEY,
        SERVER_FULL_CHAIN,
    );
    let server_thread = std::thread::spawn(move || {
        let res = invalid_hmac_server.recv_message();
        assert_eq!(res, Err(SecureServerError::InvalidHmac));
    });

    invalid_hmac_client.send_msg(b"Hello, World!".to_vec());
    assert!(matches!(server_thread.join(), Ok(_)));
}

/// Simply sends a message
fn send_example() {
    let mac_key_provider = MacKeyProviderType::new(vec![17, 18]);

    let (mut target_client, mut target_server) = setup_process(&mac_key_provider);

    println!("setup_process done");
    let target_thread = std::thread::spawn(move || {
        let received = target_server.recv_message().unwrap();
        println!(
            "Received message: {}",
            std::str::from_utf8(received.as_ref()).unwrap()
        );
    });

    target_client.send_msg(b"Hello World!".to_vec());
    assert!(matches!(target_thread.join(), Ok(_)));
}

fn main() {
    // invalid_hmac_example();
    send_example();
}
