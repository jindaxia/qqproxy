mod utils;
mod socks;

use log::LevelFilter;
use net2::TcpStreamExt;
use simple_logger::SimpleLogger;
use tokio::{io::{self}, net::{TcpListener, TcpStream}};

#[tokio::main]
async fn main() -> io::Result<()>  {
	SimpleLogger::new().with_utc_timestamps().with_utc_timestamps().with_colors(true).init().unwrap();
	::log::set_max_level(LevelFilter::Info);

	let local_address : String = "0.0.0.0:41088".to_string();
	log::info!("listen to : {}" ,local_address);
	
	let listener = match TcpListener::bind(&local_address).await{
		Err(e) => {
			log::error!("error : {}", e);
			return Ok(());
		},
		Ok(p) => p
	};

	loop{
		let (stream , addr) = listener.accept().await.unwrap();
		log::info!("accept from : {}" ,addr);
		let raw_stream = stream.into_std().unwrap();
		raw_stream.set_keepalive(Some(std::time::Duration::from_secs(10))).unwrap();
		let stream = TcpStream::from_std(raw_stream).unwrap();

		tokio::spawn(async {
			socks::socksv5_handle(stream).await;
		});
	}
}
