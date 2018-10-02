
This document describes the formatting of messages and other system data to improve system routing.

## Messages

All messages sent within the system must contain a "networking section". This section contains all
information necessary to help route a message within the network. Some data within this section is
only produced as the message travels from the producer to the destination.

  ```json
  "sender": {
      "addr": "41.249.102.177",
      "role": "app",
      "uuid": "84707676-d2c3-50ce-8c1e-f8fe62ffda3c"
  },
  "route": [ "41.249.102.177", "20.85.205.72" ],
  "dest": {
      "role": "<app>",
      "addr": "49.227.57.56",
      "uuid": "4b1d37cd-d6c6-508b-9935-86ed62f1095b",
      "intra-device": true
  },
  "forward": null,
  "message_id": "5e17c49d-9332-587a-98d0-3a16ff21c3fb",
  "parent_id": null,
  "ack_uuid": "084c32bc-8f7a-54c9-ae26-41c3c4c4bae4"
  ```

Every message **must** be assigned a guid at creation in the 'message_id' field.

Any responses to a message *A*, **must** assign the guid of *A* to the 'parent_id' field. NOTE: This
will help plugins/apps to perform some degree of event-oriented programming when querying
other system apps/resources (among other uses)

Every message **must** produce a json dictionary in the 'sender' field identifying the originating
plugin and device. This dictionary **must** consist of at least the following fields once it leaves the
device-manager: 'addr', 'role', and 'uuid'. NOTE: Plugins are not required to fill in all data where
unable, particularly with the 'addr' field

> The 'addr' field **must** contain the device's ip address

> The 'role' field **must** contain the plugin "role" data corresponding to the production of the message (???)

> The 'uuid' field **must** contain the guid of the specific producing plugin/manager combo

If an ACK message is desired (such as when a message from *A* triggers actions in another app *B*), the
'ack_uuid' field **may** be set to the value of 'sender.uuid'.

When a manager receives a message such that the 'ack_uuid' field of the message isn't *null* and the
destination app cannot be the same as the 'ack_uuid' app, then it **must** split the message and forward
it as follows:

> An ACK message **must** be created and sent to the device specified by 'ack_uuid' value

> The 'ack_uuid' field of the original message **must** be set to *null* or removed and the message forwarded as normal

Since the system is built along a distributed model, we take no guarantees that the entire routing structure
will be knowable to any specific node within the program. As such, at every routing step (ie. when a message
is received by a manager), the receiving device's ip address **should** be appended to the 'route' field.
NOTE: This is not strictly necessary for routing, but it may be very helpful for network maintenance/etc.

All messages **must** specify a json dictionary in the 'dest' data field, indicating where/how the message
should be routed. NOTE: In general, this dictionary acts as more of a routing "hint" than any explicit
requirement - depending on which fields/field combinations are specified. This allows us to satisfy any
of three general requirements:

> Find me the system default app for this **ROLE**

> Use the app for this **ROLE** on this **DEVICE**

> Send a message to this **APP**

The routing system is **not required** to follow this general formula for all actions and **may** short-circuit
routing where possible/desired/necessary

NOTE: I may find it beneficial to introduce the ability to "forward" messages, in order to reduce network traffic.
These kinds of messages would enable a plugin to send a message that relies on data the sender does not have access
to. The specific story here is to handle a "find" request for a file: the original message travels to the dispatch
plugin in order to be deciphered, but 'dispatch' does not have access to the network's file system. With event-driven
programming, it is possible to send a request to the 'fs' app and then to incorporate the results into the response.
However, with "forwarding" we can instead send the response to the 'fs' app which will append it's results into the
response, replace any message routing data/etc. with the values specified in the 'forward' field, and then send
the message to it's original source (a reduction of 1 connection).
