# Overview

- **can-viewer** is a tool support showing can packets based on DBC input file using Rust + Slint.
- **can-viewer** is able to see real-time data on CAN bus and set a list of filter CAN IDs.
- **can-viewer** is a open-source project and willing to receive any contribution from community.

![image](https://github.com/TuEmb/can-viewer/assets/126753419/617dfe3a-f2d9-41f3-b8c3-91db4734593d)

The column format:
```
<CAN ID> <signal name> <signal value> <unit> <factor>
```

# Setup
## Linux
Currently, **can-viewer** is using socket can of system to read can packet. Refer https://cantact.io/socketcan/socketcan.html understand and install socket can for Linux environment.
- You must make sure socket can name "can0" is on your system (use `ifconfig` command to check).
- You must make sure socket can is able to read CAN packets from CAN bus (use `candump can0` command to check).
- Build the app by command:
```
cargo build
```
- Run the app by command:
```
cargo run
```
## Window (not support)
## IOS (not support)
