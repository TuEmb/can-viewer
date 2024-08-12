# Overview

- **can-viewer** is a tool support showing can packets based on DBC input file using Rust + Slint.
- **can-viewer** is able to see real-time data on CAN bus and set a list of filter CAN IDs.
- **can-viewer** is an open-source project and willing to receive any contributions from community.

![image](https://github.com/user-attachments/assets/963712eb-a0ba-471d-9d14-9b3f483e0cbf)


The column format:
```
<CAN ID> <signal name> <signal value + unit> <raw data>
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
## Window
**can-viewer** is supporting Peak CAN for window only
- Build the app by command:
```
cargo build --release
```
- Run the app by command:
```
cargo run --release
```
## IOS (not support)
