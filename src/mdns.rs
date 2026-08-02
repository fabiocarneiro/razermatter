/*
 *
 *    Copyright (c) 2025-2026 Project CHIP Authors
 *
 *    Licensed under the Apache License, Version 2.0 (the "License");
 *    you may not use this file except in compliance with the License.
 *    You may obtain a copy of the License at
 *
 *        http://www.apache.org/licenses/LICENSE-2.0
 *
 *    Unless required by applicable law or agreed to in writing, software
 *    distributed under the License is distributed on an "AS IS" BASIS,
 *    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *    See the License for the specific language governing permissions and
 *    limitations under the License.
 */

//! A module containing the mDNS code used in the examples
#![allow(unexpected_cfgs)]

use rs_matter::Matter;
use rs_matter::{crypto::Crypto, error::Error};

use socket2::{Domain, Protocol, Socket, Type};

#[allow(unused)]
pub async fn run_mdns<C: Crypto + Copy>(matter: &Matter<'_>, crypto: C) -> Result<(), Error> {
    #[cfg(feature = "astro-dnssd")]
    rs_matter::transport::network::mdns::astro::AstroMdns::new()
        .run(matter)
        .await?;

    #[cfg(all(feature = "zeroconf", not(feature = "astro-dnssd")))]
    rs_matter::transport::network::mdns::zeroconf::ZeroconfMdns::new()
        .run(matter)
        .await?;

    #[cfg(all(
        feature = "resolve",
        not(any(feature = "zeroconf", feature = "astro-dnssd"))
    ))]
    rs_matter::transport::network::mdns::resolve::ResolveMdns::new(
        rs_matter::utils::zbus::Connection::system().await.unwrap(),
    )
    .run(matter)
    .await?;

    #[cfg(all(
        feature = "avahi",
        not(any(feature = "resolve", feature = "zeroconf", feature = "astro-dnssd"))
    ))]
    rs_matter::transport::network::mdns::avahi::AvahiMdns::new(
        rs_matter::utils::zbus::Connection::system().await.unwrap(),
    )
    .run(matter)
    .await?;

    #[cfg(not(any(
        feature = "avahi",
        feature = "resolve",
        feature = "zeroconf",
        feature = "astro-dnssd"
    )))]
    run_builtin_mdns(matter, crypto).await?;

    Ok(())
}

#[allow(unused)]
async fn run_builtin_mdns<C: Crypto + Copy>(matter: &Matter<'_>, crypto: C) -> Result<(), Error> {
    use std::net::UdpSocket;

    use log::{debug, error, info, warn};

    use rs_matter::transport::network::{Ipv4Addr, Ipv6Addr};
    use rs_matter::transport::network::mdns::builtin::{BuiltinMdns, Host};
    use rs_matter::transport::network::mdns::{
        MDNS_IPV4_BROADCAST_ADDR, MDNS_IPV6_BROADCAST_ADDR, MDNS_SOCKET_DEFAULT_BIND_ADDR,
    };
    use futures_lite::future::FutureExt;

    #[inline(never)]
    fn initialize_network() -> Result<(Ipv4Addr, Ipv6Addr, u32), Error> {
        use rs_matter::error::ErrorCode;

        let all = if_addrs::get_if_addrs().map_err(|_| ErrorCode::StdIoError)?;
        debug!("Available network interfaces: {:?}", all);

        let find_ipv6_candidate = |ipv6_filter: fn(std::net::Ipv6Addr) -> bool| {
            all.iter()
                .filter(|ia| !ia.is_loopback())
                .filter_map(|ia| match ia.addr {
                    if_addrs::IfAddr::V6(ref v6) if ipv6_filter(v6.ip) => {
                        Some((ia.name.clone(), v6.ip, ia.index.unwrap_or(0)))
                    }
                    _ => None,
                })
                .find_map(|(iname, ipv6, index)| {
                    all.iter()
                        .filter(|ia2| ia2.name == iname)
                        .find_map(|ia2| match ia2.addr {
                            if_addrs::IfAddr::V4(ref v4) => {
                                Some((iname.clone(), v4.ip, ipv6, index))
                            }
                            _ => None,
                        })
                })
        };

        let find_fallback_candidate = || {
            all.iter()
                .filter(|ia| !ia.is_loopback())
                .filter(|ia| ia.name.starts_with("eth") || ia.name.starts_with("eno") || ia.name.starts_with("wlan") || ia.name.starts_with("wlp"))
                .map(|ia| match ia.addr {
                    if_addrs::IfAddr::V4(ref v4) => (
                        ia.name.clone(),
                        v4.ip,
                        std::net::Ipv6Addr::UNSPECIFIED,
                        ia.index.unwrap_or(0),
                    ),
                    if_addrs::IfAddr::V6(ref v6) => (
                        ia.name.clone(),
                        std::net::Ipv4Addr::UNSPECIFIED,
                        v6.ip,
                        ia.index.unwrap_or(0),
                    ),
                })
                .next()
        };

        let candidate = find_ipv6_candidate(|ip| ip.is_unicast_link_local())
            .or_else(|| find_ipv6_candidate(|_| true))
            .or_else(|| {
                warn!("No network interface with a suitable IPv6 address found");
                find_fallback_candidate()
            })
            .ok_or_else(|| {
                error!("Cannot find network interface suitable for mDNS broadcasting");
                ErrorCode::StdIoError
            })?;

        let (iname, ip, ipv6, index) = candidate;

        debug!("Selected network interface {iname} with {ip}/{ipv6} for mDNS");

        Ok((ip.octets().into(), ipv6.octets().into(), index))
    }

    loop {
        let (ipv4_addr, ipv6_addr, interface) = match initialize_network() {
            Ok(res) => res,
            Err(e) => {
                warn!("mDNS network initialization failed: {:?}, retrying in 5 seconds...", e);
                async_io::Timer::after(std::time::Duration::from_secs(5)).await;
                continue;
            }
        };

        info!("Starting mDNS engine on interface index {interface} with IPs {ipv4_addr:?}/{ipv6_addr:?}");

        let create_socket = || -> Result<async_io::Async<UdpSocket>, Error> {
            use rs_matter::error::ErrorCode;
            let mut socket = Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::UDP))
                .map_err(|_| ErrorCode::StdIoError)?;
            socket.set_reuse_address(true).map_err(|_| ErrorCode::StdIoError)?;
            socket.set_only_v6(false).map_err(|_| ErrorCode::StdIoError)?;
            socket.bind(&MDNS_SOCKET_DEFAULT_BIND_ADDR.into()).map_err(|_| ErrorCode::StdIoError)?;
            let socket = async_io::Async::<UdpSocket>::new_nonblocking(socket.into())
                .map_err(|_| ErrorCode::StdIoError)?;

            let _ = socket.get_ref().join_multicast_v6(&MDNS_IPV6_BROADCAST_ADDR, interface);
            let _ = socket.get_ref().join_multicast_v4(&MDNS_IPV4_BROADCAST_ADDR, &ipv4_addr);
            Ok(socket)
        };

        let socket = match create_socket() {
            Ok(s) => s,
            Err(e) => {
                warn!("Failed to bind mDNS socket: {:?}, retrying in 5 seconds...", e);
                async_io::Timer::after(std::time::Duration::from_secs(5)).await;
                continue;
            }
        };

        let mut mdns = BuiltinMdns::new();
        let host = Host {
            hostname: "001122334455",
            ip: ipv4_addr,
            ipv6: ipv6_addr,
        };

        let mdns_runner = mdns.run(
            &socket,
            &socket,
            &host,
            Some(ipv4_addr),
            Some(interface),
            matter,
            crypto,
        );

        let network_watcher = async {
            loop {
                async_io::Timer::after(std::time::Duration::from_secs(15)).await;
                match initialize_network() {
                    Ok(current) => {
                        if current != (ipv4_addr, ipv6_addr, interface) {
                            info!(
                                "Network interface or IP changed ({:?} -> {:?}), re-binding mDNS...",
                                (ipv4_addr, ipv6_addr, interface), current
                            );
                            break;
                        }
                    }
                    Err(_) => {
                        warn!("Network interface lost, checking again in 5s...");
                    }
                }
            }
            Ok(())
        };

        let _ = mdns_runner.or(network_watcher).await;
        info!("Network state changed or mDNS task ended, refreshing mDNS socket...");
        async_io::Timer::after(std::time::Duration::from_secs(1)).await;
    }
}
