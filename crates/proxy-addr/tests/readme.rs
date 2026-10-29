//! Runs the README example, so the documentation cannot drift from the API.

#[test]
fn readme_examples() {
    use proxy_addr::{Trust, addr, all};
    use std::net::IpAddr;

    let peer: IpAddr = "10.0.0.5".parse().unwrap();

    let header = "1.2.3.4, 198.51.100.7";
    let ingress = Trust::parse("10.0.0.0/8").unwrap();
    assert_eq!(addr(peer, Some(header), &ingress), "198.51.100.7");

    assert_eq!(
        all(peer, Some(header), Some(&ingress)),
        ["10.0.0.5", "198.51.100.7"]
    );

    let direct: IpAddr = "203.0.113.9".parse().unwrap();
    assert_eq!(addr(direct, Some(header), &ingress), "203.0.113.9");
}
