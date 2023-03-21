# qqproxy

## Features

* Async
* Single executable
* Linux/Windows/Mac/BSD support
* Support reverse mode(Not bind any port in client)

## Build & Run

`$> cargo build --release --target=i686-pc-windows-msvc`

## Installation

`$> cargo install rsocx`


## Socks5 Protocol Support

- [x] IPV6 Support
- [ ] `SOCKS5` Authentication Methods
  - [x] `NOAUTH` 
  - [ ] `USERPASS`
- [ ] `SOCKS5` Commands
  - [x] `CONNECT`
  - [ ] `BIND`
  - [ ] `ASSOCIATE` 
