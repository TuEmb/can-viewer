# Overview

- **can-viewer** is a tool support showing can packets based on DBC input file using Rust + Slint.
- **can-viewer** is able to see real-time data on CAN bus and set a list of filter CAN IDs.
- **can-viewer** is a open-source project and willing to receive any contribution from community.

![image](https://github.com/TuEmb/can-viewer/assets/126753419/cb3469f8-a50f-48de-abed-46ce966b16c1)


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
cargo build --release
```
- Run the app by command:
```
cargo run --release
```
## Window (not support)
## IOS (not support)
