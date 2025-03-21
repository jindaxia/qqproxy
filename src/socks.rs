use std::{ net::{Ipv6Addr, SocketAddrV6}};
use std::io::*;
use std::time::Duration;
use net2::TcpStreamExt;
use tokio::{net::TcpStream, io::{AsyncWriteExt, AsyncReadExt}, time::{timeout}};

use crate::utils::makeword;

#[derive(Debug, Clone)]
pub enum Addr {
	V4([u8; 4]),
	V6([u8; 16]),
	Domain(Box<[u8]>)
}

fn format_ip_addr(addr :& Addr) -> Result<String> {
	match addr {
		Addr::V4(addr) => {
			Ok(format!("{}.{}.{}.{}" , addr[0], addr[1] ,addr[2], addr[3]))
		},
		Addr::V6(addr) => {
			Ok(format!("{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}" , addr[0], addr[1] ,addr[2], addr[3], addr[4], addr[5] ,addr[6], addr[7] , addr[8], addr[9] ,addr[10], addr[11], addr[12], addr[13] ,addr[14], addr[15]))
		},
		Addr::Domain(addr) => match String::from_utf8(addr.to_vec()) {
			Ok(p) => Ok(p) ,
			Err(e) => {
				log::error!("parse domain faild. {}" , e);
				Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid domain"))
			},
		}
	}
}

async fn tcp_transfer(stream : &mut TcpStream , addr : &Addr, address : &String , port :u16 ){
	log::info!("proxy connect to {}" , address);
	let time_out = Duration::from_millis(50);
	let client: std::result::Result<TcpStream, Error>  = match addr{
		Addr::V4(_) => {
			timeout(
				time_out,
				async move { TcpStream::connect(address.clone()).await},
			).await.expect("connect timeout")
		},
		Addr::V6(x) => {
			let ipv6 = Ipv6Addr::new(
				makeword(x[0] , x[1]) , 
				makeword(x[2] , x[3]) , 
				makeword(x[4] , x[5]) , 
				makeword(x[6] , x[7])  , 
				makeword(x[8] , x[9]) , 
				makeword(x[10] , x[11]) , 
				makeword(x[12] , x[13]) , 
				makeword(x[14] , x[15])
			);
			let v6sock = SocketAddrV6::new(ipv6 , port , 0 , 0 );
			timeout(
				time_out,
				async move { TcpStream::connect(v6sock).await},
			).await.expect("connect timeout")
		},
		Addr::Domain(_) => {
			timeout(
				time_out,
				async move { TcpStream::connect(address.clone()).await},
			).await.expect("connect timeout")
		}
	};

	let client = match client {
		Err(_) => {
			log::warn!("connect[{}] faild" , address);
			return;
		},
		Ok(p) => p
	};

	let raw_stream = client.into_std().unwrap();
	raw_stream.set_keepalive(Some(std::time::Duration::from_secs(10))).unwrap();
	let mut client = TcpStream::from_std(raw_stream).unwrap();

	let remote_port = client.local_addr().unwrap().port();

	let mut reply = Vec::with_capacity(22);
	reply.extend_from_slice(&[5, 0, 0]);

	match addr {
		Addr::V4(x) => {
			reply.push(1);
			reply.extend_from_slice(x);
		},
		Addr::V6(x) => {
			reply.push(4);
			reply.extend_from_slice(x);
		},
		Addr::Domain(x) => {
			reply.push(3);
			reply.push(x.len() as u8);
			reply.extend_from_slice(x);
		}
	}

	reply.push((remote_port >> 8) as u8);
	reply.push(remote_port as u8);

	if let Err(e) = stream.write_all(&reply).await{
		log::error!("error : {}" , e);
		return;
	};

	let mut buf1 = [0u8 ; 1024];
	let mut buf2 = [0u8 ; 1024];
	loop{
		tokio::select! {
			a = client.read(&mut buf1) => {

				let len = match a {
					Err(_) => {
						break;
					}
					Ok(p) => p
				};
				match stream.write_all(&buf1[..len]).await {
					Err(_) => {
						break;
					}
					Ok(p) => p
				};

				if len == 0 {
					break;
				}
			},
			b = stream.read(&mut buf2) =>  { 
				let len = match b{
					Err(_) => {
						break;
					}
					Ok(p) => p
				};
				match client.write_all(&buf2[..len]).await {
					Err(_) => {
						break;
					}
					Ok(p) => p
				};
				if len == 0 {
					break;
				}
			},
		}
	}
}

pub async fn socksv5_handle(mut stream: TcpStream) {
	const AUTH_USERNAME: &str = "synb123";
	const AUTH_PASSWORD: &str = "qqNBNo.1";

	let mut header = [0u8 ; 2];
    stream.read_exact(&mut header).await.expect("error reading header");
	
	if header[0] != 5 {
		log::error!("not support protocol version {}", header[0]);
		return;
	}
	
	let mut methods = vec![0u8; header[1] as usize].into_boxed_slice();
	stream.read_exact(&mut methods).await.expect("error reading methods");
	
	if methods.contains(&2u8){
		// authentication response
		stream.write_all(&[5, 2]).await.expect("error writing authentication response");

		let mut auth_request = [0u8; 2];
		stream.read_exact(&mut auth_request).await.expect("error reading auth request");

		if auth_request[0] != 1 {
			log::error!("not support auth method: {}", auth_request[0]);
			return;
		}
		let username_len = auth_request[1] as usize;
		if username_len != AUTH_USERNAME.len() {
			log::error!("wrong username length: {}", username_len);
			return;
		}

		let mut username = vec![0u8; username_len].into_boxed_slice();
		stream.read_exact(&mut username).await.expect("error reading username");
		
		let mut password_len = [0u8; 1];
		stream.read_exact(&mut password_len).await.expect("error reading password length");
		if password_len[0] as usize != AUTH_PASSWORD.len() {
			log::error!("wrong password length: {}", password_len[0]);
			return;
		}
		let mut password = vec![0u8; password_len[0] as usize].into_boxed_slice();
		stream.read_exact(&mut password).await.expect("error reading password");

		if String::from_utf8_lossy(&username) != AUTH_USERNAME || String::from_utf8_lossy(&password) != AUTH_PASSWORD {
			// auth failed
			stream.write_all(&[1, 1]).await.expect("error writing auth failure message");
			log::error!("auth failed");
			return;
		}
		// auth succeeded
		stream.write_all(&[1, 0]).await.expect("error writing authentication result");
	}else{
		stream.write_all(&[5, 0]).await.expect("error writing authentication result");
	};

	let mut request =  [0u8; 4];
	stream.read_exact(&mut request).await.expect("error reading");

	if request[0] != 5 {
		log::error!("say again not support version: {}" , request[0]);
		return;
	}

	let cmd = request[1];

	if cmd != 1 {
		log::error!("not support cmd: {}" , cmd);
		return;
	}

	let addr = match request[3] {
		0x01 => {
			let mut ipv4 =  [0u8; 4];
			stream.read_exact(&mut ipv4).await.expect("error reading ipv4 address");
			Addr::V4(ipv4)
		},
		0x04 => {
			let mut ipv6 =  [0u8; 16];
			stream.read_exact(&mut ipv6).await.expect("error reading ipv6 address");
			Addr::V6(ipv6)
		},
		0x03 => {
			let mut domain_size =  [0u8; 1];
			stream.read_exact(&mut domain_size).await.expect("error reading domain size");
			let mut domain =  vec![0u8; domain_size[0] as usize].into_boxed_slice();
			stream.read_exact(&mut domain).await.expect("error reading domain");

			Addr::Domain(domain)
		},
		_ => {
			log::error!("unknow atyp {}" , request[3]);
			return;
		}
	};

	let mut port = [0u8 ; 2];
	stream.read_exact(&mut port).await.expect("error reading port");

	let port = (port[0] as u16) << 8 | port[1] as u16;
	let address_prefix = match format_ip_addr(&addr){
		Err(_) => {
			return;
		}
		Ok(p) => p
	};
	let address = format!("{}:{}" , address_prefix , port);

	if cmd == 1 {
		tcp_transfer(&mut stream , &addr , &address , port).await;
	}
	
	log::info!("connection [{}] finished" , address);

}