
The end-design of this system is to facilitate the operation of a single computational system across a variety of computing devices. The system-level capabilities should be extensible during system operation, allowing for the loading and registration of new capabilities on one device to be accessible and used on any other device within the network.

# System Architecture Design

Due primarily to the necessity of operating across many, heterogeneous computing devices, the system is architected using a distributed network model. The network will be built up out of a collection of nodes, apps, and overseers - combining to produce emergent behavior tailored to the users needs.

"Nodes" are any devices that are added into the network. Nodes must maintain a global "device manager", responsible for marshaling communication between the device and the network and for monitoring and otherwise handling the device's state. Nodes may also co-locate a collection of apps, or "modalities", to provide computational and interactives resources into the broader network.

"Apps" are the general term for any configurable program that runs within the system. Apps are responsible for introducing user interaction requests into the network and, when not covered by hardcoded procedures, responsible for fulfilling those requests.

Finally, the network will maintain a dynamic system of specific "overseers" in order to monitor and manage capabilities, load, and other requirements across the system to ensure peak performance and usability.

# Node Architecture

The primary piece of an individual node is the device-manager as all communications with the broader network **must** pass through this piece at some point. Additionally, any broadly required systems capabilities, such as file-system indexing, are implemented within the device-manager.

The device manager is responsible for 3 broad workflows: Routing app messages into the wider network, dispatching network requests to device apps, and maintaining the system "registry".

Nodes additionally server as a point for "co-locating" apps for high level user-network interactions.

TODO

To ensure speed, robustness, and throughput, the device manager is implemented using Rust.

# App Architecture

TODO

The requirements for an app are rather simple: It must communicate using the standard networking protocol. This communication must be performed on a tcp port connecting to the device-manager on the localhost.

To simplify development, python libraries are provided to automate all of the network specific setup, requiring only the implementation of the `Plugin` interface. Additional callbacks may be registered to enable to plugin to respond to network events and requests. Finally, the python framework also implements the easy ability to specify a "command line" to parse configuration values (TODO: How is configuration data passed on to the apps).

### Networking Protocol

To simplify development (as I do not expect this to go much further), messages are passed using a json format. At the socket level, messages are framed using a simple length-delimited protocol.

For more information about what goes into a message, see `messages.md`.
